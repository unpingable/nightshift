use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{TimeZone, Utc};
use nightshift_casework::{
    load_run_at, load_runs_at, CaseworkError, CaseworkRunV1, CASEWORK_RUN_DIGEST_DOMAIN_V1,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
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
    assert_eq!(
        glasshopper.outcome.state.recognized_string.as_deref(),
        Some("CLOSEOUT-COMPLETE-NOT-QUALIFIED")
    );
    assert_eq!(
        glasshopper
            .outcome
            .result_classification
            .recognized_string
            .as_deref(),
        Some("CLOSEOUT-COMPLETE-CAMPAIGN-NOT-QUALIFIED")
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

fn independently_derived_id(domain: &str, components: &[&str]) -> String {
    let canonical = serde_jcs::to_vec(components).unwrap();
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(canonical);
    format!("sha256:{:x}", digest.finalize())
}

#[test]
fn checked_in_projection_and_exact_identity_vectors_are_independently_recalculated() {
    let run = load_run_at(Path::new(GOLDEN), instant()).unwrap();
    let checked_in_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../qualification/nightshift-casework-mvp-20260829/velvet-orrery.casework-run.v1.json"
    ));
    let checked_in: CaseworkRunV1 = serde_json::from_slice(checked_in_bytes).unwrap();
    assert_eq!(checked_in, run.projection);

    let projection = &run.projection;
    let packet_digest = projection.packet.packet_digest.as_str();
    let first_item = &projection.work_items[0];
    assert_eq!(
        first_item.derived_id,
        "sha256:ca0c5713fb4fde3ef1e013e928a28346d437ea53222338834ad66a502e5f5fac"
    );
    assert_eq!(
        first_item.derived_id,
        independently_derived_id(
            "nightshift.casework.work-item/v1",
            &[packet_digest, first_item.id.as_str()],
        )
    );

    let first_question = &projection.human_questions[0];
    let exact_question = first_question
        .exact_question
        .recognized_string
        .as_deref()
        .unwrap();
    let independent_question_id = independently_derived_id(
        "nightshift.casework.question/v1",
        &[
            packet_digest,
            first_question.linked_work_item.as_deref().unwrap(),
            exact_question,
        ],
    );
    assert_eq!(
        first_question.derived_id.as_deref(),
        Some("sha256:e299a26068a20631a8c4985fbb249cbc991a3d141be7c2f9cf253a0c11b3e088")
    );
    assert_eq!(
        first_question.derived_id.as_deref(),
        Some(independent_question_id.as_str())
    );
    assert_eq!(
        first_question.navigation_id,
        "sha256:0455c0e4aa1aaa9dc8ae3b2f1382a12cda87c36271926de2b4c2f99607a16b9d"
    );
    assert_eq!(
        first_question.navigation_id,
        independently_derived_id("nightshift.casework.question-row/v1", &[packet_digest, "0"],)
    );

    let first_starting = &projection.packet.repository_custody[0];
    assert_eq!(
        first_starting.derived_id,
        "sha256:6b1ea0f47b73aa697c60ed5562b5429d505e4ec767d00c0c4511f1107be2adb4"
    );
    assert_eq!(
        first_starting.derived_id,
        independently_derived_id(
            "nightshift.casework.custody-row/v1",
            &[
                packet_digest,
                "packet",
                first_starting.repository.as_str(),
                "0"
            ],
        )
    );

    let first_final = &projection.final_repository_custody[0];
    assert_eq!(
        first_final.derived_id.as_deref(),
        Some("sha256:01c5dda9a9208668481eb54dfb06cf8eebd829217478f3dbb12d5ed64904648f")
    );
    assert_eq!(
        first_final.derived_id.as_deref().unwrap(),
        independently_derived_id(
            "nightshift.casework.custody-row/v1",
            &[
                packet_digest,
                "receipts",
                first_final.repository.recognized_string.as_deref().unwrap(),
                "0"
            ],
        )
    );
    assert_eq!(
        first_final.navigation_id,
        "sha256:d53dae4f65913014d5cf4521a56ba5fa8efd50bd7462931af1fa691cb329be85"
    );
    assert_eq!(
        first_final.navigation_id,
        independently_derived_id(
            "nightshift.casework.custody-row-navigation/v1",
            &[packet_digest, "receipts", "0"],
        )
    );

    let mut digest_value = serde_json::to_value(projection).unwrap();
    digest_value
        .as_object_mut()
        .unwrap()
        .remove("projection_digest");
    let canonical = serde_jcs::to_vec(&digest_value).unwrap();
    let mut digest = Sha256::new();
    digest.update(CASEWORK_RUN_DIGEST_DOMAIN_V1);
    digest.update(canonical);
    let independent_projection_digest = format!("sha256:{:x}", digest.finalize());
    assert_eq!(
        projection.projection_digest,
        "sha256:aa2e823cf8d8f323af1ed2e6a1cfc27dc84e8193f3915de75a03a348654651e8"
    );
    assert_eq!(projection.projection_digest, independent_projection_digest);
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
    assert_eq!(repositories.recognized_rows, None);
}

