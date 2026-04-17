//! Night Shift daemon — `nightshift` CLI entry.

use clap::{Parser, Subcommand};

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
    store: std::path::PathBuf,

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
    /// Run an agenda by id or path.
    Run {
        /// Path to an agenda YAML file, or an agenda_id already known to the store.
        agenda: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Watchbill { action } => match action {
            WatchbillAction::Run { agenda } => {
                println!(
                    "nightshift watchbill run: agenda={} no_governor={} store={}",
                    agenda,
                    cli.no_governor,
                    cli.store.display()
                );
                println!("(v1 pipeline not yet wired — commit A scaffolding only)");
                Ok(())
            }
        },
    }
}
