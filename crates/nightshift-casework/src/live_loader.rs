use std::{collections::BTreeMap, path::Path};

use chrono::{DateTime, Utc};
use nightshift_foreman::{
    read_only_run_snapshot, ExecutionProfileV2, ForemanAdmissionV1, ReadOnlyRunSnapshotV1,
};
use nightshiftd::packet::NightshiftPacketV1;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

use crate::live_model::*;

#[derive(Debug, Error)]
pub enum LiveCaseworkError {
    #[error("foreman read snapshot failed: {0}")]
    Foreman(String),
    #[error("live packet is invalid: {0}")]
    Packet(String),
    #[error("live foreman contract is invalid: {0}")]
    Contract(String),
    #[error("live source identities do not agree: {0}")]
    Identity(&'static str),
    #[error("live projection serialization failed: {0}")]
    Projection(String),
}

#[derive(Clone, Debug)]
pub struct LoadedLiveRun {
    pub projection: CaseworkLiveRunV1,
    pub packet_bytes: Vec<u8>,
    pub admission_bytes: Vec<u8>,
    pub profile_bytes: Vec<u8>,
    pub journal_framing_bytes: Vec<u8>,
    pub accepted_receipts_framing_bytes: Vec<u8>,
    pub event_bytes: BTreeMap<u64, Vec<u8>>,
    pub accepted_receipt_bytes: BTreeMap<String, Vec<u8>>,
    pub final_snapshot_bytes: Option<Vec<u8>>,
}

pub fn load_live_run_at(
    store_path: &Path,
    run_id: &str,
    evaluated_at: DateTime<Utc>,
) -> Result<LoadedLiveRun, LiveCaseworkError> {
    let snapshot = read_only_run_snapshot(store_path, run_id)
        .map_err(|error| LiveCaseworkError::Foreman(error.to_string()))?;
    project(snapshot, evaluated_at)
}

fn project(
    snapshot: ReadOnlyRunSnapshotV1,
    evaluated_at: DateTime<Utc>,
) -> Result<LoadedLiveRun, LiveCaseworkError> {
    if snapshot.projection.work_items.len() > 4096
        || snapshot.projection.resource_claims.len() > 4096
        || snapshot.events.len() > 1_048_576
        || snapshot.terminal_receipts.len() > 4096
        || snapshot
            .events
            .iter()
            .any(|event| event.raw_bytes.is_empty() || event.raw_bytes.len() > 16 * 1024 * 1024)
    {
        return Err(LiveCaseworkError::Contract(
            "live snapshot exceeds projection collection or byte bounds".to_owned(),
        ));
    }
    let packet = NightshiftPacketV1::from_slice(&snapshot.packet_bytes)
        .map_err(|error| LiveCaseworkError::Packet(error.to_string()))?;
    packet
        .validate_integrity()
        .map_err(|error| LiveCaseworkError::Packet(error.to_string()))?;
    let admission = ForemanAdmissionV1::from_slice(&snapshot.admission_bytes)
        .map_err(|error| LiveCaseworkError::Contract(error.to_string()))?;
    admission
        .validate()
        .map_err(|error| LiveCaseworkError::Contract(error.to_string()))?;
    let profile = ExecutionProfileV2::from_slice(&snapshot.profile_bytes)
        .map_err(|error| LiveCaseworkError::Contract(error.to_string()))?;
    profile
        .validate()
        .map_err(|error| LiveCaseworkError::Contract(error.to_string()))?;
    if snapshot.run_id != admission.run_id
        || snapshot.projection.run_id != admission.run_id
        || snapshot.projection.packet_id != packet.packet_id
    {
        return Err(LiveCaseworkError::Identity("run_id or packet_id"));
    }
    if admission.packet_digest != packet.packet_digest
        || profile.packet_digest != packet.packet_digest
        || snapshot.projection.packet_digest != packet.packet_digest
    {
        return Err(LiveCaseworkError::Identity("packet_digest"));
    }
    if profile.admission_digest != admission.admission_digest
        || snapshot.projection.admission_digest != admission.admission_digest
        || snapshot.projection.profile_digest != profile.profile_digest
    {
        return Err(LiveCaseworkError::Identity(
            "admission_digest or profile_digest",
        ));
    }

    let packet_sha = plain_sha256(&snapshot.packet_bytes);
    let admission_sha = plain_sha256(&snapshot.admission_bytes);
    let profile_sha = plain_sha256(&snapshot.profile_bytes);
    let journal_framing_bytes = journal_framing(&snapshot);
    let journal_sha = plain_sha256(&journal_framing_bytes);
    let final_snapshot_sha = snapshot.final_snapshot_bytes.as_deref().map(plain_sha256);
    let receipts_by_item: BTreeMap<_, _> = snapshot
        .terminal_receipts
        .iter()
        .map(|receipt| (receipt.work_item_id.as_str(), receipt))
        .collect();
    let event_bytes = snapshot
        .events
        .iter()
        .map(|event| (event.sequence, event.raw_bytes.clone()))
        .collect();
    let accepted_receipt_bytes = snapshot
        .terminal_receipts
        .iter()
        .map(|receipt| (receipt.work_item_id.clone(), receipt.raw_bytes.clone()))
        .collect();
    let accepted_receipts_framing_bytes = accepted_receipts_framing(&accepted_receipt_bytes);
    let accepted_receipts_framing_sha = plain_sha256(&accepted_receipts_framing_bytes);
    let packet_items: BTreeMap<_, _> = packet
        .work_items
        .iter()
        .map(|item| (&item.id, item))
        .collect();

    let mut state_counts = BTreeMap::new();
    let mut work_items = Vec::with_capacity(snapshot.projection.work_items.len());
    for mechanism in &snapshot.projection.work_items {
        let intent = packet_items
            .get(&mechanism.work_item_id)
            .ok_or(LiveCaseworkError::Identity("projection work_item_id"))?;
        let scheduler_state = serde_json::to_value(&mechanism.scheduler_state)
            .map_err(|error| LiveCaseworkError::Projection(error.to_string()))?
            .as_str()
            .ok_or_else(|| {
                LiveCaseworkError::Projection("scheduler state is not a string".to_owned())
            })?
            .to_owned();
        *state_counts.entry(scheduler_state.clone()).or_insert(0) += 1;
        let accepted_receipt = receipts_by_item
            .get(mechanism.work_item_id.as_str())
            .copied();
        let accepted_outcome = accepted_receipt.map(|receipt| LiveAcceptedOutcomeV1 {
            state: receipt.state.clone(),
            result_classification: receipt.result_classification.clone(),
            receipt_digest: receipt.receipt_digest.clone(),
        });
        let accepted_receipt_kind = accepted_receipt.map(|receipt| receipt.receipt_kind.clone());
        let accepted_outcome_absent_reason = if accepted_outcome.is_none() {
            Some("NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT".to_owned())
        } else {
            None
        };
        work_items.push(LiveWorkItemV1 {
            work_item_id: intent.id.clone(),
            track: intent.track.clone(),
            campaign_codename: intent.campaign.codename.clone(),
            campaign_slug: intent.campaign.canonical_slug.clone(),
            dependencies: intent.dependencies.clone(),
            entry_predicates: intent.entry_predicates.clone(),
            stop_conditions: intent.stop_conditions.clone(),
            scheduler_state,
            scheduler_state_recognized: true,
            dependency_terminality: mechanism.dependency_terminality.clone(),
            resource_lock_keys: mechanism.resource_lock_keys.clone(),
            active_attempt_id: mechanism.active_attempt_id.clone(),
            adapter_id: mechanism.adapter_id.clone(),
            adapter_version: mechanism.adapter_version.clone(),
            provider_model_class: mechanism.provider_model_class.clone(),
            provider_identity: mechanism.provider_identity.clone(),
            model_identity: mechanism.model_identity.clone(),
            session_identity: mechanism.session_identity.clone(),
            thread_identity: mechanism.thread_identity.clone(),
            turn_identity: mechanism.turn_identity.clone(),
            queue_identity: mechanism.queue_identity.clone(),
            last_event_sequence: mechanism.last_event_sequence,
            last_event_digest: mechanism.last_event_digest.clone(),
            human_questions: mechanism
                .human_questions
                .iter()
                .map(|question| LiveQuestionV1 {
                    navigation_id: question_navigation_id(
                        &mechanism.work_item_id,
                        &question.question_id,
                    ),
                    question_id: question.question_id.clone(),
                    question: question.question.clone(),
                    exhausted_evidence: question.exhausted_evidence.clone(),
                    safe_default: question.safe_default.clone(),
                    consequences: question.consequences.clone(),
                    resume_point: question.resume_point.clone(),
                })
                .collect(),
            accepted_receipt_kind,
            accepted_outcome,
            accepted_outcome_absent_reason,
        });
    }
    if work_items.len() != packet.work_items.len() {
        return Err(LiveCaseworkError::Identity("work item count"));
    }

    let events = snapshot
        .events
        .iter()
        .map(|event| LiveEventV1 {
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            work_item_id: event.work_item_id.clone(),
            attempt_id: event.attempt_id.clone(),
            kind: event.kind.clone(),
            recorded_at: event.recorded_at.clone(),
            retained_raw_digest: event.raw_digest.clone(),
            exact_bytes_sha256: plain_sha256(&event.raw_bytes),
            raw_length: event.raw_bytes.len(),
        })
        .collect();
    let lifecycle = if snapshot.final_snapshot_bytes.is_some() {
        "CLOSED_EXACT_FINAL_SNAPSHOT_RETAINED"
    } else {
        "OPEN"
    }
    .to_owned();
    let terminal_receipt_count = snapshot
        .terminal_receipts
        .iter()
        .filter(|receipt| receipt.receipt_kind == "terminal")
        .count();
    let not_started_receipt_count = snapshot
        .terminal_receipts
        .iter()
        .filter(|receipt| receipt.receipt_kind == "not_started")
        .count();

    let mut projection = CaseworkLiveRunV1 {
        schema: CASEWORK_LIVE_RUN_SCHEMA_V1.to_owned(),
        projection_digest: String::new(),
        navigation_id: navigation_id(&snapshot.run_id),
        run_id: snapshot.run_id.clone(),
        evaluated_at: evaluated_at.to_rfc3339(),
        packet: LivePacketV1 {
            packet_id: packet.packet_id.clone(),
            packet_digest: packet.packet_digest.clone(),
            exact_bytes_sha256: packet_sha.clone(),
            integrity: "VALID".to_owned(),
            created_at: packet.created_at.to_rfc3339(),
            current_until: packet.current_until.to_rfc3339(),
            currentness: currentness(evaluated_at, packet.created_at, packet.current_until),
        },
        admission: LiveAdmissionV1 {
            admission_digest: admission.admission_digest.clone(),
            exact_bytes_sha256: admission_sha.clone(),
            admitted_at: admission.admitted_at.to_rfc3339(),
            expires_at: admission.expires_at.to_rfc3339(),
            currentness: currentness(evaluated_at, admission.admitted_at, admission.expires_at),
            maximum_concurrent_workers: admission.maximum_concurrent_workers,
        },
        execution_profile: LiveExecutionProfileV1 {
            profile_digest: profile.profile_digest.clone(),
            exact_bytes_sha256: profile_sha.clone(),
            budget_policy_ref: profile.budget_policy_ref,
            capacity_binding_status: "POLICY_REFERENCE_ONLY_NO_RECORDED_DECISION".to_owned(),
        },
        foreman: LiveForemanV1 {
            source_schema: snapshot.projection.schema.clone(),
            lifecycle,
            scheduler_state_counts: state_counts,
            terminal_receipt_count,
            not_started_receipt_count,
            closed_final_receipts_digest: snapshot
                .projection
                .closed_final_receipts_digest
                .clone(),
        },
        work_items,
        resource_claims: snapshot
            .projection
            .resource_claims
            .iter()
            .map(|claim| LiveResourceClaimV1 {
                resource_lock_key: claim.resource_lock_key.clone(),
                work_item_id: claim.work_item_id.clone(),
                attempt_id: claim.attempt_id.clone(),
            })
            .collect(),
        events,
        raw_sources: LiveRawSourcesV1 {
            packet_sha256: packet_sha,
            admission_sha256: admission_sha,
            profile_sha256: profile_sha,
            journal_framing_sha256: journal_sha,
            accepted_receipts_framing_sha256: accepted_receipts_framing_sha,
            final_snapshot_sha256: final_snapshot_sha,
        },
        sealed_case_run_id: None,
        provider_capacity: LiveProviderCapacityV1 {
            status: "NOT_RECORDED_BY_FOREMAN".to_owned(),
            observation_digest: None,
            policy_digest: None,
            decision_digest: None,
            explanation:
                "The execution profile retains only a policy reference; no exact capacity observation or decision is recorded in this foreman journal."
                    .to_owned(),
        },
        authority_effect: "READ_ONLY_OPERATOR_PROJECTION".to_owned(),
    };
    projection.projection_digest = projection_digest(&projection)?;
    Ok(LoadedLiveRun {
        projection,
        packet_bytes: snapshot.packet_bytes,
        admission_bytes: snapshot.admission_bytes,
        profile_bytes: snapshot.profile_bytes,
        journal_framing_bytes,
        accepted_receipts_framing_bytes,
        event_bytes,
        accepted_receipt_bytes,
        final_snapshot_bytes: snapshot.final_snapshot_bytes,
    })
}

pub(crate) fn reseal_live_projection(
    projection: &mut CaseworkLiveRunV1,
) -> Result<(), LiveCaseworkError> {
    projection.projection_digest = projection_digest(projection)?;
    Ok(())
}

fn currentness(now: DateTime<Utc>, begins: DateTime<Utc>, ends: DateTime<Utc>) -> String {
    if now < begins {
        "NOT_YET_CURRENT"
    } else if now > ends {
        "EXPIRED"
    } else {
        "CURRENT"
    }
    .to_owned()
}

fn plain_sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn navigation_id(run_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CASEWORK_LIVE_NAVIGATION_DOMAIN_V1);
    hasher.update(run_id.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn question_navigation_id(work_item_id: &str, question_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CASEWORK_LIVE_QUESTION_NAVIGATION_DOMAIN_V1);
    hasher.update((work_item_id.len() as u64).to_be_bytes());
    hasher.update(work_item_id.as_bytes());
    hasher.update((question_id.len() as u64).to_be_bytes());
    hasher.update(question_id.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn journal_framing(snapshot: &ReadOnlyRunSnapshotV1) -> Vec<u8> {
    let capacity = snapshot
        .events
        .iter()
        .fold(FOREMAN_JOURNAL_FRAMING_V1.len(), |total, event| {
            total
                .saturating_add(16)
                .saturating_add(event.raw_bytes.len())
        });
    let mut framed = Vec::with_capacity(capacity);
    framed.extend_from_slice(FOREMAN_JOURNAL_FRAMING_V1);
    for event in &snapshot.events {
        framed.extend_from_slice(&event.sequence.to_be_bytes());
        framed.extend_from_slice(&(event.raw_bytes.len() as u64).to_be_bytes());
        framed.extend_from_slice(&event.raw_bytes);
    }
    framed
}

fn accepted_receipts_framing(receipts: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let capacity = receipts.iter().fold(
        FOREMAN_ACCEPTED_RECEIPTS_FRAMING_V1.len(),
        |total, (work_item_id, bytes)| {
            total
                .saturating_add(16)
                .saturating_add(work_item_id.len())
                .saturating_add(bytes.len())
        },
    );
    let mut framed = Vec::with_capacity(capacity);
    framed.extend_from_slice(FOREMAN_ACCEPTED_RECEIPTS_FRAMING_V1);
    for (work_item_id, bytes) in receipts {
        framed.extend_from_slice(&(work_item_id.len() as u64).to_be_bytes());
        framed.extend_from_slice(work_item_id.as_bytes());
        framed.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        framed.extend_from_slice(bytes);
    }
    framed
}

fn projection_digest<T: Serialize>(projection: &T) -> Result<String, LiveCaseworkError> {
    let mut value = serde_json::to_value(projection)
        .map_err(|error| LiveCaseworkError::Projection(error.to_string()))?;
    let Value::Object(object) = &mut value else {
        return Err(LiveCaseworkError::Projection(
            "live projection must serialize as an object".to_owned(),
        ));
    };
    object.remove("projection_digest");
    let canonical = serde_jcs::to_vec(&value)
        .map_err(|error| LiveCaseworkError::Projection(error.to_string()))?;
    let mut hasher = Sha256::new();
    hasher.update(CASEWORK_LIVE_RUN_DIGEST_DOMAIN_V1);
    hasher.update(canonical);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::{collections::BTreeMap, fs};

    use chrono::{Duration, TimeZone as _};
    use nightshift_foreman::{
        AdapterRegistrationV2, ExecutionProfileV2, ForemanAdmissionV1, ForemanStore,
        NotStartedReceiptV1, WorkItemExecutionV1, FOREMAN_ADMISSION_SCHEMA_V1,
        FOREMAN_EXECUTION_PROFILE_SCHEMA_V2, WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1,
    };

    use super::*;

    const PACKET: &[u8] = include_bytes!(
        "../../../qualification/nightshift-operational-spine-ecad-v2-20260829/packet.v1.json"
    );

    pub(crate) fn instant() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap()
    }

    pub(crate) fn fixture() -> (tempfile::TempDir, std::path::PathBuf, String) {
        let packet = NightshiftPacketV1::from_slice(PACKET).unwrap();
        packet.validate_integrity().unwrap();
        let mut admission = ForemanAdmissionV1 {
            schema: FOREMAN_ADMISSION_SCHEMA_V1.to_owned(),
            admission_digest: format!("sha256:{}", "0".repeat(64)),
            run_id: "ledger/live:fixture".to_owned(),
            packet_digest: packet.packet_digest.clone(),
            operator_basis_digest: format!("sha256:{}", "a".repeat(64)),
            admitted_at: instant() - Duration::hours(1),
            expires_at: instant() + Duration::hours(1),
            local_runtime_identity: "ledger-fixture".to_owned(),
            maximum_concurrent_workers: 2,
            allowed_adapter_ids: vec!["fixture-adapter".to_owned()],
            allowed_provider_model_classes: vec!["bounded".to_owned()],
            maximum_new_attempts_per_work_item: 1,
            authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
            target_effects_authorized: false,
        };
        admission.seal().unwrap();
        let work_items = packet
            .work_items
            .iter()
            .map(|item| {
                (
                    item.id.clone(),
                    WorkItemExecutionV1 {
                        adapter_id: "fixture-adapter".to_owned(),
                        workspace_identity: format!("workspace:{}", item.id),
                        resource_lock_keys: vec![format!("resource:{}", item.id)],
                        provider_model_class: "bounded".to_owned(),
                    },
                )
            })
            .collect();
        let mut profile = ExecutionProfileV2 {
            schema: FOREMAN_EXECUTION_PROFILE_SCHEMA_V2.to_owned(),
            profile_digest: format!("sha256:{}", "0".repeat(64)),
            packet_digest: packet.packet_digest.clone(),
            admission_digest: admission.admission_digest.clone(),
            adapters: BTreeMap::from([(
                "fixture-adapter".to_owned(),
                AdapterRegistrationV2 {
                    adapter_id: "fixture-adapter".to_owned(),
                    protocol: "fixture.adapter/v1".to_owned(),
                    adapter_version: "fixture.adapter/v1".to_owned(),
                    executable_identity: format!("sha256:{}", "b".repeat(64)),
                    bounded_arguments: Vec::new(),
                },
            )]),
            work_items,
            budget_policy_ref: "policy:fixture".to_owned(),
            log_custody_root: "/tmp/ledger-fixture/log".to_owned(),
            receipt_custody_root: "/tmp/ledger-fixture/receipts".to_owned(),
            maximum_event_bytes: 65_536,
            maximum_receipt_bytes: 65_536,
            adapter_timeout_seconds: 60,
            closeout_policy: "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED".to_owned(),
        };
        profile.seal().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("foreman.sqlite");
        ForemanStore::open(&path)
            .unwrap()
            .admit(
                PACKET,
                &serde_jcs::to_vec(&admission).unwrap(),
                &serde_jcs::to_vec(&profile).unwrap(),
                instant(),
            )
            .unwrap();
        (directory, path, admission.run_id)
    }

    pub(crate) fn closed_fixture() -> (tempfile::TempDir, std::path::PathBuf, String, Vec<u8>) {
        let (directory, path, run_id) = fixture();
        let packet = NightshiftPacketV1::from_slice(PACKET).unwrap();
        let store = ForemanStore::open(&path).unwrap();
        for item in &packet.work_items {
            let mut receipt = NotStartedReceiptV1 {
                schema: WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1.to_owned(),
                receipt_digest: format!("sha256:{}", "0".repeat(64)),
                packet_digest: packet.packet_digest.clone(),
                run_id: run_id.clone(),
                work_item_id: item.id.clone(),
                recorded_at: instant(),
                state: "NOT-STARTED-FIXTURE".to_owned(),
                result_classification: "INDEPENDENT-FIXTURE".to_owned(),
                evidence: vec!["bounded entry evidence".to_owned()],
                remaining_trigger: "explicit successor evidence".to_owned(),
                next_lawful_action: "inspect exact receipt".to_owned(),
                human_questions: Vec::new(),
                extensions: BTreeMap::new(),
            };
            receipt.seal().unwrap();
            store
                .accept_not_started(&serde_jcs::to_vec(&receipt).unwrap())
                .unwrap();
        }
        let final_bytes = store.close(&run_id, instant()).unwrap();
        (directory, path, run_id, final_bytes)
    }

    fn database_census(path: &Path) -> BTreeMap<String, Vec<u8>> {
        let directory = path.parent().unwrap();
        let mut census = BTreeMap::new();
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                census.insert(
                    entry.file_name().to_string_lossy().into_owned(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
        census
    }

    #[test]
    fn live_projection_is_exact_separate_and_query_only() {
        let (_directory, path, run_id) = fixture();
        let before = database_census(&path);
        let loaded = load_live_run_at(&path, &run_id, instant()).unwrap();
        let after = database_census(&path);

        assert_eq!(before, after);
        assert_eq!(loaded.packet_bytes, PACKET);
        assert_eq!(loaded.projection.schema, CASEWORK_LIVE_RUN_SCHEMA_V1);
        assert_eq!(loaded.projection.run_id, "ledger/live:fixture");
        assert_eq!(loaded.projection.navigation_id.len(), 64);
        assert_eq!(loaded.projection.foreman.lifecycle, "OPEN");
        assert_eq!(
            loaded.projection.provider_capacity.status,
            "NOT_RECORDED_BY_FOREMAN"
        );
        assert!(loaded
            .projection
            .work_items
            .iter()
            .all(|item| item.accepted_outcome.is_none()
                && item.accepted_outcome_absent_reason.is_some()));
        assert!(loaded
            .journal_framing_bytes
            .starts_with(FOREMAN_JOURNAL_FRAMING_V1));
        assert_eq!(
            loaded.accepted_receipts_framing_bytes,
            FOREMAN_ACCEPTED_RECEIPTS_FRAMING_V1
        );
        assert_eq!(loaded.projection.events.len(), 1);
        assert_eq!(
            loaded.event_bytes[&loaded.projection.events[0].sequence],
            loaded.journal_framing_bytes[FOREMAN_JOURNAL_FRAMING_V1.len() + 16..]
        );
    }

    #[test]
    fn absent_and_substituted_read_sources_fail_closed_without_files() {
        let directory = tempfile::tempdir().unwrap();
        let absent = directory.path().join("absent.sqlite");
        assert!(load_live_run_at(&absent, "run", instant()).is_err());
        assert!(!absent.exists());

        let (_owned, path, _) = fixture();
        assert!(load_live_run_at(&path, "substituted-run", instant()).is_err());
    }

    #[test]
    fn repeated_lane_local_question_ids_have_distinct_navigation_ids() {
        let first = question_navigation_id("lane-a", "question:shared");
        let second = question_navigation_id("lane-b", "question:shared");
        assert_ne!(first, second);
        assert_eq!(first, question_navigation_id("lane-a", "question:shared"));
        assert_eq!(first.len(), 64);
    }

    #[test]
    #[ignore = "set NIGHTSHIFT_LEDGER_FIXTURE_DIR for the installed-browser qualification journey"]
    fn emit_installed_browser_fixture() {
        let output = std::env::var_os("NIGHTSHIFT_LEDGER_FIXTURE_DIR")
            .map(std::path::PathBuf::from)
            .expect("NIGHTSHIFT_LEDGER_FIXTURE_DIR must name an explicit temporary directory");
        assert!(output.is_dir());
        let (source, database, run_id, final_bytes) = closed_fixture();
        for entry in fs::read_dir(source.path()).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                fs::copy(entry.path(), output.join(entry.file_name())).unwrap();
            }
        }
        let case = output.join("sealed-case");
        fs::create_dir(&case).unwrap();
        fs::write(case.join("packet.v1.json"), PACKET).unwrap();
        fs::write(case.join("run-receipts.v1.json"), final_bytes).unwrap();
        assert!(output.join(database.file_name().unwrap()).is_file());
        assert_eq!(run_id, "ledger/live:fixture");
    }
}
