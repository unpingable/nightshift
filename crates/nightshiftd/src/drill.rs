//! D0d-a — deterministic drill runner. Cross-repo bridge to AG.
//!
//! Night Shift owns the `nightshift watchbill run wal-bloat-review --drill
//! --scenario=all-green` entry point (per agent_gov campaign §3b actuation
//! pin: "ONE entry point. Nightshift owns it"). The actuator drives AG's
//! cooked-context orchestrator end-to-end against an NQ-shaped
//! FindingSnapshot carrying `origin_mode=drill`, then renders a
//! deterministic transcript of the chain.
//!
//! The transcript is a **receipt render of the ledger** — not LLM
//! narration. Every byte derives from inputs (per §3b: "narrative
//! laundering at the presentation layer would undo every gate beneath
//! it, invisibly"). The deterministic-stub proposal packet cites
//! receipt ids without invoking a model. No LLM is called in this
//! slice; the all-green proposal packet is a bounded fixed template.
//!
//! ## D0-Origin upgrade (2026-06-10)
//!
//! D0d-a shipped with an AG-constructed deterministic FindingSnapshot
//! fixture — the four-link chain ran, but the NQ side of the bridge was
//! a Python dict, not a genuine NQ-produced wire DTO. D0-Origin replaces
//! that with the real article:
//!
//!   1. `wal_bloat_stager::stage_wal_bloat` builds a sandbox SQLite DB
//!      with a genuinely bloated WAL (wal_pct > 5%). The condition is
//!      operator-staged; the smoke is real.
//!   2. `nq-monitor drill wal-bloat --sandbox-db <staged> --origin-mode
//!      drill` invokes NQ's production evaluator pipeline against the
//!      sandbox. The same `sqlite_health::collect`, `publish_batch`,
//!      `detect::run_all`, and `update_warning_state_with_origin_mode`
//!      code paths the `serve` loop runs every minute. NQ emits
//!      `FindingSnapshot` JSON stamped `origin_mode=drill` via the
//!      native detector path (migration 057 + new symmetric plumbing).
//!   3. The captured JSON is written to a temp file. Night Shift then
//!      shells `python3 -m governor.drill_runner --finding-json <path>
//!      --root <receipts>` so AG consumes the genuine FindingSnapshot,
//!      not the deterministic fixture.
//!
//! The bridge from operator-staged condition to provenance-correct
//! testimony to refusal-bearing receipt chain is now end-to-end real.
//! The fixture path is preserved as an internal default for tests that
//! exercise the runner outside the cross-repo harness.
//!
//! Cross-repo wiring shape:
//!
//!   * Night Shift shells into AG via `python3 -m governor.drill_runner
//!     --finding-json <path> --root=<tmp>`. AG is Python; Night Shift is
//!     Rust. The subprocess boundary is the only allowed AG-side
//!     surface (one entry point invariant).
//!   * `governor` (CLI) lives on PATH — the same binary the rest of
//!     AG's surface uses. Night Shift then shells `governor why
//!     <leaf-receipt-id> --json` to verify the chain walk's DRILL
//!     prefix rendering.
//!   * Receipt root is operator-supplied; defaults to a tmp dir under
//!     `$TMPDIR/nightshift-drill-<scenario>`.
//!
//! D0d-a happy-path emission gap (closed by D0d-b):
//!
//!   AG's standing/LA clients now emit GateReceipts on the happy path
//!   per D0d-b. The all-green chain produces FOUR GateReceipts in emit
//!   order (standing_seam, wicket_seam, la_seam grant, la_seam consume),
//!   so the transcript walks four real links via `governor why`.

