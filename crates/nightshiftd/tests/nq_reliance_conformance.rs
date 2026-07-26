//! Consumer-side conformance against NQ's golden reliance vectors.
//!
//! The fixtures under `tests/fixtures/nq_reliance/` are `nq.reliance.receipt.v1`
//! documents **produced by NQ** — emitted by `nq-monitor reliance evaluate` over
//! NQ's own golden vectors. Night Shift verifies them independently here: no
//! shared library guarantees agreement, and none of NQ's evaluator is copied
//! into this repository.
//!
//! Two properties are under test:
//!
//! 1. Night Shift accepts what it should and **refuses what it should**,
//!    including receipts addressed to a different consumer.
//! 2. Every accepted receipt maps to a read-only posture that grants nothing.

use std::collections::BTreeMap;

use nightshiftd::nq_disposition::{
    derive_disposition, Disposition, NqRelianceReceiptDto, SourceState,
};

const NOW: &str = "2026-07-26T00:00:00Z";

fn fixtures() -> BTreeMap<String, Vec<u8>> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/nq_reliance");
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).expect("fixture dir") {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) == Some("json") {
            let name = p.file_stem().unwrap().to_string_lossy().to_string();
            out.insert(name, std::fs::read(&p).unwrap());
        }
    }
    assert!(!out.is_empty(), "fixtures must exist");
    out
}

/// Receipts addressed to `nightshift-readonly`, and the posture each must yield.
fn expected_for_this_consumer() -> BTreeMap<&'static str, Disposition> {
    BTreeMap::from([
        ("valid_reliance_nightshift_readonly", Disposition::ContinueObserving),
        ("contradiction_retained", Disposition::HumanJudgmentRequired),
        ("premise_not_accepted", Disposition::HumanJudgmentRequired),
        ("residual_blocks_reliance", Disposition::HumanJudgmentRequired),
        ("custody_basis_not_accepted", Disposition::HumanJudgmentRequired),
        ("unauthorized_claim", Disposition::Stop),
        ("unauthorized_purpose", Disposition::Stop),
    ])
}

#[test]
fn nq_golden_vectors_addressed_to_this_consumer_map_to_the_expected_posture() {
    let fx = fixtures();
    for (name, expected) in expected_for_this_consumer() {
        let bytes = fx.get(name).unwrap_or_else(|| panic!("missing fixture {name}"));
        let dto = NqRelianceReceiptDto::parse_checked(bytes)
            .unwrap_or_else(|e| panic!("{name} must parse: {e}"));
        let rec = derive_disposition(&SourceState::Fresh, Some(&dto), NOW);
        assert_eq!(rec.disposition, expected, "{name}");
    }
}

#[test]
fn receipts_addressed_to_another_consumer_are_refused_not_reinterpreted() {
    let fx = fixtures();
    // These NQ vectors are real and valid — they are simply not ours.
    for name in [
        "valid_operational_health_reliance",
        "cannot_testify_is_not_authorization",
        "safe_to_merge_has_no_consumer",
        "recursive_self_witness_attempt",
        "stale_health_packet",
        "substituted_health_packet",
        "unknown_consumer",
    ] {
        let bytes = fx.get(name).unwrap_or_else(|| panic!("missing fixture {name}"));
        let err = NqRelianceReceiptDto::parse_checked(bytes)
            .err()
            .unwrap_or_else(|| panic!("{name} is not addressed to this consumer and must refuse"));
        assert!(
            err.to_string().contains("consumer"),
            "{name}: refusal must name the consumer mismatch, got {err}"
        );
    }
}

#[test]
fn no_accepted_vector_yields_an_action_or_capability() {
    let fx = fixtures();
    for name in expected_for_this_consumer().keys() {
        let dto = NqRelianceReceiptDto::parse_checked(&fx[*name]).unwrap();
        let rec = derive_disposition(&SourceState::Fresh, Some(&dto), NOW);
        let text = serde_json::to_string(&rec).unwrap();
        for forbidden in ["\"capability\"", "\"lease\"", "\"execute\"", "\"retry\""] {
            assert!(!text.contains(forbidden), "{name} must not carry {forbidden}");
        }
        assert!(rec
            .does_not_establish
            .iter()
            .any(|d| d.contains("no action was executed or authorized")));
    }
}

#[test]
fn carried_facts_survive_every_accepted_vector() {
    let fx = fixtures();
    // The three vectors that carry premises, contradictions, and residuals.
    for (name, field) in [
        ("premise_not_accepted", "premises"),
        ("contradiction_retained", "retained_contradictions"),
        ("residual_blocks_reliance", "unresolved_residuals"),
    ] {
        let raw: serde_json::Value = serde_json::from_slice(&fx[name]).unwrap();
        let source_len = raw[field].as_array().map_or(0, Vec::len);
        let dto = NqRelianceReceiptDto::parse_checked(&fx[name]).unwrap();
        let rec = derive_disposition(&SourceState::Fresh, Some(&dto), NOW);
        let carried = serde_json::to_value(rec.source.as_ref().unwrap()).unwrap();
        assert_eq!(
            carried[field].as_array().map_or(0, Vec::len),
            source_len,
            "{name}: {field} must survive the projection unchanged"
        );
    }
}

/// The no-response case is a Night Shift event with no NQ receipt behind it,
/// and there is deliberately no fixture for it — a fixture would be a
/// fabricated NQ document.
#[test]
fn the_no_response_case_has_no_nq_fixture_because_it_is_not_an_nq_document() {
    let fx = fixtures();
    for bytes in fx.values() {
        let v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(v["schema"], "nq.reliance.receipt.v1");
    }
    let rec = derive_disposition(
        &SourceState::NoResponse {
            elapsed_seconds: 30,
            timeout_seconds: 30,
        },
        None,
        NOW,
    );
    assert_eq!(rec.disposition, Disposition::EvidenceUnavailable);
    assert!(rec.source.is_none());
}
