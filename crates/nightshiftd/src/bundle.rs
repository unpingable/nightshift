//! Context bundle — the declared evidence object for a run.
//!
//! Two phases: capture (when the agenda is set) and reconciliation
//! (when the run begins). Inputs enter as `observed`; the Reconciler
//! assigns a status + reliance class + valid_for scope.
//!
//! See `SCHEMA-bundle.md`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::coordination::ConcurrentActivity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputStatus {
    Observed,
    Committed,
    Changed,
    Stale,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelianceClass {
    Authoritative,
    AuthoritativeForCoordination,
    Hint,
    Historical,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidFor {
    Authorization,
    Proposal,
    Diagnosis,
    PacketContext,
    CoordinationGating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InvalidationRule {
    FindingAbsentForNGenerations { n: u32 },
    HostUnreachable,
    RepoHeadChanged,
    PolicyHashChanged,
    ExpiresAt { at: DateTime<Utc> },
    ConcurrentActorTransitionedState,
    ConcurrentActorOpenedNewScopeOverlap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Freshness {
    pub captured_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub invalidates_if: Vec<InvalidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureInput {
    pub input_id: String,
    pub source: String,
    pub kind: String,
    pub status: InputStatus,
    pub evidence_hash: String,
    pub freshness: Freshness,
    pub payload_ref: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub admissible_for: Vec<ValidFor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inadmissible_for: Vec<ValidFor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturePhase {
    pub captured_at: DateTime<Utc>,
    pub captured_by: String,
    pub capture_reason: String,
    pub inputs: Vec<CaptureInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelianceScope {
    pub run_id: String,
    pub valid_for: Vec<ValidFor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub input_id: String,
    pub status: InputStatus,
    pub reliance_class: RelianceClass,
    pub scope: RelianceScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_evidence_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_evidence_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrent_activity: Option<ConcurrentActivity>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    #[serde(default)]
    pub admissible_for_authorization: Vec<String>,
    #[serde(default)]
    pub admissible_for_proposal: Vec<String>,
    #[serde(default)]
    pub admissible_for_diagnosis: Vec<String>,
    #[serde(default)]
    pub hints_only: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
    #[serde(default)]
    pub downgraded: Vec<String>,
    #[serde(default)]
    pub coordination_gating: Vec<String>,
    pub ok_to_proceed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationPhase {
    pub reconciled_at: DateTime<Utc>,
    pub reconciled_by: String,
    pub results: Vec<ReconciliationResult>,
    pub summary: ReconciliationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub bundle_version: u32,
    pub agenda_id: String,
    pub run_id: String,
    pub capture: CapturePhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconciliation: Option<ReconciliationPhase>,
}
