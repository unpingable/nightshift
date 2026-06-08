//! A.5 acceptance — NS refuses to cook a Wicket Intent when the NQ
//! receipt's status is not `verified`.
//!
//! Drives `run_pipeline` against a captured `nq.receipt.v1` fixture
//! for `host:lil-nas-x` with `status: not_verified` /
//! `status_reasons: [contradictory_observation]`. Asserts:
//!
//! 1. `MvpAResult::Refused(_)` is returned.
//! 2. The refusal artifact at the known sink path carries:
//!    - `schema: "ns.refusal.v1"`
//!    - `reason_code: "BASIS_NOT_VERIFIED_UNREPRESENTABLE"`
//!    - `nq_content_hash` matching the fixture
//!    - `nq_subject == "host:lil-nas-x"`
//!    - `nq_status == "not_verified"`
//!    - `nq_status_reasons` array including the originating reason
//!    - `nq_cannot_testify` array preserving NQ's self-limiting list
//!    - `agenda_id`, `finding_key`, `run_id`, `refused_at` populated
//! 3. The Cooked-path sinks (wicket-intent, wicket-outcome,
//!    wlp-authorization, wlp-handling) **do not exist** on disk.
//! 4. The posture-packet sink **does** exist (operator-visible
//!    record either way).
//! 5. Determinism: re-running with the same inputs at the same
//!    reference_time produces a byte-identical refusal artifact.
//!
//! Why this matters: Wicket Intent v0.3's `Evidence.status` is a
//! closed enum (`valid | stale | unavailable`). Mapping NQ
//! `not_verified` to any of those would launder the contradiction
//! into a verdict the chain cannot honestly represent. NS refuses;
//! the refusal artifact is itself the demonstration that the chain
//! refused honestly rather than producing a misleading verdict.

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use serde_json::Value;

