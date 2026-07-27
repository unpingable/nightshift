//! Night Shift daemon — `nightshift` CLI entry.

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

use nightshiftd::agenda::Agenda;
use nightshiftd::finding::FindingKey;
use nightshiftd::governor_client::{GovernorClient, JsonRpcGovernorClient};
use nightshiftd::horizon_policy::{FixtureHorizonPolicySource, HorizonPolicySource};
use nightshiftd::liveness::{
    CliLivenessSource, LivenessSource, DEFAULT_STALENESS_THRESHOLD_SECONDS,
};
use nightshiftd::liveness_peek::{
    render_peek_text as render_liveness_peek_text, LivenessPeekDocument, ThresholdSource,
};
use nightshiftd::nq::{CliNqSource, FixtureNqSource, NqListFilter, NqSource};
use nightshiftd::nq_peek::{render_peek_text, PeekDocument};
use nightshiftd::pipeline::{
    capture_phase, reconcile_phase, reconcile_phase_with_horizon, run_watchbill_with_liveness,
    CaptureOutcome, PipelineOptions,
};
use nightshiftd::posture::{
    list_postures, load_posture, render_list_row, render_show, PostureFilter,
};
use nightshiftd::attention::{AttentionRow, ReAckDisposition};
use nightshiftd::scheduled::{check_scheduled_idempotency, ScheduledOutcome};
use nightshiftd::store::{sqlite::SqliteStore, RunTrigger, Store};

#[derive(Parser, Debug)]
#[command(
    name = "nightshift",
    about = "Deferred agent work with receipts, reconciliation, and governed promotion",
    version
)]
struct Cli {
    /// Run without Governor. Promotion ceiling is lowered to `advise`;
    /// mutation, publication, paging, and staged actions are disabled.
    #[arg(long, global = true)]
    no_governor: bool,

    /// Path to SQLite store (v1 default: ./nightshift.sqlite).
    #[arg(long, global = true, default_value = "nightshift.sqlite")]
    store: PathBuf,

    /// Path to NQ fixture manifest. Used when `--nq-db` is not set.
    #[arg(long, global = true, default_value = "tests/fixtures/nq-manifest.json")]
    nq_fixture: PathBuf,

    /// Path to a real NQ SQLite database. When set, Night Shift asks the
    /// configured NQ executable to run `findings export --db <path>` and
    /// consumes the canonical snapshot contract
    /// (schema nq.finding_snapshot.v1). Overrides --nq-fixture.
    #[arg(long, global = true)]
    nq_db: Option<PathBuf>,

    /// Path to the `nq-monitor` executable. `nq disposition` requires this
    /// option or NIGHTSHIFT_NQ_BIN. Finding and liveness reads additionally
    /// fall back to an executable named `nq` on PATH when neither is set.
    #[arg(long, global = true)]
    nq_bin: Option<PathBuf>,

    /// Treat Continuity as configured for this deployment. v1 does
    /// not yet query Continuity; this flag controls preflight
    /// behavior for risky-class agendas (see GAP-parallel-ops.md).
    #[arg(long, global = true)]
    continuity_configured: bool,

    /// Path to NQ's liveness artifact (typically liveness.json
    /// alongside the NQ database). When set, Night Shift consults
    /// NQ's `liveness export` before capturing finding evidence; a
    /// stale or skewed witness halts the run with a Stale-shape
    /// packet (revalidate-only proposal). Optional — when omitted,
    /// no liveness gating is performed.
    #[arg(long, global = true)]
    nq_liveness: Option<PathBuf>,

    /// Liveness staleness threshold in seconds. Applies only when
    /// `--nq-liveness` is set. Default: 90s (~1.5x typical NQ scan
    /// cadence).
    #[arg(long, global = true)]
    nq_liveness_threshold_secs: Option<u64>,

    /// Path to a horizon-policy fixture JSON manifest declaring
    /// per-finding tolerance windows. When set, `watchbill run` and
    /// `watchbill reconcile` invoke `reconcile_phase_with_horizon`,
    /// which writes tolerance records on `Defer`, promotes packet
    /// Attention to `WatchUntil`, and forwards declarations to
    /// Governor via `record_receipt`. Requires `--governor-socket`.
    ///
    /// **Without this flag, the default `watchbill` path is
    /// governor-blind**: it runs through `run_watchbill_with_liveness`
    /// (the local/liveness-only path), emits no `record_receipt`
    /// calls, and produces a packet without Governor receipt
    /// references. The split is intentional — Governor receipt
    /// emission is opt-in because it is an authority/coordination
    /// act, not just logging.
    ///
    /// See `docs/working/gaps/GAP-imported-basis-freshness.md` and
    /// `src/horizon_policy.rs`.
    #[arg(long, global = true)]
    horizon_policy: Option<PathBuf>,

    /// Path to the Governor JSON-RPC Unix socket. Required together
    /// with `--horizon-policy` to activate the horizon path —
    /// horizon-driven deferrals emit `record_receipt` calls so the
    /// tolerance declaration shows up in Governor's receipt chain.
    /// One fresh connection per call; no persistent connection.
    ///
    /// **Without this flag (and `--horizon-policy`), no
    /// `record_receipt` calls are emitted to Governor.** The default
    /// `watchbill` invocation produces a local packet only; Governor
    /// receipt emission is opt-in. Either flag alone is a hard error.
    #[arg(long, global = true)]
    governor_socket: Option<PathBuf>,

