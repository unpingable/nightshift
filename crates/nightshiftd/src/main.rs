//! Night Shift daemon — `nightshift` CLI entry.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use nightshiftd::agenda::Agenda;
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::{CliNqSource, FixtureNqSource, NqListFilter, NqSource};
use nightshiftd::nq_peek::{render_peek_text, PeekDocument};
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
use nightshiftd::posture::{list_postures, load_posture, render_list_row, render_show, PostureFilter};
use nightshiftd::store::sqlite::SqliteStore;

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

    #[command(subcommand)]
    command: Command,
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
    /// Run an agenda by path to its YAML definition.
    Run {
        /// Path to an agenda YAML file.
        agenda_path: PathBuf,

        /// Stable finding key to target: `<source>:<detector>:<subject>`.
        #[arg(long)]
        finding: String,
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
    }
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

fn run_watchbill_cmd(cli: &Cli, agenda_path: &std::path::Path, finding: &str) -> anyhow::Result<()> {
    let agenda = Agenda::from_yaml_file(agenda_path)?;
    let nq = build_nq_source(cli)?;
    let store = SqliteStore::open(&cli.store)?;
    let target = parse_finding_arg(finding)?;

    let opts = PipelineOptions {
        no_governor: cli.no_governor,
        continuity_configured: cli.continuity_configured,
        trigger: None,
    };

    let packet = run_watchbill(&agenda, &target, nq.as_ref(), &store, &opts)?;

    // v1: emit packet to stdout as YAML.
    let rendered = serde_yaml::to_string(&packet)?;
    println!("{rendered}");
    Ok(())
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

