//! Canonical operator-posture and temporal-attention surface tests.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use nightshiftd::canonical_store::{TemporalDecisionV1, TemporalPostureV1};
use nightshiftd::currentness::TemporalHoldExpiryV1;
use nightshiftd::diagnostic_posture::{
    evaluate_posture, DiagnosticInputs, Headline, PosturePolicy, RecurrenceEvidence,
};

const POLICY: &[u8] =
    include_bytes!("../../../docs/operator/examples/diagnostic-posture-v1/policy.json");
const INPUTS: &[u8] =
    include_bytes!("../../../docs/operator/examples/diagnostic-posture-v1/inputs.json");
const RECURRENCE: &[u8] =
    include_bytes!("../../../docs/operator/examples/diagnostic-posture-v1/recurrence.json");

fn at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn evaluate(value: &str) -> nightshiftd::diagnostic_posture::OperationalPosture {
    evaluate_posture(
        &serde_json::from_slice::<PosturePolicy>(POLICY).unwrap(),
        &serde_json::from_slice::<DiagnosticInputs>(INPUTS).unwrap(),
        &serde_json::from_slice::<RecurrenceEvidence>(RECURRENCE).unwrap(),
        at(value),
    )
    .unwrap()
}

#[test]
fn lossy_headline_never_replaces_retained_currentness_axes() {
    let current = evaluate("2026-07-27T20:00:10Z");
    let stale = evaluate("2026-07-27T20:01:10Z");
    let hidden_current = current.project(&BTreeSet::new());
    let hidden_stale = stale.project(&BTreeSet::new());

    assert!(current.current);
    assert!(!stale.current);
    assert_eq!(hidden_current.headline, Headline::Incomplete);
    assert_eq!(hidden_stale.headline, Headline::Incomplete);
    assert_ne!(current.posture_id, stale.posture_id);
    assert_eq!(hidden_current.source_posture_id, current.posture_id);
    assert_eq!(hidden_stale.source_posture_id, stale.posture_id);
}

#[test]
fn tolerability_hold_and_expiry_are_nonauthorizing_temporal_posture() {
    let expiry = TemporalHoldExpiryV1 {
        scheduler_clock_id: "clock:nightshift-scheduler".into(),
        at: at("2026-08-11T12:05:00Z"),
    };
    let held = TemporalPostureV1::evaluate(
        "policy:tolerability".into(),
        format!("sha256:{}", "a".repeat(64)),
        Some(expiry.clone()),
        "clock:nightshift-scheduler",
        at("2026-08-11T12:04:59Z"),
    )
    .unwrap();
    let attention = TemporalPostureV1::evaluate(
        "policy:tolerability".into(),
        format!("sha256:{}", "a".repeat(64)),
        Some(expiry),
        "clock:nightshift-scheduler",
        at("2026-08-11T12:05:00Z"),
    )
    .unwrap();

    assert_eq!(held.decision, TemporalDecisionV1::Hold);
    assert_eq!(attention.decision, TemporalDecisionV1::Attention);
    let serialized = serde_json::to_value(attention).unwrap();
    for forbidden in ["standing", "authorization", "dispatch", "effect"] {
        assert!(serialized.get(forbidden).is_none());
    }
}
