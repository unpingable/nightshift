//! Operator attention state, durable across runs.
//!
//! Slice 3 (incremental — ack + silence only): the operator can
//! durably touch a finding via `nightshift attention ack` /
//! `attention silence`. The attention row is keyed on
//! `(agenda_id, finding_key)` per `GAP-attention-state.md`'s
//! "attention is keyed to stable finding identity." The reconciler
//! consults this store on each run and applies a *read-time
//! projection* onto the packet's attention block.
//!
//! Invariants enforced here:
//! - **Ack is not closure.** An ack with `ack_expires_at` set
//!   produces a packet with `attention_state = Acknowledged` only
//!   while `now < ack_expires_at`. After expiry the projection
//!   returns `AppliedAttention::Expired` and the packet renders as
//!   `Unowned` with urgency bumped one step.
//! - **Silence is not handling.** A silenced finding still
//!   reconciles; the packet carries the silence detail. `runs list`
//!   without `--held-only` still shows it.
//! - **Suppression needs expiry or reason.** A silence row requires
//!   both `silence_until` and `silence_reason` at the construction
//!   site; the CLI enforces this at parse time, and `SilenceRow`
//!   requires both fields in its constructor.
//! - **Attention never raises authority.** The applied attention
//!   does not feed into `effective_ceiling`; the reconciler reads it
//!   only for packet rendering and urgency adjustment.
//!
//! Investigate / handoff / request-revalidation verbs are explicitly
//! out of scope; see roadmap Slice 3 status note. Each is a smaller
//! follow-on (different attention_state value, no new TTL machinery)
//! and can land independently when the forcing case shows up.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::finding::FindingKey;
use crate::packet::OperationalUrgency;

/// The subset of operator-attention states this slice persists.
/// `Acknowledged` and `Silenced` are the two operator verbs Slice 3
/// ships. The packet's `AttentionState` enum is broader (includes
/// `Investigating`, `HandedOff`, `WatchUntil`, `Unowned`); when more
/// operator verbs land, this enum extends and `to_packet_state`
/// expands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistedAttentionState {
    Acknowledged,
    Silenced,
}

impl PersistedAttentionState {
    pub fn as_str(self) -> &'static str {
        match self {
            PersistedAttentionState::Acknowledged => "acknowledged",
            PersistedAttentionState::Silenced => "silenced",
        }
    }

    pub fn parse_token(s: &str) -> Option<Self> {
        match s {
            "acknowledged" => Some(Self::Acknowledged),
            "silenced" => Some(Self::Silenced),
            _ => None,
        }
    }
}

/// Frozen six-value re-ack disposition from
/// `architecture/GAP-reack-doctrine.md`. Required on any ack that
/// finds a prior attention row for the same key — re-ack is
/// mini-re-triage, not a courtesy tap. Initial ack (no prior row)
/// may omit the disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReAckDisposition {
    Advanced,
    UnchangedWaiting,
    Blocked,
    HandedOff,
    Escalated,
    Resolved,
}

impl ReAckDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            ReAckDisposition::Advanced => "advanced",
            ReAckDisposition::UnchangedWaiting => "unchanged-waiting",
            ReAckDisposition::Blocked => "blocked",
            ReAckDisposition::HandedOff => "handed-off",
            ReAckDisposition::Escalated => "escalated",
            ReAckDisposition::Resolved => "resolved",
        }
    }

    pub fn parse_token(s: &str) -> Option<Self> {
        match s {
            "advanced" => Some(Self::Advanced),
            "unchanged-waiting" => Some(Self::UnchangedWaiting),
            "blocked" => Some(Self::Blocked),
            "handed-off" => Some(Self::HandedOff),
            "escalated" => Some(Self::Escalated),
            "resolved" => Some(Self::Resolved),
            _ => None,
        }
    }
}

/// Persisted attention row. One per `(agenda_id, finding_key)`.
///
/// Construction goes through `AttentionRow::ack` /
/// `AttentionRow::silence` so the at-rest invariants (silence has
/// both `silence_until` and `silence_reason`; ack carries an
/// `acknowledged_at`) are enforced by the type system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttentionRow {
    pub agenda_id: String,
    pub finding_key: FindingKey,
    pub state: PersistedAttentionState,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub ack_expires_at: Option<DateTime<Utc>>,
    pub silence_until: Option<DateTime<Utc>>,
    pub silence_reason: Option<String>,
    pub owner: Option<String>,
    pub note: Option<String>,
    /// Required when re-acking an existing row; None on the initial
    /// ack of a finding that had no prior attention.
    pub disposition: Option<ReAckDisposition>,
    pub last_touched_by: String,
    pub last_touched_at: DateTime<Utc>,
}

