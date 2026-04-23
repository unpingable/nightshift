//! Governor client — the real authority-boundary RPC surface.
//!
//! Distinct from `horizon_policy.rs`: horizon is **producer-local**
//! (Night Shift declares), while this module is where Night Shift
//! forwards lifecycle events into Governor's receipt chain for
//! archival. Governor is the archivist; this module is the wire.
//!
//! # Phase B.1 scope
//!
//! Narrow trait with one method: `record_receipt`. This is the
//! horizon-audit path: when Night Shift defers under a horizon,
//! it emits a lifecycle event so Governor's receipt chain carries
//! the declaration. The other two NS adapter methods —
//! `check_policy` (action-gate eval) and `authorize_transition`
//! (authority-ladder promotion) — come in later commits when the
//! NS pipeline has real action-propose and promotion points to
//! wire them to. Today NS is capped at advise with no mutation,
//! so those calls have no natural firing sites.
//!
//! # Transport
//!
//! Phase B.1 ships `FixtureGovernorClient` only — in-memory
//! call recorder for tests. Phase B.2 adds `JsonRpcGovernorClient`
//! speaking JSON-RPC 2.0 to the Governor daemon over a Unix
//! socket (per `agent_gov/src/governor/daemon.py:serve_unix`).
//! The trait shape does not grow for that transition.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::errors::{NightShiftError, Result};
use crate::horizon::HorizonBlock;

/// Lifecycle event kind. Governor's frozen v1 closed enum per
/// `agent_gov/src/governor/nightshift_adapter.py:EventKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Authority-role receipt. An agenda moved up the authority
    /// ladder.
    #[serde(rename = "agenda.promoted")]
    AgendaPromoted,
    /// Authority-role receipt. An action was authorized. This is
    /// the horizon-bearing event Night Shift emits on deferral —
    /// NS authorized itself to continue observing under horizon.
    #[serde(rename = "action.authorized")]
    ActionAuthorized,
    /// Authority-role receipt. An action ran.
    #[serde(rename = "action.applied")]
    ActionApplied,
    /// Authority-role receipt. An action was denied.
    #[serde(rename = "action.denied")]
    ActionDenied,
    /// Measurement-role receipt. Post-action verification.
    #[serde(rename = "action.verified")]
    ActionVerified,
    /// Measurement-role receipt. An operator was paged.
    #[serde(rename = "escalation.paged")]
    EscalationPaged,
}

/// Request payload for `nightshift.record_receipt`.
///
/// Mirrors `agent_gov/src/governor/nightshift_adapter.py:RecordReceiptRequest`.
/// Governor validates every field; we model the same constraints here
/// so fixtures produce payloads the real daemon will accept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordReceiptRequest {
    pub event_kind: EventKind,
    pub run_id: String,
    pub agenda_id: String,
    /// NS-computed sha256:<64-hex>. Subject of the event — typically
    /// a hash of the finding identity.
    pub subject_hash: String,
    /// sha256:<64-hex>. Hash of the evidence backing the decision.
    pub evidence_hash: String,
    /// sha256:<64-hex>. Hash of the policy artifact that justified
    /// the decision.
    pub policy_hash: String,
    /// Authority level the event transitions FROM (for promotion
    /// events). Required for `agenda.promoted`; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_level: Option<String>,
    /// Authority level the event transitions TO (for promotion
    /// events). Required for `agenda.promoted`; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_level: Option<String>,
    /// Optional tolerability declaration. Present on events where
    /// NS is declaring a tolerance window (typically
    /// `action.authorized` for horizon-driven deferral).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<HorizonBlock>,
}

/// Response payload for `nightshift.record_receipt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordReceiptResponse {
    pub receipt_id: String,
    pub receipt_hash: String,
}

/// Trait for speaking to Governor. Phase B.1 exposes one method;
/// Phase B.2+ extends for `check_policy` and `authorize_transition`
/// without changing the existing surface.
pub trait GovernorClient: Send + Sync {
    fn record_receipt(&self, request: &RecordReceiptRequest) -> Result<RecordReceiptResponse>;
}

/// In-memory test fixture. Records every `record_receipt` call so
/// tests can assert on the emitted payload. Thread-safe for use
/// through `&dyn GovernorClient`.
pub struct FixtureGovernorClient {
    calls: Mutex<Vec<RecordReceiptRequest>>,
    /// Counter for generating synthetic receipt_ids. Monotonic per
    /// fixture instance so tests can predict values.
    counter: Mutex<u64>,
}

