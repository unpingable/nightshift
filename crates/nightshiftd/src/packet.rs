//! Review packet — the reviewable output of a Night Shift run.
//!
//! See `SCHEMA-packet.md`. The v1 packet is the minimum reviewable
//! object: finding summary, reconciliation summary, diagnosis,
//! proposed action (advise-only for v1), authority result,
//! diagnosis review, attention state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agenda::AuthorityLevel;
use crate::bundle::ReconciliationSummary;
use crate::finding::{EvidenceState, FindingKey, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionState {
    Unowned,
    Acknowledged,
    Investigating,
    HandedOff,
    WatchUntil,
    Silenced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalUrgency {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub source: String,
    pub detector: String,
    pub host: String,
    pub subject: String,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub persistence_generations: u32,
    pub first_seen_at: DateTime<Utc>,
    pub current_status: EvidenceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub regime: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub confidence: Confidence,
    #[serde(default)]
    pub alternatives_considered: Vec<AlternativeRegime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeRegime {
    pub regime: String,
    pub ruled_out_by: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedActionKind {
    Advisory,
    StagedCommand,
    StagedDiff,
    Publication,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub kind: ProposedActionKind,
    pub steps: Vec<String>,
    #[serde(default)]
    pub risk_notes: Vec<String>,
    pub reversible: bool,
    pub blast_radius: String,
    pub requested_authority_level: AuthorityLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorityResult {
    pub requested: AuthorityLevel,
    pub governor_present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governor_verdict: Option<String>,
    #[serde(default)]
    pub authority_receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ceiling_note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosisReviewMode {
    Singleton,
    SelfCheck,
    Conference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisReview {
    pub mode: DiagnosisReviewMode,
    #[serde(default)]
    pub unsafe_assumptions: Vec<String>,
    #[serde(default)]
    pub stale_context_risks: Vec<String>,
    #[serde(default)]
    pub promotion_overreach: Vec<String>,
    #[serde(default)]
    pub missing_verification: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_downgrade: Option<AuthorityLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attention {
    /// Stable finding identity. Per GAP-attention-state.md, attention
    /// keys on finding_key, not run-local objects.
    pub attention_key: FindingKey,
    pub evidence_state: EvidenceState,
    pub attention_state: AttentionState,
    pub operational_urgency: OperationalUrgency,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_touched_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_touched_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acknowledged_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ack_expires_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up_by: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re_alert_after: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silence_reason: Option<String>,
    /// Required when `attention_state = WatchUntil`. Identifies the
    /// horizon declaration the watch was granted under (e.g. a named
    /// maintenance window, an observed quiescence, a scheduled
    /// rollout). Paired with `tolerance_basis_hash` so a later run
    /// can detect basis invalidation. `re_alert_after` carries the
    /// expiry T; together they are the `(T, basis=B)` write payload
    /// named in the 2026-04-23 observatory-family hand-off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_basis_id: Option<String>,
    /// Content hash of the basis artifact. Required when
    /// `attention_state = WatchUntil`. See `tolerance_basis_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance_basis_hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReceiptReferences {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_ledger: Option<String>,
    #[serde(default)]
    pub governor_receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_bundle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    pub packet_version: u32,
    pub packet_id: String,
    pub agenda_id: String,
    pub run_id: String,
    pub produced_at: DateTime<Utc>,
    pub finding_summary: FindingSummary,
    pub reconciliation_summary: ReconciliationSummary,
    pub diagnosis: Diagnosis,
    pub proposed_action: ProposedAction,
    pub authority_result: AuthorityResult,
    pub diagnosis_review: DiagnosisReview,
    pub attention: Attention,
    pub receipt_references: ReceiptReferences,
}
