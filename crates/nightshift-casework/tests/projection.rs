use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{TimeZone, Utc};
use nightshift_casework::{load_run_at, load_runs_at, CaseworkError};
use serde_json::{json, Value};
use tempfile::TempDir;

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../qualification/nightshift-packet-v1/velvet-orrery"
);

fn instant() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap()
}

fn fixture() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(
        Path::new(GOLDEN).join("packet.v1.json"),
        dir.path().join("packet.v1.json"),
    )
    .unwrap();
    fs::copy(
        Path::new(GOLDEN).join("run-receipts.v1.json"),
        dir.path().join("run-receipts.v1.json"),
    )
    .unwrap();
    dir
}

fn mutate_receipts(dir: &Path, mutation: impl FnOnce(&mut Value)) {
    let path = dir.join("run-receipts.v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    mutation(&mut value);
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn velvet_projects_exact_golden_counts_and_historical_currentness() {
    let run = load_run_at(Path::new(GOLDEN), instant()).unwrap();
    assert_eq!(run.projection.summary.work_item_count, 14);
    assert_eq!(run.projection.summary.human_question_count, 6);
    assert_eq!(
        run.projection.packet.currentness_at_receipt_snapshot,
        "CURRENT"
    );
    assert_eq!(run.projection.packet.currentness_now, "EXPIRED");
    assert_eq!(
        run.packet_bytes,
        fs::read(Path::new(GOLDEN).join("packet.v1.json")).unwrap()
    );
    assert_eq!(
        run.receipt_bytes,
        fs::read(Path::new(GOLDEN).join("run-receipts.v1.json")).unwrap()
    );
    let glasshopper = run
        .projection
        .work_items
        .iter()
        .find(|item| item.campaign.codename == "GLASSHOPPER")
        .unwrap();
    assert_eq!(glasshopper.outcome.state, "CLOSEOUT-COMPLETE-NOT-QUALIFIED");
    assert_eq!(
        glasshopper.outcome.result_classification,
        "CLOSEOUT-COMPLETE-CAMPAIGN-NOT-QUALIFIED"
    );
}

#[test]
fn projection_and_derived_ids_are_deterministic() {
    let first = load_run_at(Path::new(GOLDEN), instant()).unwrap();
    let second = load_run_at(Path::new(GOLDEN), instant()).unwrap();
    assert_eq!(first.projection, second.projection);
    assert!(first.projection.projection_digest.starts_with("sha256:"));
    assert!(first
        .projection
        .work_items
        .iter()
        .all(|item| item.derived_id.starts_with("sha256:")));
}

#[test]
fn unknown_extensions_stay_only_in_exact_raw_bytes() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["future_extension"] = json!({"meaning": "must remain raw"});
        value["work_items"][0]["future_item_extension"] = json!("raw-only");
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    let projected = serde_json::to_string(&run.projection).unwrap();
    assert!(!projected.contains("future_extension"));
    assert!(!projected.contains("future_item_extension"));
    assert!(String::from_utf8(run.receipt_bytes)
        .unwrap()
        .contains("future_extension"));
}

#[test]
fn renderer_accepted_repository_json_is_not_retroactively_rejected() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["work_items"][0]["repositories"] = json!({"future_shape": [1, 2, 3]});
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    let repositories = &run.projection.work_items[0].outcome.repositories;
    assert_eq!(repositories.canonical_json, r#"{"future_shape":[1,2,3]}"#);
    assert_eq!(repositories.recognized_rows, None);
}

#[test]
fn unknown_state_and_classification_remain_verbatim() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["work_items"][0]["state"] = json!("STATE-NOT-IN-ANY-TAXONOMY");
        value["work_items"][0]["result_classification"] = json!("UNCLASSIFIED-LITERAL");
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    assert_eq!(
        run.projection.work_items[0].outcome.state,
        "STATE-NOT-IN-ANY-TAXONOMY"
    );
    assert_eq!(
        run.projection.work_items[0].outcome.result_classification,
        "UNCLASSIFIED-LITERAL"
    );
}

#[test]
fn receipt_packet_digest_mismatch_is_refused() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["packet_digest"] =
            json!("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    });
    assert!(load_run_at(dir.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("receipt packet digest mismatch"));
}

#[test]
fn duplicate_unknown_and_missing_items_are_refused() {
    let duplicate = fixture();
    mutate_receipts(duplicate.path(), |value| {
        let row = value["work_items"][0].clone();
        value["work_items"].as_array_mut().unwrap().push(row);
    });
    assert!(load_run_at(duplicate.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("duplicate receipt work item"));

    let unknown = fixture();
    mutate_receipts(unknown.path(), |value| {
        value["work_items"][0]["id"] = json!("unknown-item");
    });
    assert!(load_run_at(unknown.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("unknown receipt work item"));

    let missing = fixture();
    mutate_receipts(missing.path(), |value| {
        value["work_items"].as_array_mut().unwrap().pop();
    });
    assert!(load_run_at(missing.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("missing receipt work item"));
}

#[test]
fn malformed_question_and_custody_are_refused() {
    let question = fixture();
    mutate_receipts(question.path(), |value| {
        value["human_questions"][0]
            .as_object_mut()
            .unwrap()
            .remove("safe_default");
    });
    assert!(load_run_at(question.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("safe_default"));

    let custody = fixture();
    mutate_receipts(custody.path(), |value| {
        value["repository_custody"][0]["dirty"] = Value::Bool(false);
    });
    assert!(load_run_at(custody.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("repository_custody.dirty"));
}

#[test]
fn substituted_packet_digest_is_refused_before_projection() {
    let dir = fixture();
    let path = dir.path().join("packet.v1.json");
    let mut packet: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    packet["packet_digest"] =
        json!("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    packet["switchyard"]["plan_ref"] = json!(
        "nightshift-packet://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    fs::write(path, serde_json::to_vec(&packet).unwrap()).unwrap();
    assert!(load_run_at(dir.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("packet digest mismatch"));
}

#[test]
fn duplicate_run_digest_is_refused() {
    let dirs = vec![PathBuf::from(GOLDEN), PathBuf::from(GOLDEN)];
    assert!(matches!(
        load_runs_at(&dirs, instant()),
        Err(CaseworkError::DuplicateRun(_))
    ));
}

#[cfg(unix)]
#[test]
fn symlinked_input_file_is_refused() {
    use std::os::unix::fs::symlink;

    let dir = fixture();
    let packet = dir.path().join("packet.v1.json");
    fs::remove_file(&packet).unwrap();
    symlink(Path::new(GOLDEN).join("packet.v1.json"), &packet).unwrap();
    assert!(load_run_at(dir.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("non-symlink"));
}
