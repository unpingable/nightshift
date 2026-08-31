//! Emit exact EPOCH/Casework condition directories for SILICON-ORCHARD.

use std::{collections::BTreeMap, env, fs, path::PathBuf};

use chrono::{TimeZone as _, Utc};
use nightshiftd::operational_lineage::{
    admit_operational_lineage, evaluate_reobservation, OperationalObservationLineageV1,
    ReobservationProfileV1,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

#[derive(Deserialize)]
struct Bundle {
    schema: String,
    monitor_result_head: String,
    distant_result_head: String,
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    scenario: String,
    subject_identity_digest: String,
    signed_monitor_record_json: String,
}

#[derive(Deserialize)]
struct NqBundle {
    schema: String,
    monitor_fixture_head: String,
    monitor_bundle_digest: String,
    cases: Vec<NqCase>,
}

#[derive(Deserialize)]
struct NqCase {
    scenario: String,
    qualification: Value,
    eligibility: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerBinding {
    schema: String,
    monitor_fixture_head: String,
    nq_result_head: String,
    monitor_bundle_sha256: String,
    nq_artifact_sha256: String,
    claim_deck_digest: String,
    grants_authority: bool,
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    assert_eq!(
        arguments.len(),
        5,
        "usage: emit_silicon_conditions MONITOR_BUNDLE NQ_ARTIFACT OWNER_BINDING OUTPUT_DIRECTORY"
    );
    let monitor_bundle = fs::read(&arguments[1]).expect("Monitor bundle");
    let nq_bytes = fs::read(&arguments[2]).expect("NQ artifact");
    let owner_bytes = fs::read(&arguments[3]).expect("owner binding");
    let output = PathBuf::from(&arguments[4]);
    fs::create_dir(&output).expect("new output directory");
    let bundle: Bundle = serde_json::from_slice(&monitor_bundle).expect("exact Monitor bundle");
    let nq_bundle: NqBundle = serde_json::from_slice(&nq_bytes).expect("exact NQ bundle");
    let owner: OwnerBinding = serde_json::from_slice(&owner_bytes).expect("exact owner binding");
    assert_eq!(owner.schema, "nightshift.silicon-owner-binding/v1");
    assert_eq!(
        owner.monitor_fixture_head,
        "bb75c4325f903f2c544e9758b5ea8d30c8bbc773"
    );
    assert_eq!(
        owner.nq_result_head,
        "78ba5137c83089d6f1cd2bada65f6f7bdda2669c"
    );
    assert_eq!(owner.monitor_bundle_sha256, sha256(&monitor_bundle));
    assert_eq!(owner.nq_artifact_sha256, sha256(&nq_bytes));
    assert_eq!(
        owner.claim_deck_digest,
        "sha256:7f9ba67910df6962e4e02cb2e1fa75562a59889e16cef3c9133c90aa090cea0d"
    );
    assert!(!owner.grants_authority);
    assert_eq!(bundle.schema, "monitor.ecad-golden-fixture/v1");
    assert_eq!(
        bundle.monitor_result_head,
        "b2d52fe34f146774cbf5601819982c267c7fb082"
    );
    assert_eq!(
        bundle.distant_result_head,
        "8a1adaae27a5da70398b445c152cd4e7548b0289"
    );
    assert_eq!(nq_bundle.schema, "nq.ecad-qualification-bundle/v1");
    assert_eq!(nq_bundle.monitor_fixture_head, owner.monitor_fixture_head);
    assert_eq!(nq_bundle.monitor_bundle_digest, owner.monitor_bundle_sha256);
    assert_eq!(nq_bundle.cases.len(), 20);
    assert!(nq_bundle
        .cases
        .iter()
        .all(|case| case.eligibility["deck_digest"] == owner.claim_deck_digest));
    let admitted_at = Utc
        .with_ymd_and_hms(2026, 8, 30, 16, 41, 0)
        .single()
        .expect("fixed admission time");
    let evaluated_at = Utc
        .with_ymd_and_hms(2026, 8, 30, 16, 50, 0)
        .single()
        .expect("fixed evaluation time");
    let mut histories: BTreeMap<String, Vec<OperationalObservationLineageV1>> = BTreeMap::new();
    for entry in bundle.entries {
        let directory = output.join(&entry.scenario);
        fs::create_dir(&directory).expect("new condition directory");
        let monitor_bytes = entry.signed_monitor_record_json.into_bytes();
        let input_id = format!("silicon:{}", entry.scenario);
        let case = nq_bundle
            .cases
            .iter()
            .find(|value| value.scenario == entry.scenario)
            .expect("closed NQ case");
        let case_nq_bytes = serde_json::to_vec(&case.qualification).expect("NQ case serializes");
        let history = histories.entry(entry.subject_identity_digest).or_default();
        let (lineage, _) = admit_operational_lineage(
            &monitor_bytes,
            &case_nq_bytes,
            &input_id,
            admitted_at,
            history,
        )
        .expect("exact EPOCH admission");
        history.push(lineage.clone());
        let profile = ReobservationProfileV1 {
            profile_id: format!("profile:silicon-currentness:{}", entry.scenario),
            max_age_seconds: 3600,
        };
        let evaluation =
            evaluate_reobservation(&lineage, &profile, evaluated_at).expect("exact evaluation");
        fs::write(directory.join("monitor.v1.json"), monitor_bytes).expect("Monitor bytes");
        fs::write(directory.join("nq.v1.json"), case_nq_bytes).expect("NQ bytes");
        fs::write(
            directory.join("lineage.v1.json"),
            serde_json::to_vec(&lineage).expect("lineage serializes"),
        )
        .expect("lineage bytes");
        fs::write(
            directory.join("profile.v1.json"),
            serde_json::to_vec(&profile).expect("profile serializes"),
        )
        .expect("profile bytes");
        fs::write(
            directory.join("evaluation.v1.json"),
            serde_json::to_vec(&evaluation).expect("evaluation serializes"),
        )
        .expect("evaluation bytes");
    }
}
