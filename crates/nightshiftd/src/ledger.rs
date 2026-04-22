//! Run ledger — the append-only record of what the scheduler did.
//!
//! Run-ledger events are NOT authority receipts. Governor emits
//! authority receipts. Night Shift records run events. A run may
//! contain many receipts, but Night Shift does not manufacture
//! authority by logging itself.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunLedgerEventKind {
    RunCaptured,
    RunReconciled,
    RunCompleted,
    RunSurprise,
    RunScopeExpanded,
    RunPartial,
    RunEscalated,
    RunPreflightCleared,
    RunPreflightHold,
    RunPreflightCoordinate,
    RunPreflightBlocked,
    RunPreflightOverride,
    RunAttentionChanged,
    /// The NQ liveness gate refused to clear before capture (witness
    /// stale, skewed, or unreachable). The run terminated as Stale
    /// per the slice-5 contract; no findings were captured.
    RunLivenessGateFailed,
    /// The NQ liveness gate cleared. Recorded so the operator can
    /// inspect what age/verdict was observed at gate time.
    RunLivenessGateCleared,
    /// Reconcile-time live acquisition step. One explicit call to the
    /// finding source to capture current state for adjudication; the
    /// result is persisted into the run's bundle so subsequent
    /// adjudication is deterministic. Per GAP-deferred-run-split.md,
    /// this is the only reconcile-time live dependency; after this
    /// event the run has no further live NQ dependency.
    RunCurrentSnapshotAcquired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunLedgerEvent {
    pub event_id: String,
    pub run_id: String,
    pub kind: RunLedgerEventKind,
    pub at: DateTime<Utc>,
    #[serde(default)]
    pub payload: serde_json::Value,
}
