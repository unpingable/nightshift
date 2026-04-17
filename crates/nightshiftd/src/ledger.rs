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