    /// How this invocation was triggered. `scheduled` (timer / cron)
    /// activates idempotency: if the most recent completed run for
    /// `(agenda, finding)` already reconciled the current NQ
    /// `snapshot_generation`, the invocation skips with a one-line
    /// report pointing at the prior run. `manual` (default) and
    /// `event` always run. Recorded on the run row regardless.
    #[arg(long, global = true, value_enum, default_value_t = TriggerArg::Manual)]
    trigger: TriggerArg,

    /// Path to an `nq.receipt.v1` JSON document. When set together
    /// with `watchbill run` / `watchbill reconcile`, Night Shift runs
    /// the MVP-A cross-component pipeline after the packet is built:
    /// cook a Wicket Intent that chains to this receipt's
    /// `content_hash`, invoke `wicket::check()`, wrap the Outcome as
    /// a WLP `AuthorizationReceipt`, call `wlp::handle()`. Three
    /// artifacts are written to `--mvp-a-out-dir`. Without this flag,
    /// the existing pipeline runs unchanged.
    ///
    /// See `docs/working/decisions/MVP_A_SLICES_2_3_4_PACKET.md`.
    #[arg(long, global = true)]
    nq_receipt: Option<PathBuf>,

    /// Output directory for MVP-A artifacts. Only consulted when
    /// `--nq-receipt` is set. Files written:
    /// `ns-posture-<run_id>.json`, `ns-wicket-outcome-<run_id>.json`,
    /// `ns-wlp-handling-<run_id>.json`. Default: `/tmp`.
    #[arg(long, global = true, default_value = "/tmp")]
    mvp_a_out_dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

/// CLI mirror of `nightshiftd::store::RunTrigger`. Kept separate so
/// clap derive doesn't reach into the library crate.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "snake_case")]
enum TriggerArg {
    Manual,
    Scheduled,
    Event,
}

impl From<TriggerArg> for RunTrigger {
    fn from(t: TriggerArg) -> Self {
        match t {
            TriggerArg::Manual => RunTrigger::Manual,
            TriggerArg::Scheduled => RunTrigger::Scheduled,
            TriggerArg::Event => RunTrigger::Event,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Ops-mode agendas (Watchbill).
    Watchbill {
        #[command(subcommand)]
        action: WatchbillAction,
    },
    /// Query persisted runs: what happened, what held, and why.
    Runs {
        #[command(subcommand)]
        action: RunsAction,
    },
    /// Inspection surface for a live NQ database.
    Nq {
        #[command(subcommand)]
        action: NqAction,
    },
    /// Inspection surface for the NQ liveness artifact (the same DTO
    /// the pipeline gate consumes).
    Liveness {
        #[command(subcommand)]
        action: LivenessAction,
    },
    /// Operator attention commands (Slice 3: ack + silence).
    ///
    /// Attention is durable across runs and keyed on the stable
    /// `(agenda, finding_key)` per `GAP-attention-state.md`. On the
    /// next scheduled run, the reconciler reads the attention store
    /// and applies the projection to the packet.
    Attention {
        /// Agenda this attention applies to. Attention is scoped to
        /// the agenda; one agenda's ack does not silence another's.
        #[arg(long)]
        agenda: String,
        /// Stable finding key: `<source>:<detector>:<subject>`.
        #[arg(long)]
        finding: String,
        /// Operator identifier recorded on the row. Required so
        /// `last_touched_by` is never empty.
        #[arg(long)]
        operator: String,
        #[command(subcommand)]
        action: AttentionAction,
    },
}

#[derive(Subcommand, Debug)]
enum AttentionAction {
    /// Acknowledge a finding. `--ttl` is recommended (acks should
    /// have a half-life). If a prior attention row exists for this
    /// `(agenda, finding)`, `--disposition` is required — re-ack is
    /// mini-re-triage per `architecture/GAP-reack-doctrine.md`, not a
    /// courtesy tap.
    Ack {
        /// How long the ack lasts before re-surface. Format: a
        /// duration like `4h`, `90m`, `2d`. Omitting it means
        /// "use the agenda's `criticality.re_alert_after`."
        #[arg(long)]
        ttl: Option<String>,
        /// Optional free-text note.
        #[arg(long)]
        note: Option<String>,
        /// Required when re-acking an existing row. One of:
        /// advanced, unchanged-waiting, blocked, handed-off,
        /// escalated, resolved.
        #[arg(long)]
        disposition: Option<String>,
    },
    /// Silence a finding until a timestamp, with a reason. Both
    /// `--until` and `--reason` are required — no open-ended
    /// silence (GAP-attention-state invariant).
    Silence {
        /// RFC3339 timestamp when the silence expires.
        #[arg(long)]
        until: String,
        /// Required free-text reason. The reconciler refuses an
        /// empty string.
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand, Debug)]
enum LivenessAction {
    /// Show what NQ's `liveness export` returns and what Night Shift's
    /// gate would do with it. Both views appear side-by-side so the
    /// operator can see when upstream `freshness.fresh` and Night
    /// Shift's verdict diverge (the clock-skew wrinkle).
    ///
    /// Requires `--nq-liveness <path>` (global). Threshold defaults to
    /// `DEFAULT_STALENESS_THRESHOLD_SECONDS` (90s) when
    /// `--nq-liveness-threshold-secs` is not set.
    Peek {
        /// Output format: `text` (default) or `json`.
        #[arg(long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
enum NqAction {
    /// Translation-only listing of NQ findings as Night Shift would
    /// consume them. Use `--format json` for diff-friendly output.
    Peek {
        /// Restrict to a specific detector kind (e.g. `wal_bloat`).
        #[arg(long)]
        detector: Option<String>,

        /// Restrict to a specific host.
        #[arg(long)]
        host: Option<String>,

        /// Exact-match on NQ's canonical finding_key
        /// (e.g. `local/host/detector/subject`, URL-encoded).
        #[arg(long)]
        finding_key: Option<String>,

        /// Output format: `text` (default) or `json`.
        #[arg(long, default_value = "text")]
        format: String,

        /// Include NQ's full raw JSONL payload alongside the
        /// translated view (for cross-checking).
        #[arg(long)]
        show_raw: bool,
    },

    /// Ask NQ for a reliance decision and derive Night Shift's read-only
    /// disposition. Prints the `nightshift.readonly_disposition.v1` record;
    /// sends nothing anywhere. Exit 0 means a disposition was derived —
    /// including `stop`; the record, not the shell status, is the product.
    Disposition {
        /// `nq.reliance.request.v1` document.
        #[arg(long)]
        request: PathBuf,

        /// The sealed `nq.receipt.v1` being relied upon.
        #[arg(long)]
        receipt: PathBuf,

        /// `nq.reliance.profiles.v1` catalog.
        #[arg(long)]
        profiles: PathBuf,

        /// Optional evidence context document.
        #[arg(long)]
        evidence: Option<PathBuf>,

        /// Sealed supporting receipts, passed through to NQ verbatim
        /// (repeatable). Whether they satisfy the profile is NQ's law.
        #[arg(long)]
        supporting: Vec<PathBuf>,

        /// The consumer profile the returned receipt must be addressed to.
        #[arg(long, default_value = nightshiftd::nq_disposition::EXPECTED_CONSUMER_PROFILE)]
        expected_profile: String,

        /// Night Shift's own timeout for the NQ invocation, in seconds.
        #[arg(long, default_value_t = 30)]
        timeout_seconds: u64,

        /// Night Shift's own freshness policy for the returned receipt.
        #[arg(long, default_value_t = 900)]
        max_age_seconds: u64,

        /// Output format: `json` (default; the record verbatim) or `text`.
        #[arg(long, default_value = "json")]
        format: String,
    },
}

#[derive(Subcommand, Debug)]
enum RunsAction {
    /// List recent runs with status and target finding_key.
    List {
        /// Filter to a single agenda.
        #[arg(long)]
        agenda: Option<String>,

        /// Filter to a single target finding_key (`<source>:<detector>:<subject>`).
        #[arg(long)]
        finding: Option<String>,

        /// Only show runs held before reconcile.
        #[arg(long)]
        held_only: bool,

        /// Limit number of rows printed.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Show one run's posture: metadata, ceiling, hold reason, event timeline.
    Show {
        /// The run_id to display.
        run_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum WatchbillAction {
    /// Run an agenda end-to-end (capture then reconcile in one
    /// invocation). Thin convenience over `capture` + `reconcile`;
    /// same semantics, same-generation timing.
    ///
    /// Drill mode: when `--drill` is passed, the positional becomes a
    /// workload name (e.g. `wal-bloat-review`), not a YAML path. The
    /// drill runner shells into AG's `python3 -m
    /// governor.drill_runner` and renders a deterministic transcript
    /// of the receipt chain. `--scenario` selects which drill
    /// scenario; D0d-a admits only `all-green`. In drill mode,
    /// `--finding` is not consulted.
    Run {
        /// Path to an agenda YAML file. In `--drill` mode this is the
        /// workload name (e.g. `wal-bloat-review`).
        agenda_path: PathBuf,

        /// Stable finding key to target: `<source>:<detector>:<subject>`.
        /// Not required in `--drill` mode (the drill scenario controls
        /// the finding).
        #[arg(long)]
        finding: Option<String>,

        /// Run a deterministic drill scenario instead of an agenda.
        /// The positional argument becomes the workload name. D0d-a:
        /// only `wal-bloat-review` + `--scenario=all-green` is wired.
        ///
        /// The drill emits real GateReceipts (origin_mode=drill) via
        /// AG's cooked-context orchestrator and renders a
        /// deterministic transcript — receipt render, NOT model
        /// narration.
        #[arg(long)]
        drill: bool,

        /// Drill scenario name. D0d-a admits only `all-green`. Ignored
        /// when `--drill` is not set.
        #[arg(long)]
        scenario: Option<String>,

        /// Override the receipt root directory the drill writes
        /// receipts into. Defaults to
        /// `$TMPDIR/nightshift-drill-<workload>-<scenario>`. Ignored
        /// when `--drill` is not set.
        #[arg(long)]
        drill_receipt_root: Option<PathBuf>,

        /// D3 — inject a confabulated citation into the proposal packet
        /// step. Closed role set: ``standing`` (bogus standing receipt
        /// id; existence-fail) / ``evidence`` (LA grant receipt id
        /// cited in the standing slot; kind-fit-fail). Both modes
        /// produce a ``dangling_receipt_reference`` refusal receipt
        /// at the proposal_validator_seam after the chain reaches
        /// Consumed; ``effect_count`` stays at 1 (failure cost real
        /// budget). Requires ``--scenario=all-green``; AG refuses at
        /// construction otherwise. Ignored when ``--drill`` is not set.
        #[arg(long, value_parser = ["standing", "evidence"])]
        confabulate_citation: Option<String>,
    },
    /// Freeze a baseline observation into a new run and leave the
    /// run open, awaiting `reconcile`. Runs capture-time gates
    /// (authority, liveness, preflight); on gate hold the run is
    /// closed immediately with a held packet. On success, prints
    /// the new run_id to stdout (consumable by `reconcile`).
    ///
    /// See `docs/working/gaps/GAP-deferred-run-split.md`.
    Capture {
        /// Path to an agenda YAML file.
        agenda_path: PathBuf,

        /// Stable finding key to target: `<source>:<detector>:<subject>`.
        #[arg(long)]
        finding: String,
    },
    /// Reconcile a previously captured, still-open run: re-check
    /// preflight, perform the one explicit reconcile-time NQ
    /// acquisition, adjudicate, emit the packet, finalize the run.
    /// One-shot: reconciling a completed run is an error.
    Reconcile {
        /// The run_id returned by a prior `capture`.
        run_id: String,
    },
    /// D0e — run all six gauntlet scenarios + the D3 confabulated
    /// citation closing beat and render the ticket-shaped show-surface
    /// poster on stdout. Operator-facing single entry point per §3b
    /// "Nightshift owns it". Shells AG's `python3 -m
    /// governor.drill_poster --root <path>` and prints the resulting
    /// deterministic poster. Exits nonzero if any harness assertion
    /// fails (e.g., a BA3-classified internal guard fired a denial
    /// during any spine run).
    ///
    /// Per the slice operator constraint: display-only plus invariant
    /// assertions. No dashboard. No curses. No "operator experience"
    /// pilgrimage. Receipt poster, deterministic, plain text.
    Demo {
        /// Workload name. D0e admits only `wal-bloat-review`.
        workload: PathBuf,

        /// `--drill` flag. Required (the demo is the drill gauntlet);
        /// matched for CLI symmetry with `watchbill run --drill`.
        #[arg(long)]
        drill: bool,

        /// Override the poster root directory. Defaults to
        /// `$TMPDIR/nightshift-demo-<workload>`. Per-scenario receipts
        /// land under `<root>/<scenario>/`.
        #[arg(long)]
        poster_root: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Watchbill { action } => match action {
            WatchbillAction::Run {
                agenda_path,
                finding,
                drill,
                scenario,
                drill_receipt_root,
                confabulate_citation,
            } => {
                if *drill {
                    // D0d-a drill path. Positional is the workload
                    // name (e.g. `wal-bloat-review`). The drill module
                    // shells into AG's `python3 -m
                    // governor.drill_runner` and renders a
                    // deterministic transcript.
                    let workload = agenda_path
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!(
                            "drill workload name must be valid UTF-8"
                        ))?;
                    let scenario = scenario.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "watchbill run --drill requires --scenario; \
                             D0d-1 admits one of: all-green, no-standing, \
                             standing-expired, wicket-denied, \
                             wicket-gap-accounted, replay-budget \
                             (accepted alias: already-consumed)"
                        )
                    })?;
                    let transcript = nightshiftd::drill::run_drill_command(
                        workload,
                        scenario,
                        drill_receipt_root.clone(),
                        confabulate_citation.as_deref(),
                    )?;
                    print!("{transcript}");
                    Ok(())
                } else {
                    let finding = finding.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "watchbill run requires --finding when --drill is not set"
                        )
                    })?;
                    run_watchbill_cmd(&cli, agenda_path, finding)
                }
            }
            WatchbillAction::Capture {
                agenda_path,
                finding,
            } => capture_cmd(&cli, agenda_path, finding),
            WatchbillAction::Reconcile { run_id } => reconcile_cmd(&cli, run_id),
            WatchbillAction::Demo {
                workload,
                drill,
                poster_root,
            } => {
                if !*drill {
                    anyhow::bail!(
                        "watchbill demo requires --drill (D0e is the drill gauntlet)"
                    );
                }
                let workload = workload.to_str().ok_or_else(|| {
                    anyhow::anyhow!("demo workload name must be valid UTF-8")
                })?;
                let poster = nightshiftd::drill::run_demo_command(
                    workload,
                    poster_root.clone(),
                )?;
                print!("{poster}");
                Ok(())
            }
        },
        Command::Runs { action } => match action {
            RunsAction::List {
                agenda,
                finding,
                held_only,
                limit,
            } => runs_list_cmd(&cli, agenda.clone(), finding.clone(), *held_only, *limit),
            RunsAction::Show { run_id } => runs_show_cmd(&cli, run_id),
        },
        Command::Nq { action } => match action {
            NqAction::Peek {
                detector,
                host,
                finding_key,
                format,
                show_raw,
            } => nq_peek_cmd(
                &cli,
                detector.clone(),
                host.clone(),
                finding_key.clone(),
                format,
                *show_raw,
            ),
            NqAction::Disposition {
                request,
                receipt,
                profiles,
                evidence,
                supporting,
                expected_profile,
                timeout_seconds,
                max_age_seconds,
                format,
            } => nq_disposition_cmd(
                &cli,
                request,
                receipt,
                profiles,
                evidence.as_ref(),
                supporting,
                expected_profile,
                *timeout_seconds,
                *max_age_seconds,
                format,
            ),
        },
        Command::Liveness { action } => match action {
            LivenessAction::Peek { format } => liveness_peek_cmd(&cli, format),
        },
        Command::Attention {
            agenda,
            finding,
            operator,
            action,
        } => attention_cmd(&cli, agenda, finding, operator, action),
    }
}

fn attention_cmd(
    cli: &Cli,
    agenda: &str,
    finding: &str,
    operator: &str,
    action: &AttentionAction,
) -> anyhow::Result<()> {
    if operator.trim().is_empty() {
        anyhow::bail!("--operator must be non-empty");
    }
    let key = parse_finding_arg(finding)?;
    let store = SqliteStore::open(&cli.store)?;
    let prior = store.get_attention(agenda, &key)?;

    match action {
        AttentionAction::Ack {
            ttl,
            note,
            disposition,
        } => {
            let disposition_parsed = match disposition {
                Some(s) => Some(ReAckDisposition::parse_token(s).ok_or_else(|| {
                    anyhow::anyhow!(
                        "unknown --disposition {s:?}; valid: advanced, unchanged-waiting, blocked, handed-off, escalated, resolved",
                    )
                })?),
                None => None,
            };
            if prior.is_some() && disposition_parsed.is_none() {
                anyhow::bail!(
                    "--disposition is required when re-acking an existing attention row (per re-ack doctrine). \
                     Valid values: advanced, unchanged-waiting, blocked, handed-off, escalated, resolved"
                );
            }
            let ack_expires_at = match ttl {
                Some(d) => Some(chrono::Utc::now() + parse_duration(d)?),
                None => None,
            };
            let row = AttentionRow::ack(
                agenda.into(),
                key,
                operator.into(),
                ack_expires_at,
                note.clone(),
                disposition_parsed,
            );
            store.save_attention(&row)?;
            println!(
                "ack: agenda={} finding={} operator={} ttl={} disposition={}",
                row.agenda_id,
                row.finding_key.as_string(),
                row.last_touched_by,
                row.ack_expires_at
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_else(|| "(none)".into()),
                row.disposition
                    .map(|d| d.as_str().to_string())
                    .unwrap_or_else(|| "(initial-ack)".into()),
            );
        }
        AttentionAction::Silence { until, reason } => {
            if reason.trim().is_empty() {
                anyhow::bail!("--reason must be non-empty (suppression needs a reason)");
            }
            let until_ts = chrono::DateTime::parse_from_rfc3339(until)
                .map_err(|e| anyhow::anyhow!("--until must be RFC3339: {e}"))?
                .with_timezone(&chrono::Utc);
            if until_ts <= chrono::Utc::now() {
                anyhow::bail!(
                    "--until must be in the future; silence with a past timestamp would expire immediately"
                );
            }
            let row = AttentionRow::silence(
                agenda.into(),
                key,
                operator.into(),
                until_ts,
                reason.clone(),
            );
            store.save_attention(&row)?;
            println!(
                "silence: agenda={} finding={} operator={} until={} reason={}",
                row.agenda_id,
                row.finding_key.as_string(),
                row.last_touched_by,
                until_ts.to_rfc3339(),
                reason,
            );
        }
    }
    Ok(())
}

/// Parse a duration like `4h`, `90m`, `30s`, or `2d`. Numeric +
/// single-letter unit. Used by `attention ack --ttl`.
fn parse_duration(s: &str) -> anyhow::Result<chrono::Duration> {
    let s = s.trim();
    let (num, unit) = match s.char_indices().rev().find(|(_, c)| !c.is_ascii_digit()) {
        Some((i, _)) => s.split_at(i),
        None => anyhow::bail!("duration {s:?} must end in a unit (s|m|h|d)"),
    };
    let n: i64 = num
        .parse()
        .map_err(|e| anyhow::anyhow!("duration {s:?}: bad number {num:?}: {e}"))?;
    match unit {
        "s" => Ok(chrono::Duration::seconds(n)),
        "m" => Ok(chrono::Duration::minutes(n)),
        "h" => Ok(chrono::Duration::hours(n)),
        "d" => Ok(chrono::Duration::days(n)),
        other => anyhow::bail!("duration {s:?}: unknown unit {other:?}; valid: s|m|h|d"),
    }
}

fn liveness_peek_cmd(cli: &Cli, format: &str) -> anyhow::Result<()> {
    let path = cli
        .nq_liveness
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("liveness peek requires --nq-liveness <path>"))?;
    let mut src = CliLivenessSource::new(path);
    if let Some(bin) = &cli.nq_bin {
        src = src.with_nq_bin(bin);
    }
    let snapshot = src.current()?;
    let (threshold, source) = match cli.nq_liveness_threshold_secs {
        Some(n) => (n, ThresholdSource::Operator),
        None => (
            DEFAULT_STALENESS_THRESHOLD_SECONDS,
            ThresholdSource::Default,
        ),
    };
    let doc = LivenessPeekDocument::build(snapshot, threshold, source);
    match format {
        "json" => println!("{}", doc.to_json_pretty()),
        _ => print!("{}", render_liveness_peek_text(&doc)),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn nq_disposition_cmd(
    cli: &Cli,
    request: &Path,
    receipt: &Path,
    profiles: &Path,
    evidence: Option<&PathBuf>,
    supporting: &[PathBuf],
    expected_profile: &str,
    timeout_seconds: u64,
    max_age_seconds: u64,
    format: &str,
) -> anyhow::Result<()> {
    use nightshiftd::nq::RelianceInvocation;
    use nightshiftd::nq_disposition::derive_disposition;

    let nq_bin = cli
        .nq_bin
        .as_ref()
        .cloned()
        .or_else(|| std::env::var_os("NIGHTSHIFT_NQ_BIN").map(PathBuf::from))
        .ok_or_else(|| {
            anyhow::anyhow!("nq disposition requires --nq-bin <path> or NIGHTSHIFT_NQ_BIN")
        })?;

    let inv = RelianceInvocation {
        nq_bin,
        request: request.to_path_buf(),
        receipt: receipt.to_path_buf(),
        evidence: evidence.cloned(),
        profiles: profiles.to_path_buf(),
        supporting: supporting.to_vec(),
        expected_profile: expected_profile.to_string(),
        timeout_seconds,
        max_age_seconds,
    };
    let now = chrono::Utc::now();
    let out = inv.evaluate(now);
    let record = derive_disposition(
        &out.state,
        out.receipt.as_ref(),
        &now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        expected_profile,
    );

    match format {
        "json" => println!("{}", serde_json::to_string_pretty(&record)?),
        "text" => {
            println!("disposition: {}", serde_json::to_value(record.disposition)?);
            println!("disposition_id: {}", record.disposition_id);
            println!("expected_consumer_profile: {}", record.expected_consumer_profile);
            for r in &record.reasons {
                println!("reason: {r}");
            }
            if let Some(src) = &record.source {
                println!("source decision: {} ({})", src.decision, src.decision_id);
                for s in &src.supporting_receipts {
                    println!("supporting: {} {} ({})", s.claim, s.content_hash, s.status);
                }
            }
        }
        other => anyhow::bail!("unknown format {other:?}: expected json or text"),
    }
    Ok(())
}

fn nq_peek_cmd(
    cli: &Cli,
    detector: Option<String>,
    host: Option<String>,
    finding_key: Option<String>,
    format: &str,
    show_raw: bool,
) -> anyhow::Result<()> {
    let db = cli
        .nq_db
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("nq peek requires --nq-db <path>"))?;
    let mut src = CliNqSource::new(db.clone());
    if let Some(bin) = &cli.nq_bin {
        src = src.with_nq_bin(bin.clone());
    }
    let filter = NqListFilter {
        detector,
        host,
        finding_key,
    };
    let items = src.list_findings(&filter)?;
    let doc = PeekDocument::build(&items, show_raw);
    match format {
        "json" => println!("{}", doc.to_json_pretty()),
        _ => print!("{}", render_peek_text(&doc, show_raw)),
    }
    Ok(())
}

fn runs_list_cmd(
    cli: &Cli,
    agenda: Option<String>,
    finding: Option<String>,
    held_only: bool,
    limit: usize,
) -> anyhow::Result<()> {
    let store = SqliteStore::open(&cli.store)?;
    let filter = PostureFilter {
        agenda_id: agenda,
        target_finding_key: finding,
        held_only,
        limit: Some(limit),
    };
    let postures = list_postures(&store, &filter)?;
    if postures.is_empty() {
        println!("(no runs match)");
        return Ok(());
    }
    for p in &postures {
        println!("{}", render_list_row(p));
    }
    Ok(())
}

fn runs_show_cmd(cli: &Cli, run_id: &str) -> anyhow::Result<()> {
    let store = SqliteStore::open(&cli.store)?;
    match load_posture(&store, run_id)? {
        Some(p) => {
            print!("{}", render_show(&p));
            Ok(())
        }
        None => anyhow::bail!("run not found: {run_id}"),
    }
}

fn build_nq_source(cli: &Cli) -> anyhow::Result<Box<dyn NqSource>> {
    if let Some(db) = &cli.nq_db {
        let mut src = CliNqSource::new(db.clone());
        if let Some(bin) = &cli.nq_bin {
            src = src.with_nq_bin(bin.clone());
        }
        Ok(Box::new(src))
    } else {
        Ok(Box::new(FixtureNqSource::load(&cli.nq_fixture)?))
    }
}

fn build_liveness_source(cli: &Cli) -> Option<Box<dyn LivenessSource>> {
    let path = cli.nq_liveness.as_ref()?;
    let mut src = CliLivenessSource::new(path);
    if let Some(bin) = &cli.nq_bin {
        src = src.with_nq_bin(bin);
    }
    Some(Box::new(src))
}

/// Paired horizon dependencies: the NS-local policy source and the
/// Governor RPC client that archives the resulting tolerance
/// receipts. Always either both or neither.
type HorizonDeps = (Box<dyn HorizonPolicySource>, Box<dyn GovernorClient>);

/// Resolve the horizon dependencies from CLI flags. Both
/// `--horizon-policy` and `--governor-socket` must be set together;
/// either flag alone is a configuration error per
/// `src/horizon_policy.rs` module docs (horizon is NS-declared,
/// archived through Governor's `record_receipt`).
fn build_horizon_deps(cli: &Cli) -> anyhow::Result<Option<HorizonDeps>> {
    match (&cli.horizon_policy, &cli.governor_socket) {
        (Some(policy_path), Some(socket_path)) => {
            let policy = FixtureHorizonPolicySource::load(policy_path)?;
            let governor = JsonRpcGovernorClient::new(socket_path.clone());
            Ok(Some((Box::new(policy), Box::new(governor))))
        }
        (Some(_), None) => {
            anyhow::bail!("--horizon-policy requires --governor-socket (horizon receipts are forwarded via Governor)")
        }
        (None, Some(_)) => {
            anyhow::bail!("--governor-socket requires --horizon-policy (no firing site without a policy source)")
        }
        (None, None) => Ok(None),
    }
}

fn run_watchbill_cmd(
    cli: &Cli,
    agenda_path: &std::path::Path,
    finding: &str,
) -> anyhow::Result<()> {
    let agenda = Agenda::from_yaml_file(agenda_path)?;
    let nq = build_nq_source(cli)?;
    let liveness = build_liveness_source(cli);
    let store = SqliteStore::open(&cli.store)?;
    let target = parse_finding_arg(finding)?;
    let horizon_deps = build_horizon_deps(cli)?;

    let opts = pipeline_opts(cli);

    // Scheduled-trigger idempotency: skip if the most recent
    // completed run for (agenda, finding) already reconciled the
    // current NQ snapshot_generation. See `src/scheduled.rs` and the
    // Slice 1 close-out in
    // `docs/working/roadmaps/nightshift_v1_runtime_ladder.md`.
    if cli.trigger == TriggerArg::Scheduled {
        if let ScheduledOutcome::Skipped(report) =
            check_scheduled_idempotency(&agenda, &target, nq.as_ref(), &store)?
        {
            println!("{}", report.message());
            return Ok(());
        }
    }

    let packet = match horizon_deps {
        // No horizon configured — original same-generation path.
        None => run_watchbill_with_liveness(
            &agenda,
            &target,
            nq.as_ref(),
            liveness.as_deref(),
            &store,
            &opts,
        )?,
        // Horizon configured — compose capture + horizon-aware
        // reconcile by hand. Matches the deferred CLI path
        // (`capture` + `reconcile`) per
        // `docs/working/gaps/GAP-deferred-run-split.md`.
        Some((policy, governor)) => match capture_phase(
            &agenda,
            &target,
            nq.as_ref(),
            liveness.as_deref(),
            &store,
            &opts,
        )? {
            CaptureOutcome::HeldPacket(packet) => *packet,
            CaptureOutcome::Captured { run_id } => reconcile_phase_with_horizon(
                &run_id,
                nq.as_ref(),
                Some(policy.as_ref()),
                Some(governor.as_ref()),
                &store,
                &opts,
            )?,
        },
    };

    // v1: emit packet to stdout as YAML.
    let rendered = serde_yaml::to_string(&packet)?;
    println!("{rendered}");

    // MVP-A Slices 2/3/4 tail step (opt-in via --nq-receipt). When
    // configured, cook a Wicket Intent chaining to the NQ receipt,
    // invoke Wicket, wrap the Outcome as a WLP AuthorizationReceipt,
    // call wlp::handle(), and write all three artifacts to disk. The
    // existing watchbill output above is unchanged regardless.
    maybe_run_mvp_a(cli, &packet)?;
    Ok(())
}

/// `watchbill capture` — freeze a baseline, persist an open run.
///
/// On `Captured`, prints the run_id alone (machine-consumable). On
/// a held outcome (liveness/preflight), the run is already closed
/// and the held packet is printed as YAML — operators still see what
/// was attempted and why.
fn capture_cmd(cli: &Cli, agenda_path: &std::path::Path, finding: &str) -> anyhow::Result<()> {
    let agenda = Agenda::from_yaml_file(agenda_path)?;
    let nq = build_nq_source(cli)?;
    let liveness = build_liveness_source(cli);
    let store = SqliteStore::open(&cli.store)?;
    let target = parse_finding_arg(finding)?;

    let opts = pipeline_opts(cli);

    match capture_phase(
        &agenda,
        &target,
        nq.as_ref(),
        liveness.as_deref(),
        &store,
        &opts,
    )? {
        CaptureOutcome::Captured { run_id } => {
            println!("{run_id}");
        }
        CaptureOutcome::HeldPacket(packet) => {
            let rendered = serde_yaml::to_string(&*packet)?;
            println!("{rendered}");
        }
    }
    Ok(())
}

/// `watchbill reconcile <run_id>` — complete a captured run. One-shot.
fn reconcile_cmd(cli: &Cli, run_id: &str) -> anyhow::Result<()> {
    let nq = build_nq_source(cli)?;
    let store = SqliteStore::open(&cli.store)?;
    let horizon_deps = build_horizon_deps(cli)?;

    let opts = pipeline_opts(cli);

    let packet = match horizon_deps {
        None => reconcile_phase(run_id, nq.as_ref(), &store, &opts)?,
        Some((policy, governor)) => reconcile_phase_with_horizon(
            run_id,
            nq.as_ref(),
            Some(policy.as_ref()),
            Some(governor.as_ref()),
            &store,
            &opts,
        )?,
    };
    let rendered = serde_yaml::to_string(&packet)?;
    println!("{rendered}");
    maybe_run_mvp_a(cli, &packet)?;
    Ok(())
}

/// MVP-A tail step. No-op when `--nq-receipt` is unset (legacy
/// pipeline path). When set, runs cook → Wicket → WLP and prints a
/// one-line summary naming the three sink paths and the walkable
/// chain hashes.
fn maybe_run_mvp_a(cli: &Cli, packet: &nightshiftd::packet::Packet) -> anyhow::Result<()> {
    let Some(receipt_path) = cli.nq_receipt.as_deref() else {
        return Ok(());
    };
    let nq_receipt = nightshiftd::mvp_a::NqReceiptRef::from_file(receipt_path)?;
    // Subject-boundary check: refuse to proceed if the NQ receipt
    // subject no longer matches the host-fs-state interpretation the
    // MVP-A packet's "Subject-boundary" section anchors against.
    // Anything not starting with `host:` is reported and refused;
    // the operator decides whether to re-anchor or report upstream.
    if !nq_receipt.subject.starts_with("host:") {
        anyhow::bail!(
            "MVP-A subject-boundary tripwire: NQ receipt subject `{}` does not match \
             expected `host:<name>` shape. Refusing to construct Wicket Intent. \
             See docs/working/decisions/MVP_A_SLICES_2_3_4_PACKET.md §Subject-boundary.",
            nq_receipt.subject
        );
    }
    let result = nightshiftd::mvp_a::run_pipeline(
        packet,
        &nq_receipt,
        &cli.mvp_a_out_dir,
        packet.produced_at,
    )?;
    match result {
        nightshiftd::mvp_a::MvpAResult::Cooked(outcome) => {
            eprintln!(
                "mvp-a: posture={} wicket-intent={} wicket-outcome={} \
                 wlp-authorization={} wlp-handling={}",
                outcome.posture_packet_path.display(),
                outcome.wicket_intent_path.display(),
                outcome.wicket_outcome_path.display(),
                outcome.wlp_authorization_path.display(),
                outcome.wlp_handling_path.display(),
            );
            eprintln!(
                "mvp-a-chain: nq={} wicket_input={} wlp_auth={} wlp_handling={}",
                nq_receipt.content_hash,
                outcome.wicket_receipt_input_hash,
                outcome.wlp_authorization_artifact_hash,
                outcome.wlp_handling_artifact_hash,
            );
        }
        nightshiftd::mvp_a::MvpAResult::Refused(refusal) => {
            // A.5: NS refused to cook because the NQ receipt's status
            // cannot be honestly represented in a Wicket Intent. The
            // refusal artifact at `refusal.refusal_artifact_path`
            // records the upstream NQ receipt and the reason; no
            // Wicket / WLP artifacts were produced. The posture
            // packet was still emitted (operator-visible record).
            eprintln!(
                "mvp-a-refused: artifact={} reason={} nq_status={} \
                 nq_content_hash={} nq_subject={}",
                refusal.refusal_artifact_path.display(),
                refusal.reason_code,
                refusal.nq_status,
                refusal.nq_content_hash,
                refusal.nq_subject,
            );
        }
        nightshiftd::mvp_a::MvpAResult::WlpAuthorizationRefused(refusal) => {
            // WLP3: NS cooked + classified the packet (Wicket Intent
            // and Outcome are on disk), but `packet.unsettled` carries
            // a ratified refusal kind. No WLP AuthorizationReceipt
            // was minted; the receiver-side gate refused to warrant
            // downstream reliance. The Wicket chain remains walkable;
            // the WLP-side warranty is intentionally absent.
            eprintln!(
                "mvp-a-wlp-refused: artifact={} reason={} unsettled_kinds={:?} \
                 wicket_intent={} wicket_outcome={} governor_receipts={:?}",
                refusal.refusal_artifact_path.display(),
                refusal.reason_code,
                refusal.unsettled_kinds,
                refusal.wicket_intent_path.display(),
                refusal.wicket_outcome_path.display(),
                refusal.governor_receipt_ids,
            );
        }
    }
    Ok(())
}

fn pipeline_opts(cli: &Cli) -> PipelineOptions {
    PipelineOptions {
        no_governor: cli.no_governor,
        continuity_configured: cli.continuity_configured,
        trigger: Some(cli.trigger.into()),
        liveness_threshold_seconds: cli.nq_liveness_threshold_secs,
        imported_basis_freshness_window_seconds: None,
    }
}

fn parse_finding_arg(s: &str) -> anyhow::Result<FindingKey> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    match parts.as_slice() {
        [source, detector, subject] => Ok(FindingKey {
            source: (*source).into(),
            detector: (*detector).into(),
            subject: (*subject).into(),
        }),
        _ => anyhow::bail!("finding must be `<source>:<detector>:<subject>`, got: {s}"),
    }
}
