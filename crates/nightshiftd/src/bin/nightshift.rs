//! Canonical Nightshift runtime CLI. It observes and delegates exact proposals
//! to AG; it has no standing, authorization, Docket, or executor surface.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{de::DeserializeOwned, Serialize};

use nightshiftd::ag_port::{
    parse_ag_refusal, AgLoopCtlPortV1, AgOccurrencePortV1, AgOpenOccurrenceRequestV1,
};
use nightshiftd::canonical_runtime::{
    CanonicalCycleRequestV1, CanonicalRuntime, CycleRunOutcomeV1,
};
use nightshiftd::canonical_store::{
    AgOccurrenceReferenceV1, CanonicalStore, ObservationCycleId, ObservationCycleV1,
};
use nightshiftd::currentness::{
    CommandPresentEvidencePortV1, PresentEvidencePortV1, PresentEvidenceQueryV1, QualifiedSupportV1,
};

#[derive(Debug, Parser)]
#[command(
    name = "nightshift",
    version,
    about = "Canonical temporal observation and attention office"
)]
struct Arguments {
    #[arg(long, global = true, default_value = "nightshift.sqlite")]
    store: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Cycle {
        #[command(subcommand)]
        command: CycleCommand,
    },
}

#[derive(Debug, Subcommand)]
enum CycleCommand {
    /// Run the sole production observation-cycle path.
    Run {
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        present_evidence_resolver: PathBuf,
        #[arg(long)]
        ag_loopctl: Option<PathBuf>,
        #[arg(long)]
        ag_database: Option<PathBuf>,
        #[arg(long)]
        ag_observation_resolver: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Read one exact AG occurrence through AG and record status only.
    SyncAg {
        #[arg(long)]
        cycle_id: String,
        #[arg(long)]
        ag_loopctl: PathBuf,
        #[arg(long)]
        ag_database: PathBuf,
        #[arg(long)]
        ag_observation_resolver: PathBuf,
        #[arg(long)]
        observed_at: String,
    },
    /// Consume an exact durable AG refusal outcome. A refusal closes this
    /// observation cycle and creates no program-counter transition.
    RecordRefusal {
        #[arg(long)]
        cycle_id: String,
        #[arg(long)]
        refusal: PathBuf,
        #[arg(long)]
        observed_at: String,
    },
    /// Apply restart noninheritance, then query AG-bound cycles by status only.
    Recover {
        #[arg(long)]
        ag_loopctl: PathBuf,
        #[arg(long)]
        ag_database: PathBuf,
        #[arg(long)]
        ag_observation_resolver: PathBuf,
        #[arg(long)]
        observed_at: String,
    },
    Show {
        #[arg(long)]
        cycle_id: String,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Read-only export of every persisted observation with one exact
    /// observation identity, including lineage position. It never mutates
    /// cycle state, acquires leases, or contacts AG.
    ExportObservation {
        #[arg(long)]
        observation_id: String,
    },
    List,
    Replay {
        #[arg(long)]
        cycle_id: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "snake_case")]
enum OutputFormat {
    Json,
    Text,
}

struct NoAgPort;

impl AgOccurrencePortV1 for NoAgPort {
    fn open_occurrence(
        &mut self,
        _request: &AgOpenOccurrenceRequestV1,
    ) -> Result<AgOccurrenceReferenceV1, String> {
        Err("posture-only cycle has no AG adapter".into())
    }

    fn status(&mut self, _: &str, _: &str) -> Result<AgOccurrenceReferenceV1, String> {
        Err("posture-only cycle has no AG adapter".into())
    }
}

struct NoPresentEvidencePort;

impl PresentEvidencePortV1 for NoPresentEvidencePort {
    fn resolve(&mut self, _: &PresentEvidenceQueryV1) -> Result<QualifiedSupportV1, String> {
        Err("recovery never resolves persisted evidence currentness".into())
    }
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();
    match arguments.command {
        Command::Cycle { command } => run_cycle_command(&arguments.store, command),
    }
}

fn run_cycle_command(store_path: &Path, command: CycleCommand) -> anyhow::Result<()> {
    match command {
        CycleCommand::Run {
            request,
            present_evidence_resolver,
            ag_loopctl,
            ag_database,
            ag_observation_resolver,
            format,
        } => {
            let request: CanonicalCycleRequestV1 = read_exact(&request)?;
            let has_proposal = request.proposal.is_some();
            let mut store = CanonicalStore::open(store_path)?;
            let mut support = CommandPresentEvidencePortV1::new(present_evidence_resolver)
                .map_err(anyhow::Error::msg)?;
            let outcome = if has_proposal {
                let (program, database, resolver) =
                    match (ag_loopctl, ag_database, ag_observation_resolver) {
                        (Some(program), Some(database), Some(resolver)) => {
                            (program, database, resolver)
                        }
                        _ => bail!(
                            "exact work requires --ag-loopctl, --ag-database, and --ag-observation-resolver"
                        ),
                    };
                let mut ag = AgLoopCtlPortV1::new(program, database, resolver)
                    .map_err(anyhow::Error::msg)?;
                CanonicalRuntime::new(&mut store, &mut support, &mut ag).run_cycle(request)?
            } else {
                if ag_loopctl.is_some()
                    || ag_database.is_some()
                    || ag_observation_resolver.is_some()
                {
                    bail!("AG options are forbidden for a posture-only cycle");
                }
                let mut ag = NoAgPort;
                CanonicalRuntime::new(&mut store, &mut support, &mut ag).run_cycle(request)?
            };
            render_outcome(&outcome, format)
        }
        CycleCommand::SyncAg {
            cycle_id,
            ag_loopctl,
            ag_database,
            ag_observation_resolver,
            observed_at,
        } => {
            let id = ObservationCycleId::parse(cycle_id)?;
            let now = parse_time(&observed_at)?;
            let mut store = CanonicalStore::open(store_path)?;
            let mut support = NoPresentEvidencePort;
            let mut ag = AgLoopCtlPortV1::new(ag_loopctl, ag_database, ag_observation_resolver)
                .map_err(anyhow::Error::msg)?;
            write_exact(
                &CanonicalRuntime::new(&mut store, &mut support, &mut ag).sync_ag(&id, now)?,
            )
        }
        CycleCommand::Recover {
            ag_loopctl,
            ag_database,
            ag_observation_resolver,
            observed_at,
        } => {
            let now = parse_time(&observed_at)?;
            let mut store = CanonicalStore::open(store_path)?;
            let candidates = store.recover_after_restart(now)?;
            let mut support = NoPresentEvidencePort;
            let mut ag = AgLoopCtlPortV1::new(ag_loopctl, ag_database, ag_observation_resolver)
                .map_err(anyhow::Error::msg)?;
            let mut recovered = Vec::new();
            for cycle in candidates {
                if cycle.prepared_ag_request.is_some() {
                    recovered.push(
                        CanonicalRuntime::new(&mut store, &mut support, &mut ag)
                            .sync_ag(&cycle.cycle_id, now)
                            .with_context(|| {
                                format!("AG status failed for {}", cycle.cycle_id.as_str())
                            })?,
                    );
                } else {
                    recovered.push(cycle);
                }
            }
            write_exact(&recovered)
        }
        CycleCommand::RecordRefusal {
            cycle_id,
            refusal,
            observed_at,
        } => {
            let id = ObservationCycleId::parse(cycle_id)?;
            let now = parse_time(&observed_at)?;
            let exact_outcome: serde_json::Value = read_exact(&refusal)?;
            let mut store = CanonicalStore::open(store_path)?;
            let cycle = store.get_cycle(&id)?;
            let request = cycle
                .prepared_ag_request
                .as_ref()
                .context("cycle has no exact prepared AG request")?;
            let refusal =
                parse_ag_refusal(exact_outcome, &request.campaign_id, &request.occurrence_id)
                    .map_err(anyhow::Error::msg)?;
            write_exact(&store.record_ag_refusal(&id, &cycle.state_digest, refusal, now)?)
        }
        CycleCommand::Show { cycle_id, format } => {
            let store = CanonicalStore::open(store_path)?;
            let cycle = store.get_cycle(&ObservationCycleId::parse(cycle_id)?)?;
            match format {
                OutputFormat::Json => write_exact(&cycle),
                OutputFormat::Text => render_cycle_text(&cycle),
            }
        }
        CycleCommand::List => write_exact(&CanonicalStore::open(store_path)?.list_cycles()?),
        CycleCommand::ExportObservation { observation_id } => {
            let store = CanonicalStore::open(store_path)?;
            write_exact(
                &store
                    .export_observation(&observation_id)
                    .map_err(anyhow::Error::msg)?,
            )
        }
        CycleCommand::Replay { cycle_id } => {
            let store = CanonicalStore::open(store_path)?;
            write_exact(&store.replay(&ObservationCycleId::parse(cycle_id)?)?)
        }
    }
}

fn render_outcome(outcome: &CycleRunOutcomeV1, format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => write_exact(outcome),
        OutputFormat::Text => {
            let cycle = match outcome {
                CycleRunOutcomeV1::Missed { cycle }
                | CycleRunOutcomeV1::PostureOnly { cycle }
                | CycleRunOutcomeV1::AgOccurrenceOpened { cycle } => cycle,
            };
            render_cycle_text(cycle)
        }
    }
}

fn render_cycle_text(cycle: &ObservationCycleV1) -> anyhow::Result<()> {
    println!("cycle: {}", cycle.cycle_id.as_str());
    println!("slot: {}", cycle.slot.slot_id.as_str());
    println!("status: {:?}", cycle.status);
    println!("state_digest: {}", cycle.state_digest);
    println!("headline_display_only: {:?}", cycle.headline());
    Ok(())
}

fn read_exact<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let value = T::deserialize(&mut deserializer)
        .with_context(|| format!("decode exact JSON {}", path.display()))?;
    deserializer
        .end()
        .with_context(|| format!("reject trailing JSON in {}", path.display()))?;
    Ok(value)
}

fn write_exact<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let bytes = serde_jcs::to_vec(value).context("canonicalize output")?;
    println!(
        "{}",
        String::from_utf8(bytes).context("canonical JSON is UTF-8")?
    );
    Ok(())
}

fn parse_time(value: &str) -> anyhow::Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .context("observed_at must be RFC3339")?
        .with_timezone(&Utc))
}