use nightshiftd::agenda::AuthorityLevel;
use nightshiftd::bundle::ReconciliationSummary;
use nightshiftd::closure::ClosureCandidate;
use nightshiftd::finding::{EvidenceState, FindingKey, Severity};
use nightshiftd::mvp_a::{run_pipeline, MvpAResult, NqReceiptRef};
use nightshiftd::packet::{
    Attention, AttentionState, AuthorityResult, Confidence, Diagnosis, DiagnosisReview,
    DiagnosisReviewMode, FindingSummary, OperationalUrgency, Packet, ProposedAction,
    ProposedActionKind, ReceiptReferences,
};
use nightshiftd::posture_class::PostureClass;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Build a packet shaped like the lil-nas-x A.5 demo would produce:
/// `nq:zfs_pool_degraded:lil-nas-x:tank` as the target finding,
/// IncidentShape posture (the receipt has actively contradictory
/// substrate evidence), Critical urgency.
fn lil_nas_x_packet() -> Packet {
    let key = FindingKey {
        source: "nq".into(),
        detector: "zfs_pool_degraded".into(),
        subject: "lil-nas-x:tank".into(),
    };
    let produced_at = Utc.with_ymd_and_hms(2026, 5, 28, 20, 0, 0).unwrap();
    Packet {
        packet_version: 0,
        packet_id: "pkt_mvp_a5_test".into(),
        agenda_id: "lil-nas-x-disk-state".into(),
        run_id: "run_mvp_a5_test".into(),
        produced_at,
        finding_summary: FindingSummary {
            source: key.source.clone(),
            detector: key.detector.clone(),
            host: "lil-nas-x".into(),
            subject: key.subject.clone(),
            severity: Severity::Critical,
            domain: None,
            persistence_generations: 100,
            first_seen_at: produced_at,
            current_status: EvidenceState::Active,
            origin: None,
            silence: None,

            position: None,            posture_class: PostureClass::IncidentShape,
        },
        reconciliation_summary: ReconciliationSummary::default(),
        diagnosis: Diagnosis {
            regime: "committed: contradictory substrate evidence on lil-nas-x:tank".into(),
            evidence: vec!["zfs_pool_degraded; smart_status_lies; smart_uncorrected_errors_nonzero"
                .into()],
            confidence: Confidence::Low,
            alternatives_considered: vec![],
        },
        proposed_action: ProposedAction {
            kind: ProposedActionKind::Advisory,
            steps: vec!["operator review".into()],
            risk_notes: vec![],
            reversible: true,
            blast_radius: "none — advise only".into(),
            requested_authority_level: AuthorityLevel::Advise,
        },
        authority_result: AuthorityResult {
            requested: AuthorityLevel::Advise,
            governor_present: false,
            governor_verdict: Some("n/a (--no-governor)".into()),
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
            posture_class: PostureClass::IncidentShape,
            operational_urgency: OperationalUrgency::Critical,
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
        closure_candidate: ClosureCandidate::UnassessableMissingConsequenceWitness,
    }
}

#[test]
fn ns_refuses_to_cook_not_verified_receipt_and_writes_refusal_artifact() {
    let receipt_path = fixtures_dir().join("lil-nas-x-disk-state-receipt.json");
    let nq_receipt = NqReceiptRef::from_file(&receipt_path)
        .expect("lil-nas-x fixture nq.receipt.v1 must load");

    // Fixture preconditions — guard against accidental fixture
    // mutation (subject drift, status drift).
    assert_eq!(
        nq_receipt.subject, "host:lil-nas-x",
        "A.5 subject-boundary tripwire: NQ receipt subject must remain host:lil-nas-x"
    );
    assert_eq!(
        nq_receipt.status, "not_verified",
        "fixture must remain a `not_verified` receipt for this test"
    );
    assert!(
        nq_receipt
            .status_reasons
            .iter()
            .any(|r| r == "contradictory_observation"),
        "fixture must carry the contradictory_observation reason"
    );

    let packet = lil_nas_x_packet();
    let out_dir = tempfile::tempdir().expect("tempdir for a.5 sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline run must succeed (refusal is a successful pipeline outcome)");

    // --- Acceptance 1: Refused variant. ---
    let refusal = match result {
        MvpAResult::Refused(r) => r,
        MvpAResult::Cooked(_) => panic!(
            "not_verified NQ receipt must refuse, not cook; got Cooked. \
             This is path (c) — forbidden by the A.5 plan."
        ),
    };
    assert_eq!(refusal.reason_code, "BASIS_NOT_VERIFIED_UNREPRESENTABLE");
    assert_eq!(refusal.nq_content_hash, nq_receipt.content_hash);
    assert_eq!(refusal.nq_subject, "host:lil-nas-x");
    assert_eq!(refusal.nq_status, "not_verified");

    // --- Acceptance 2: Refusal artifact carries the full record. ---
    let artifact_bytes_1 =
        std::fs::read(&refusal.refusal_artifact_path).expect("refusal artifact readable");
    let artifact: Value =
        serde_json::from_slice(&artifact_bytes_1).expect("refusal artifact valid JSON");

    assert_eq!(artifact["schema"].as_str(), Some("ns.refusal.v1"));
    assert_eq!(
        artifact["reason_code"].as_str(),
        Some("BASIS_NOT_VERIFIED_UNREPRESENTABLE")
    );
    assert_eq!(
        artifact["nq_content_hash"].as_str(),
        Some(nq_receipt.content_hash.as_str())
    );
    assert_eq!(artifact["nq_subject"].as_str(), Some("host:lil-nas-x"));
    assert_eq!(artifact["nq_status"].as_str(), Some("not_verified"));
    let status_reasons = artifact["nq_status_reasons"]
        .as_array()
        .expect("nq_status_reasons must be an array");
    assert!(
        status_reasons
            .iter()
            .any(|r| r.as_str() == Some("contradictory_observation")),
        "refusal artifact must carry the originating NQ status reason"
    );
    // NQ's `cannot_testify` includes the replacement-workflow refusal
    // for disk_state — that line must survive into the refusal record
    // so the operator-visible artifact preserves NQ's self-limiting
    // statement.
    let cannot_testify = artifact["nq_cannot_testify"]
        .as_array()
        .expect("nq_cannot_testify must be an array");
    assert!(
        cannot_testify.iter().any(|c| c
            .as_str()
            .map(|s| s.contains("Replacement workflow"))
            .unwrap_or(false)),
        "refusal artifact must preserve NQ's Replacement-workflow refusal"
    );
    assert_eq!(artifact["agenda_id"].as_str(), Some("lil-nas-x-disk-state"));
    assert_eq!(
        artifact["finding_key"].as_str(),
        Some("nq:zfs_pool_degraded:lil-nas-x:tank")
    );
    assert_eq!(artifact["run_id"].as_str(), Some("run_mvp_a5_test"));
    assert!(
        artifact["refused_at"].as_str().is_some(),
        "refused_at must be populated"
    );

    // --- Acceptance 3: Cooked-path sinks do not exist on disk. ---
    let run_id = &packet.run_id;
    for not_emitted in [
        format!("ns-wicket-intent-{run_id}.json"),
        format!("ns-wicket-outcome-{run_id}.json"),
        format!("ns-wlp-authorization-{run_id}.json"),
        format!("ns-wlp-handling-{run_id}.json"),
    ] {
        let p = out_dir.path().join(&not_emitted);
        assert!(
            !p.exists(),
            "{not_emitted} must not exist on the refused path — \
             the chain refused honestly, no Wicket/WLP artifacts produced"
        );
    }

    // --- Acceptance 4: Posture sink exists (operator-visible record). ---
    let posture_path = out_dir.path().join(format!("ns-posture-{run_id}.json"));
    assert!(
        posture_path.exists(),
        "posture sink must exist on the refused path too — operator visibility is preserved"
    );
    let posture: Value =
        serde_json::from_slice(&std::fs::read(&posture_path).unwrap()).unwrap();
    assert_eq!(
        posture["nq_receipt_ref"].as_str(),
        Some(format!("nq-receipt://{}", nq_receipt.content_hash).as_str()),
        "posture sink must still pin the NQ receipt reference"
    );

    // --- Acceptance 5: Determinism on re-run. ---
    let result_2 = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("re-run must succeed");
    let refusal_2 = match result_2 {
        MvpAResult::Refused(r) => r,
        MvpAResult::Cooked(_) => panic!("re-run must also refuse"),
    };
    let artifact_bytes_2 = std::fs::read(&refusal_2.refusal_artifact_path).unwrap();
    assert_eq!(
        artifact_bytes_1, artifact_bytes_2,
        "refusal artifact must be byte-deterministic across runs"
    );
}
