//! SILICON-ORCHARD exact EPOCH-to-Casework golden journeys.

use std::{collections::BTreeMap, fs, path::PathBuf};

use nightshift_casework::{load_operational_conditions_at, server::Api};
use nightshiftd::operational_lineage::ReobservationDispositionV1;
use sha2::{Digest as _, Sha256};

const CONDITIONS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../qualification/ecad-operational-observation-golden-journey-v1-20260831/conditions"
);

fn directories() -> Vec<PathBuf> {
    let mut values = fs::read_dir(CONDITIONS)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    values.sort();
    values
}

#[test]
fn all_independent_ecad_cases_reach_exact_epoch_and_casework() {
    let loaded = load_operational_conditions_at(&directories()).unwrap();
    assert_eq!(loaded.len(), 20);
    let by_scenario = loaded
        .values()
        .map(|condition| {
            (
                condition
                    .projection
                    .profile
                    .profile_id
                    .strip_prefix("profile:silicon-currentness:")
                    .unwrap()
                    .to_owned(),
                condition,
            )
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        by_scenario["license-no-response"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::AcquisitionFailure
    );
    assert_eq!(
        by_scenario["stale-artifact"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::Stale
    );
    for scenario in [
        "scheduler-contradiction-a",
        "scheduler-contradiction-b",
        "agent-contradiction",
    ] {
        assert_eq!(
            by_scenario[scenario].projection.evaluation.disposition,
            ReobservationDispositionV1::Contradictory
        );
        assert!(!by_scenario[scenario].projection.questions.is_empty());
    }
    let missing = &by_scenario["exit-zero-missing-output"];
    assert_eq!(
        missing.projection.evaluation.disposition,
        ReobservationDispositionV1::Current
    );
    let nq = String::from_utf8(missing.nq_bytes.clone()).unwrap();
    assert!(nq.contains("ecad:process-exit-code"));
    assert!(nq.contains("ecad:output-present"));
    assert!(nq.contains("ecad:observed-output-artifact"));

    assert_eq!(
        by_scenario["license-unavailable-before-start"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::Current
    );
    assert_ne!(
        by_scenario["license-unavailable-before-start"]
            .projection
            .evaluation
            .disposition,
        by_scenario["license-no-response"]
            .projection
            .evaluation
            .disposition
    );
    assert_eq!(
        by_scenario["healthy-wrong-subject"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::Refused
    );
    assert_eq!(
        by_scenario["repository-custody-historical"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::Stale
    );
    assert_eq!(
        by_scenario["repository-custody-successor"]
            .projection
            .evaluation
            .disposition,
        ReobservationDispositionV1::Current
    );
    assert_eq!(
        by_scenario["repository-custody-successor"]
            .projection
            .lineage
            .sequence,
        1
    );
    for scenario in ["scheduler-running-source", "worker-absent-source"] {
        assert_eq!(
            by_scenario[scenario].projection.evaluation.disposition,
            ReobservationDispositionV1::Contradictory
        );
        assert!(!by_scenario[scenario].projection.questions.is_empty());
    }
    let design_nq =
        String::from_utf8(by_scenario["wrong-design-revision"].nq_bytes.clone()).unwrap();
    let repository_nq = String::from_utf8(by_scenario["wrong-revision"].nq_bytes.clone()).unwrap();
    assert!(design_nq.contains("ecad:observed-design-revision"));
    assert!(repository_nq.contains("ecad:observed-repository-revision"));
    assert_ne!(design_nq, repository_nq);

    let source_nq: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../qualification/ecad-operational-observation-golden-journey-v1-20260831/source/nq-artifact.v1.json"
    )))
    .unwrap();
    assert_eq!(
        source_nq["distant_intake_binding"]["partition_attempt_record"]["outcome"],
        "partition"
    );
    assert_eq!(
        source_nq["distant_intake_binding"]["retry_custody_attempt_record"]["outcome"],
        "custody_confirmed"
    );
    assert!(!serde_json::to_string(&missing.projection)
        .unwrap()
        .contains("aggregate_health"));
}

#[test]
fn operational_api_keeps_raw_bytes_head_parity_and_no_write_plane() {
    let loaded = load_operational_conditions_at(&directories()).unwrap();
    let api = Api::new(BTreeMap::new())
        .with_operational_conditions(&directories())
        .unwrap();
    assert_eq!(
        api.response("GET", "/api/v1/operational-conditions").status,
        200
    );
    for condition in loaded.values() {
        let id = &condition.projection.navigation_id;
        let detail = format!("/api/v1/operational-conditions/{id}");
        let get = api.response("GET", &detail);
        let head = api.response("HEAD", &detail);
        assert_eq!(get.status, 200);
        assert_eq!(head.status, 200);
        assert_eq!(head.etag, get.etag);
        let raw_artifacts = [
            ("monitor", &condition.monitor_bytes),
            ("nq", &condition.nq_bytes),
            ("lineage", &condition.lineage_bytes),
            ("profile", &condition.profile_bytes),
            ("evaluation", &condition.evaluation_bytes),
        ];
        for (kind, exact_bytes) in raw_artifacts {
            let path = format!("/api/v1/operational-conditions/{id}/raw/{kind}");
            let raw_get = api.response("GET", &path);
            let raw_head = api.response("HEAD", &path);
            assert_eq!(raw_get.status, 200);
            assert_eq!(raw_head.status, 200);
            assert_eq!(raw_get.body, *exact_bytes);
            assert_eq!(raw_head.etag, raw_get.etag);
            assert_eq!(raw_head.content_type, raw_get.content_type);
            assert_eq!(raw_head.allow, raw_get.allow);
            for method in ["POST", "PUT", "PATCH", "DELETE"] {
                assert_eq!(api.response(method, &path).status, 405);
            }
        }
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            assert_eq!(api.response(method, &detail).status, 405);
            assert_eq!(
                api.response(method, "/api/v1/operational-conditions")
                    .status,
                405
            );
        }
    }
    let projections = loaded
        .values()
        .map(|value| &value.projection)
        .collect::<Vec<_>>();
    let serialized = serde_json::to_string(&projections).unwrap();
    for forbidden in ["action_url", "commands", "controls", "aggregate_health"] {
        assert!(!serialized.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn accepted_owner_heads_and_exact_source_bytes_are_closed() {
    let monitor = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../qualification/ecad-operational-observation-golden-journey-v1-20260831/source/monitor-bundle.v1.json"
    ));
    let nq = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../qualification/ecad-operational-observation-golden-journey-v1-20260831/source/nq-artifact.v1.json"
    ));
    let owner: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../qualification/ecad-operational-observation-golden-journey-v1-20260831/source/owner-binding.v1.json"
    )))
    .unwrap();
    let monitor_digest = format!("sha256:{:x}", Sha256::digest(monitor));
    let nq_digest = format!("sha256:{:x}", Sha256::digest(nq));
    assert_eq!(owner["schema"], "nightshift.silicon-owner-binding/v1");
    assert_eq!(
        owner["monitor_fixture_head"],
        "bb75c4325f903f2c544e9758b5ea8d30c8bbc773"
    );
    assert_eq!(
        owner["nq_result_head"],
        "78ba5137c83089d6f1cd2bada65f6f7bdda2669c"
    );
    assert_eq!(owner["monitor_bundle_sha256"], monitor_digest);
    assert_eq!(owner["nq_artifact_sha256"], nq_digest);
    assert_eq!(
        owner["claim_deck_digest"],
        "sha256:7f9ba67910df6962e4e02cb2e1fa75562a59889e16cef3c9133c90aa090cea0d"
    );
    assert_eq!(owner["grants_authority"], false);

    let nq_value: serde_json::Value = serde_json::from_slice(nq).unwrap();
    assert_eq!(
        nq_value["monitor_fixture_head"],
        owner["monitor_fixture_head"]
    );
    assert_eq!(
        nq_value["monitor_bundle_digest"],
        owner["monitor_bundle_sha256"]
    );
    assert!(nq_value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|case| case["eligibility"]["deck_digest"] == owner["claim_deck_digest"]));
    assert_eq!(nq_value["cases"].as_array().unwrap().len(), 20);
}
