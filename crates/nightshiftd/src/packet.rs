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
use crate::governor_client::NonDischargeKind;
use crate::finding::{
    EvidenceState, FindingKey, FindingOrigin, FindingSilence, Severity, WitnessPosition,
};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefusalKind {
    /// The witness is silent: age exceeded the staleness threshold.
    /// Both numbers participated in the verdict that produced this
    /// refusal.
    LivenessStale {
        age_seconds: i64,
        threshold_seconds: u64,
    },
    /// The witness timestamp is in the future — an epistemic hole,
    /// *not* staleness (see `liveness::LivenessVerdict::Skewed`).
    ///
    /// Deliberately carries no `threshold_seconds`: skew is detected
    /// before any threshold comparison happens (`liveness::verdict_for`
    /// returns `Skewed` on negative age and never consults the
    /// threshold), so there is no threshold to report. Synthesizing one
    /// here would state a number that took no part in the verdict.
    LivenessSkewed {
        age_seconds: i64,
    },
    /// The captured evidence premise no longer holds: the reconciler
    /// ruled the NQ input `Invalidated`. Carries the two hashes that
    /// participated in that verdict. `current_evidence_hash` is the
    /// discriminant between the two invalidation shapes without a
    /// separate flag: `None` means the finding is *absent* from the
    /// current NQ generation (under a declared absence rule); `Some`
    /// means *identity drift* — NQ now returns a different finding
    /// under the same input_id, so the two hashes differ.
    /// `previous_evidence_hash` is the captured basis and is present
    /// in both shapes.
    BasisInvalidated {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_evidence_hash: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        current_evidence_hash: Option<String>,
    },
    /// A horizon tolerance grant's justifying basis diverged from the
    /// live artifact, voiding the grant regardless of expiry
    /// (`HorizonAction::EscalateBasisInvalidated`). Deliberately
    /// *distinct* from `BasisInvalidated`: this refuses a **tolerance
    /// grant**, not an evidence premise, and the evidence that
    /// participated is the grant's basis id plus the tolerated and
    /// live basis hashes — not evidence hashes. Collapsing the two
    /// into one variant would be the near-miss the registry forbids.
    ToleranceBasisInvalidated {
        tolerated_basis_id: String,
        tolerated_basis_hash: String,
        current_basis_hash: String,
    },
    PreflightHeld,
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
    /// `DURABLE_ARTIFACT_SUBSTRATE_GAP` V1 (Slice A visibility-only):
    /// origin envelope passed through from the underlying snapshot.
    /// Present for ingested findings; absent for native NQ findings.
    /// Visibility-only — Night Shift does not branch on this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<FindingOrigin>,
    /// SILENCE_UNIFICATION shared envelope passed through. V1
    /// populates only on `extraction_stale`; legacy silence detectors
    /// omit pending migration. Visibility-only — Night Shift does
    /// not branch on this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silence: Option<FindingSilence>,
    /// NQ witness observation lane (substrate / application_internal
    /// / platform). Render-only — Night Shift does not branch on
    /// this field. Absent in every packet today because
    /// `nq findings export` does not carry it; field exists so that
    /// when any NQ wire surface threads position onto the
    /// snapshot, the packet renders the lane without further
    /// code changes. Inference from `detector` / `witness_type` is
    /// forbidden by sentinel test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<WitnessPosition>,
    /// Slice C.1 posture class (surface-only). Derived from the
    /// finding's wire shape. Defaults to `Unknown` per the
    /// SILENCE_UNIFICATION rule: absence of envelope is "not yet
    /// unified," not "not silence." See
    /// `docs/working/gaps/GAP-silence-aware-posture.md`.
    #[serde(default)]
    pub posture_class: crate::posture_class::PostureClass,
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
    /// Slice C.1 posture class (surface-only). Distinct from
    /// `attention_state` — `attention_state` is the operator's
    /// view of the finding's lifecycle (Unowned / Acknowledged /
    /// Investigating / WatchUntil / Silenced=operator-silenced);
    /// `posture_class` is the *shape* of the finding itself
    /// (IncidentShape / SilenceShape / Unknown).
    ///
    /// **`AttentionState::Silenced` and `PostureClass::SilenceShape`
    /// are different concepts that happen to share a word.**
    /// The former is "operator suppressed this attention row"; the
    /// latter is "the finding's evidence is shape-of-absence." A
    /// row can be `attention_state = Acknowledged` AND
    /// `posture_class = SilenceShape` simultaneously (an operator
    /// acked a silence-shaped finding).
    ///
    /// Per `docs/architecture/GAP-reack-doctrine.md` invariants 3–4: an ack on
    /// this row applies only to this `attention_key`'s lineage; it
    /// does not transfer to attention rows for other findings,
    /// regardless of whether those rows have the same or different
    /// `posture_class`. The field surfaces the class so consumers
    /// can see what kind of ack the row carries.
    #[serde(default)]
    pub posture_class: crate::posture_class::PostureClass,
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

/// Operator-visible summary of one non-discharge claim from a
/// Governor receipt. **Display-only / derived.** The authority
/// artifact remains the v4 `GateReceipt.unsettled` on Governor's
/// side; this is a convenience surface so an operator reading the
/// packet does not have to cross to `governor receipts show <id>`
/// to see what the verdict left unsettled.
///
/// `receipt_id` is the binding back to the source receipt. The
/// summary intentionally drops the claim's optional
/// `required_consumer` / `required_witness` fields — those belong
/// to the authority artifact, not the display surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsettledSummary {
    pub kind: NonDischargeKind,
    pub reason: String,
    pub receipt_id: String,
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
    /// Operator-visible summaries of Governor receipt non-discharge
    /// claims. Empty for non-horizon paths or for outcomes that
    /// emit no `unsettled` (Allow / Deny / Escalate / Render in
    /// current B.1 scope). Derived display surface ONLY — the
    /// authority artifact is the v4 `GateReceipt.unsettled` on the
    /// Governor side. `receipt_id` on each summary is the binding
    /// back to the source.
    ///
    /// **Invariant:** the packet may *display* unsettled; it may
    /// not become the thing that *settles* unsettled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsettled: Vec<UnsettledSummary>,
    /// Slice 4 — review-gating closure verdict per
    /// `working/decisions/pre-positioned-doctrine-gates.md` Gate 1.
    /// **Not closure authority**: the predicate refuses closure
    /// under named blocker conditions; it does not authorize a
    /// close verb (there is no close verb in v1). See
    /// `crate::closure` for the assessment function and the
    /// `EligibleForClosureReview` unreachability invariant.
    ///
    /// `#[serde(default)]` so pre-Slice-4 stored packets deserialize
    /// cleanly. Default is `UnassessableMissingConsequenceWitness`
    /// — the conservative refusal that mirrors what a freshly
    /// assessed IncidentShape packet would return when no blocker
    /// fires.
    #[serde(default = "default_closure_candidate")]
    pub closure_candidate: crate::closure::ClosureCandidate,
    /// NS-1 — typed refusal reason when a packet is emitted because a
    /// gate (liveness, preflight, etc.) blocked the run. `None`
    /// everywhere except refusal paths. The free-text `blocked` array
    /// on `reconciliation_summary` serves as display; this field
    /// provides structured classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<RefusalKind>,
}

