use std::{
    fs,
    io::{self, Write as _},
    path::PathBuf,
};

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use nightshift_foreman::ForemanStore;

#[derive(Parser)]
#[command(
    name = "nightshift-foreman",
    about = "Durable non-authorizing local agent-compute scheduler"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Admit {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        packet: PathBuf,
        #[arg(long)]
        admission: PathBuf,
        #[arg(long)]
        profile: PathBuf,
        #[arg(long)]
        evaluated_at: String,
    },
    Run {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        work_item: String,
        #[arg(long)]
        recorded_at: String,
    },
    Resume {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        work_item: String,
        #[arg(long)]
        attempt_id: String,
        #[arg(long)]
        recorded_at: String,
    },
    AcceptEvent {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        event: PathBuf,
    },
    AcceptReceipt {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
    },
    AcceptNotStarted {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
    },
    Status {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
    },
    Events {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
    },
    Close {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        updated_at: String,
    },
    ExportLive {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
    },
    ExportFinal {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        run_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Admit {
            db,
            packet,
            admission,
            profile,
            evaluated_at,
        } => {
            let store = ForemanStore::open(db)?;
            let run_id = store.admit(
                &read(&packet)?,
                &read(&admission)?,
                &read(&profile)?,
                instant(&evaluated_at)?,
            )?;
            print_json(&serde_json::json!({"run_id": run_id}))?;
        }
        Command::Run {
            db,
            run_id,
            work_item,
            recorded_at,
        } => {
            let store = ForemanStore::open(db)?;
            let request = store.prepare_attempt(&run_id, &work_item, instant(&recorded_at)?)?;
            store.record_dispatch_requested(
                &run_id,
                &work_item,
                &request.attempt_id,
                instant(&recorded_at)?,
            )?;
            print_json(&request)?;
        }
        Command::Resume {
            db,
            run_id,
            work_item,
            attempt_id,
            recorded_at,
        } => {
            let store = ForemanStore::open(db)?;
            store.record_resume_requested(
                &run_id,
                &work_item,
                &attempt_id,
                instant(&recorded_at)?,
            )?;
            print_json(&store.projection(&run_id)?)?;
        }
        Command::AcceptEvent { db, event } => {
            ForemanStore::open(db)?.accept_adapter_event(&read(&event)?)?;
        }
        Command::AcceptReceipt { db, receipt } => {
            ForemanStore::open(db)?.accept_terminal_receipt(&read(&receipt)?)?;
        }
        Command::AcceptNotStarted { db, receipt } => {
            ForemanStore::open(db)?.accept_not_started(&read(&receipt)?)?;
        }
        Command::Status { db, run_id } | Command::ExportLive { db, run_id } => {
            print_json(&ForemanStore::open(db)?.projection(&run_id)?)?;
        }
        Command::Events { db, run_id } => {
            print_json(&ForemanStore::open(db)?.export_events(&run_id)?)?;
        }
        Command::Close {
            db,
            run_id,
            updated_at,
        } => write_raw(&ForemanStore::open(db)?.close(&run_id, instant(&updated_at)?)?)?,
        Command::ExportFinal { db, run_id } => {
            write_raw(&ForemanStore::open(db)?.export_final(&run_id)?)?;
        }
    }
    Ok(())
}

fn instant(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid RFC3339 timestamp: {value}"))?
        .with_timezone(&Utc))
}

fn read(path: &PathBuf) -> Result<Vec<u8>> {
    fs::read(path).with_context(|| format!("cannot read {}", path.display()))
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_raw(&bytes)
}

fn write_raw(bytes: &[u8]) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    Ok(())
}