use crate::wal_bloat_stager::{stage_wal_bloat, StagedSandbox, StagerError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Closed scenario vocabulary. D0d-a shipped with a single scenario
/// (`all-green`). D0d-1 widens to the six-scenario gauntlet ratified
/// at agent_gov campaign §3 D2.
pub const SCENARIO_ALL_GREEN: &str = "all-green";
pub const SCENARIO_NO_STANDING: &str = "no-standing";
pub const SCENARIO_STANDING_EXPIRED: &str = "standing-expired";
pub const SCENARIO_WICKET_DENIED: &str = "wicket-denied";
pub const SCENARIO_WICKET_GAP_ACCOUNTED: &str = "wicket-gap-accounted";
pub const SCENARIO_REPLAY_BUDGET: &str = "replay-budget";

/// Operator-ratified alias for `replay-budget`. Resolved at the CLI gate;
/// AG's `drill_runner.py` also accepts the alias (single normalization
/// site on each side; the wire passes the alias through verbatim and AG
/// canonicalizes).
pub const SCENARIO_ALIAS_ALREADY_CONSUMED: &str = "already-consumed";

/// Closed six-scenario set. CLI refuses any name not in this set (plus
/// the accepted alias).
pub const SUPPORTED_SCENARIOS: &[&str] = &[
    SCENARIO_ALL_GREEN,
    SCENARIO_NO_STANDING,
    SCENARIO_STANDING_EXPIRED,
    SCENARIO_WICKET_DENIED,
    SCENARIO_WICKET_GAP_ACCOUNTED,
    SCENARIO_REPLAY_BUDGET,
];

/// Returns true iff `scenario` is in the closed set OR the accepted
/// alias. Single membership check site — kept narrow so future
/// vocabulary changes touch exactly one function.
pub fn is_supported_scenario(scenario: &str) -> bool {
    if scenario == SCENARIO_ALIAS_ALREADY_CONSUMED {
        return true;
    }
    SUPPORTED_SCENARIOS.contains(&scenario)
}

/// Closed NQ-side origin_mode vocabulary mirror (per AG D0-Bridge).
/// Used here only for the drill scenario; other origins are out of
/// scope for D0d-a.
pub const ORIGIN_MODE_DRILL: &str = "drill";

/// D3 — closed confabulation-role vocabulary, mirrors AG's
/// ``governor.drill_runner.CONFABULATION_ROLES``. Two roles:
///
///   * ``standing`` — inject a bogus standing_receipt_id into the
///     proposal-packet citation step (existence-fail target). The
///     validator detects the lookup miss and emits a
///     ``dangling_receipt_reference`` refusal receipt.
///   * ``evidence`` — cite the real LA grant receipt id in the
///     standing slot (kind-fit-fail target). The validator detects
///     the structural mismatch and emits the same refusal kind with
///     a distinct ``citation_check`` evidence-bundle tag.
///
/// Both modes require ``--scenario=all-green`` (only scenario reaching
/// the proposal-packet step); AG raises ``InvalidConfabulationRoleError``
/// at construction time otherwise. The flag is plumbed verbatim across
/// the subprocess boundary.
pub const CONFAB_ROLE_STANDING: &str = "standing";
pub const CONFAB_ROLE_EVIDENCE: &str = "evidence";
pub const CONFABULATION_ROLES: &[&str] = &[CONFAB_ROLE_STANDING, CONFAB_ROLE_EVIDENCE];

/// Returns true iff ``role`` is in the closed set.
pub fn is_supported_confab_role(role: &str) -> bool {
    CONFABULATION_ROLES.contains(&role)
}

/// Name of the NQ binary that exposes the `drill wal-bloat` subcommand
/// (per D0-Origin). Resolves via `NIGHTSHIFT_NQ_BIN` env var; defaults
/// to `nq-monitor` on PATH. Override is offered for sandbox testing.
pub const DEFAULT_NQ_BIN: &str = "nq-monitor";

/// Result of running the all-green drill subprocess. Holds the
/// deterministic JSON envelope verbatim so the operator can inspect
/// any field; the transcript is the human-readable receipt render.
#[derive(Debug, Clone)]
pub struct DrillRunResult {
    /// Scenario name. Always `all-green` in D0d-a.
    pub scenario: String,
    /// Receipt root directory used by the drill. Receipts persist
    /// here after the run; operators can inspect via
    /// `governor receipts --json`.
    pub receipt_root: PathBuf,
    /// The deterministic transcript — `nightshift`-flavored receipt
    /// render. Two invocations with identical inputs produce
    /// byte-identical transcripts. See `assert_deterministic` test.
    pub transcript: String,
}

/// Errors the drill can surface. Closed set, no ambient `anyhow`-shaped
/// shapes — the actuator's failure modes are bounded.
#[derive(Debug, thiserror::Error)]
pub enum DrillError {
    #[error(
        "unsupported drill scenario {scenario:?}; D0d-1 admits only \
         {{'all-green', 'no-standing', 'standing-expired', \
         'wicket-denied', 'wicket-gap-accounted', 'replay-budget'}} \
         (accepted alias: 'already-consumed' → 'replay-budget')"
    )]
    UnsupportedScenario { scenario: String },

    #[error(
        "unsupported confabulate-citation role {role:?}; D3 admits only \
         {{'standing', 'evidence'}}"
    )]
    UnsupportedConfabRole { role: String },

    #[error(
        "drill_runner subprocess failed (exit code {code:?}): {stderr}"
    )]
    SubprocessFailed { code: Option<i32>, stderr: String },

    #[error("drill_runner subprocess produced invalid JSON: {0}")]
    InvalidJsonEnvelope(#[from] serde_json::Error),

    #[error("io error invoking drill_runner: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAL-bloat staging failed: {0}")]
    Stager(#[from] StagerError),

    #[error(
        "nq-monitor drill wal-bloat refused or failed (exit code \
         {code:?}); stderr={stderr}"
    )]
    NqEvaluatorFailed { code: Option<i32>, stderr: String },

    #[error(
        "nq-monitor drill wal-bloat emitted no FindingSnapshot JSON \
         despite reporting success. This is a contract violation in the \
         NQ-side bridge."
    )]
    NqEvaluatorEmittedNoFinding,

    #[error(
        "nq-monitor drill wal-bloat emitted finding with origin_mode \
         {actual:?}, expected {expected:?}. The native detector path's \
         origin_mode plumbing is broken."
    )]
    NqOriginModeMismatch { actual: String, expected: String },
}