impl Default for FixtureGovernorClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureGovernorClient {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            counter: Mutex::new(0),
        }
    }

    /// Snapshot of every `record_receipt` call made so far, in
    /// insertion order.
    pub fn recorded_calls(&self) -> Vec<RecordReceiptRequest> {
        self.calls
            .lock()
            .expect("fixture call log never poisoned")
            .clone()
    }

    /// Convenience: total call count.
    pub fn call_count(&self) -> usize {
        self.calls
            .lock()
            .expect("fixture call log never poisoned")
            .len()
    }
}

impl GovernorClient for FixtureGovernorClient {
    fn record_receipt(&self, request: &RecordReceiptRequest) -> Result<RecordReceiptResponse> {
        let mut counter = self
            .counter
            .lock()
            .map_err(|e| NightShiftError::Store(format!("fixture counter poisoned: {e}")))?;
        *counter += 1;
        let n = *counter;
        drop(counter);
        self.calls
            .lock()
            .map_err(|e| NightShiftError::Store(format!("fixture call log poisoned: {e}")))?
            .push(request.clone());
        Ok(RecordReceiptResponse {
            receipt_id: format!("fixture_receipt_{n:04}"),
            receipt_hash: format!("sha256:{:064x}", n),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::HorizonClass;

    fn basic_request(event_kind: EventKind) -> RecordReceiptRequest {
        RecordReceiptRequest {
            event_kind,
            run_id: "run_abc".into(),
            agenda_id: "wal-bloat-review".into(),
            subject_hash: format!("sha256:{}", "a".repeat(64)),
            evidence_hash: format!("sha256:{}", "b".repeat(64)),
            policy_hash: format!("sha256:{}", "c".repeat(64)),
            from_level: None,
            to_level: None,
            horizon: None,
        }
    }

    #[test]
    fn event_kind_serializes_as_dotted_wire_value() {
        let s = serde_json::to_string(&EventKind::ActionAuthorized).unwrap();
        assert_eq!(s, "\"action.authorized\"");
        let s = serde_json::to_string(&EventKind::AgendaPromoted).unwrap();
        assert_eq!(s, "\"agenda.promoted\"");
        let s = serde_json::to_string(&EventKind::EscalationPaged).unwrap();
        assert_eq!(s, "\"escalation.paged\"");
    }

    #[test]
    fn request_round_trips_through_json() {
        let r = RecordReceiptRequest {
            horizon: Some(HorizonBlock {
                class: HorizonClass::Hours,
                basis_id: Some("policy:defer".into()),
                basis_hash: Some(format!("sha256:{}", "d".repeat(64))),
                expiry: Some("2026-04-24T03:00:00Z".parse().unwrap()),
            }),
            ..basic_request(EventKind::ActionAuthorized)
        };
        let s = serde_json::to_string(&r).unwrap();
        // Governor expects horizon class on the wire as "kind", not "class".
        assert!(s.contains("\"kind\":\"hours\""), "wire must use kind: {s}");
        assert!(s.contains("\"event_kind\":\"action.authorized\""));
        let r2: RecordReceiptRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(r2.event_kind, EventKind::ActionAuthorized);
        assert_eq!(r2.horizon.unwrap().class, HorizonClass::Hours);
    }

    #[test]
    fn from_to_level_omitted_when_none() {
        let r = basic_request(EventKind::ActionAuthorized);
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("from_level"));
        assert!(!s.contains("to_level"));
    }

    #[test]
    fn horizon_omitted_when_none() {
        let r = basic_request(EventKind::ActionAuthorized);
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("horizon"));
    }

    #[test]
    fn fixture_client_records_every_call_in_order() {
        let client = FixtureGovernorClient::new();
        let r1 = basic_request(EventKind::ActionAuthorized);
        let r2 = basic_request(EventKind::EscalationPaged);
        let resp1 = client.record_receipt(&r1).unwrap();
        let resp2 = client.record_receipt(&r2).unwrap();
        assert_ne!(resp1.receipt_id, resp2.receipt_id);
        assert_eq!(client.call_count(), 2);
        let calls = client.recorded_calls();
        assert_eq!(calls[0].event_kind, EventKind::ActionAuthorized);
        assert_eq!(calls[1].event_kind, EventKind::EscalationPaged);
    }

    #[test]
    fn fixture_client_receipt_ids_are_monotonic() {
        let client = FixtureGovernorClient::new();
        let r = basic_request(EventKind::ActionAuthorized);
        let a = client.record_receipt(&r).unwrap().receipt_id;
        let b = client.record_receipt(&r).unwrap().receipt_id;
        assert_eq!(a, "fixture_receipt_0001");
        assert_eq!(b, "fixture_receipt_0002");
    }
}
