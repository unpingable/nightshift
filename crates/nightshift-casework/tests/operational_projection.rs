use std::{fs, os::unix::fs::symlink};

use chrono::{TimeZone as _, Utc};
use nightshift_casework::{
    load_operational_conditions_at, OperationalQuestionSourceV1,
    CASEWORK_OPERATIONAL_CONDITION_SCHEMA_V1,
};
use nightshiftd::operational_lineage::{
    admit_operational_lineage, evaluate_reobservation, ReobservationProfileV1,
};
use serde_json::Value;
use tempfile::TempDir;

const MONITOR: &[u8] = include_bytes!(
    "../../nightshiftd/tests/fixtures/operational_lineage/field-monitor.accepted.json"
);
const NQ: &[u8] =
    include_bytes!("../../nightshiftd/tests/fixtures/operational_lineage/field-nq.accepted.json");

struct Fixture {
    _temp: TempDir,
    directory: std::path::PathBuf,
    monitor: Vec<u8>,
    nq: Vec<u8>,
}

fn fixture(nq: Vec<u8>) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("condition");
    fs::create_dir(&directory).unwrap();
    let admitted_at = Utc.with_ymd_and_hms(2026, 8, 30, 3, 0, 1).single().unwrap();
    let lineage = admit_operational_lineage(MONITOR, &nq, "input:field-vector", admitted_at, &[])
        .unwrap()
        .0;
    let profile = ReobservationProfileV1 {
        profile_id: "profile:shift-atlas-fixture".into(),
        max_age_seconds: 60,
    };
    let evaluation = evaluate_reobservation(
        &lineage,
        &profile,
        Utc.with_ymd_and_hms(2026, 8, 30, 3, 0, 1).single().unwrap(),
    )
    .unwrap();
    fs::write(directory.join("monitor.v1.json"), MONITOR).unwrap();
    fs::write(directory.join("nq.v1.json"), &nq).unwrap();
    fs::write(
        directory.join("lineage.v1.json"),
        serde_json::to_vec(&lineage).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.join("profile.v1.json"),
        serde_json::to_vec(&profile).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.join("evaluation.v1.json"),
        serde_json::to_vec(&evaluation).unwrap(),
    )
    .unwrap();
    Fixture {
        _temp: temp,
        directory,
        monitor: MONITOR.to_vec(),
        nq,
    }
}

fn accepted() -> Fixture {
    fixture(NQ.to_vec())
}

#[test]
fn exact_owner_artifacts_project_deterministically_and_keep_raw_bytes() {
    let fixture = accepted();
    let first = load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).unwrap();
    let second = load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).unwrap();
    assert_eq!(first, second);
    let loaded = first.values().next().unwrap();
    assert_eq!(
        loaded.projection.schema,
        CASEWORK_OPERATIONAL_CONDITION_SCHEMA_V1
    );
    assert_eq!(loaded.monitor_bytes, fixture.monitor);
    assert_eq!(loaded.nq_bytes, fixture.nq);
    assert_eq!(loaded.projection.subject, loaded.projection.lineage.subject);
    assert_eq!(
        loaded.projection.producer,
        loaded.projection.lineage.producer
    );
    assert_eq!(
        loaded.projection.acquisition_outcome,
        loaded.projection.lineage.acquisition_outcome
    );
    assert_eq!(
        loaded.projection.evaluation.lineage_id,
        loaded.projection.lineage.lineage_id
    );
    assert!(loaded.projection.questions.is_empty());
    assert_eq!(
        loaded.projection.authority_effect,
        "read_only_projection_no_authority"
    );
}

#[test]
fn exact_monitor_substitution_and_evaluation_extension_are_refused() {
    let fixture = accepted();
    fs::write(
        fixture.directory.join("monitor.v1.json"),
        include_bytes!(
            "../../nightshiftd/tests/fixtures/operational_lineage/field-monitor.unknown-locator.refused.json"
        ),
    )
    .unwrap();
    assert!(load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).is_err());

    let fixture = accepted();
    let path = fixture.directory.join("evaluation.v1.json");
    let mut evaluation: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    evaluation
        .as_object_mut()
        .unwrap()
        .insert("aggregate_health".into(), Value::String("healthy".into()));
    fs::write(path, serde_json::to_vec(&evaluation).unwrap()).unwrap();
    assert!(load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).is_err());
}

#[test]
fn symlinked_fixed_source_and_absent_source_are_refused() {
    let fixture = accepted();
    let profile = fixture.directory.join("profile.v1.json");
    let moved = fixture.directory.join("profile.actual.json");
    fs::rename(&profile, &moved).unwrap();
    symlink(&moved, &profile).unwrap();
    assert!(load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).is_err());

    let fixture = accepted();
    fs::remove_file(fixture.directory.join("nq.v1.json")).unwrap();
    assert!(load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).is_err());
}

#[test]
fn upstream_nonclaim_becomes_only_a_bound_presentation_question() {
    let mut nq: Value = serde_json::from_slice(NQ).unwrap();
    let input = nq["inputs"][0].as_object_mut().unwrap();
    let support = input
        .get_mut("claim_support")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .remove(0);
    let claim_id = support["claim_id"].as_str().unwrap().to_owned();
    input.insert("claim_support".into(), Value::Array(Vec::new()));
    input.insert(
        "cannot_testify".into(),
        serde_json::json!([{
            "claim_id": claim_id,
            "reason": "profile claim absent from exact observation payload"
        }]),
    );
    let fixture = fixture(serde_json::to_vec(&nq).unwrap());
    let loaded = load_operational_conditions_at(std::slice::from_ref(&fixture.directory)).unwrap();
    let projection = &loaded.values().next().unwrap().projection;
    assert_eq!(projection.questions.len(), 1);
    let question = &projection.questions[0];
    assert!(question.presentation_only);
    assert_eq!(
        question.next_lawful_action,
        projection.evaluation.next_lawful_action
    );
    match &question.source {
        OperationalQuestionSourceV1::CannotTestify(source) => {
            assert_eq!(source.claim_id, "claim:availability");
            assert_eq!(
                source.reason,
                "profile claim absent from exact observation payload"
            );
        }
        other => panic!("unexpected question source: {other:?}"),
    }
    assert!(!serde_json::to_string(projection)
        .unwrap()
        .contains("question_disposition"));
}
