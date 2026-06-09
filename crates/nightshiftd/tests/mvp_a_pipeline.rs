//! MVP-A Slices 2/3/4 integration test — walkable hash chain.
//!
//! Single test that drives the full cook → Wicket → WLP pipeline
//! against the captured `nq.receipt.v1` fixture for sushi-k
//! `disk_state` (the same receipt verified in NQ Slice 1).
//!
//! Acceptance criteria the test asserts, mirroring
//! `docs/working/decisions/MVP_A_SLICES_2_3_4_PACKET.md`:
//!
//! 1. NS produces a Wicket Intent that wicket can consume (type
//!    construction via `wicket::Intent` is what guarantees schema
//!    validity per Wicket SPEC §4 — the model types ARE the schema).
//! 2. NS posture-packet appears at the local sink path with
//!    deterministic canonical content (JCS bytes byte-equal on
//!    re-run).
//! 3. Wicket Outcome JSON captured from `wicket::check()`.
//! 4. WLP HandlingReceipt JSON captured from
//!    `wlp::handle(&artifact, &[], &opts)` AND verdict is
//!    `HandlingVerdict::Accepted`.
//! 5. Hash chain is walkable from disk alone:
//!    - WLP HandlingReceipt.custody.causal_parents[0] ==
//!      AuthorizationReceipt.custody.artifact_hash
//!    - AuthorizationReceipt.custody.causal_parents[0] ==
//!      Wicket Outcome.receipt.input_hash
//!    - Wicket Intent.claimed_basis.evidence_refs[k] (kind ==
//!      prior_receipt) == NQ receipt content_hash
//!    - NS posture-packet.receipt_references.evidence_bundle
//!      contains nq receipt content_hash
//! 6. Round-trip determinism: re-running the pipeline against the
//!    same inputs at the same reference_time produces byte-identical
//!    artifacts.
//!
//! Forbidden cycle check (cf. NQ-NS-CHANNEL-SPLIT.md):
//! - NS posture (closure_candidate, posture_class) appears as
//!   NS-emitted content in Wicket Intent payload and WLP
//!   AuthorizationReceipt — never as NQ truth.
//! - There is no code path in NS that emits posture back to a
//!   NQ-readable substrate-truth surface (structural absence held by
//!   construction).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

