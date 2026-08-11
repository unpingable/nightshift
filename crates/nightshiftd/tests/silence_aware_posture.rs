//! Canonical hostile tests against Boolean diagnostic laundering.
//!
//! The complete NQ artifact, not a historical Watchbill packet bit, carries
//! the distinction between explicit absence, refusal, and no response.

use nightshiftd::diagnostic_posture::DiagnosticExecutionV1;

const POSITIVE: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/positive.json");
const REFUSED: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/refused.json");
const NO_RESPONSE: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/provider_no_response.json");

fn exact(bytes: &[u8]) -> (DiagnosticExecutionV1, serde_json::Value) {
    let artifact: DiagnosticExecutionV1 = serde_json::from_slice(bytes).unwrap();
    artifact.validate().unwrap();
    assert_eq!(serde_jcs::to_vec(&artifact).unwrap(), bytes);
    (artifact, serde_json::from_slice(bytes).unwrap())
}

#[test]
fn explicit_absence_is_not_permission_or_recovery() {
    let (artifact, value) = exact(POSITIVE);
    assert_eq!(value["outcome"]["condition"], "explicitly_absent");
    assert!(artifact
        .nonclaims
        .iter()
        .any(|nonclaim| nonclaim.contains("grants no reliance or authorization")));
    assert!(artifact.claims[0]
        .nonclaims
        .iter()
        .any(|nonclaim| nonclaim.contains("no remediation is authorized")));
}

#[test]
fn refusal_and_no_response_remain_distinct_under_the_same_coarse_unresolved_state() {
    let (refused, refused_value) = exact(REFUSED);
    let (silent, silent_value) = exact(NO_RESPONSE);

    assert_eq!(refused_value["outcome"]["condition"], "unresolved");
    assert_eq!(silent_value["outcome"]["condition"], "unresolved");
    assert_eq!(refused_value["outcome"]["derivation"], "refused");
    assert_eq!(silent_value["outcome"]["derivation"], "partial");
    assert_eq!(refused.inputs.refused.len(), 1);
    assert!(refused.inputs.failed.is_empty());
    assert_eq!(silent.inputs.failed.len(), 1);
    assert!(silent.inputs.refused.is_empty());
    assert_ne!(refused.artifact_id, silent.artifact_id);
}

#[test]
fn display_summary_cannot_erase_exact_failure_or_refusal_basis() {
    let (_, refused) = exact(REFUSED);
    let (_, silent) = exact(NO_RESPONSE);
    assert_ne!(refused["outcome"]["summary"], silent["outcome"]["summary"]);
    assert!(refused["inputs"]["refused"].as_array().unwrap().len() == 1);
    assert!(silent["inputs"]["failed"].as_array().unwrap().len() == 1);
}