/// Configuration for one drill invocation. Construction-time bounded:
/// invalid scenarios are refused before the subprocess is launched.
#[derive(Debug, Clone)]
pub struct DrillConfig {
    pub scenario: String,
    pub receipt_root: PathBuf,
    /// Override the Python interpreter used to invoke the AG-side
    /// drill runner. Resolves via `NIGHTSHIFT_PYTHON` env var if not
    /// set; defaults to `python3` on PATH.
    pub python_bin: Option<PathBuf>,
    /// Override the NQ binary path. Resolves via `NIGHTSHIFT_NQ_BIN`
    /// env var if not set; defaults to `nq-monitor` on PATH.
    pub nq_bin: Option<PathBuf>,
    /// Sandbox dir for the WAL-bloat staging. Defaults to a unique
    /// tmp dir under `$TMPDIR/nightshift-drill-sandbox-<pid>`.
    pub sandbox_dir: Option<PathBuf>,
    /// D0-Origin mode flag. When `true` (default), Night Shift stages
    /// a real WAL-bloat sandbox and invokes NQ to produce a genuine
    /// FindingSnapshot before driving AG. When `false`, AG uses its
    /// internal deterministic fixture (the D0d-a path). The fixture
    /// path is preserved for tests that exercise the runner outside
    /// the cross-repo harness.
    pub origin_real_nq: bool,

