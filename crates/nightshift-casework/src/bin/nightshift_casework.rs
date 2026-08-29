use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use nightshift_casework::{
    load_runs_at,
    server::{bind_loopback, serve, Api},
};

#[derive(Debug, Parser)]
#[command(
    name = "nightshift-casework",
    about = "Serve exact Nightshift packet and receipt snapshots as read-only casework"
)]
struct Args {
    /// Explicit run directory containing packet.v1.json and run-receipts.v1.json.
    #[arg(long = "run-dir", required = true)]
    run_dirs: Vec<PathBuf>,

    /// Loopback socket address. Non-loopback addresses are refused.
    #[arg(long, default_value = "127.0.0.1:0")]
    bind: SocketAddr,

    /// Optional deterministic currentness evaluation instant for qualification.
    #[arg(long)]
    evaluated_at: Option<DateTime<Utc>>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let evaluated_now = args.evaluated_at.unwrap_or_else(Utc::now);
    let runs = load_runs_at(&args.run_dirs, evaluated_now).context("load casework runs")?;
    let listener = bind_loopback(args.bind).context("bind read-only loopback API")?;
    println!(
        "nightshift-casework listening on http://{}",
        listener.local_addr()?
    );
    serve(listener, Api::new(runs)).context("serve read-only loopback API")
}
