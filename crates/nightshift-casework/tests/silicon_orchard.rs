//! SILICON-ORCHARD exact EPOCH-to-Casework golden journeys.

use std::{collections::BTreeMap, fs, path::PathBuf};

use nightshift_casework::{load_operational_conditions_at, server::Api};
use nightshiftd::operational_lineage::ReobservationDispositionV1;

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
    assert_eq!(loaded.len(), 13);
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
        let raw = api.response(
            "GET",
            &format!("/api/v1/operational-conditions/{id}/raw/monitor"),
        );
        assert_eq!(raw.body, condition.monitor_bytes);
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            assert_eq!(api.response(method, &detail).status, 405);
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