use nightshiftd::closure::ClosureCandidate;
use nightshiftd::finding::{EvidenceState, FindingKey, Severity};
use nightshiftd::governor_client::NonDischargeKind;
use nightshiftd::mvp_a::{run_pipeline, MvpAResult, NqReceiptRef, NS_ACTOR, NS_POLICY_REF};
use nightshiftd::packet::{
    Attention, AttentionState, AuthorityResult, Confidence, Diagnosis, DiagnosisReview,
    DiagnosisReviewMode, FindingSummary, OperationalUrgency, Packet, ProposedAction,
    ProposedActionKind, ReceiptReferences, UnsettledSummary,
};
use nightshiftd::posture_class::PostureClass;
use nightshiftd::agenda::AuthorityLevel;
use nightshiftd::bundle::ReconciliationSummary;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Build a packet that mirrors the shape the live dogfood produced
/// (commit e0b51e0): sushi-k disk_pressure, IncidentShape, Critical,
/// `Unowned` attention, closure refused via
/// `UnassessableMissingConsequenceWitness`. Hand-constructed so the
/// MVP-A test does not require the full reconciler scaffolding.
fn sushi_k_packet() -> Packet {
    let key = FindingKey {
        source: "nq".into(),
        detector: "disk_pressure".into(),
        subject: "sushi-k:".into(),
    };
    let produced_at = Utc.with_ymd_and_hms(2026, 5, 28, 19, 0, 0).unwrap();
    Packet {
        packet_version: 0,
        packet_id: "pkt_mvp_a_test".into(),
        agenda_id: "sushi-k-disk-pressure".into(),
        run_id: "run_mvp_a_test".into(),
        produced_at,
        finding_summary: FindingSummary {
            source: key.source.clone(),
            detector: key.detector.clone(),
            host: "sushi-k".into(),
            subject: key.subject.clone(),
            severity: Severity::Critical,
            domain: None,
            persistence_generations: 22579,
            first_seen_at: Utc.with_ymd_and_hms(2026, 5, 12, 17, 56, 6).unwrap(),
            current_status: EvidenceState::Active,
            origin: None,
            silence: None,

            position: None,            posture_class: PostureClass::IncidentShape,
        },
        reconciliation_summary: ReconciliationSummary::default(),
        diagnosis: Diagnosis {
            regime: "committed: captured evidence matches current NQ snapshot byte-for-byte"
                .into(),
            evidence: vec!["finding nq:disk_pressure:sushi-k: persisted for 22579 generations"
                .into()],
            confidence: Confidence::Low,
            alternatives_considered: vec![],
        },
        proposed_action: ProposedAction {
            kind: ProposedActionKind::Advisory,
            steps: vec!["human review of the finding and reconciliation summary".into()],
            risk_notes: vec![],
            reversible: true,
            blast_radius: "none — advise only".into(),
            requested_authority_level: AuthorityLevel::Advise,
        },
        authority_result: AuthorityResult {
            requested: AuthorityLevel::Advise,
            governor_present: false,
            governor_verdict: Some(
                "n/a (--no-governor; ceiling capped at advise)".into(),
            ),
            authority_receipts: vec![],
            ceiling_note: None,
        },
        diagnosis_review: DiagnosisReview {
            mode: DiagnosisReviewMode::SelfCheck,
            unsafe_assumptions: vec![],
            stale_context_risks: vec![],
            promotion_overreach: vec![],
            missing_verification: vec![
                "v1 has no LLM self-check; placeholder review only".into(),
            ],
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
        unsettled: vec![],
        closure_candidate: ClosureCandidate::UnassessableMissingConsequenceWitness,
    }
}

#[test]
fn mvp_a_pipeline_produces_walkable_hash_chain_against_sushi_k_receipt() {
    let receipt_path = fixtures_dir().join("sushi-k-disk-state-receipt.json");
    let nq_receipt =
        NqReceiptRef::from_file(&receipt_path).expect("fixture nq.receipt.v1 must load");

    // Subject-boundary anchor — sushi-k host filesystem state, NOT NQ,
    // NOT NS, NOT observation loop. If this assert ever fails because
    // the fixture's subject changed, stop and check the NQ Slice 1
    // packet's subject-boundary section before re-anchoring.
    assert_eq!(
        nq_receipt.subject, "host:sushi-k",
        "subject-boundary tripwire: NQ receipt subject must remain host:sushi-k \
         filesystem/resource state per MVP_A_SLICE_1_PACKET.md"
    );
    assert_eq!(nq_receipt.evaluator.evaluator, "disk_state");
    assert_eq!(nq_receipt.evaluator.version, 1);
    assert!(
        nq_receipt
            .cannot_testify
            .iter()
            .any(|c| c.contains("Incident closure readiness")),
        "NQ self-limiting claim must explicitly refuse incident closure"
    );

    let packet = sushi_k_packet();
    let out_dir = tempfile::tempdir().expect("tempdir for mvp-a sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline must succeed");
    let outcome = match result {
        MvpAResult::Cooked(o) => o,
        MvpAResult::Refused(r) => panic!(
            "verified NQ receipt must cook, not refuse; got refused with reason `{}`",
            r.reason_code
        ),
        MvpAResult::WlpAuthorizationRefused(r) => panic!(
            "baseline packet has empty unsettled; WLP3 refusal not expected: {}",
            r.reason_code
        ),
    };

    // --- Acceptance (1): Wicket Intent is wicket-consumable. ---
    //
    // Construction via `wicket::Intent` is the schema-validity proof
    // (Wicket SPEC §4: the model types are the schema). The
    // `wicket::check()` call inside run_pipeline would have panicked
    // or rejected the Intent during dimensional accounting if any
    // required field were missing.

    // --- Acceptance (2): Posture-packet sink has deterministic
    // canonical content AND references NQ receipt at top level. ---
    let posture_bytes_1 =
        std::fs::read(&outcome.posture_packet_path).expect("posture sink readable");
    let posture_json: Value =
        serde_json::from_slice(&posture_bytes_1).expect("posture sink is valid JSON");
    let nq_ref = posture_json["nq_receipt_ref"]
        .as_str()
        .expect("posture sink must carry top-level nq_receipt_ref");
    assert_eq!(
        nq_ref,
        format!("nq-receipt://{}", nq_receipt.content_hash),
        "posture-packet nq_receipt_ref must be nq-receipt://<content_hash>"
    );
    assert_eq!(
        posture_json["nq_subject"].as_str(),
        Some(nq_receipt.subject.as_str()),
        "posture-packet must pin NQ subject for the subject-boundary anchor"
    );
    // Existing packet field (evidence_bundle) is preserved with the
    // run-ledger pointer; the NQ ref lives at top level alongside.
    assert!(
        posture_json["receipt_references"].is_object(),
        "the existing Packet structure must be flattened intact at top level"
    );

    // --- Acceptance (3a): Wicket Intent on disk, sha256(file) ==
    // receipt.input_hash. This is the hash a verifier reads from
    // Wicket Outcome.receipt.input_hash; the on-disk file is the
    // exact bytes Wicket hashed (RFC 8785 JCS via serde_jcs). ---
    let intent_bytes =
        std::fs::read(&outcome.wicket_intent_path).expect("wicket intent sink readable");
    let intent_disk_hash = {
        let mut h = Sha256::new();
        h.update(&intent_bytes);
        format!("sha256:{:x}", h.finalize())
    };
    assert_eq!(
        intent_disk_hash, outcome.wicket_receipt_input_hash,
        "sha256(intent file bytes) must equal Wicket receipt.input_hash"
    );
    let intent_json: Value =
        serde_json::from_slice(&intent_bytes).expect("intent JSON parseable");
    // The intent file is canonical; its evidence_refs contain the NQ
    // content_hash as a prior_receipt — verify from disk, not by
    // reconstructing via cook_intent.
    let intent_evidence = intent_json["claimed_basis"]["evidence_refs"]
        .as_array()
        .expect("intent claimed_basis.evidence_refs must be an array");
    let disk_prior_receipt_refs: Vec<&str> = intent_evidence
        .iter()
        .filter(|e| e["kind"].as_str() == Some("prior_receipt"))
        .map(|e| e["ref"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        disk_prior_receipt_refs.len(),
        1,
        "exactly one prior_receipt evidence ref expected in intent on disk"
    );
    assert_eq!(
        disk_prior_receipt_refs[0], nq_receipt.content_hash,
        "intent prior_receipt evidence ref must equal NQ content_hash (disk-read)"
    );

    // --- Acceptance (3b): Wicket Outcome captured. ---
    let wicket_bytes =
        std::fs::read(&outcome.wicket_outcome_path).expect("wicket outcome sink readable");
    let wicket_json: Value =
        serde_json::from_slice(&wicket_bytes).expect("wicket outcome JSON parseable");
    let receipt_input_hash = wicket_json["receipt"]["input_hash"]
        .as_str()
        .expect("wicket outcome must carry receipt.input_hash");
    assert_eq!(
        receipt_input_hash, outcome.wicket_receipt_input_hash,
        "outcome struct hash and disk hash must agree"
    );
    assert_eq!(
        wicket_json["operation_class"].as_str(),
        Some("interpret"),
        "cook layer requests interpret operation class"
    );

    // --- Acceptance (4): WLP HandlingReceipt captured AND accepted. ---
    let wlp_bytes =
        std::fs::read(&outcome.wlp_handling_path).expect("wlp handling sink readable");
    let wlp_json: Value =
        serde_json::from_slice(&wlp_bytes).expect("wlp handling JSON parseable");
    assert_eq!(
        wlp_json["kind"].as_str(),
        Some("HandlingReceipt"),
        "wlp sink must carry HandlingReceipt"
    );
    assert_eq!(
        wlp_json["admissibility"]["verdict"].as_str(),
        Some("accepted"),
        "wlp::handle must return Accepted for a well-formed AuthorizationReceipt \
         with policy_refs populated and scheme registered in HandleOpts"
    );
    assert_eq!(
        wlp_json["acted"].as_bool(),
        Some(true),
        "Accepted HandlingReceipt records acted=true per wlp::validate::decide"
    );

    // --- Acceptance (5): Walkable hash chain. ---
    //
    // 5a: HandlingReceipt.custody.causal_parents[0] equals
    //     AuthorizationReceipt.custody.artifact_hash.
    let handling_parent = wlp_json["custody"]["causal_parents"][0]
        .as_str()
        .expect("HandlingReceipt must have a causal parent");
    assert_eq!(
        handling_parent, outcome.wlp_authorization_artifact_hash,
        "HandlingReceipt parent must reference AuthorizationReceipt hash"
    );

    // 5b: AuthorizationReceipt.custody.causal_parents[0] equals
    //     Wicket Outcome.receipt.input_hash.
    let auth_bytes = std::fs::read(&outcome.wlp_authorization_path)
        .expect("authorization sink readable");
    let auth_json: Value =
        serde_json::from_slice(&auth_bytes).expect("authorization JSON parseable");
    assert_eq!(
        auth_json["kind"].as_str(),
        Some("AuthorizationReceipt"),
        "authorization sink must carry AuthorizationReceipt"
    );
    let auth_parent = auth_json["custody"]["causal_parents"][0]
        .as_str()
        .expect("AuthorizationReceipt must have a causal parent");
    assert_eq!(
        auth_parent, outcome.wicket_receipt_input_hash,
        "AuthorizationReceipt parent must reference Wicket receipt hash"
    );

    // 5c: Wicket Intent on disk → NQ link is asserted as part of
    //     acceptance (3a) above (the disk-read prior_receipt ref equals
    //     NQ content_hash).

    // 5d: NS posture-packet → NQ link is asserted as part of
    //     acceptance (2) above (top-level nq_receipt_ref).

    // Cross-check: actor identities and policy refs survived from
    // NS code into the artifacts on disk.
    assert_eq!(
        wicket_json["receipt"]["evidence_ref_hashes"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        2,
        "Wicket receipt must hash exactly the two evidence refs we passed"
    );
    assert_eq!(auth_json["actor"].as_str(), Some(NS_ACTOR));
    assert_eq!(wlp_json["actor"].as_str(), Some(NS_ACTOR));
    assert!(
        auth_json["admissibility"]["basis"]
            .as_str()
            .unwrap_or("")
            .contains(NS_POLICY_REF),
        "AuthorizationReceipt admissibility.basis must name the NS policy ref"
    );

    // --- Acceptance (6): Determinism on re-run. ---
    //
    // Same packet + same NQ receipt + same reference_time + same
    // out_dir = byte-identical artifacts. This is what makes the
    // pipeline integration-testable: a verifier can replay and get
    // the same content_hashes.
    let result_2 = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("re-run must succeed");
    let outcome_2 = match result_2 {
        MvpAResult::Cooked(o) => o,
        MvpAResult::Refused(_) => panic!("re-run must also cook (determinism)"),
        MvpAResult::WlpAuthorizationRefused(_) => {
            panic!("baseline re-run: WLP3 refusal not expected on empty unsettled")
        }
    };
    let posture_bytes_2 = std::fs::read(&outcome_2.posture_packet_path).unwrap();
    let intent_bytes_2 = std::fs::read(&outcome_2.wicket_intent_path).unwrap();
    let wicket_bytes_2 = std::fs::read(&outcome_2.wicket_outcome_path).unwrap();
    let auth_bytes_2 = std::fs::read(&outcome_2.wlp_authorization_path).unwrap();
    let wlp_bytes_2 = std::fs::read(&outcome_2.wlp_handling_path).unwrap();
    assert_eq!(posture_bytes_1, posture_bytes_2, "posture sink must be deterministic");
    assert_eq!(intent_bytes, intent_bytes_2, "intent sink must be deterministic");
    assert_eq!(wicket_bytes, wicket_bytes_2, "wicket sink must be deterministic");
    assert_eq!(auth_bytes, auth_bytes_2, "authorization sink must be deterministic");
    assert_eq!(wlp_bytes, wlp_bytes_2, "handling sink must be deterministic");
    assert_eq!(
        outcome.wicket_receipt_input_hash, outcome_2.wicket_receipt_input_hash,
        "Wicket receipt hash must be deterministic across runs"
    );
    assert_eq!(
        outcome.wlp_authorization_artifact_hash, outcome_2.wlp_authorization_artifact_hash,
        "AuthorizationReceipt hash must be deterministic"
    );
    assert_eq!(
        outcome.wlp_handling_artifact_hash, outcome_2.wlp_handling_artifact_hash,
        "HandlingReceipt hash must be deterministic"
    );

    // --- Cycle prohibition (sanity): NS posture content appears in
    //     NS-emitted artifacts only, never as NQ truth. ---
    //
    // The cook + wrap layer encodes NS posture/closure into the
    // Wicket Intent payload + WLP AuthorizationReceipt transition
    // payload. Both are NS-emitted artifacts; there is no NS code
    // path that re-injects them into a `nq.finding_snapshot.v1`,
    // `nq.witness.v1`, or `nq.receipt.v1` shape. This test
    // documents the structural absence; the audit-owed item in
    // AUDIT-BACKLOG.md "Self-subject-collapse: NS forbidden-cycle
    // structural-absence audit owed" tracks the broader audit.
    let auth_payload = &auth_json["transition"]["payload"];
    assert!(
        auth_payload["ns_closure_candidate"].is_object()
            || auth_payload["ns_closure_candidate"].is_string(),
        "NS closure_candidate must appear in NS-emitted authorization payload"
    );
    assert_eq!(
        auth_payload["ns_posture_class"].as_str(),
        Some("incident_shape"),
        "NS posture_class survives into the authorization payload as NS-emitted content"
    );
    // The reverse (NQ truth surface carrying NS posture) is
    // structurally absent in the codebase: no `nq.*` shape in
    // `crates/nightshiftd/src/nq.rs` consumes or emits NS posture.
    // That absence is the test's keeper; this assertion documents
    // intent without grepping the entire codebase at test time.
}

/// WLP1 observational carry-forward: a packet whose `unsettled` field
/// carries a typed non-discharge claim must surface that claim — kind,
/// reason, AND the binding `receipt_id` — in the wrapped WLP
/// `AuthorizationReceipt`'s `transition.payload.ns_unsettled`. The
/// packet's `receipt_references.governor_receipts` must also appear as
/// `transition.payload.governor_receipt_ids`. Both are preservation
/// only: forwarding the claim is **not** accepting or rejecting it; a
/// downstream receiver-side gate decides what reliance is appropriate.
///
/// This test uses `NonDischargeKind::Authority` rather than `Freshness`
/// because WLP3 refuses to wrap when `Freshness` is present. The WLP1
/// carry-forward invariant must hold for any kind that has NOT been
/// ratified to trigger WLP3 refusal — that's the trap the WLP3 slice
/// exists to avoid. A separate WLP3 test (below) covers the
/// freshness-refusal path.
#[test]
fn wlp1_observational_carry_forward_preserves_unsettled_and_receipts() {
    let receipt_path = fixtures_dir().join("sushi-k-disk-state-receipt.json");
    let nq_receipt =
        NqReceiptRef::from_file(&receipt_path).expect("fixture nq.receipt.v1 must load");

    // Reuse the verified sushi-k packet shape and overlay a populated
    // `unsettled` + `governor_receipts` pair. Use `Authority` (not
    // `Freshness`) so WLP3 does NOT refuse — this test asserts WLP1
    // carry-forward, not WLP3 refusal. The Authority kind is a
    // non-ratified-for-refusal placeholder; if future slices ratify
    // it, update this test to use whatever kinds remain non-refusal.
    let mut packet = sushi_k_packet();
    let receipt_id = "fixture_receipt_authority_001".to_string();
    packet.unsettled = vec![UnsettledSummary {
        kind: NonDischargeKind::Authority,
        reason: "synthetic test claim — not a ratified WLP3 refusal kind".into(),
        receipt_id: receipt_id.clone(),
    }];
    packet.receipt_references.governor_receipts = vec![receipt_id.clone()];

    let out_dir = tempfile::tempdir().expect("tempdir for mvp-a sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline must succeed");
    let outcome = match result {
        MvpAResult::Cooked(o) => o,
        MvpAResult::Refused(r) => {
            panic!("WLP1 carry-forward must NOT short-circuit cook: {r:?}")
        }
        MvpAResult::WlpAuthorizationRefused(r) => panic!(
            "WLP3 must NOT fire on non-freshness unsettled kinds — \
             trap-avoidance regression. Got reason `{}`, kinds {:?}",
            r.reason_code, r.unsettled_kinds
        ),
    };

    let auth_bytes = std::fs::read(&outcome.wlp_authorization_path)
        .expect("AuthorizationReceipt artifact must exist");
    let auth_json: Value =
        serde_json::from_slice(&auth_bytes).expect("AuthorizationReceipt must be valid JSON");
    let payload = &auth_json["transition"]["payload"];

    // -- ns_unsettled: one non-freshness claim with the prose reason
    //    and the source receipt_id intact. (Freshness would have
    //    routed through WLP3 refusal; see wlp3_* tests below.) --
    let unsettled = payload["ns_unsettled"].as_array().expect(
        "WLP1: transition.payload.ns_unsettled must be an array (forwarded from packet.unsettled)",
    );
    assert_eq!(
        unsettled.len(),
        1,
        "WLP1: exactly one unsettled summary must survive the cook/wrap"
    );
    let claim = &unsettled[0];
    assert_eq!(
        claim["kind"].as_str(),
        Some("authority"),
        "WLP1: closed-enum kind must serialize as snake_case 'authority'"
    );
    let reason = claim["reason"].as_str().expect("reason string");
    assert!(
        reason.contains("synthetic test claim"),
        "WLP1: freeform reason must survive verbatim, got {reason:?}"
    );
    assert_eq!(
        claim["receipt_id"].as_str(),
        Some(receipt_id.as_str()),
        "WLP1: receipt_id binding back to Governor receipt must survive the forward"
    );

    // -- governor_receipt_ids: the packet's receipt_references list
    //    propagated unchanged. --
    let receipt_ids = payload["governor_receipt_ids"].as_array().expect(
        "WLP1: transition.payload.governor_receipt_ids must be an array",
    );
    assert_eq!(
        receipt_ids.len(),
        1,
        "WLP1: exactly the receipt_id from packet.receipt_references.governor_receipts"
    );
    assert_eq!(
        receipt_ids[0].as_str(),
        Some(receipt_id.as_str()),
        "WLP1: forwarded id must equal the packet's governor_receipts entry"
    );
}

/// Empty `packet.unsettled` and empty `governor_receipts` still emit
/// `ns_unsettled: []` and `governor_receipt_ids: []` — empty array is
/// the positive claim "no unsettled claims surfaced," NOT silence.
/// Mirrors the v4 GateReceipt schema's same discipline.
#[test]
fn wlp1_empty_unsettled_emits_empty_array_not_missing_field() {
    let receipt_path = fixtures_dir().join("sushi-k-disk-state-receipt.json");
    let nq_receipt =
        NqReceiptRef::from_file(&receipt_path).expect("fixture nq.receipt.v1 must load");

    let packet = sushi_k_packet(); // unsettled defaults to vec![]
    assert!(packet.unsettled.is_empty(), "baseline: fixture has no unsettled");

    let out_dir = tempfile::tempdir().expect("tempdir for mvp-a sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline must succeed");
    let outcome = match result {
        MvpAResult::Cooked(o) => o,
        MvpAResult::Refused(r) => panic!("baseline must cook: {r:?}"),
        MvpAResult::WlpAuthorizationRefused(r) => panic!(
            "empty unsettled must NOT trigger WLP3 refusal: {}",
            r.reason_code
        ),
    };

    let auth_bytes = std::fs::read(&outcome.wlp_authorization_path).expect("artifact must exist");
    let auth_json: Value = serde_json::from_slice(&auth_bytes).expect("must be valid JSON");
    let payload = &auth_json["transition"]["payload"];

    let unsettled = payload["ns_unsettled"].as_array().expect(
        "ns_unsettled must be present even when empty — missing != zero",
    );
    assert!(
        unsettled.is_empty(),
        "empty unsettled must serialize as an empty array, not be absent"
    );
    let receipt_ids = payload["governor_receipt_ids"].as_array().expect(
        "governor_receipt_ids must be present even when empty — missing != zero",
    );
    assert!(
        receipt_ids.is_empty(),
        "no governor receipts must serialize as an empty array"
    );
}

/// WLP3 receiver-side refusal: a packet carrying `ns_unsettled` with
/// `kind == Freshness` must (a) still cook into a Wicket Intent and
/// produce a Wicket Outcome (classification is honest), (b) NOT mint
/// a WLP `AuthorizationReceipt` or `HandlingReceipt` (no warranty for
/// downstream reliance), and (c) emit a `ns.wlp_refusal.v1` artifact
/// carrying the closed reason code, the triggering kinds, the
/// governor receipt ids, and references to the Wicket artifacts.
///
/// The hard fence under test: refusal here is "I will not stake my
/// name on this," NOT "this never happened." The Wicket chain is
/// preserved; the warranty is intentionally absent.
#[test]
fn wlp3_freshness_unsettled_refuses_authorization_but_preserves_wicket_chain() {
    use nightshiftd::mvp_a::{WlpAuthorizationRefusal, WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED};

    let receipt_path = fixtures_dir().join("sushi-k-disk-state-receipt.json");
    let nq_receipt =
        NqReceiptRef::from_file(&receipt_path).expect("fixture nq.receipt.v1 must load");

    let mut packet = sushi_k_packet();
    let governor_receipt_id = "fixture_receipt_freshness_001".to_string();
    let reason_prose =
        "defer outcome does not settle closure authority while horizon remains active";
    packet.unsettled = vec![UnsettledSummary {
        kind: NonDischargeKind::Freshness,
        reason: reason_prose.into(),
        receipt_id: governor_receipt_id.clone(),
    }];
    packet.receipt_references.governor_receipts = vec![governor_receipt_id.clone()];

    let out_dir = tempfile::tempdir().expect("tempdir for mvp-a sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline must succeed");

    // -- Acceptance 1, 5: WlpAuthorizationRefused variant. --
    let refusal: WlpAuthorizationRefusal = match result {
        MvpAResult::WlpAuthorizationRefused(r) => r,
        MvpAResult::Cooked(_) => panic!(
            "freshness-unsettled packet must refuse at WLP3, not produce a Cooked variant"
        ),
        MvpAResult::Refused(r) => panic!(
            "freshness-unsettled must refuse at WLP3 (downstream), NOT at A.5 \
             (upstream). Got A.5 reason `{}`.",
            r.reason_code
        ),
    };

    // -- Acceptance 6: refusal carries the reason code, kinds, and
    //    governor receipt ids. --
    assert_eq!(
        refusal.reason_code, WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED,
        "WLP3 reason code must be the closed-vocabulary constant"
    );
    assert_eq!(
        refusal.unsettled_kinds,
        vec![NonDischargeKind::Freshness],
        "WLP3 refusal must enumerate the triggering kinds (Freshness only here)"
    );
    assert_eq!(
        refusal.governor_receipt_ids,
        vec![governor_receipt_id.clone()],
        "WLP3 refusal must carry the source Governor receipt ids"
    );

    // -- Acceptance 1, 2: Wicket Intent + Outcome still on disk. --
    assert!(
        refusal.wicket_intent_path.exists(),
        "Wicket Intent artifact must exist on the WLP3-refused path \
         (classification surface preserved)"
    );
    assert!(
        refusal.wicket_outcome_path.exists(),
        "Wicket Outcome artifact must exist on the WLP3-refused path"
    );

    // -- Acceptance 3: no WLP AuthorizationReceipt, no HandlingReceipt. --
    let wlp_auth_path =
        out_dir.path().join(format!("ns-wlp-authorization-{}.json", packet.run_id));
    let wlp_handling_path =
        out_dir.path().join(format!("ns-wlp-handling-{}.json", packet.run_id));
    assert!(
        !wlp_auth_path.exists(),
        "WLP3 must NOT mint an AuthorizationReceipt; found {wlp_auth_path:?}"
    );
    assert!(
        !wlp_handling_path.exists(),
        "WLP3 must NOT mint a HandlingReceipt; found {wlp_handling_path:?}"
    );

    // -- Acceptance 4: ns.wlp_refusal.v1 artifact present and well-formed. --
    let artifact_bytes = std::fs::read(&refusal.refusal_artifact_path)
        .expect("ns.wlp_refusal.v1 artifact must exist on disk");
    let artifact: Value =
        serde_json::from_slice(&artifact_bytes).expect("refusal artifact must be valid JSON");
    assert_eq!(
        artifact["schema"].as_str(),
        Some("ns.wlp_refusal.v1"),
        "refusal artifact must declare the ns.wlp_refusal.v1 schema (distinct from ns.refusal.v1)"
    );
    assert_eq!(
        artifact["reason_code"].as_str(),
        Some(WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED),
        "refusal artifact reason_code must match the closed constant"
    );
    let kinds_arr = artifact["unsettled_kinds"]
        .as_array()
        .expect("unsettled_kinds must be an array");
    assert_eq!(kinds_arr.len(), 1);
    assert_eq!(kinds_arr[0].as_str(), Some("freshness"));
    let receipts_arr = artifact["governor_receipt_ids"]
        .as_array()
        .expect("governor_receipt_ids must be an array");
    assert_eq!(receipts_arr[0].as_str(), Some(governor_receipt_id.as_str()));
    let unsettled_arr = artifact["ns_unsettled"]
        .as_array()
        .expect("ns_unsettled (full UnsettledSummary list) must be carried in the artifact");
    assert_eq!(unsettled_arr.len(), 1);
    assert_eq!(unsettled_arr[0]["kind"].as_str(), Some("freshness"));
    assert!(unsettled_arr[0]["reason"]
        .as_str()
        .map(|r| r.contains("defer outcome does not settle"))
        .unwrap_or(false));
}

/// WLP3 trap-avoidance: a packet whose `ns_unsettled` contains kinds
/// OTHER than `Freshness` must NOT trigger WLP3 refusal. The slice
/// covers exactly one ratified kind; the others remain carried-but-
/// not-adjudicated until they earn their own ratification.
///
/// This test is the structural defense against the laundering move
/// `if packet.unsettled.is_empty() ... else refuse`. If anyone ever
/// "simplifies" `wlp3_refusal_triggering_kinds` to a non-empty check,
/// this test fails.
#[test]
fn wlp3_non_freshness_unsettled_does_not_refuse() {
    let receipt_path = fixtures_dir().join("sushi-k-disk-state-receipt.json");
    let nq_receipt =
        NqReceiptRef::from_file(&receipt_path).expect("fixture nq.receipt.v1 must load");

    let mut packet = sushi_k_packet();
    packet.unsettled = vec![UnsettledSummary {
        kind: NonDischargeKind::Authority, // NOT Freshness
        reason: "synthetic Authority claim — non-ratified for WLP3 refusal".into(),
        receipt_id: "fixture_receipt_authority_001".into(),
    }];

    let out_dir = tempfile::tempdir().expect("tempdir for mvp-a sinks");
    let reference_time = packet.produced_at;

    let result = run_pipeline(&packet, &nq_receipt, out_dir.path(), reference_time)
        .expect("mvp-a pipeline must succeed");

    match result {
        MvpAResult::Cooked(_) => {} // expected
        MvpAResult::WlpAuthorizationRefused(r) => panic!(
            "WLP3 trap regression: non-freshness unsettled MUST NOT refuse; got `{}` for kinds {:?}",
            r.reason_code, r.unsettled_kinds
        ),
        MvpAResult::Refused(r) => panic!("unexpected A.5 refusal: {}", r.reason_code),
    }
}
