//! Store trait — the persistence boundary.
//!
//! v1 is SQLite only; `GAP-storage.md` specifies the contract so
//! Postgres (v2) is a natural implementation, not a rewrite.
//!
//! The store owns state, not intelligence.

pub mod sqlite;

use serde::{Deserialize, Serialize};

use crate::agenda::Agenda;
use crate::bundle::Bundle;
use crate::errors::Result;
use crate::finding::FindingKey;
use crate::ledger::RunLedgerEvent;
use crate::packet::Packet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrigger {
    Scheduled,
    Event,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: String,
    pub agenda_id: String,
    pub trigger: RunTrigger,
    pub target_finding_key: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Default, Clone)]
pub struct RunFilter {
    pub agenda_id: Option<String>,
    pub target_finding_key: Option<String>,
    pub limit: Option<usize>,
}

pub trait Store: Send + Sync {
    fn create_agenda(&self, agenda: &Agenda) -> Result<String>;
    fn get_agenda(&self, agenda_id: &str) -> Result<Option<Agenda>>;

    /// Create a new run record. `target` is the stable finding_key
    /// this run is targeting; nullable for non-finding-driven runs.
    fn create_run(
        &self,
        agenda_id: &str,
        trigger: RunTrigger,
        target: Option<&FindingKey>,
    ) -> Result<String>;

    /// Mark a run as completed (sets completed_at).
    fn complete_run(&self, run_id: &str) -> Result<()>;

    /// Fetch a single run's summary. Returns `None` if no such run
    /// exists. Used to distinguish captured-but-open runs from
    /// completed runs when the pipeline enforces one-shot reconcile.
    fn get_run_summary(&self, run_id: &str) -> Result<Option<RunSummary>>;

    fn append_run_event(&self, event: &RunLedgerEvent) -> Result<()>;
    fn list_events(&self, run_id: &str) -> Result<Vec<RunLedgerEvent>>;

    fn save_bundle(&self, run_id: &str, bundle: &Bundle) -> Result<()>;
    fn get_bundle(&self, run_id: &str) -> Result<Option<Bundle>>;

    fn save_packet(&self, run_id: &str, packet: &Packet) -> Result<()>;
    fn get_packet(&self, run_id: &str) -> Result<Option<Packet>>;

    fn list_runs(&self, filter: RunFilter) -> Result<Vec<RunSummary>>;
}
