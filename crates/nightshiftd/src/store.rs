//! Store trait — the persistence boundary.
//!
//! v1 is SQLite only; `GAP-storage.md` specifies the contract so
//! Postgres (v2) is a natural implementation, not a rewrite.
//!
//! The store owns state, not intelligence.

pub mod sqlite;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agenda::Agenda;
use crate::bundle::Bundle;
use crate::errors::Result;
use crate::finding::FindingKey;
use crate::horizon::{HorizonClass, PriorTolerance};
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

/// Persisted tolerance grant for one finding.
///
/// Written when the reconciler sees `HorizonAction::Defer` and the
/// consumer must carry the grant forward to the next run. Read when
/// a later run reconciles the same finding and needs to distinguish
/// "previously tolerated, now expired" from "brand new incident"
/// (the four-way A5 distinction).
///
/// Keyed by `FindingKey` because tolerance is a property of the
/// finding across runs, not of any single run. `granted_in_run_id`
/// is carried for diagnostics; it is not the key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToleranceRecord {
    pub finding_key: FindingKey,
    pub basis_id: String,
    pub basis_hash: String,
    pub prior_class: HorizonClass,
    pub expires_at: DateTime<Utc>,
    pub granted_at: DateTime<Utc>,
    pub granted_in_run_id: String,
}

impl ToleranceRecord {
    /// Project the record down to the consumer-logic shape the
    /// horizon module's `action_for` accepts. Drops provenance
    /// (granted_at, granted_in_run_id) that is useful for operator
    /// rendering but not for the decision function.
    pub fn to_prior_tolerance(&self) -> PriorTolerance {
        PriorTolerance {
            basis_id: self.basis_id.clone(),
            basis_hash: self.basis_hash.clone(),
            prior_class: self.prior_class,
            expired_at: self.expires_at,
        }
    }
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

    /// Persist a tolerance grant for a finding. Upsert semantics:
    /// writing under the same `finding_key` replaces the prior
    /// record (matches the single-grant-per-finding spec).
    fn save_tolerance(&self, record: &ToleranceRecord) -> Result<()>;

    /// Fetch the tolerance grant for a finding, if any. Returns
    /// `None` if no grant was ever written OR if the grant has been
    /// cleared by `clear_tolerance`.
    fn load_tolerance(&self, key: &FindingKey) -> Result<Option<ToleranceRecord>>;

    /// Remove the tolerance grant for a finding. Called on the
    /// escalate paths (`EscalateExpired`, `EscalateBasisInvalidated`)
    /// so the next run sees `None` and does not re-apply the stale
    /// grant.
    fn clear_tolerance(&self, key: &FindingKey) -> Result<()>;
}