#[test]
fn unknown_state_and_classification_remain_verbatim() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["work_items"][0]["state"] = json!(Some("STATE-NOT-IN-ANY-TAXONOMY"));
        value["work_items"][0]["result_classification"] = json!(Some("UNCLASSIFIED-LITERAL"));
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    assert_eq!(
        run.projection.work_items[0]
            .outcome
            .state
            .recognized_string
            .as_deref(),
        Some("STATE-NOT-IN-ANY-TAXONOMY")
    );
    assert_eq!(
        run.projection.work_items[0]
            .outcome
            .result_classification
            .recognized_string
            .as_deref(),
        Some("UNCLASSIFIED-LITERAL")
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
        value["repository_custody"][0]
            .as_object_mut()
            .unwrap()
            .remove("dirty");
    });
    assert!(load_run_at(custody.path(), instant())
        .unwrap_err()
        .to_string()
        .contains("missing required field dirty"));
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
#[test]
fn renderer_loose_scalars_and_joinables_are_accepted_but_not_promoted() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["work_items"][0]["state"] = json!(17);
        value["work_items"][0]["result_classification"] = json!({"future": true});
        value["work_items"][0]["remaining_trigger"] = json!(false);
        value["work_items"][0]["next_lawful_action"] = json!(["not", "text"]);
        value["work_items"][0]["tests"] = json!("ab");
        value["work_items"][0]["evidence"] = json!({"second": 2, "first": 1});
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    let outcome = &run.projection.work_items[0].outcome;
    assert_eq!(outcome.state.recognized_string, None);
    assert_eq!(outcome.result_classification.recognized_string, None);
    assert_eq!(outcome.remaining_trigger.recognized_string, None);
    assert_eq!(outcome.next_lawful_action.recognized_string, None);
    assert_eq!(outcome.tests.recognized_strings, None);
    assert_eq!(outcome.evidence.recognized_strings, None);
    assert_eq!(run.projection.summary.unrecognized_state_count, 1);
}

#[test]
fn renderer_loose_question_and_custody_cells_remain_raw_only() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["human_questions"][0]["work_item"] = json!({"future": "link"});
        value["human_questions"][0]["exact_question"] = json!({"future": "question"});
        value["human_questions"][0]["safe_default"] = json!(false);
        value["repository_custody"][0]["dirty"] = json!(false);
        value["repository_custody"][0]["repository"] = json!(["future"]);
        value["repository_custody"][0]["teardown"] = json!(["none"]);
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    let question = &run.projection.human_questions[0];
    assert_eq!(question.derived_id, None);
    assert_eq!(question.work_item.recognized_string, None);
    assert_eq!(question.linked_work_item, None);
    assert!(question.navigation_id.starts_with("sha256:"));
    let custody = &run.projection.final_repository_custody[0];
    assert_eq!(custody.derived_id, None);
    assert_eq!(custody.repository.recognized_string, None);
    assert!(custody.navigation_id.starts_with("sha256:"));
    assert_eq!(question.exact_question.recognized_string, None);
    assert_eq!(question.safe_default.recognized_string, None);
    assert_eq!(
        run.projection.final_repository_custody[0]
            .dirty
            .recognized_string,
        None
    );
    assert_eq!(
        run.projection.final_repository_custody[0]
            .teardown
            .recognized_string,
        None
    );
}

#[test]
fn renderer_unlinked_string_question_is_retained_without_semantic_linkage() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["human_questions"][0]["work_item"] = json!("future-work-item");
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    let question = &run.projection.human_questions[0];
    assert_eq!(
        question.work_item.recognized_string.as_deref(),
        Some("future-work-item")
    );
    assert_eq!(question.linked_work_item, None);
    assert_eq!(question.derived_id, None);
    assert!(question.navigation_id.starts_with("sha256:"));
}

#[test]
fn non_rfc3339_snapshot_time_is_accepted_with_unavailable_currentness() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        value["updated_at"] = json!({"future_clock": 1});
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    assert_eq!(run.projection.receipts.updated_at.recognized_string, None);
    assert_eq!(run.projection.receipts.updated_at.recognized_rfc3339, None);
    assert_eq!(
        run.projection.packet.currentness_at_receipt_snapshot,
        "UNAVAILABLE"
    );
}

#[test]
fn duplicate_questions_keep_base_identity_and_unique_navigation_ids() {
    let dir = fixture();
    mutate_receipts(dir.path(), |value| {
        let duplicate = value["human_questions"][0].clone();
        value["human_questions"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
    });
    let run = load_run_at(dir.path(), instant()).unwrap();
    assert_eq!(run.projection.human_questions.len(), 7);
    let first = &run.projection.human_questions[0];
    let duplicate = &run.projection.human_questions[6];
    assert_eq!(first.derived_id, duplicate.derived_id);
    assert_ne!(first.navigation_id, duplicate.navigation_id);
    assert_ne!(first.source_ordinal, duplicate.source_ordinal);
}