impl AttentionRow {
    /// Build an `Acknowledged` row. `ack_expires_at` is optional; the
    /// projection treats an ack with no expiry as never-expiring,
    /// which is *not* the recommended path (operators should set a
    /// TTL), but is permitted for the special case where re-alert is
    /// already governed by agenda criticality.
    pub fn ack(
        agenda_id: String,
        finding_key: FindingKey,
        operator: String,
        ack_expires_at: Option<DateTime<Utc>>,
        note: Option<String>,
        disposition: Option<ReAckDisposition>,
    ) -> Self {
        let now = Utc::now();
        Self {
            agenda_id,
            finding_key,
            state: PersistedAttentionState::Acknowledged,
            acknowledged_at: Some(now),
            ack_expires_at,
            silence_until: None,
            silence_reason: None,
            owner: None,
            note,
            disposition,
            last_touched_by: operator,
            last_touched_at: now,
        }
    }

    /// Build a `Silenced` row. Both `silence_until` and
    /// `silence_reason` are required at the type level — per
    /// GAP-attention-state invariant: "Suppression needs an expiry
    /// or a reason. No open-ended silence."
    pub fn silence(
        agenda_id: String,
        finding_key: FindingKey,
        operator: String,
        silence_until: DateTime<Utc>,
        silence_reason: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            agenda_id,
            finding_key,
            state: PersistedAttentionState::Silenced,
            acknowledged_at: None,
            ack_expires_at: None,
            silence_until: Some(silence_until),
            silence_reason: Some(silence_reason),
            owner: None,
            note: None,
            disposition: None,
            last_touched_by: operator,
            last_touched_at: now,
        }
    }
}

/// Read-time projection result. The reconciler calls
/// `project_attention` and applies the outcome to the packet's
/// attention block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAttention {
    /// No prior attention row, or projection has nothing to apply.
    None,
    /// Acknowledged and still within TTL (or TTL absent).
    Acknowledged {
        acknowledged_at: DateTime<Utc>,
        ack_expires_at: Option<DateTime<Utc>>,
        owner: Option<String>,
        note: Option<String>,
        disposition: Option<ReAckDisposition>,
    },
    /// Silenced and within the silence window.
    Silenced {
        silence_until: DateTime<Utc>,
        silence_reason: String,
    },
    /// A prior row exists but its TTL has elapsed. The packet
    /// renders as `Unowned` with `bump_urgency()` applied; the row
    /// stays in the store (next operator interaction is a re-ack
    /// per `GAP-reack-doctrine.md`).
    Expired {
        kind: PersistedAttentionState,
        expired_at: DateTime<Utc>,
    },
}

impl AppliedAttention {
    /// One-step urgency bump used when an ack TTL expires and the
    /// finding re-surfaces. Cap at `Critical`.
    pub fn bump_urgency(u: OperationalUrgency) -> OperationalUrgency {
        match u {
            OperationalUrgency::Low => OperationalUrgency::Medium,
            OperationalUrgency::Medium => OperationalUrgency::High,
            OperationalUrgency::High => OperationalUrgency::Critical,
            OperationalUrgency::Critical => OperationalUrgency::Critical,
        }
    }
}

/// Apply a projection result to a packet's attention block. Returns
/// a short label describing what was applied, suitable for the
/// `RunAttentionChanged` ledger event payload. Returns `None` when
/// nothing was applied (no prior row).
///
/// Invariants enforced here:
/// - Operator attention does NOT touch authority. The packet's
///   `proposed_action.requested_authority_level` and
///   `authority_result` are not modified.
/// - Operator attention DOES set `attention_state`, the relevant
///   timestamps, and (on `Silenced`) `re_alert_after = silence_until`
///   so the operator-facing `next check` field is meaningful.
/// - `Expired` does not change `attention_state` (the packet's
///   default — typically `Unowned`, or `WatchUntil` from horizon —
///   wins) but DOES bump `operational_urgency` by one step.
pub fn apply_to_packet(
    attention: &mut crate::packet::Attention,
    applied: &AppliedAttention,
) -> Option<&'static str> {
    use crate::packet::AttentionState;
    match applied {
        AppliedAttention::None => None,
        AppliedAttention::Acknowledged {
            acknowledged_at,
            ack_expires_at,
            owner,
            note,
            ..
        } => {
            attention.attention_state = AttentionState::Acknowledged;
            attention.acknowledged_at = Some(*acknowledged_at);
            attention.ack_expires_at = *ack_expires_at;
            attention.owner = owner.clone();
            attention.last_touched_at = Some(*acknowledged_at);
            // Operator ack overwrites any horizon-driven re_alert_after
            // so the operator's TTL is what the next scheduled run
            // consults. `tolerance_basis_id`/`hash` stay untouched —
            // those record Governor receipts, which an operator ack
            // cannot retract.
            attention.re_alert_after = *ack_expires_at;
            if note.is_some() {
                attention.silence_reason = note.clone();
            }
            Some("acknowledged")
        }
        AppliedAttention::Silenced {
            silence_until,
            silence_reason,
        } => {
            attention.attention_state = AttentionState::Silenced;
            attention.silence_reason = Some(silence_reason.clone());
            attention.re_alert_after = Some(*silence_until);
            attention.last_touched_at = Some(*silence_until);
            Some("silenced")
        }
        AppliedAttention::Expired { .. } => {
            attention.operational_urgency =
                AppliedAttention::bump_urgency(attention.operational_urgency);
            Some("expired")
        }
    }
}

