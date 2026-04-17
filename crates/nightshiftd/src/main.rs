//! Night Shift daemon — `nightshift` CLI entry.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use nightshiftd::agenda::Agenda;
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::FixtureNqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
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

    /// Path to NQ fixture manifest (v1 uses a fixture source; real
    /// NQ client comes in a later slice).
    #[arg(long, global = true, default_value = "tests/fixtures/nq-manifest.json")]
    nq_fixture: PathBuf,

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
    }
}

fn run_watchbill_cmd(cli: &Cli, agenda_path: &std::path::Path, finding: &str) -> anyhow::Result<()> {
    let agenda = Agenda::from_yaml_file(agenda_path)?;
    let nq = FixtureNqSource::load(&cli.nq_fixture)?;
    let store = SqliteStore::open(&cli.store)?;
    let target = parse_finding_arg(finding)?;

    let opts = PipelineOptions {
        no_governor: cli.no_governor,
        continuity_configured: cli.continuity_configured,
        trigger: None,
    };

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts)?;

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