fn default_closure_candidate() -> crate::closure::ClosureCandidate {
    crate::closure::ClosureCandidate::UnassessableMissingConsequenceWitness
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_kind_liveness_stale_serializes_with_tag() {
        let refusal = RefusalKind::LivenessStale {
            age_seconds: 150,
            threshold_seconds: 90,
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "liveness_stale");
        assert_eq!(json["age_seconds"], 150);
        assert_eq!(json["threshold_seconds"], 90);
    }

    #[test]
    fn refusal_kind_basis_invalidated_absent_serializes_without_current_hash() {
        // Finding absent from the current generation: no current hash
        // took part, so the field must not be synthesized (same
        // discipline as `LivenessSkewed` omitting a threshold).
        let refusal = RefusalKind::BasisInvalidated {
            previous_evidence_hash: Some("sha256:abc".into()),
            current_evidence_hash: None,
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "basis_invalidated");
        assert_eq!(json["previous_evidence_hash"], "sha256:abc");
        assert!(
            !json.as_object().unwrap().contains_key("current_evidence_hash"),
            "absence carries no current hash; got {json}"
        );
        let round: RefusalKind = serde_json::from_value(json).expect("round-trip");
        assert_eq!(round, refusal);
    }

    #[test]
    fn refusal_kind_basis_invalidated_drift_round_trips_both_hashes() {
        // Identity drift: both hashes participated and both survive.
        let refusal = RefusalKind::BasisInvalidated {
            previous_evidence_hash: Some("sha256:old".into()),
            current_evidence_hash: Some("sha256:new".into()),
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "basis_invalidated");
        let round: RefusalKind = serde_json::from_value(json).expect("round-trip");
        assert_eq!(round, refusal);
    }

    #[test]
    fn refusal_kind_tolerance_basis_invalidated_is_a_distinct_tag() {
        // The horizon tolerance-basis refusal must not wear the
        // reconciler's `basis_invalidated` tag: distinct claims,
        // distinct tags, no collapse.
        let refusal = RefusalKind::ToleranceBasisInvalidated {
            tolerated_basis_id: "runbook:v3".into(),
            tolerated_basis_hash: "sha256:tol".into(),
            current_basis_hash: "sha256:live".into(),
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "tolerance_basis_invalidated");
        assert_eq!(json["tolerated_basis_id"], "runbook:v3");
        assert_eq!(json["tolerated_basis_hash"], "sha256:tol");
        assert_eq!(json["current_basis_hash"], "sha256:live");
        let round: RefusalKind = serde_json::from_value(json).expect("round-trip");
        assert_eq!(round, refusal);
    }

    #[test]
    fn refusal_kind_preflight_held_serializes_with_tag() {
        let refusal = RefusalKind::PreflightHeld;
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "preflight_held");
    }

    #[test]
    fn refusal_kind_liveness_skewed_serializes_with_tag() {
        let refusal = RefusalKind::LivenessSkewed {
            age_seconds: -116750929,
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        assert_eq!(json["kind"], "liveness_skewed");
        assert_eq!(json["age_seconds"], -116750929i64);
        // Load-bearing: skew carries no threshold, because none took
        // part in the verdict. A synthesized threshold here would be a
        // number the free-text never renders.
        assert!(
            !json.as_object().unwrap().contains_key("threshold_seconds"),
            "skew refusal must not carry a staleness threshold; got {json}"
        );
    }

    #[test]
    fn refusal_kind_liveness_stale_round_trips() {
        // Non-negative age: a negative age is a Skewed verdict, never
        // Stale (see `liveness::verdict_for`).
        let refusal = RefusalKind::LivenessStale {
            age_seconds: 150,
            threshold_seconds: 90,
        };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        let deserialized: RefusalKind =
            serde_json::from_value(json).expect("deserialization must succeed");
        assert_eq!(deserialized, refusal);
    }

    #[test]
    fn refusal_kind_liveness_skewed_round_trips() {
        let refusal = RefusalKind::LivenessSkewed { age_seconds: -42 };
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        let deserialized: RefusalKind =
            serde_json::from_value(json).expect("deserialization must succeed");
        assert_eq!(deserialized, refusal);
    }

    #[test]
    fn refusal_kind_preflight_held_round_trips() {
        let refusal = RefusalKind::PreflightHeld;
        let json = serde_json::to_value(&refusal).expect("serialization must succeed");
        let deserialized: RefusalKind =
            serde_json::from_value(json).expect("deserialization must succeed");
        assert_eq!(deserialized, refusal);
    }

    #[test]
    fn packet_with_refusal_none_skips_field_in_json() {
        use crate::agenda::AuthorityLevel;
        use crate::bundle::ReconciliationSummary;
        use crate::finding::{EvidenceState, FindingKey, Severity};
        use chrono::Utc;

        let key = FindingKey {
            source: "nq".into(),
            detector: "test".into(),
            subject: "test".into(),
        };
        let now = Utc::now();

        let packet = Packet {
            packet_version: 0,
            packet_id: "test".into(),
            agenda_id: "test".into(),
            run_id: "test".into(),
            produced_at: now,
            finding_summary: FindingSummary {
                source: "nq".into(),
                detector: "test".into(),
                host: "test".into(),
                subject: "test".into(),
                severity: Severity::Warning,
                domain: None,
                persistence_generations: 0,
                first_seen_at: now,
                current_status: EvidenceState::Active,
                origin: None,
                silence: None,
                position: None,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
            },
            reconciliation_summary: ReconciliationSummary::default(),
            diagnosis: Diagnosis {
                regime: "test".into(),
                evidence: vec![],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            proposed_action: ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec![],
                risk_notes: vec![],
                reversible: true,
                blast_radius: "none".into(),
                requested_authority_level: AuthorityLevel::Advise,
            },
            authority_result: AuthorityResult {
                requested: AuthorityLevel::Advise,
                governor_present: false,
                governor_verdict: None,
                authority_receipts: vec![],
                ceiling_note: None,
            },
            diagnosis_review: DiagnosisReview {
                mode: DiagnosisReviewMode::SelfCheck,
                unsafe_assumptions: vec![],
                stale_context_risks: vec![],
                promotion_overreach: vec![],
                missing_verification: vec![],
                recommended_downgrade: None,
            },
            attention: Attention {
                attention_key: key,
                evidence_state: EvidenceState::Active,
                attention_state: AttentionState::Unowned,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
                operational_urgency: OperationalUrgency::Medium,
                owner: None,
                last_touched_by: None,
                last_touched_at: None,
                acknowledged_at: None,
                ack_expires_at: None,
                follow_up_by: None,
                handoff_note: None,
                re_alert_after: None,
                silence_reason: None,
                tolerance_basis_id: None,
                tolerance_basis_hash: None,
            },
            receipt_references: ReceiptReferences::default(),
            unsettled: vec![],
            closure_candidate: crate::closure::ClosureCandidate::UnassessableMissingConsequenceWitness,
            refusal: None,
        };

        let serialized = serde_json::to_value(&packet).expect("serialization must succeed");
        assert!(!serialized.as_object().unwrap().contains_key("refusal"));
    }

    #[test]
    fn packet_with_refusal_some_includes_field_in_json() {
        use crate::agenda::AuthorityLevel;
        use crate::bundle::ReconciliationSummary;
        use crate::finding::{EvidenceState, FindingKey, Severity};
        use chrono::Utc;

        let key = FindingKey {
            source: "nq".into(),
            detector: "test".into(),
            subject: "test".into(),
        };
        let now = Utc::now();

        let packet = Packet {
            packet_version: 0,
            packet_id: "test".into(),
            agenda_id: "test".into(),
            run_id: "test".into(),
            produced_at: now,
            finding_summary: FindingSummary {
                source: "nq".into(),
                detector: "test".into(),
                host: "test".into(),
                subject: "test".into(),
                severity: Severity::Warning,
                domain: None,
                persistence_generations: 0,
                first_seen_at: now,
                current_status: EvidenceState::Active,
                origin: None,
                silence: None,
                position: None,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
            },
            reconciliation_summary: ReconciliationSummary::default(),
            diagnosis: Diagnosis {
                regime: "test".into(),
                evidence: vec![],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            proposed_action: ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec![],
                risk_notes: vec![],
                reversible: true,
                blast_radius: "none".into(),
                requested_authority_level: AuthorityLevel::Advise,
            },
            authority_result: AuthorityResult {
                requested: AuthorityLevel::Advise,
                governor_present: false,
                governor_verdict: None,
                authority_receipts: vec![],
                ceiling_note: None,
            },
            diagnosis_review: DiagnosisReview {
                mode: DiagnosisReviewMode::SelfCheck,
                unsafe_assumptions: vec![],
                stale_context_risks: vec![],
                promotion_overreach: vec![],
                missing_verification: vec![],
                recommended_downgrade: None,
            },
            attention: Attention {
                attention_key: key,
                evidence_state: EvidenceState::Active,
                attention_state: AttentionState::Unowned,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
                operational_urgency: OperationalUrgency::Medium,
                owner: None,
                last_touched_by: None,
                last_touched_at: None,
                acknowledged_at: None,
                ack_expires_at: None,
                follow_up_by: None,
                handoff_note: None,
                re_alert_after: None,
                silence_reason: None,
                tolerance_basis_id: None,
                tolerance_basis_hash: None,
            },
            receipt_references: ReceiptReferences::default(),
            unsettled: vec![],
            closure_candidate: crate::closure::ClosureCandidate::UnassessableMissingConsequenceWitness,
            refusal: Some(RefusalKind::LivenessStale {
                age_seconds: 200,
                threshold_seconds: 90,
            }),
        };

        let serialized = serde_json::to_value(&packet).expect("serialization must succeed");
        assert!(serialized.as_object().unwrap().contains_key("refusal"));
        assert_eq!(serialized["refusal"]["kind"], "liveness_stale");
        assert_eq!(serialized["refusal"]["age_seconds"], 200);
        assert_eq!(serialized["refusal"]["threshold_seconds"], 90);
    }
}
