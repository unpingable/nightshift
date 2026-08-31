//! Emit exact EPOCH/Casework condition directories for SILICON-ORCHARD.

use std::{collections::BTreeMap, env, fs, path::PathBuf};

use chrono::{TimeZone as _, Utc};
use nightshiftd::operational_lineage::{
    admit_operational_lineage, evaluate_reobservation, OperationalObservationLineageV1,
    ReobservationProfileV1,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Bundle {
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
    cases: Vec<NqCase>,
}

#[derive(Deserialize)]
struct NqCase {
    scenario: String,
    qualification: Value,
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    assert_eq!(
        arguments.len(),
        4,
        "usage: emit_silicon_conditions MONITOR_BUNDLE NQ_ARTIFACT OUTPUT_DIRECTORY"
    );
    let monitor_bundle = fs::read(&arguments[1]).expect("Monitor bundle");
    let nq_bytes = fs::read(&arguments[2]).expect("NQ artifact");
    let output = PathBuf::from(&arguments[3]);
    fs::create_dir(&output).expect("new output directory");
    let bundle: Bundle = serde_json::from_slice(&monitor_bundle).expect("exact Monitor bundle");
    let nq_bundle: NqBundle = serde_json::from_slice(&nq_bytes).expect("exact NQ bundle");
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
