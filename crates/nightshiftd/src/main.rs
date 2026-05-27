//! Night Shift daemon — `nightshift` CLI entry.

use std::path::PathBuf;

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
use nightshiftd::scheduled::{check_scheduled_idempotency, ScheduledOutcome};
use nightshiftd::store::{sqlite::SqliteStore, RunTrigger};

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

    /// Path to a real NQ SQLite database. When set, Night Shift
    /// shells out to `nq findings export --db <path>` and consumes
    /// the canonical snapshot contract (schema nq.finding_snapshot.v1).
    /// Overrides --nq-fixture.
    #[arg(long, global = true)]
    nq_db: Option<PathBuf>,

    /// Override the `nq` binary location. Otherwise resolved via
    /// NIGHTSHIFT_NQ_BIN env var, then PATH.
    #[arg(long, global = true)]
    nq_bin: Option<PathBuf>,

    /// Treat Continuity as configured for this deployment. v1 does
    /// not yet query Continuity; this flag controls preflight
    /// behavior for risky-class agendas (see GAP-parallel-ops.md).
    #[arg(long, global = true)]
    continuity_configured: bool,

    /// Path to NQ's liveness artifact (typically liveness.json
    /// alongside the NQ database). When set, Night Shift consults
    /// `nq liveness export` before capturing finding evidence; a
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
    /// See `docs/working/gaps/GAP-imported-basis-freshness.md` and
    /// `src/horizon_policy.rs`.
    #[arg(long, global = true)]
    horizon_policy: Option<PathBuf>,

    /// Path to the Governor JSON-RPC Unix socket. Required together
    /// with `--horizon-policy` to activate the horizon path —
    /// horizon-driven deferrals emit `record_receipt` calls so the
    /// tolerance declaration shows up in Governor's receipt chain.
    /// One fresh connection per call; no persistent connection.
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
}

#[derive(Subcommand, Debug)]
enum LivenessAction {
    /// Show what `nq liveness export` returns and what Night Shift's
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
    Run {
        /// Path to an agenda YAML file.
        agenda_path: PathBuf,

        /// Stable finding key to target: `<source>:<detector>:<subject>`.
        #[arg(long)]
        finding: String,
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Watchbill { action } => match action {
            WatchbillAction::Run {
                agenda_path,
                finding,
            } => run_watchbill_cmd(&cli, agenda_path, finding),
            WatchbillAction::Capture {
                agenda_path,
                finding,
            } => capture_cmd(&cli, agenda_path, finding),
            WatchbillAction::Reconcile { run_id } => reconcile_cmd(&cli, run_id),
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
        },
        Command::Liveness { action } => match action {
            LivenessAction::Peek { format } => liveness_peek_cmd(&cli, format),
        },
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