    /// D3 — confabulate-citation role. When set, the AG-side validator
    /// runs after the chain reaches Consumed and injects a bogus
    /// citation in the cited slot. AG raises
    /// ``InvalidConfabulationRoleError`` if the role is unknown or if
    /// the scenario is not ``all-green``; Night Shift refuses ahead of
    /// the subprocess so the failure surfaces with a Rust-side enum
    /// variant rather than a Python traceback. ``None`` for non-D3 runs
    /// (the default).
    pub confabulate_citation: Option<String>,
}

impl DrillConfig {
    pub fn new(scenario: impl Into<String>, receipt_root: PathBuf) -> Result<Self, DrillError> {
        let scenario = scenario.into();
        if !is_supported_scenario(&scenario) {
            return Err(DrillError::UnsupportedScenario { scenario });
        }
        Ok(Self {
            scenario,
            receipt_root,
            python_bin: None,
            nq_bin: None,
            sandbox_dir: None,
            // Default: real NQ. D0-Origin landed; the cross-repo bridge
            // is the canonical path. Tests that need the deterministic
            // fixture flip this off.
            origin_real_nq: true,
            confabulate_citation: None,
        })
    }

    /// D3 — attach a confabulate-citation role. Refuses at construction
    /// time on unknown roles. Pairing the flag with a non-all-green
    /// scenario surfaces as ``DrillError::UnsupportedConfabRole`` at
    /// ``run_drill_command``-time rather than here (this method only
    /// validates the role vocabulary; the scenario-compatibility check
    /// is downstream so error attribution stays unambiguous).
    pub fn with_confabulate_citation(mut self, role: impl Into<String>) -> Result<Self, DrillError> {
        let role = role.into();
        if !is_supported_confab_role(&role) {
            return Err(DrillError::UnsupportedConfabRole { role });
        }
        self.confabulate_citation = Some(role);
        Ok(self)
    }

    fn python_bin_resolved(&self) -> PathBuf {
        if let Some(p) = &self.python_bin {
            return p.clone();
        }
        if let Ok(env_bin) = std::env::var("NIGHTSHIFT_PYTHON") {
            return PathBuf::from(env_bin);
        }
        PathBuf::from("python3")
    }

    fn nq_bin_resolved(&self) -> PathBuf {
        if let Some(p) = &self.nq_bin {
            return p.clone();
        }
        if let Ok(env_bin) = std::env::var("NIGHTSHIFT_NQ_BIN") {
            return PathBuf::from(env_bin);
        }
        PathBuf::from(DEFAULT_NQ_BIN)
    }

    fn sandbox_dir_resolved(&self) -> PathBuf {
        if let Some(p) = &self.sandbox_dir {
            return p.clone();
        }
        let base = std::env::var_os("TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        base.join(format!(
            "nightshift-drill-sandbox-{}-{}",
            self.scenario,
            std::process::id()
        ))
    }
}

