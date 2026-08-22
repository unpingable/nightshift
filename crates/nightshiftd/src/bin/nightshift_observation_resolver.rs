//! Read-only Nightshift observation resolver for AG's frozen
//! `ag.governed-loop.observation-resolution/v2` boundary.
//!
//! One-shot process contract: read one exact AG observation request from
//! stdin, write exactly one canonical resolution JSON document to stdout,
//! exit 0. Malformed requests, misconfiguration, and store IO failures exit
//! non-zero with a diagnostic on stderr; they are never encoded as evidence
//! statuses. This binary opens the canonical store strictly read-only and
//! has no cycle-mutation, lease, AG, Docket, or executor surface.

use std::io::Read as _;
use std::path::PathBuf;

use anyhow::{bail, Context as _};
use clap::Parser;

use nightshiftd::canonical_store::CanonicalStore;
use nightshiftd::observation_resolver::{
    resolve_observation, AgObservationRequestV1, ObservationResolverConfigV1,
};

#[derive(Debug, Parser)]
#[command(
    name = "nightshift-observation-resolver",
    version,
    about = "Read-only Nightshift observation-evidence resolver for AG"
)]
struct Arguments {
    /// Canonical Nightshift store, opened strictly read-only.
    #[arg(long)]
    store: PathBuf,
    /// Exact resolver identity AG is configured to expect.
    #[arg(long)]
    resolver_id: String,
    /// Bounded evidence window: fresh_until = evaluated_at + this TTL.
    #[arg(long)]
    default_ttl_ms: u64,
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::parse();
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("read observation request from stdin")?;
    let request: AgObservationRequestV1 =
        serde_json::from_str(&input).context("parse observation request")?;
    let config = ObservationResolverConfigV1 {
        resolver_id: arguments.resolver_id,
        default_ttl_ms: arguments.default_ttl_ms,
    };
    let store = CanonicalStore::open_read_only(&arguments.store)
        .context("open canonical store read-only")?;
    let resolution = resolve_observation(&store, &request, &config)
        .map_err(anyhow::Error::new)
        .context("resolve observation")?;
    let body = serde_jcs::to_vec(&resolution).context("canonicalize resolution")?;
    let body = String::from_utf8(body).context("resolution is UTF-8")?;
    if body.contains('\n') {
        bail!("canonical resolution must be a single JSON line");
    }
    println!("{body}");
    Ok(())
}
