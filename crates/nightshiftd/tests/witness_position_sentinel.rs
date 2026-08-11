//! Canonical missing-distinction sentinel.
//!
//! The successor diagnostic contract does not infer an omitted witness or
//! workflow distinction from detector names or matching projections.

use nightshiftd::diagnostic_posture::DiagnosticExecutionV1;

const MATCH: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/hostile_projection_collision_match.json");
const MISMATCH: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/hostile_projection_collision_mismatch.json");

#[test]
fn matching_lossy_projection_never_infers_an_omitted_required_distinction() {
    let matching: DiagnosticExecutionV1 = serde_json::from_slice(MATCH).unwrap();
    let mismatching: DiagnosticExecutionV1 = serde_json::from_slice(MISMATCH).unwrap();

    assert_eq!(serde_jcs::to_vec(&matching).unwrap(), MATCH);
    assert_eq!(serde_jcs::to_vec(&mismatching).unwrap(), MISMATCH);
    assert_eq!(
        matching.inputs.admitted[0].projected_artifact_id,
        mismatching.inputs.admitted[0].projected_artifact_id
    );
    assert_ne!(matching.artifact_id, mismatching.artifact_id);
    for artifact in [matching, mismatching] {
        let error = artifact.validate().unwrap_err();
        assert!(
            error.contains("claim requires a distinction omitted by the projection"),
            "{error}"
        );
        assert!(artifact.claims[0]
            .required_distinctions
            .contains(&"workflow_attempt".to_string()));
    }
}