/// Project a persisted row onto the current moment.
pub fn project_attention(row: Option<&AttentionRow>, now: DateTime<Utc>) -> AppliedAttention {
    let Some(row) = row else {
        return AppliedAttention::None;
    };
    match row.state {
        PersistedAttentionState::Acknowledged => match row.ack_expires_at {
            Some(exp) if exp <= now => AppliedAttention::Expired {
                kind: PersistedAttentionState::Acknowledged,
                expired_at: exp,
            },
            _ => AppliedAttention::Acknowledged {
                acknowledged_at: row.acknowledged_at.unwrap_or(row.last_touched_at),
                ack_expires_at: row.ack_expires_at,
                owner: row.owner.clone(),
                note: row.note.clone(),
                disposition: row.disposition,
            },
        },
        PersistedAttentionState::Silenced => match (row.silence_until, &row.silence_reason) {
            (Some(until), Some(reason)) if until > now => AppliedAttention::Silenced {
                silence_until: until,
                silence_reason: reason.clone(),
            },
            (Some(until), _) => AppliedAttention::Expired {
                kind: PersistedAttentionState::Silenced,
                expired_at: until,
            },
            _ => AppliedAttention::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn key() -> FindingKey {
        FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "h:/db".into(),
        }
    }

    #[test]
    fn projection_returns_none_for_missing_row() {
        let outcome = project_attention(None, Utc::now());
        assert_eq!(outcome, AppliedAttention::None);
    }

    #[test]
    fn projection_returns_acknowledged_when_within_ttl() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 10, 0, 0).unwrap();
        let row = AttentionRow::ack(
            "a".into(),
            key(),
            "alice".into(),
            Some(Utc.with_ymd_and_hms(2026, 5, 27, 16, 0, 0).unwrap()),
            Some("looking now".into()),
            None,
        );
        let outcome = project_attention(Some(&row), now);
        match outcome {
            AppliedAttention::Acknowledged { ack_expires_at, .. } => {
                assert!(ack_expires_at.is_some());
            }
            other => panic!("expected Acknowledged, got {other:?}"),
        }
    }

    #[test]
    fn projection_returns_expired_when_ttl_elapsed() {
        let expired_at = Utc.with_ymd_and_hms(2026, 5, 27, 9, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 10, 0, 0).unwrap();
        let row = AttentionRow::ack(
            "a".into(),
            key(),
            "alice".into(),
            Some(expired_at),
            None,
            None,
        );
        let outcome = project_attention(Some(&row), now);
        assert_eq!(
            outcome,
            AppliedAttention::Expired {
                kind: PersistedAttentionState::Acknowledged,
                expired_at,
            }
        );
    }

    #[test]
    fn projection_returns_silenced_when_within_window() {
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 10, 0, 0).unwrap();
        let until = Utc.with_ymd_and_hms(2026, 5, 27, 18, 0, 0).unwrap();
        let row = AttentionRow::silence(
            "a".into(),
            key(),
            "alice".into(),
            until,
            "rolling restart underway".into(),
        );
        let outcome = project_attention(Some(&row), now);
        assert_eq!(
            outcome,
            AppliedAttention::Silenced {
                silence_until: until,
                silence_reason: "rolling restart underway".into(),
            }
        );
    }

    #[test]
    fn projection_returns_expired_when_silence_window_elapsed() {
        let until = Utc.with_ymd_and_hms(2026, 5, 27, 9, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 27, 10, 0, 0).unwrap();
        let row = AttentionRow::silence(
            "a".into(),
            key(),
            "alice".into(),
            until,
            "rolling restart underway".into(),
        );
        let outcome = project_attention(Some(&row), now);
        assert_eq!(
            outcome,
            AppliedAttention::Expired {
                kind: PersistedAttentionState::Silenced,
                expired_at: until,
            }
        );
    }

    #[test]
    fn bump_urgency_steps_up_and_caps_at_critical() {
        assert_eq!(
            AppliedAttention::bump_urgency(OperationalUrgency::Low),
            OperationalUrgency::Medium
        );
        assert_eq!(
            AppliedAttention::bump_urgency(OperationalUrgency::Medium),
            OperationalUrgency::High
        );
        assert_eq!(
            AppliedAttention::bump_urgency(OperationalUrgency::High),
            OperationalUrgency::Critical
        );
        assert_eq!(
            AppliedAttention::bump_urgency(OperationalUrgency::Critical),
            OperationalUrgency::Critical
        );
    }

    #[test]
    fn disposition_round_trips_through_strings() {
        for d in [
            ReAckDisposition::Advanced,
            ReAckDisposition::UnchangedWaiting,
            ReAckDisposition::Blocked,
            ReAckDisposition::HandedOff,
            ReAckDisposition::Escalated,
            ReAckDisposition::Resolved,
        ] {
            let s = d.as_str();
            assert_eq!(ReAckDisposition::parse_token(s), Some(d));
        }
        assert_eq!(ReAckDisposition::parse_token("nonsense"), None);
    }
}