/// Stage WAL bloat + invoke NQ's production evaluator + capture the
/// genuine FindingSnapshot JSON. Returns the captured JSON bytes plus
/// the staged sandbox metadata (for transcript reporting). This is the
/// D0-Origin spine on the Night Shift side; it is the only path by
/// which a real NQ-produced FindingSnapshot reaches AG.
pub fn capture_genuine_nq_finding(
    config: &DrillConfig,
) -> Result<(StagedSandbox, Vec<u8>), DrillError> {
    let sandbox_dir = config.sandbox_dir_resolved();
    let staged = stage_wal_bloat(&sandbox_dir)?;

    // Invoke NQ. The nq-monitor binary emits the FindingSnapshot JSON
    // (one snapshot, since the sandbox has a single DB) on stdout. We
    // route migration noise to inherit so the operator sees it on
    // their tty if running interactively, but we capture stdout for
    // parsing.
    let nq_bin = config.nq_bin_resolved();
    // Fresh NQ db every invocation so re-runs don't observe stale
    // generations from a prior staging. The NQ side is a transient
    // observation store for the drill — production deployments use
    // serve loops to maintain state, not one-shot drills.
    let nq_db = sandbox_dir.join("nq.db");
    if nq_db.exists() {
        std::fs::remove_file(&nq_db)?;
    }
    let output = Command::new(&nq_bin)
        .arg("drill")
        .arg("wal-bloat")
        .arg("--sandbox-db")
        .arg(&staged.db_path)
        .arg("--db")
        .arg(&nq_db)
        .arg("--origin-mode")
        .arg(ORIGIN_MODE_DRILL)
        .arg("--format")
        .arg("json")
        // The NQ-side migration logger defaults to stdout — suppress
        // anything below "warn" so the JSON snapshot is the only thing
        // on stdout. The drill subcommand emits no info-level logs in
        // its own code path.
        .env("RUST_LOG", "warn")
        .output()?;

    if !output.status.success() {
        return Err(DrillError::NqEvaluatorFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let json_bytes = output.stdout;
    if json_bytes.is_empty() {
        return Err(DrillError::NqEvaluatorEmittedNoFinding);
    }

    // Parse to verify origin_mode integrity at the boundary. If a
    // future NQ regression dropped or misset the discriminator, we'd
    // rather fail here than launder a misprovenanced finding
    // downstream.
    let finding: serde_json::Value = serde_json::from_slice(&json_bytes)?;
    // `--format json` produces a pretty-printed array; pick the first
    // snapshot (the staging produces exactly one).
    let snapshot = match &finding {
        serde_json::Value::Array(arr) if !arr.is_empty() => &arr[0],
        serde_json::Value::Object(_) => &finding,
        _ => return Err(DrillError::NqEvaluatorEmittedNoFinding),
    };
    let origin_mode = snapshot
        .get("origin_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if origin_mode != ORIGIN_MODE_DRILL {
        return Err(DrillError::NqOriginModeMismatch {
            actual: origin_mode.to_string(),
            expected: ORIGIN_MODE_DRILL.to_string(),
        });
    }

    // The bytes we hand to AG should be the SINGLE snapshot as a JSON
    // object, not the array, because AG's drill_runner consumes one
    // FindingSnapshot per invocation. Re-serialize the chosen snapshot
    // so the wire shape across the AG boundary is unambiguous.
    let canonical = serde_json::to_vec(snapshot)?;
    Ok((staged, canonical))
}

/// Run the drill end-to-end and return the rendered transcript.
///
/// Shells into AG's `python3 -m governor.drill_runner` with the
/// scenario + receipt root. The AG side runs the full standing →
/// wicket → LA chain through the cooked-context orchestrator, emits
/// the four chain GateReceipts (D0d-b: standing/wicket/grant/consume),
/// and returns a deterministic JSON envelope including the transcript.
///
/// **D0-Origin path (default):** when `config.origin_real_nq` is true,
/// Night Shift first stages WAL bloat and invokes NQ to produce a
/// genuine `FindingSnapshot` JSON. That snapshot is written to a temp
/// file and passed to AG via `--finding-json <path>`. AG consumes the
/// real wire DTO instead of its internal deterministic fixture.
///
/// **The transcript is deterministic** modulo timestamps/paths/ids,
/// which AG normalizes at render time. Receipt ids are content-
/// addressed; two runs against the same staged condition produce
/// byte-identical normalized transcripts.
pub fn run_drill(config: &DrillConfig) -> Result<DrillRunResult, DrillError> {
    // Ensure receipt root exists. AG side also does this; we do it
    // here so the subprocess doesn't have to create dirs under
    // arbitrary paths the operator may not own.
    std::fs::create_dir_all(&config.receipt_root)?;

    let python_bin = config.python_bin_resolved();

    // D0-Origin path: stage WAL bloat, capture genuine NQ FindingSnapshot.
    let staged_metadata = if config.origin_real_nq {
        let (staged, json_bytes) = capture_genuine_nq_finding(config)?;
        let finding_path = config.receipt_root.join("nq_finding.json");
        std::fs::write(&finding_path, &json_bytes)?;
        Some((staged, finding_path))
    } else {
        None
    };

    let mut cmd = Command::new(&python_bin);
    cmd.arg("-m")
        .arg("governor.drill_runner")
        .arg("--scenario")
        .arg(&config.scenario)
        .arg("--root")
        .arg(&config.receipt_root);
    if let Some((_, ref finding_path)) = staged_metadata {
        cmd.arg("--finding-json").arg(finding_path);
    }
    // D3 — confabulate-citation passes verbatim. AG re-validates the
    // role on its side; Night Shift validated it at DrillConfig
    // construction time, so this branch only fires when the flag is
    // set and AG is the authoritative gate.
    if let Some(role) = &config.confabulate_citation {
        cmd.arg("--confabulate-citation").arg(role);
    }
    let output = cmd.output()?;

    if !output.status.success() {
        return Err(DrillError::SubprocessFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let transcript = envelope
        .get("transcript")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    Ok(DrillRunResult {
        scenario: config.scenario.clone(),
        receipt_root: config.receipt_root.clone(),
        transcript,
    })
}

/// Render the drill result to a Night-Shift-flavored final transcript.
///
/// The AG-side `transcript` field is the receipt-render of the ledger.
/// We pass it through verbatim — Night Shift does NOT prepend its own
/// narration (per §3b: narration at the presentation layer is exactly
/// the laundering surface the demo refuses).
pub fn render_drill_transcript(result: &DrillRunResult) -> String {
    result.transcript.clone()
}

/// Determine the default receipt root for a drill invocation. Stable
/// per (scenario, workload) so re-runs land in the same directory
/// unless the operator overrides. Determinism caveat: receipts persist
/// across runs in the default root, so for byte-identical determinism
/// the test path uses fresh dirs.
pub fn default_receipt_root(scenario: &str, workload: &str) -> PathBuf {
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join(format!("nightshift-drill-{workload}-{scenario}"))
}

/// Workload name the operator types as the positional argument.
/// D0d-a hard-pins this to wal-bloat-review.
pub const DRILL_WORKLOAD_WAL_BLOAT_REVIEW: &str = "wal-bloat-review";

/// Validate that the workload name is the one D0d-a recognizes.
pub fn validate_drill_workload(workload: &str) -> Result<(), DrillError> {
    if workload == DRILL_WORKLOAD_WAL_BLOAT_REVIEW {
        Ok(())
    } else {
        Err(DrillError::UnsupportedScenario {
            scenario: format!(
                "workload {workload:?} not supported by D0d-a; only \
                 'wal-bloat-review' is wired"
            ),
        })
    }
}

/// Top-level entry called by `nightshift watchbill run <workload>
/// --drill --scenario=<scenario>`.
///
/// Returns the deterministic transcript that should be printed to
/// stdout. Errors surface to the caller for normal exit-code handling.
///
/// D3: when ``confabulate_citation`` is ``Some(role)``, the runner
/// shells AG with the ``--confabulate-citation=<role>`` flag and the
/// proposal-validator seam fires after the chain reaches Consumed.
/// The transcript will surface a fifth ``proposal_validator`` chain
/// line; the resulting refusal carries the closed-vocab kind
/// ``dangling_receipt_reference``.
pub fn run_drill_command(
    workload: &str,
    scenario: &str,
    receipt_root_override: Option<PathBuf>,
    confabulate_citation: Option<&str>,
) -> Result<String, DrillError> {
    validate_drill_workload(workload)?;
    let receipt_root = receipt_root_override
        .unwrap_or_else(|| default_receipt_root(scenario, workload));
    let mut config = DrillConfig::new(scenario, receipt_root)?;
    if let Some(role) = confabulate_citation {
        config = config.with_confabulate_citation(role)?;
    }
    let result = run_drill(&config)?;
    Ok(render_drill_transcript(&result))
}

/// Shell into `governor why <receipt-id>` and capture the text. Used
/// by operator-facing tooling that wants the chain walk separately
/// from the embedded transcript. The transcript already embeds the
/// walk, so this is a *convenience* — not a load-bearing path for
/// D0d-a.
///
/// The AG-side `governor` binary must be on `PATH`. Failure here is
/// surfaced as `DrillError::SubprocessFailed` so the operator sees
/// the AG-side stderr, not a generic crash.
pub fn shell_governor_why(receipt_id: &str) -> Result<String, DrillError> {
    let output = Command::new("governor")
        .arg("why")
        .arg(receipt_id)
        .output()?;
    if !output.status.success() {
        return Err(DrillError::SubprocessFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Sanity helper for callers that need to verify their `governor` CLI
/// is reachable before driving the drill. Returns whether the binary
/// answers `--help` cleanly. Not load-bearing for the drill itself.
pub fn governor_cli_reachable() -> bool {
    Command::new("governor")
        .arg("--help")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Sanity helper: detect whether the AG-side drill_runner module is
/// importable. Returns true if `python3 -c "import
/// governor.drill_runner"` exits 0.
pub fn drill_runner_module_importable(python_bin: Option<&Path>) -> bool {
    let bin = python_bin
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::var_os("NIGHTSHIFT_PYTHON").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("python3"));
    Command::new(&bin)
        .arg("-c")
        .arg("import governor.drill_runner")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// =========================================================================
// D0e — demo (show-surface poster) entry point.
//
// Single operator-load-bearing entry: `nightshift watchbill demo
// wal-bloat-review --drill`. Shells AG's `python3 -m
// governor.drill_poster --root <path>` and prints the deterministic
// poster on stdout. Per the slice constraint: display-only plus
// invariant assertions. No dashboard. The exit code is nonzero iff
// any harness assertion fails (AG-side determines this; Night Shift
// surfaces the subprocess exit code).
//
// Per §3b "ONE entry point. Nightshift owns it." The AG side does
// NOT add a competing `governor demo` CLI — `python3 -m
// governor.drill_poster` is the library-shell-in path only.
// =========================================================================

/// Default poster root for `watchbill demo`. Per-workload directory
/// under `$TMPDIR` so re-runs are self-contained.
pub fn default_poster_root(workload: &str) -> PathBuf {
    let base = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join(format!("nightshift-demo-{workload}"))
}

/// Top-level entry called by `nightshift watchbill demo <workload>
/// --drill [--poster-root <path>]`.
///
/// Returns the deterministic poster text that should be printed to
/// stdout. The AG side determines exit-code semantics: a nonzero
/// subprocess exit raises `DrillError::SubprocessFailed` here, which
/// the main wrapper surfaces as a nonzero exit code.
pub fn run_demo_command(
    workload: &str,
    poster_root_override: Option<PathBuf>,
) -> Result<String, DrillError> {
    validate_drill_workload(workload)?;
    let poster_root =
        poster_root_override.unwrap_or_else(|| default_poster_root(workload));
    std::fs::create_dir_all(&poster_root)?;
    let python_bin = std::env::var_os("NIGHTSHIFT_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("python3"));
    let output = Command::new(&python_bin)
        .arg("-m")
        .arg("governor.drill_poster")
        .arg("--root")
        .arg(&poster_root)
        .arg("--format")
        .arg("text")
        .output()?;
    if !output.status.success() {
        return Err(DrillError::SubprocessFailed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Sanity helper: detect whether the AG-side drill_poster module is
/// importable. Same shape as `drill_runner_module_importable`.
pub fn drill_poster_module_importable(python_bin: Option<&Path>) -> bool {
    let bin = python_bin
        .map(|p| p.to_path_buf())
        .or_else(|| std::env::var_os("NIGHTSHIFT_PYTHON").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("python3"));
    Command::new(&bin)
        .arg("-c")
        .arg("import governor.drill_poster")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
