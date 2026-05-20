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
use crate::finding::FindingSnapshot;

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
    /// The captured finding snapshot itself, when this input is
    /// NQ-backed. Required for the reconciler to perform the
    /// three-axis comparison (`GAP-nq-nightshift-contract.md`):
    /// `evidence_hash` mismatch alone does not mean semantic change,
    /// so the reconciler must read the captured semantic-axis fields
    /// directly. Optional for non-NQ inputs and for legacy bundles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captured_finding_snapshot: Option<FindingSnapshot>,
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
    /// The current snapshot observed at reconcile-time, persisted so
    /// adjudication is deterministic over the stored bundle and does
    /// not require re-acquiring live state. Populated by the
    /// reconcile-time acquisition step for NQ-backed inputs; None
    /// when the finding was absent from the current generation, or
    /// for non-NQ inputs that do not carry a snapshot. See
    /// `GAP-deferred-run-split.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_finding_snapshot: Option<FindingSnapshot>,
    /// Imported Basis Freshness receipt (Slice B.1, observe-only).
    /// Populated for each NQ-backed input whose `current_finding_snapshot`
    /// is present; `None` otherwise. Records what the freshness
    /// assessment WOULD have said about this finding under the
    /// configured window and skew. Per `docs/GAP-imported-basis-freshness.md`
    /// §"Build shape": the seam is made visible before it is made
    /// binding — this field surfaces the verdict on the receipt
    /// without (yet) mutating `EvidenceState` or the
    /// `ReliacenClass` derived from `status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<crate::freshness::FreshnessReceipt>,
    /// Silence-Aware Posture class (Slice C.1, surface-only).
    /// Derived from the finding's wire shape via
    /// `crate::posture_class::derive_posture_class`. Per
    /// `docs/GAP-silence-aware-posture.md` and
    /// `docs/GAP-reack-doctrine.md` invariants 3–5: a silence-shaped
    /// finding's ack lineage is distinct from an active-finding's
    /// ack lineage; this field surfaces the class so downstream
    /// consumers can tell which kind of ack the row carries.
    ///
    /// **Defaults to `Unknown`**, never to `IncidentShape`. Per the
    /// SILENCE_UNIFICATION rule: absence of the silence envelope is
    /// "not yet unified," not "not silence." Slice C.1 does NOT
    /// flip `EvidenceState`, `InputStatus`, or `RelianceClass` based
    /// on this field — those remain owned by Slice 5 / Slice B.
    #[serde(default)]
    pub posture_class: crate::posture_class::PostureClass,
}

/// Cross-input summary of a reconciliation pass.
///
/// **Read all fields before acting.** No single field is an
/// authorization summary. The presence of an input in
/// `admissible_for_authorization` does not imply the run should
/// proceed (other inputs may be invalidated); the absence of an
/// input from `blocked` does not imply it is fresh (it may have
/// been downgraded). Per Slice 5 / `GAP-imported-basis-freshness.md`
/// (Slice B): consumers must inspect `reliance_class`, `valid_for`,
/// `Attention.evidence_state`, `ProposedAction`, and the freshness
/// receipt — not summary booleans in isolation.
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
    /// **`ok_to_proceed` is NOT an authorization summary.** Under v1
    /// semantics it means exactly *"no input was hard-invalidated."*
    /// It does **not** mean:
    ///
    /// - all evidence is fresh (stale inputs land in `downgraded`
    ///   with `ok_to_proceed = true`)
    /// - all inputs are admissible for the action under consideration
    ///   (an input may be Historical / `PacketContext` only)
    /// - the run is safe to act on without revalidation (Slice 5's
    ///   advise/revalidate-only pathway leaves this field `true` and
    ///   surfaces the caution through `ProposedAction` and
    ///   `Attention.evidence_state` instead)
    ///
    /// Concretely: an ingested finding whose producer extracted
    /// before the freshness window emits
    /// `InputStatus::Stale` + `RelianceClass::Historical` +
    /// `EvidenceState::Stale` + `regime: "stale: …"` + a revalidate-
    /// only `ProposedAction`, while this field stays `true` because
    /// nothing was invalidated. See
    /// `docs/GAP-imported-basis-freshness.md` §"Closeout."
    ///
    /// Consumers that branch on this field alone are misreading the
    /// contract. The sentinel test
    /// `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`
    /// in `tests/nq_integration.rs` pins this invariant.
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
