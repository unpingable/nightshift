use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};

use chrono::{Duration, TimeZone as _, Utc};
use nightshift_foreman::{
    read_only_run_snapshot, verify_adapter_contract, AdapterEventKindV1, AdapterEventV1,
    AdapterRegistrationV2, CapacityAdmissionEvidenceV1, CapacityCostClassV1, ContractError,
    ExecutionProfileV2, ForemanAdmissionV1, ForemanCapacityAdmissionV1,
    ForemanCapacityRequirementV1, ForemanError, ForemanStore, HumanQuestionV1, NotStartedReceiptV1,
    ReceiptRepositoryV1, SchedulerStateV1, TeardownDeclarationV1, TerminalReceiptV1,
    WorkItemExecutionV1, WorkerAdapterCapabilitiesV1, WorkerBriefV2, WorkerStartRequestV2,
    FOREMAN_ADMISSION_SCHEMA_V1, FOREMAN_CAPACITY_ADMISSION_SCHEMA_V1,
    FOREMAN_CAPACITY_REQUIREMENT_SCHEMA_V1, FOREMAN_EXECUTION_PROFILE_SCHEMA_V2,
    MAXIMUM_CAPACITY_HISTORY_BYTES, MAXIMUM_WORKER_BRIEF_BYTES,
    WORKER_ADAPTER_CAPABILITIES_SCHEMA_V1, WORKER_ADAPTER_EVENT_SCHEMA_V1,
    WORKER_BRIEF_BASIS_SCHEMA_V2, WORKER_START_REQUEST_SCHEMA_V2,
    WORKER_TERMINAL_RECEIPT_SCHEMA_V1, WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1,
};
use nightshift_provider_capacity::{
    decide_capacity, AdmissionDisposition as CapacityAdmissionDisposition, CapacityDecisionV1,
    CapacityObservationV1, CapacityPolicyV1, CapacityState, CapacityWindow, Confidence,
    ObservationDisposition, ObservationEvidence, SourceClass, WindowType,
    CAPACITY_OBSERVATION_SCHEMA_V1,
};
use nightshiftd::packet::{
    AuthoringIdentityV1, CampaignIdentityV1, CanonicalizationV1, ExactWorkRefV1,
    GlobalConstraintsV1, ModelRoutingV1, NightshiftPacketV1, RepositoryCustodyV1,
    SourceEvidenceRefV1, SwitchyardRegistrationV1, WorkItemV1, WorkerBudgetV1,
    EXACT_WORK_PROPOSAL_SCHEMA_V1, NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1,
    NIGHTSHIFT_PACKET_SCHEMA_V1,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use tempfile::TempDir;

fn bind_predecessor_fixture(
    mut brief: Value,
    dependency: &str,
    receipt_raw: &[u8],
    request: &WorkerStartRequestV2,
) -> (Vec<u8>, WorkerStartRequestV2) {
    let mut retained = Sha256::new();
    retained.update(b"nightshift.foreman-retained-raw.digest/v1\0");
    retained.update(receipt_raw);
    brief["predecessor_receipts"][dependency]["retained_raw_digest"] =
        Value::String(format!("sha256:{:x}", retained.finalize()));
    brief["predecessor_receipts"][dependency]["bytes_hex"] =
        Value::String(hex::encode(receipt_raw));
    let raw = serde_jcs::to_vec(&brief).unwrap();
    let mut rebound = request.clone();
    let mut digest = Sha256::new();
    digest.update(b"nightshift.worker-brief.digest/v2\0");
    digest.update(&raw);
    rebound.worker_brief_digest = format!("sha256:{:x}", digest.finalize());
    rebound.seal().unwrap();
    (raw, rebound)
}

fn instant(second: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 29, 16, 0, second).unwrap()
}

fn work_item(id: &str, codename: &str, dependencies: Vec<&str>) -> WorkItemV1 {
    WorkItemV1 {
        id: id.into(),
        track: "fixture".into(),
        campaign: CampaignIdentityV1 {
            codename: codename.into(),
            canonical_slug: format!("fixture-{id}"),
        },
        predecessor_lineage: vec![],
        dependencies: dependencies.into_iter().map(str::to_owned).collect(),
        exact_work_refs: vec![ExactWorkRefV1 {
            contract_kind: "exact_work_proposal_v1".into(),
            contract_schema: EXACT_WORK_PROPOSAL_SCHEMA_V1.into(),
            repository: "fixture".into(),
            branch: "campaign/fixture".into(),
            commit: "a".repeat(40),
            path: "proposal.json".into(),
            proposal_ref: format!("sha256:{}", "b".repeat(64)),
        }],
        entry_predicates: vec!["worker inspects exact predecessor evidence".into()],
        allowed_mutation_surfaces: vec!["fixture".into()],
        forbidden_actions: vec!["target actuation".into()],
        acceptance_tests: vec!["exact receipt".into()],
        stop_conditions: vec!["entry predicate fails".into()],
        expected_receipts: vec!["terminal or not-started".into()],
        closeout_requirements: vec!["clean local fixture".into()],
        model_routing: ModelRoutingV1 {
            class: "bounded".into(),
            reason: "deterministic test".into(),
            maximum_mutating_workers: 1,
        },
    }
}

fn packet() -> NightshiftPacketV1 {
    let mut packet = NightshiftPacketV1 {
        schema: NIGHTSHIFT_PACKET_SCHEMA_V1.into(),
        packet_id: "foreman-fixture".into(),
        packet_digest: String::new(),
        created_at: instant(0),
        current_until: instant(0) + Duration::hours(2),
        authoring: AuthoringIdentityV1 {
            agent: "fixture".into(),
            session: "fixture-session".into(),
            authority_basis: "local deterministic qualification".into(),
        },
        canonicalization: CanonicalizationV1 {
            algorithm: "RFC8785-JCS".into(),
            digest_algorithm: "SHA-256".into(),
            digest_preimage: NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1.into(),
        },
        source_evidence: vec![SourceEvidenceRefV1 {
            repository: "nightshift".into(),
            branch: "campaign/fixture".into(),
            commit: "a".repeat(40),
            path: "fixture".into(),
            file_digest: format!("sha256:{}", "c".repeat(64)),
            predecessor_classification: "EXACT-FIXTURE".into(),
        }],
        repository_custody: vec![RepositoryCustodyV1 {
            repository: "nightshift".into(),
            path: "/tmp/nightshift-fixture".into(),
            branch: "campaign/fixture".into(),
            commit: "a".repeat(40),
            remote: None,
            remote_commit: None,
            worktree_clean: true,
            discrepancy: None,
        }],
        global_constraints: GlobalConstraintsV1 {
            allowed_actions: vec!["local scheduling".into()],
            forbidden_actions: vec!["target actuation".into()],
            invariants: vec!["scheduler state is not result state".into()],
        },
        work_items: vec![
            work_item("root-a", "ROOT-A", vec![]),
            work_item("root-b", "ROOT-B", vec![]),
            work_item("root-c", "ROOT-C", vec![]),
            work_item("dependent", "DEPENDENT-D", vec!["root-a"]),
        ],
        worker_budget: WorkerBudgetV1 {
            maximum_concurrent_mutating_workers: 2,
            recursive_worker_swarms_forbidden: true,
            reserve_posture: "bounded fixture".into(),
        },
        human_question_criteria: vec!["exact operator authority missing".into()],
        switchyard: SwitchyardRegistrationV1 {
            alias: "fixture".into(),
            plan_ref: String::new(),
            transport_fields: vec!["alias".into(), "plan_ref".into(), "nonce".into()],
        },
    };
    packet.seal().unwrap();
    packet
}

fn admission(packet: &NightshiftPacketV1) -> ForemanAdmissionV1 {
    let mut admission = ForemanAdmissionV1 {
        schema: FOREMAN_ADMISSION_SCHEMA_V1.into(),
        admission_digest: format!("sha256:{}", "0".repeat(64)),
        run_id: "run-fixture".into(),
        packet_digest: packet.packet_digest.clone(),
        operator_basis_digest: format!("sha256:{}", "d".repeat(64)),
        admitted_at: instant(0),
        expires_at: instant(0) + Duration::hours(1),
        local_runtime_identity: "runtime-fixture".into(),
        maximum_concurrent_workers: 2,
        allowed_adapter_ids: vec!["fixture-adapter".into()],
        allowed_provider_model_classes: vec!["bounded".into()],
        maximum_new_attempts_per_work_item: 1,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".into(),
        target_effects_authorized: false,
    };
    admission.seal().unwrap();
    admission
}

fn profile(packet: &NightshiftPacketV1, admission: &ForemanAdmissionV1) -> ExecutionProfileV2 {
    let mut work_items = BTreeMap::new();
    for item in &packet.work_items {
        let locks = match item.id.as_str() {
            "root-a" | "root-c" => vec!["repository:shared".into()],
            "root-b" => vec!["repository:other".into()],
            _ => vec!["repository:dependent".into()],
        };
        work_items.insert(
            item.id.clone(),
            WorkItemExecutionV1 {
                adapter_id: "fixture-adapter".into(),
                workspace_identity: format!("workspace:{}", item.id),
                resource_lock_keys: locks,
                provider_model_class: "bounded".into(),
            },
        );
    }
    let mut profile = ExecutionProfileV2 {
        schema: FOREMAN_EXECUTION_PROFILE_SCHEMA_V2.into(),
        profile_digest: format!("sha256:{}", "0".repeat(64)),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        adapters: BTreeMap::from([(
            "fixture-adapter".into(),
            AdapterRegistrationV2 {
                adapter_id: "fixture-adapter".into(),
                protocol: "fixture.adapter/v1".into(),
                adapter_version: "fixture.adapter/v1".into(),
                executable_identity: format!("sha256:{}", "e".repeat(64)),
                bounded_arguments: vec![],
            },
        )]),
        work_items,
        budget_policy_ref: "budget:fixture".into(),
        log_custody_root: "/tmp/foreman-fixture/log".into(),
        receipt_custody_root: "/tmp/foreman-fixture/receipts".into(),
        maximum_event_bytes: 65_536,
        maximum_receipt_bytes: 131_072,
        adapter_timeout_seconds: 60,
        closeout_policy: "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED".into(),
    };
    profile.seal().unwrap();
    profile
}

fn setup() -> (
    TempDir,
    ForemanStore,
    NightshiftPacketV1,
    ForemanAdmissionV1,
    ExecutionProfileV2,
) {
    let directory = tempfile::tempdir().unwrap();
    let packet = packet();
    let admission = admission(&packet);
    let profile = profile(&packet, &admission);
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    store
        .admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0),
        )
        .unwrap();
    (directory, store, packet, admission, profile)
}

fn terminal(
    packet: &NightshiftPacketV1,
    request: &nightshift_foreman::WorkerStartRequestV2,
    state: &str,
    classification: &str,
) -> TerminalReceiptV1 {
    let mut receipt = TerminalReceiptV1 {
        schema: WORKER_TERMINAL_RECEIPT_SCHEMA_V1.into(),
        receipt_digest: format!("sha256:{}", "0".repeat(64)),
        packet_digest: packet.packet_digest.clone(),
        run_id: request.run_id.clone(),
        work_item_id: request.work_item_id.clone(),
        attempt_id: request.attempt_id.clone(),
        adapter_id: "fixture-adapter".into(),
        adapter_version: "fixture.adapter/v1".into(),
        provider_identity: "provider:fixture".into(),
        model_identity: "model:fixture".into(),
        session_identity: Some("session:fixture".into()),
        thread_identity: None,
        turn_identity: None,
        queue_identity: None,
        started_at: instant(1),
        ended_at: instant(5),
        state: state.into(),
        result_classification: classification.into(),
        repositories: vec![ReceiptRepositoryV1 {
            repository: "fixture".into(),
            branch: "campaign/fixture".into(),
            head: "f".repeat(40),
            push_status: "sole-local fixture".into(),
        }],
        tests: vec!["fixture passed".into()],
        evidence: vec!["fixture evidence".into()],
        live_or_production_mutations: vec![],
        remaining_trigger: "none".into(),
        next_lawful_action: "inspect exact receipt".into(),
        human_questions: vec![],
        teardown: TeardownDeclarationV1 {
            live_runtime: "none".into(),
            secrets: "none".into(),
            teardown: "none".into(),
        },
        extensions: BTreeMap::from([(
            "future_field".into(),
            Value::String("retained raw, no semantics".into()),
        )]),
    };
    receipt.seal().unwrap();
    receipt
}

fn not_started(packet: &NightshiftPacketV1, work_item: &str) -> NotStartedReceiptV1 {
    let mut receipt = NotStartedReceiptV1 {
        schema: WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1.into(),
        receipt_digest: format!("sha256:{}", "0".repeat(64)),
        packet_digest: packet.packet_digest.clone(),
        run_id: "run-fixture".into(),
        work_item_id: work_item.into(),
        recorded_at: instant(6),
        state: "ENTRY-BLOCKED-EXACT".into(),
        result_classification: "UNKNOWN-CUSTOM-CLASSIFICATION".into(),
        evidence: vec!["exact entry evidence".into()],
        remaining_trigger: "new explicit successor evidence".into(),
        next_lawful_action: "create a successor occurrence".into(),
        human_questions: vec![],
        extensions: BTreeMap::new(),
    };
    receipt.seal().unwrap();
    receipt
}

#[test]
fn admission_is_closed_bound_and_current() {
    let packet = packet();
    let admission = admission(&packet);
    let profile = profile(&packet, &admission);
    let directory = tempfile::tempdir().unwrap();
    let store = ForemanStore::open(directory.path().join("db")).unwrap();

    let mut substituted = admission.clone();
    substituted.packet_digest = format!("sha256:{}", "1".repeat(64));
    substituted.seal().unwrap();
    assert!(matches!(
        store.admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&substituted).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0)
        ),
        Err(ForemanError::IdentityMismatch("packet_digest"))
            | Err(ForemanError::IdentityMismatch("admission_digest"))
    ));
    assert!(store
        .admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0) + Duration::hours(3)
        )
        .is_err());
    store
        .admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0),
        )
        .unwrap();
    assert!(matches!(
        store.admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0),
        ),
        Err(ForemanError::DuplicateRun(_))
    ));

    let mut unknown = serde_json::to_value(&admission).unwrap();
    unknown["authority"] = Value::Bool(true);
    assert!(ForemanAdmissionV1::from_slice(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut unversioned_adapter = serde_json::to_value(&profile).unwrap();
    unversioned_adapter["adapters"]["fixture-adapter"]
        .as_object_mut()
        .unwrap()
        .remove("adapter_version");
    assert!(
        ExecutionProfileV2::from_slice(&serde_json::to_vec(&unversioned_adapter).unwrap()).is_err()
    );

    let capabilities = WorkerAdapterCapabilitiesV1 {
        schema: WORKER_ADAPTER_CAPABILITIES_SCHEMA_V1.into(),
        adapter_id: "fixture-adapter".into(),
        adapter_protocol: "fixture.adapter/v1".into(),
        adapter_version: "fixture.adapter/v1".into(),
        adapter_executable_identity: format!("sha256:{}", "e".repeat(64)),
        provider_kind: "fixture-provider".into(),
        commands: ["capabilities", "start", "resume", "status", "collect"]
            .map(str::to_owned)
            .to_vec(),
        approval_policy: "SURFACE_ONLY_NO_RESPONSE".into(),
        expected_start_request_schema: WORKER_START_REQUEST_SCHEMA_V2.into(),
        event_schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
        terminal_receipt_schema: WORKER_TERMINAL_RECEIPT_SCHEMA_V1.into(),
        target_effects_authorized: false,
    };
    capabilities.validate().unwrap();
    let mut widened = capabilities;
    widened.commands.push("approve".into());
    assert!(widened.validate().is_err());
}

#[test]
fn wal_journal_locks_restart_and_classification_separation_qualify() {
    let (directory, store, packet, _, profile) = setup();
    assert_eq!(store.journal_mode().unwrap().to_ascii_lowercase(), "wal");
    let initial = store.projection("run-fixture").unwrap();
    let root_a_initial = initial
        .work_items
        .iter()
        .find(|item| item.work_item_id == "root-a")
        .unwrap();
    let dependent_initial = initial
        .work_items
        .iter()
        .find(|item| item.work_item_id == "dependent")
        .unwrap();
    assert_eq!(
        root_a_initial.scheduler_state,
        SchedulerStateV1::ReadyEntryEvaluation
    );
    assert_eq!(
        dependent_initial.scheduler_state,
        SchedulerStateV1::WaitingDependencies
    );
    assert!(initial.work_items.iter().all(|item| {
        item.accepted_terminal_outcome.is_none()
            && item.result_absent_until_terminal_receipt_acceptance
    }));
    let initial_json = serde_json::to_value(&initial).unwrap();
    assert!(initial_json.get("aggregate_result").is_none());

    let root_a = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    assert_eq!(root_a.schema, WORKER_START_REQUEST_SCHEMA_V2);
    assert_eq!(root_a.adapter_id, "fixture-adapter");
    assert_eq!(root_a.adapter_version, "fixture.adapter/v1");
    let mut excessive_timeout = root_a.clone();
    excessive_timeout.timeout_seconds = 86_401;
    assert!(matches!(
        excessive_timeout.seal(),
        Err(ContractError::InvalidField("worker start boundary"))
    ));
    let mut excessive_output = root_a.clone();
    excessive_output.maximum_output_bytes = 16 * 1024 * 1024 + 1;
    assert!(matches!(
        excessive_output.seal(),
        Err(ContractError::InvalidField("worker start boundary"))
    ));
    let registration = &profile.adapters["fixture-adapter"];
    let capabilities = WorkerAdapterCapabilitiesV1 {
        schema: WORKER_ADAPTER_CAPABILITIES_SCHEMA_V1.into(),
        adapter_id: registration.adapter_id.clone(),
        adapter_protocol: registration.protocol.clone(),
        adapter_version: registration.adapter_version.clone(),
        adapter_executable_identity: registration.executable_identity.clone(),
        provider_kind: "fixture-provider".into(),
        commands: ["capabilities", "start", "resume", "status", "collect"]
            .map(str::to_owned)
            .to_vec(),
        approval_policy: "SURFACE_ONLY_NO_RESPONSE".into(),
        expected_start_request_schema: WORKER_START_REQUEST_SCHEMA_V2.into(),
        event_schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
        terminal_receipt_schema: WORKER_TERMINAL_RECEIPT_SCHEMA_V1.into(),
        target_effects_authorized: false,
    };
    let capabilities_raw = serde_jcs::to_vec(&capabilities).unwrap();
    let verified = verify_adapter_contract(&profile, "root-a", &capabilities_raw, &root_a).unwrap();
    assert_eq!(verified.adapter_version, "fixture.adapter/v1");
    assert!(verified.capabilities_raw_digest.starts_with("sha256:"));
    let mut capabilities_digest = Sha256::new();
    capabilities_digest.update(b"nightshift.worker-adapter-capabilities.raw/v1\0");
    capabilities_digest.update(&capabilities_raw);
    assert_eq!(
        verified.capabilities_raw_digest,
        format!("sha256:{:x}", capabilities_digest.finalize())
    );
    assert!(verify_adapter_contract(
        &profile,
        "root-a",
        &serde_json::to_vec_pretty(&capabilities).unwrap(),
        &root_a,
    )
    .is_err());
    let mut substituted_capabilities = capabilities.clone();
    substituted_capabilities.adapter_executable_identity = format!("sha256:{}", "f".repeat(64));
    assert!(verify_adapter_contract(
        &profile,
        "root-a",
        &serde_jcs::to_vec(&substituted_capabilities).unwrap(),
        &root_a,
    )
    .is_err());
    let mut substituted_start = root_a.clone();
    substituted_start.adapter_version = "fixture.adapter/v2".into();
    substituted_start.seal().unwrap();
    assert!(
        verify_adapter_contract(&profile, "root-a", &capabilities_raw, &substituted_start,)
            .is_err()
    );
    let binding = root_a.attempt_binding();
    binding.validate().unwrap();
    assert_eq!(binding.request_digest, root_a.request_digest);
    let brief = store.worker_brief("run-fixture", "root-a").unwrap();
    let brief_value: Value = serde_json::from_slice(&brief).unwrap();
    WorkerBriefV2::from_slice_for_start(&brief, &root_a).unwrap();
    assert_eq!(brief_value["schema"], WORKER_BRIEF_BASIS_SCHEMA_V2);
    assert_eq!(brief_value["packet_digest"], packet.packet_digest);
    assert_eq!(
        serde_json::from_str::<Value>(brief_value["work_item"]["canonical_json"].as_str().unwrap())
            .unwrap()["id"],
        "root-a"
    );
    assert_eq!(
        hex::decode(brief_value["packet_source"]["bytes_hex"].as_str().unwrap()).unwrap(),
        packet.canonical_bytes().unwrap()
    );
    let mut digest = Sha256::new();
    digest.update(b"nightshift.worker-brief.digest/v2\0");
    digest.update(&brief);
    assert_eq!(
        root_a.worker_brief_digest,
        format!("sha256:{:x}", digest.finalize())
    );
    assert!(matches!(
        store.prepare_attempt("run-fixture", "root-c", instant(1)),
        Err(ForemanError::ResourceUnavailable(_))
    ));
    let root_b = store
        .prepare_attempt("run-fixture", "root-b", instant(1))
        .unwrap();
    assert_ne!(root_a.attempt_id, root_b.attempt_id);

    drop(store);
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    let after_restart = store.projection("run-fixture").unwrap();
    assert_eq!(
        after_restart
            .work_items
            .iter()
            .find(|item| item.work_item_id == "root-a")
            .unwrap()
            .active_attempt_id
            .as_deref(),
        Some(root_a.attempt_id.as_str())
    );
    store
        .record_resume_requested("run-fixture", "root-a", &root_a.attempt_id, instant(2))
        .unwrap();

    let mut event = AdapterEventV1 {
        schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
        event_digest: format!("sha256:{}", "0".repeat(64)),
        event_id: "event-question".into(),
        packet_digest: packet.packet_digest.clone(),
        run_id: "run-fixture".into(),
        work_item_id: "root-a".into(),
        attempt_id: root_a.attempt_id.clone(),
        adapter_id: "fixture-adapter".into(),
        adapter_version: "fixture.adapter/v1".into(),
        occurred_at: instant(3),
        kind: AdapterEventKindV1::HumanQuestion,
        provider_identity: Some("provider:fixture".into()),
        model_identity: Some("model:fixture".into()),
        session_identity: Some("session:fixture".into()),
        thread_identity: None,
        turn_identity: None,
        queue_identity: None,
        message: None,
        human_question: Some(HumanQuestionV1 {
            question_id: "question-one".into(),
            question: "Which exact protected approval exists?".into(),
            exhausted_evidence: "No approval record is present.".into(),
            safe_default: "Do not perform the protected effect.".into(),
            consequences: "This lane remains waiting.".into(),
            resume_point: "Resume this exact attempt after evidence arrives.".into(),
        }),
        extensions: BTreeMap::from([(
            "future_observation".into(),
            Value::String("raw only".into()),
        )]),
    };
    event.seal().unwrap();
    let event_raw = serde_json::to_vec(&event).unwrap();
    store.accept_adapter_event(&event_raw).unwrap();
    assert!(matches!(
        store.accept_adapter_event(&event_raw),
        Err(ForemanError::DuplicateEvent(_))
    ));
    let projection = store.projection("run-fixture").unwrap();
    assert_eq!(
        projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == "root-a")
            .unwrap()
            .scheduler_state,
        SchedulerStateV1::WaitingHuman
    );
    assert_eq!(
        projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == "root-b")
            .unwrap()
            .scheduler_state,
        SchedulerStateV1::Dispatching
    );

    let receipt = terminal(
        &packet,
        &root_a,
        "CUSTOM-RAW-STATE",
        "NOT-A-SCHEDULER-SUCCESS-TOKEN",
    );
    store
        .accept_terminal_receipt(&serde_jcs::to_vec(&receipt).unwrap())
        .unwrap();
    let projection = store.projection("run-fixture").unwrap();
    let dependent = projection
        .work_items
        .iter()
        .find(|item| item.work_item_id == "dependent")
        .unwrap();
    assert_eq!(
        dependent.scheduler_state,
        SchedulerStateV1::ReadyEntryEvaluation
    );
    assert!(dependent.accepted_terminal_outcome.is_none());
    assert!(matches!(
        store.record_resume_requested("run-fixture", "root-a", &root_a.attempt_id, instant(6)),
        Err(ForemanError::Transition(_))
    ));
    assert!(matches!(
        store.prepare_attempt("run-fixture", "root-a", instant(6)),
        Err(ForemanError::Transition(_))
    ));

    let connection = Connection::open(directory.path().join("foreman.sqlite")).unwrap();
    assert!(connection
        .execute("UPDATE events SET kind = 'changed' WHERE sequence = 1", [])
        .is_err());
}

#[test]
fn exact_closeout_requires_complete_explicit_receipts_and_reproduces() {
    let (_directory, store, packet, _, _) = setup();
    let root_a = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let root_b = store
        .prepare_attempt("run-fixture", "root-b", instant(1))
        .unwrap();
    assert!(matches!(
        store.close("run-fixture", instant(6)),
        Err(ForemanError::IncompleteCloseout(_))
    ));
    for request in [&root_a, &root_b] {
        let receipt = terminal(&packet, request, "EXACT-STATE", "ARBITRARY-CLASSIFICATION");
        let raw = serde_jcs::to_vec(&receipt).unwrap();
        store.accept_terminal_receipt(&raw).unwrap();
        assert_eq!(
            store
                .raw_terminal_receipt("run-fixture", &request.work_item_id)
                .unwrap(),
            raw
        );
    }
    for work_item in ["root-c", "dependent"] {
        let receipt = not_started(&packet, work_item);
        store
            .accept_not_started(&serde_jcs::to_vec(&receipt).unwrap())
            .unwrap();
    }
    assert!(matches!(
        store.close("run-fixture", instant(5)),
        Err(ForemanError::Transition(_))
    ));
    let first = store.close("run-fixture", instant(6)).unwrap();
    let second = store.close("run-fixture", instant(9)).unwrap();
    assert_eq!(first, second);
    assert_eq!(first, store.export_final("run-fixture").unwrap());
    let value: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(value["schema"], "nightshift.run-receipts/v1");
    assert_eq!(value["packet_digest"], packet.packet_digest);
    assert_eq!(value["work_items"].as_array().unwrap().len(), 4);
    assert!(value.get("aggregate_result").is_none());

    let case_directory = tempfile::tempdir().unwrap();
    std::fs::write(
        case_directory.path().join("packet.v1.json"),
        packet.canonical_bytes().unwrap(),
    )
    .unwrap();
    std::fs::write(case_directory.path().join("run-receipts.v1.json"), &first).unwrap();
    let loaded = nightshift_casework::load_run_at(case_directory.path(), instant(9)).unwrap();
    assert_eq!(loaded.receipt_bytes, first);
    assert_eq!(loaded.projection.work_items.len(), 4);
}

#[test]
fn provider_completion_wrong_identity_and_receipt_bounds_fail_closed() {
    let (directory, store, packet, _, _) = setup();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    store
        .record_dispatch_requested("run-fixture", "root-a", &request.attempt_id, instant(1))
        .unwrap();

    let mut wrong = AdapterEventV1 {
        schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
        event_digest: format!("sha256:{}", "0".repeat(64)),
        event_id: "wrong-attempt-event".into(),
        packet_digest: packet.packet_digest.clone(),
        run_id: "run-fixture".into(),
        work_item_id: "root-a".into(),
        attempt_id: "attempt-substitution".into(),
        adapter_id: "fixture-adapter".into(),
        adapter_version: "fixture.adapter/v1".into(),
        occurred_at: instant(2),
        kind: AdapterEventKindV1::WorkerStarted,
        provider_identity: None,
        model_identity: None,
        session_identity: None,
        thread_identity: None,
        turn_identity: None,
        queue_identity: None,
        message: None,
        human_question: None,
        extensions: BTreeMap::new(),
    };
    wrong.seal().unwrap();
    assert!(matches!(
        store.accept_adapter_event(&serde_jcs::to_vec(&wrong).unwrap()),
        Err(ForemanError::IdentityMismatch("attempt_id"))
    ));

    let mut completed = wrong;
    completed.event_id = "provider-completed".into();
    completed.attempt_id = request.attempt_id.clone();
    completed.kind = AdapterEventKindV1::ProviderCompletionObservation;
    completed.provider_identity = Some("provider:fixture".into());
    completed.model_identity = Some("model:fixture".into());
    completed.session_identity = Some("session:fixture".into());
    completed.seal().unwrap();
    store
        .accept_adapter_event(&serde_jcs::to_vec(&completed).unwrap())
        .unwrap();

    let mut wrong_version = completed.clone();
    wrong_version.event_id = "wrong-adapter-version".into();
    wrong_version.adapter_version = "fixture.adapter/v2".into();
    wrong_version.seal().unwrap();
    assert!(matches!(
        store.accept_adapter_event(&serde_jcs::to_vec(&wrong_version).unwrap()),
        Err(ForemanError::IdentityMismatch("adapter_version"))
    ));

    let mut checkpoint = completed.clone();
    checkpoint.event_id = "incremental-provider-custody".into();
    checkpoint.kind = AdapterEventKindV1::Checkpoint;
    checkpoint.thread_identity = Some("thread:fixture".into());
    checkpoint.turn_identity = Some("turn:fixture".into());
    checkpoint.queue_identity = Some("queue:fixture".into());
    checkpoint.seal().unwrap();
    store
        .accept_adapter_event(&serde_jcs::to_vec(&checkpoint).unwrap())
        .unwrap();

    for field in [
        "provider_identity",
        "model_identity",
        "session_identity",
        "thread_identity",
        "turn_identity",
        "queue_identity",
    ] {
        let mut contradiction = checkpoint.clone();
        contradiction.event_id = format!("contradiction-{field}");
        match field {
            "provider_identity" => contradiction.provider_identity = Some("provider:other".into()),
            "model_identity" => contradiction.model_identity = Some("model:other".into()),
            "session_identity" => contradiction.session_identity = Some("session:other".into()),
            "thread_identity" => contradiction.thread_identity = Some("thread:other".into()),
            "turn_identity" => contradiction.turn_identity = Some("turn:other".into()),
            "queue_identity" => contradiction.queue_identity = Some("queue:other".into()),
            _ => unreachable!(),
        }
        contradiction.seal().unwrap();
        assert!(matches!(
            store.accept_adapter_event(&serde_jcs::to_vec(&contradiction).unwrap()),
            Err(ForemanError::IdentityMismatch(observed)) if observed == field
        ));
    }

    drop(store);
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    let item = store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .into_iter()
        .find(|item| item.work_item_id == "root-a")
        .unwrap();
    assert_eq!(item.scheduler_state, SchedulerStateV1::Checkpointed);
    assert_eq!(item.adapter_version, "fixture.adapter/v1");
    assert_eq!(item.provider_identity.as_deref(), Some("provider:fixture"));
    assert_eq!(item.model_identity.as_deref(), Some("model:fixture"));
    assert_eq!(item.session_identity.as_deref(), Some("session:fixture"));
    assert_eq!(item.thread_identity.as_deref(), Some("thread:fixture"));
    assert_eq!(item.turn_identity.as_deref(), Some("turn:fixture"));
    assert_eq!(item.queue_identity.as_deref(), Some("queue:fixture"));
    assert!(item.accepted_terminal_outcome.is_none());

    assert!(store.accept_terminal_receipt(b"{}").is_err());
    for number in [
        serde_json::json!(1e-7),
        serde_json::json!(1e-6),
        serde_json::json!(1.25),
        serde_json::json!(-0.0),
        serde_json::json!(9_007_199_254_740_991_i64),
        serde_json::json!(-9_007_199_254_740_991_i64),
    ] {
        let mut admitted = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
        admitted.extensions.insert("number".into(), number);
        admitted.seal().unwrap();
        let canonical = serde_jcs::to_vec(&admitted).unwrap();
        let parsed = TerminalReceiptV1::from_slice(&canonical).unwrap();
        parsed.validate().unwrap();
        assert_eq!(serde_jcs::to_vec(&parsed).unwrap(), canonical);
    }

    let mut receipt = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    receipt.thread_identity = Some("thread:fixture".into());
    receipt.turn_identity = Some("turn:fixture".into());
    receipt.queue_identity = Some("queue:fixture".into());
    receipt.seal().unwrap();
    let mut wrong_attempt_receipt = receipt.clone();
    wrong_attempt_receipt.attempt_id = "attempt-substitution".into();
    wrong_attempt_receipt.seal().unwrap();
    assert!(matches!(
        store.accept_terminal_receipt(&serde_jcs::to_vec(&wrong_attempt_receipt).unwrap()),
        Err(ForemanError::IdentityMismatch("attempt_id"))
    ));
    for field in [
        "adapter_version",
        "provider_identity",
        "model_identity",
        "session_identity",
        "thread_identity",
        "turn_identity",
        "queue_identity",
    ] {
        let mut substitution = receipt.clone();
        match field {
            "adapter_version" => substitution.adapter_version = "fixture.adapter/v2".into(),
            "provider_identity" => substitution.provider_identity = "provider:other".into(),
            "model_identity" => substitution.model_identity = "model:other".into(),
            "session_identity" => substitution.session_identity = Some("session:other".into()),
            "thread_identity" => substitution.thread_identity = Some("thread:other".into()),
            "turn_identity" => substitution.turn_identity = Some("turn:other".into()),
            "queue_identity" => substitution.queue_identity = Some("queue:other".into()),
            _ => unreachable!(),
        }
        substitution.seal().unwrap();
        assert!(matches!(
            store.accept_terminal_receipt(&serde_jcs::to_vec(&substitution).unwrap()),
            Err(ForemanError::IdentityMismatch(observed)) if observed == field
        ));
    }
    let mut missing = serde_json::to_value(&receipt).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("next_lawful_action");
    assert!(store
        .accept_terminal_receipt(&serde_json::to_vec(&missing).unwrap())
        .is_err());
    let mut oversized = serde_jcs::to_vec(&receipt).unwrap();
    oversized.resize(131_073, b' ');
    assert!(matches!(
        store.accept_terminal_receipt(&oversized),
        Err(ForemanError::InputTooLarge("terminal receipt"))
    ));
    store
        .record_terminal_refusal(
            "run-fixture",
            "root-a",
            &request.attempt_id,
            "deterministic malformed receipt fixture",
            instant(4),
        )
        .unwrap();
    assert_eq!(
        store
            .projection("run-fixture")
            .unwrap()
            .work_items
            .into_iter()
            .find(|item| item.work_item_id == "root-a")
            .unwrap()
            .scheduler_state,
        SchedulerStateV1::TerminalReceiptRefused
    );
    store
        .accept_terminal_receipt(&serde_jcs::to_vec(&receipt).unwrap())
        .unwrap();
    assert!(matches!(
        store.record_resume_requested("run-fixture", "root-a", &request.attempt_id, instant(6)),
        Err(ForemanError::Transition(_))
    ));
}

#[test]
fn worker_brief_v2_digest_has_an_independent_fixed_vector() {
    let mut v2 = Sha256::new();
    v2.update(b"nightshift.worker-brief.digest/v2\0");
    v2.update(b"{}");
    assert_eq!(
        format!("sha256:{:x}", v2.finalize()),
        "sha256:ddd2a21b47c3abf533d27d85a53eb3ac93805d5d938f612929ea410a6ec705e7"
    );
    let mut retained = Sha256::new();
    retained.update(b"nightshift.foreman-retained-raw.digest/v1\0");
    retained.update(b"{}");
    assert_eq!(
        format!("sha256:{:x}", retained.finalize()),
        "sha256:defbb1499ef874d99cdf029e5c1dc04dc253d0fc1e0f88f966278cf3934302fe"
    );
    let mut capabilities = Sha256::new();
    capabilities.update(b"nightshift.worker-adapter-capabilities.raw/v1\0");
    capabilities.update(b"{}");
    assert_eq!(
        format!("sha256:{:x}", capabilities.finalize()),
        "sha256:4dbc0996b158b29f3e54274c8fd1ccb774422f75fb38b3fd1a1aae0662ff5c4c"
    );
    let mut v1 = Sha256::new();
    v1.update(b"nightshift.worker-brief.digest/v1\0");
    v1.update(b"{}");
    assert_ne!(
        format!("sha256:{:x}", v1.finalize()),
        "sha256:ddd2a21b47c3abf533d27d85a53eb3ac93805d5d938f612929ea410a6ec705e7"
    );
}

#[test]
fn rfc8785_cross_language_vector_covers_numeric_unicode_and_escape_edges() {
    let value = serde_json::json!({
        "numbers": [
            1e-7, 1e-6, 1e20, 1e21, -0.0,
            9_007_199_254_740_991_i64, -9_007_199_254_740_991_i64
        ],
        "unicode": {"€": "euro", "\r": "cr", "דּ": "hebrew", "😀": "grin", "\u{0080}": "control"},
        "escapes": "\u{0008}\t\n\u{000c}\r\"\\\0",
    });
    let canonical = serde_json_canonicalizer::to_vec(&value).unwrap();
    let expected = "{\"escapes\":\"\\b\\t\\n\\f\\r\\\"\\\\\\u0000\",\"numbers\":[1e-7,0.000001,100000000000000000000,1e+21,0,9007199254740991,-9007199254740991],\"unicode\":{\"\\r\":\"cr\",\"\u{0080}\":\"control\",\"€\":\"euro\",\"😀\":\"grin\",\"דּ\":\"hebrew\"}}";
    assert_eq!(canonical, expected.as_bytes());
    assert_eq!(
        format!("{:x}", Sha256::digest(&canonical)),
        "3e01e561f7ea8f1c5774a2d6f5608067675a43316cb41c9d91cdcd2440b4d90f"
    );

    let admitted = serde_json::json!({
        "numbers": [
            1e-7, 1e-6, 1.25, -0.0,
            9_007_199_254_740_991_i64, -9_007_199_254_740_991_i64
        ],
        "unicode_values": ["€", "\u{0080}", "😀", "דּ"],
        "escapes": "\u{0008}\t\n\u{000c}\r\"\\\0",
    });
    assert_eq!(
        serde_jcs::to_vec(&admitted).unwrap(),
        serde_json_canonicalizer::to_vec(&admitted).unwrap()
    );
}

#[test]
fn worker_receipts_enforce_a_serialize_parse_closed_numeric_domain() {
    let (_directory, store, packet, _, _) = setup();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let mut receipt = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    receipt.extensions.insert(
        "nested".into(),
        serde_json::json!({"unsafe_integer": 9_007_199_254_740_992_u64}),
    );
    assert!(matches!(
        receipt.seal(),
        Err(ContractError::InvalidField("RFC8785 number"))
    ));

    let mut integral_float = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    integral_float.extensions.insert(
        "nested".into(),
        serde_json::json!({"unsafe_integral_float": 1e20}),
    );
    assert!(matches!(
        integral_float.seal(),
        Err(ContractError::InvalidField("RFC8785 number"))
    ));

    let mut top_level_unicode_key =
        terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    top_level_unicode_key.extensions.insert(
        "😀".into(),
        serde_json::json!("not in the admitted object-key alphabet"),
    );
    assert!(matches!(
        top_level_unicode_key.seal(),
        Err(ContractError::InvalidField("RFC8785 object key"))
    ));

    let mut nested_unicode_key = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    nested_unicode_key.extensions.insert(
        "nested".into(),
        serde_json::json!({"😀": "not in the admitted object-key alphabet"}),
    );
    assert!(matches!(
        nested_unicode_key.seal(),
        Err(ContractError::InvalidField("RFC8785 object key"))
    ));
}

#[test]
fn worker_brief_preserves_exact_packet_and_predecessor_receipt_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let packet = packet();
    let packet_raw = serde_json::to_vec_pretty(&packet).unwrap();
    let admission = admission(&packet);
    let profile = profile(&packet, &admission);
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    store
        .admit(
            &packet_raw,
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0),
        )
        .unwrap();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let mut receipt = terminal(&packet, &request, "EXACT-RAW", "INDEPENDENT-RAW");
    receipt.extensions.insert(
        "raw_extension".into(),
        serde_json::json!({"unknown": ["preserve", "exact", "bytes"]}),
    );
    receipt.seal().unwrap();
    let receipt_raw = serde_json::to_vec_pretty(&receipt).unwrap();
    store.accept_terminal_receipt(&receipt_raw).unwrap();
    assert_eq!(
        store
            .projection("run-fixture")
            .unwrap()
            .work_items
            .iter()
            .find(|item| item.work_item_id == "dependent")
            .unwrap()
            .scheduler_state,
        SchedulerStateV1::ReadyEntryEvaluation
    );

    let dependent_request = store
        .prepare_attempt("run-fixture", "dependent", instant(3))
        .unwrap();
    let brief_raw = store.worker_brief("run-fixture", "dependent").unwrap();
    assert!(brief_raw.len() <= MAXIMUM_WORKER_BRIEF_BYTES);
    let brief: Value = serde_json::from_slice(&brief_raw).unwrap();
    let mut bad_digest_receipt: Value = serde_json::from_slice(&receipt_raw).unwrap();
    bad_digest_receipt["receipt_digest"] = Value::String(format!("sha256:{}", "0".repeat(64)));
    let (bad_digest_brief, bad_digest_request) = bind_predecessor_fixture(
        brief.clone(),
        "root-a",
        &serde_jcs::to_vec(&bad_digest_receipt).unwrap(),
        &dependent_request,
    );
    assert!(WorkerBriefV2::from_slice_for_start(&bad_digest_brief, &bad_digest_request,).is_err());
    let mut wrong_run_receipt = receipt.clone();
    wrong_run_receipt.run_id = "run-substituted".into();
    wrong_run_receipt.seal().unwrap();
    let (wrong_run_brief, wrong_run_request) = bind_predecessor_fixture(
        brief.clone(),
        "root-a",
        &serde_jcs::to_vec(&wrong_run_receipt).unwrap(),
        &dependent_request,
    );
    assert!(WorkerBriefV2::from_slice_for_start(&wrong_run_brief, &wrong_run_request,).is_err());
    WorkerBriefV2::from_slice_for_start(&brief_raw, &dependent_request).unwrap();
    let mut substituted_brief = brief.clone();
    let mut substituted_packet: Value = serde_json::from_slice(&packet_raw).unwrap();
    substituted_packet["work_items"][1]["track"] = Value::String("substituted-track".into());
    let substituted_packet_raw = serde_json::to_vec_pretty(&substituted_packet).unwrap();
    let mut retained_digest = Sha256::new();
    retained_digest.update(b"nightshift.foreman-retained-raw.digest/v1\0");
    retained_digest.update(&substituted_packet_raw);
    substituted_brief["packet_source"]["retained_raw_digest"] =
        Value::String(format!("sha256:{:x}", retained_digest.finalize()));
    substituted_brief["packet_source"]["bytes_hex"] =
        Value::String(hex::encode(&substituted_packet_raw));
    let substituted_raw = serde_jcs::to_vec(&substituted_brief).unwrap();
    let mut substituted_request = dependent_request.clone();
    let mut brief_digest = Sha256::new();
    brief_digest.update(b"nightshift.worker-brief.digest/v2\0");
    brief_digest.update(&substituted_raw);
    substituted_request.worker_brief_digest = format!("sha256:{:x}", brief_digest.finalize());
    substituted_request.seal().unwrap();
    assert!(WorkerBriefV2::from_slice_for_start(&substituted_raw, &substituted_request,).is_err());
    assert_eq!(brief["schema"], WORKER_BRIEF_BASIS_SCHEMA_V2);
    let recognized_item: Value =
        serde_json::from_str(brief["work_item"]["canonical_json"].as_str().unwrap()).unwrap();
    assert_eq!(recognized_item["id"], "dependent");
    assert_eq!(
        recognized_item["dependencies"],
        serde_json::json!(["root-a"])
    );
    assert_eq!(
        hex::decode(brief["packet_source"]["bytes_hex"].as_str().unwrap()).unwrap(),
        packet_raw
    );
    let predecessor = &brief["predecessor_receipts"]["root-a"];
    assert_eq!(predecessor["receipt_kind"], "terminal");
    assert_eq!(
        hex::decode(predecessor["bytes_hex"].as_str().unwrap()).unwrap(),
        receipt_raw
    );
    assert!(String::from_utf8(receipt_raw)
        .unwrap()
        .contains("raw_extension"));
}

#[test]
fn worker_brief_total_bound_refuses_oversized_exact_predecessor_custody() {
    let directory = tempfile::tempdir().unwrap();
    let packet = packet();
    let admission = admission(&packet);
    let mut profile = profile(&packet, &admission);
    profile.maximum_receipt_bytes = 16 * 1024 * 1024;
    profile.seal().unwrap();
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    store
        .admit(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            instant(0),
        )
        .unwrap();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let mut receipt = terminal(&packet, &request, "EXACT-RAW", "INDEPENDENT-RAW");
    receipt.extensions.insert(
        "bounded_fixture".into(),
        Value::String("x".repeat(MAXIMUM_WORKER_BRIEF_BYTES / 2 + 4096)),
    );
    receipt.seal().unwrap();
    let raw = serde_jcs::to_vec(&receipt).unwrap();
    assert!(raw.len() < profile.maximum_receipt_bytes as usize);
    store.accept_terminal_receipt(&raw).unwrap();
    assert!(matches!(
        store.prepare_attempt("run-fixture", "dependent", instant(3)),
        Err(ForemanError::InputTooLarge("worker brief"))
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .find(|item| item.work_item_id == "dependent")
        .unwrap()
        .active_attempt_id
        .is_none());
    assert!(matches!(
        store.worker_brief("run-fixture", "dependent"),
        Err(ForemanError::InputTooLarge("worker brief"))
    ));
}

fn directory_bytes(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();
            (path.file_name().unwrap().into(), fs::read(path).unwrap())
        })
        .collect()
}

fn byte_digests(snapshot: &BTreeMap<PathBuf, Vec<u8>>) -> BTreeMap<PathBuf, String> {
    snapshot
        .iter()
        .map(|(path, bytes)| (path.clone(), format!("{:x}", Sha256::digest(bytes))))
        .collect()
}

fn retained_raw_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nightshift.foreman-retained-raw.digest/v1\0");
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn schema_snapshot(path: &Path) -> Vec<(String, Option<String>)> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let mut statement = connection
        .prepare(
            "SELECT name, sql FROM sqlite_schema \
             WHERE type IN ('table', 'index', 'trigger') ORDER BY type, name",
        )
        .unwrap();
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn query_only_store_refuses_absent_and_symlink_paths_without_creation() {
    let directory = tempfile::tempdir().unwrap();
    let absent = directory.path().join("absent.sqlite");
    assert!(matches!(
        ForemanStore::open_read_only(&absent),
        Err(ForemanError::ReadOnlyStore(_))
    ));
    assert!(directory_bytes(directory.path()).is_empty());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let database = directory.path().join("database.sqlite");
        drop(ForemanStore::open(&database).unwrap());
        let link = directory.path().join("database-link.sqlite");
        symlink(&database, &link).unwrap();
        let before = directory_bytes(directory.path());
        drop(ForemanStore::open_read_only(&database).unwrap());
        let after_direct = directory_bytes(directory.path());
        assert!(
            after_direct == before,
            "before={:?} after={:?}",
            byte_digests(&before),
            byte_digests(&after_direct)
        );
        assert!(matches!(
            ForemanStore::open_read_only(&link),
            Err(ForemanError::ReadOnlyStore(_))
        ));
        assert_eq!(directory_bytes(directory.path()), before);
    }
}

#[test]
fn query_only_store_refuses_partial_sidecar_custody_without_changes() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("database.sqlite");
    drop(ForemanStore::open(&database).unwrap());
    fs::write(directory.path().join("database.sqlite-wal"), b"").unwrap();
    let before = directory_bytes(directory.path());
    assert!(matches!(
        ForemanStore::open_read_only(&database),
        Err(ForemanError::ReadOnlyStore(_))
    ));
    assert_eq!(directory_bytes(directory.path()), before);
}

#[test]
fn query_only_store_keeps_original_inode_across_pathname_replacement() {
    let (directory, store, _, _, _) = setup();
    let database = directory.path().join("foreman.sqlite");
    drop(store);
    let reader = ForemanStore::open_read_only(&database).unwrap();

    let replacement = directory.path().join("replacement.sqlite");
    drop(ForemanStore::open(&replacement).unwrap());
    let admitted = directory.path().join("admitted.sqlite");
    fs::rename(&database, &admitted).unwrap();
    for suffix in ["-wal", "-shm"] {
        let old = directory.path().join(format!("foreman.sqlite{suffix}"));
        if old.exists() {
            fs::rename(
                &old,
                directory.path().join(format!("admitted.sqlite{suffix}")),
            )
            .unwrap();
        }
    }
    fs::rename(&replacement, &database).unwrap();
    for suffix in ["-wal", "-shm"] {
        let old = directory.path().join(format!("replacement.sqlite{suffix}"));
        if old.exists() {
            fs::rename(
                &old,
                directory.path().join(format!("foreman.sqlite{suffix}")),
            )
            .unwrap();
        }
    }

    assert_eq!(
        reader.projection("run-fixture").unwrap().packet_digest,
        packet().packet_digest
    );
    assert!(ForemanStore::open_read_only(&database)
        .and_then(|store| store.projection("run-fixture"))
        .is_err());
}

#[test]
fn query_only_projection_events_and_final_export_preserve_database_and_sidecar_bytes() {
    let (directory, store, packet, _, _) = setup();
    let database = directory.path().join("foreman.sqlite");

    let schema_before_live = schema_snapshot(&database);
    let files_before_live = directory_bytes(directory.path());
    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert_eq!(
        reader.projection("run-fixture").unwrap().work_items.len(),
        4
    );
    assert!(!reader
        .worker_brief("run-fixture", "root-a")
        .unwrap()
        .is_empty());
    assert!(!reader.export_events("run-fixture").unwrap().is_empty());
    drop(reader);
    assert_eq!(schema_snapshot(&database), schema_before_live);
    assert_eq!(directory_bytes(directory.path()), files_before_live);

    let root_a = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let root_b = store
        .prepare_attempt("run-fixture", "root-b", instant(1))
        .unwrap();
    for request in [&root_a, &root_b] {
        let receipt = terminal(&packet, request, "EXACT-STATE", "EXACT-CLASSIFICATION");
        store
            .accept_terminal_receipt(&serde_jcs::to_vec(&receipt).unwrap())
            .unwrap();
    }
    for work_item in ["root-c", "dependent"] {
        let receipt = not_started(&packet, work_item);
        store
            .accept_not_started(&serde_jcs::to_vec(&receipt).unwrap())
            .unwrap();
    }
    let expected = store.close("run-fixture", instant(6)).unwrap();

    let schema_before_final = schema_snapshot(&database);
    let files_before_final = directory_bytes(directory.path());
    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert_eq!(reader.export_final("run-fixture").unwrap(), expected);
    drop(reader);
    assert_eq!(schema_snapshot(&database), schema_before_final);
    assert_eq!(directory_bytes(directory.path()), files_before_final);

    drop(store);
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER final_snapshots_no_update;
             DROP TRIGGER events_no_update;",
        )
        .unwrap();
    let mut substituted: Value = serde_json::from_slice(&expected).unwrap();
    substituted["updated_at"] =
        Value::String(instant(4).to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let substituted = serde_jcs::to_vec(&substituted).unwrap();
    let final_digest = retained_raw_digest(&substituted);
    connection
        .execute(
            "UPDATE final_snapshots SET updated_at = ?1, raw_digest = ?2, raw_bytes = ?3
             WHERE run_id = 'run-fixture'",
            rusqlite::params![instant(4).to_rfc3339(), final_digest, substituted],
        )
        .unwrap();
    let run_closed_raw: Vec<u8> = connection
        .query_row(
            "SELECT raw_bytes FROM events WHERE run_id = 'run-fixture'
             ORDER BY sequence DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut run_closed: Value = serde_json::from_slice(&run_closed_raw).unwrap();
    run_closed["payload"]["final_receipts_digest"] = Value::String(final_digest);
    let run_closed = serde_jcs::to_vec(&run_closed).unwrap();
    connection
        .execute(
            "UPDATE events SET raw_bytes = ?1, raw_digest = ?2
             WHERE sequence = (SELECT MAX(sequence) FROM events)",
            rusqlite::params![run_closed, retained_raw_digest(&run_closed)],
        )
        .unwrap();
    drop(connection);
    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert!(matches!(
        reader.read_only_run_snapshot("run-fixture"),
        Err(ForemanError::ReadOnlyStore(_))
    ));
}

#[test]
fn query_only_snapshot_refuses_substituted_journal_custody() {
    let (directory, store, _, _, _) = setup();
    let database = directory.path().join("foreman.sqlite");
    drop(store);

    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER events_no_update;
             UPDATE events SET raw_digest = 'sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'
             WHERE sequence = (SELECT MIN(sequence) FROM events);",
        )
        .unwrap();
    drop(connection);

    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert!(matches!(
        reader.read_only_run_snapshot("run-fixture"),
        Err(ForemanError::ReadOnlyStore(_))
    ));
}

#[test]
fn query_only_snapshot_refuses_substituted_accepted_receipt_custody() {
    let (directory, store, packet, _, _) = setup();
    let database = directory.path().join("foreman.sqlite");
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let receipt = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    store
        .accept_terminal_receipt(&serde_jcs::to_vec(&receipt).unwrap())
        .unwrap();
    drop(store);

    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER terminal_receipts_no_update;
             UPDATE terminal_receipts SET receipt_kind = 'not_started'
             WHERE work_item_id = 'root-a';",
        )
        .unwrap();
    drop(connection);

    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert!(reader.read_only_run_snapshot("run-fixture").is_err());
}

#[test]
fn query_only_snapshot_refuses_substituted_run_contract_columns() {
    let (directory, store, _, _, _) = setup();
    let database = directory.path().join("foreman.sqlite");
    drop(store);

    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER runs_no_update;
             UPDATE runs SET maximum_concurrent_workers = 1
             WHERE run_id = 'run-fixture';",
        )
        .unwrap();
    drop(connection);

    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert!(matches!(
        reader.read_only_run_snapshot("run-fixture"),
        Err(ForemanError::ReadOnlyStore(_))
    ));
}

#[test]
fn query_only_snapshot_refuses_individually_valid_cross_contract_substitution() {
    let (directory, store, _, mut admission, mut profile) = setup();
    let database = directory.path().join("foreman.sqlite");
    admission.run_id = "substituted-run".to_owned();
    admission.seal().unwrap();
    profile.admission_digest = admission.admission_digest.clone();
    profile.seal().unwrap();
    let admission_bytes = serde_jcs::to_vec(&admission).unwrap();
    let profile_bytes = serde_jcs::to_vec(&profile).unwrap();
    drop(store);

    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch("DROP TRIGGER runs_no_update;")
        .unwrap();
    connection
        .execute(
            "UPDATE runs
             SET admission_digest = ?1, admission_bytes = ?2,
                 profile_digest = ?3, profile_bytes = ?4
             WHERE run_id = 'run-fixture'",
            rusqlite::params![
                admission.admission_digest,
                admission_bytes,
                profile.profile_digest,
                profile_bytes
            ],
        )
        .unwrap();
    drop(connection);

    let reader = ForemanStore::open_read_only(&database).unwrap();
    assert!(reader.read_only_run_snapshot("run-fixture").is_err());
}

#[test]
fn receipt_and_event_text_bounds_count_unicode_codepoints() {
    let (_directory, store, packet, _, _) = setup();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let mut receipt = terminal(&packet, &request, "é", "EXACT-CLASSIFICATION");
    receipt.state = "é".repeat(65_536);
    receipt.seal().unwrap();
    receipt.state.push('é');
    assert!(matches!(
        receipt.seal(),
        Err(ContractError::InvalidField("state"))
    ));

    let mut event = AdapterEventV1 {
        schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
        event_digest: format!("sha256:{}", "0".repeat(64)),
        event_id: "event-unicode-bound".into(),
        packet_digest: packet.packet_digest.clone(),
        run_id: request.run_id.clone(),
        work_item_id: request.work_item_id.clone(),
        attempt_id: request.attempt_id.clone(),
        adapter_id: request.adapter_id.clone(),
        adapter_version: request.adapter_version.clone(),
        occurred_at: instant(2),
        kind: AdapterEventKindV1::Checkpoint,
        provider_identity: None,
        model_identity: None,
        session_identity: None,
        thread_identity: None,
        turn_identity: None,
        queue_identity: None,
        message: Some("é".repeat(65_536)),
        human_question: None,
        extensions: BTreeMap::new(),
    };
    event.seal().unwrap();
    event.message.as_mut().unwrap().push('é');
    assert!(matches!(
        event.seal(),
        Err(ContractError::InvalidField("adapter event bounds"))
    ));
}

#[test]
fn receipt_timestamp_lexemes_are_canonical_utc_and_nanosecond_exact() {
    let (_directory, store, packet, _, _) = setup();
    let request = store
        .prepare_attempt("run-fixture", "root-a", instant(1))
        .unwrap();
    let mut receipt = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    receipt.started_at = chrono::DateTime::parse_from_rfc3339("2026-08-29T16:00:01.000000100Z")
        .unwrap()
        .with_timezone(&Utc);
    receipt.ended_at = chrono::DateTime::parse_from_rfc3339("2026-08-29T16:00:05.123456Z")
        .unwrap()
        .with_timezone(&Utc);
    receipt.seal().unwrap();
    let raw = serde_jcs::to_vec(&receipt).unwrap();
    assert_eq!(TerminalReceiptV1::from_slice(&raw).unwrap(), receipt);

    for substituted in [
        "2026-08-29T12:00:01-04:00",
        "2026-08-29T16:00:01.1000Z",
        "2026-08-29T16:00:01.123000Z",
        "2026-08-29T16:00:01.0000001Z",
    ] {
        let mut value = serde_json::to_value(&receipt).unwrap();
        value["started_at"] = Value::String(substituted.into());
        assert!(matches!(
            TerminalReceiptV1::from_slice(&serde_json::to_vec(&value).unwrap()),
            Err(ContractError::InvalidField("started_at"))
        ));
    }

    let mut absent = not_started(&packet, "root-b");
    absent.recorded_at = chrono::DateTime::parse_from_rfc3339("2026-08-29T16:00:06.123Z")
        .unwrap()
        .with_timezone(&Utc);
    absent.seal().unwrap();
    let raw = serde_jcs::to_vec(&absent).unwrap();
    assert_eq!(NotStartedReceiptV1::from_slice(&raw).unwrap(), absent);
    let mut value = serde_json::to_value(&absent).unwrap();
    value["recorded_at"] = Value::String("2026-08-29T16:00:06.123000Z".into());
    assert!(matches!(
        NotStartedReceiptV1::from_slice(&serde_json::to_vec(&value).unwrap()),
        Err(ContractError::InvalidField("recorded_at"))
    ));
}

#[test]
fn checked_in_contract_schemas_are_closed_json_documents() {
    for bytes in [
        include_bytes!("../../../schemas/nightshift.foreman-admission.v1.schema.json").as_slice(),
        include_bytes!("../../../schemas/nightshift.foreman-capacity-requirement.v1.schema.json")
            .as_slice(),
        include_bytes!("../../../schemas/nightshift.foreman-capacity-admission.v1.schema.json")
            .as_slice(),
        include_bytes!("../../../schemas/nightshift.foreman-execution-profile.v2.schema.json")
            .as_slice(),
        include_bytes!("../../../schemas/nightshift.worker-adapter.v2.schema.json").as_slice(),
    ] {
        let schema: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(schema.get("$id").is_some());
    }
}

fn capacity_observation(at: chrono::DateTime<Utc>, remaining: f64) -> CapacityObservationV1 {
    let mut observation = CapacityObservationV1 {
        schema: CAPACITY_OBSERVATION_SCHEMA_V1.into(),
        provider_id: "provider:fixture".into(),
        account_profile_locator: "fixture-profile".into(),
        model_family: None,
        observed_at: at - Duration::seconds(1),
        expires_at: at + Duration::minutes(10),
        source_class: SourceClass::Observed,
        confidence: Confidence::High,
        disposition: ObservationDisposition::Usable,
        unknown_reasons: vec![],
        windows: vec![
            CapacityWindow {
                window_id: "five-hour".into(),
                window_type: WindowType::FiveHour,
                remaining_fraction: Some(remaining),
                remaining_units: None,
                resets_at: Some(at + Duration::hours(1)),
            },
            CapacityWindow {
                window_id: "weekly".into(),
                window_type: WindowType::Weekly,
                remaining_fraction: Some(remaining),
                remaining_units: None,
                resets_at: Some(at + Duration::days(1)),
            },
        ],
        evidence: ObservationEvidence {
            probe_id: "gauge-latch-fixture".into(),
            protocol_method: "fixture/read".into(),
            protocol_version: Some("fixture/v1".into()),
            executable_path: Some("/fixture/provider-observer".into()),
            executable_digest: Some(format!("sha256:{}", "1".repeat(64))),
            raw_source_digest: format!("sha256:{}", "2".repeat(64)),
        },
        observation_digest: String::new(),
    };
    observation.observation_digest = observation.compute_digest().unwrap();
    observation
}

fn capacity_requirement(
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
    policy: &CapacityPolicyV1,
) -> ForemanCapacityRequirementV1 {
    let mut requirement = ForemanCapacityRequirementV1 {
        schema: FOREMAN_CAPACITY_REQUIREMENT_SCHEMA_V1.into(),
        capacity_requirement_digest: format!("sha256:{}", "0".repeat(64)),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        profile_digest: profile.profile_digest.clone(),
        run_id: admission.run_id.clone(),
        policy_id: policy.policy_id.clone(),
        provider_id: "provider:fixture".into(),
        model_cost_classes: packet
            .work_items
            .iter()
            .map(|work| {
                let cost = match work.model_routing.class.as_str() {
                    "small" | "bounded" => CapacityCostClassV1::Cheap,
                    "medium" | "large" => CapacityCostClassV1::Expensive,
                    _ => panic!("fixture model class must be closed"),
                };
                (work.model_routing.class.clone(), cost)
            })
            .collect(),
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".into(),
    };
    requirement.seal().unwrap();
    requirement
}

struct CapacityFixtureOwner<'a> {
    packet: &'a NightshiftPacketV1,
    admission: &'a ForemanAdmissionV1,
    profile: &'a ExecutionProfileV2,
    requirement: &'a ForemanCapacityRequirementV1,
    policy: &'a CapacityPolicyV1,
}

fn capacity_bundle(
    owner: CapacityFixtureOwner<'_>,
    work_item_id: &str,
    at: chrono::DateTime<Utc>,
    remaining: f64,
) -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, CapacityDecisionV1) {
    let CapacityFixtureOwner {
        packet,
        admission,
        profile,
        requirement,
        policy,
    } = owner;
    let observation = capacity_observation(at, remaining);
    let decision = decide_capacity(&observation, policy, at).unwrap();
    let mut exact = ForemanCapacityAdmissionV1 {
        schema: FOREMAN_CAPACITY_ADMISSION_SCHEMA_V1.into(),
        capacity_admission_digest: format!("sha256:{}", "0".repeat(64)),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        profile_digest: profile.profile_digest.clone(),
        capacity_requirement_digest: requirement.capacity_requirement_digest.clone(),
        run_id: admission.run_id.clone(),
        work_item_id: work_item_id.into(),
        adapter_id: profile.work_items[work_item_id].adapter_id.clone(),
        provider_id: observation.provider_id.clone(),
        packet_model_class: packet
            .work_items
            .iter()
            .find(|work| work.id == work_item_id)
            .unwrap()
            .model_routing
            .class
            .clone(),
        profile_model_class: profile.work_items[work_item_id]
            .provider_model_class
            .clone(),
        cost_class: match packet
            .work_items
            .iter()
            .find(|work| work.id == work_item_id)
            .unwrap()
            .model_routing
            .class
            .as_str()
        {
            "small" | "bounded" => CapacityCostClassV1::Cheap,
            "medium" | "large" => CapacityCostClassV1::Expensive,
            _ => panic!("fixture model class must be closed"),
        },
        policy_id: policy.policy_id.clone(),
        observation_digest: observation.observation_digest.clone(),
        policy_digest: policy.policy_digest.clone(),
        decision_digest: decision.decision_digest.clone(),
        evaluated_at: at,
        speculative_requested: false,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".into(),
    };
    exact.seal().unwrap();
    (
        serde_jcs::to_vec(&exact).unwrap(),
        serde_jcs::to_vec(&observation).unwrap(),
        serde_jcs::to_vec(policy).unwrap(),
        serde_jcs::to_vec(&decision).unwrap(),
        decision,
    )
}

fn capacity_evidence<'a>(
    admission_bytes: &'a [u8],
    observation_bytes: &'a [u8],
    policy_bytes: &'a [u8],
    decision_bytes: &'a [u8],
) -> CapacityAdmissionEvidenceV1<'a> {
    CapacityAdmissionEvidenceV1 {
        admission_bytes,
        observation_bytes,
        policy_bytes,
        decision_bytes,
    }
}

fn capacity_matrix_case(
    model_class: &str,
    remaining: f64,
    unknown_observation: bool,
    unknown_allows_new_cheap_work: bool,
) -> (
    CapacityState,
    CapacityAdmissionDisposition,
    Result<(), ForemanError>,
) {
    let directory = tempfile::tempdir().unwrap();
    let mut packet = packet();
    for work in &mut packet.work_items {
        work.model_routing.class = model_class.into();
    }
    packet.seal().unwrap();
    let mut admission = admission(&packet);
    admission.allowed_provider_model_classes = vec![model_class.into()];
    admission.seal().unwrap();
    let mut policy = CapacityPolicyV1::default();
    policy.unknown_allows_new_cheap_work = unknown_allows_new_cheap_work;
    policy.policy_digest = policy.compute_digest().unwrap();
    let mut profile = profile(&packet, &admission);
    for execution in profile.work_items.values_mut() {
        execution.provider_model_class = model_class.into();
    }
    profile.budget_policy_ref = policy.policy_id.clone();
    profile.seal().unwrap();
    let requirement = capacity_requirement(&packet, &admission, &profile, &policy);
    let store = ForemanStore::open(directory.path().join("foreman.sqlite")).unwrap();
    store
        .admit_with_capacity_requirement(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            &serde_jcs::to_vec(&requirement).unwrap(),
            instant(0),
        )
        .unwrap();
    let (
        mut capacity_admission_bytes,
        mut observation_bytes,
        policy_bytes,
        mut decision_bytes,
        mut decision,
    ) = capacity_bundle(
        CapacityFixtureOwner {
            packet: &packet,
            admission: &admission,
            profile: &profile,
            requirement: &requirement,
            policy: &policy,
        },
        "root-a",
        instant(1),
        remaining,
    );
    if unknown_observation {
        let mut observation: CapacityObservationV1 =
            serde_json::from_slice(&observation_bytes).unwrap();
        observation.source_class = SourceClass::Unknown;
        observation.confidence = Confidence::Low;
        observation.disposition = ObservationDisposition::Unknown;
        observation.unknown_reasons = vec!["FIXTURE_SOURCE_UNAVAILABLE".into()];
        observation.windows.clear();
        observation.observation_digest = observation.compute_digest().unwrap();
        decision = decide_capacity(&observation, &policy, instant(1)).unwrap();
        let mut capacity_admission =
            ForemanCapacityAdmissionV1::from_slice(&capacity_admission_bytes).unwrap();
        capacity_admission.observation_digest = observation.observation_digest.clone();
        capacity_admission.decision_digest = decision.decision_digest.clone();
        capacity_admission.seal().unwrap();
        capacity_admission_bytes = serde_jcs::to_vec(&capacity_admission).unwrap();
        observation_bytes = serde_jcs::to_vec(&observation).unwrap();
        decision_bytes = serde_jcs::to_vec(&decision).unwrap();
    }
    let state = decision.state;
    let disposition = decision.admission;
    let result = store
        .prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &capacity_admission_bytes,
                &observation_bytes,
                &policy_bytes,
                &decision_bytes,
            ),
            instant(1),
        )
        .map(|_| ());
    (state, disposition, result)
}

struct CapacityRunFixture {
    _directory: TempDir,
    path: PathBuf,
    store: ForemanStore,
    packet: NightshiftPacketV1,
    admission: ForemanAdmissionV1,
    profile: ExecutionProfileV2,
    requirement: ForemanCapacityRequirementV1,
    policy: CapacityPolicyV1,
}

fn capacity_run_fixture() -> CapacityRunFixture {
    capacity_run_fixture_with_event_maximum(65_536)
}

fn capacity_run_fixture_with_event_maximum(maximum_event_bytes: u64) -> CapacityRunFixture {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite");
    let packet = packet();
    let admission = admission(&packet);
    let policy = CapacityPolicyV1::default();
    let mut profile = profile(&packet, &admission);
    profile.budget_policy_ref = policy.policy_id.clone();
    profile.maximum_event_bytes = maximum_event_bytes;
    profile.seal().unwrap();
    let requirement = capacity_requirement(&packet, &admission, &profile, &policy);
    let store = ForemanStore::open(&path).unwrap();
    store
        .admit_with_capacity_requirement(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            &serde_jcs::to_vec(&requirement).unwrap(),
            instant(0),
        )
        .unwrap();
    CapacityRunFixture {
        _directory: directory,
        path,
        store,
        packet,
        admission,
        profile,
        requirement,
        policy,
    }
}

fn prepare_capacity_fixture(
    fixture: &CapacityRunFixture,
    work_item_id: &str,
    at: chrono::DateTime<Utc>,
) -> WorkerStartRequestV2 {
    let (admission, observation, policy, decision, _) = capacity_bundle(
        CapacityFixtureOwner {
            packet: &fixture.packet,
            admission: &fixture.admission,
            profile: &fixture.profile,
            requirement: &fixture.requirement,
            policy: &fixture.policy,
        },
        work_item_id,
        at,
        0.60,
    );
    fixture
        .store
        .prepare_attempt_with_capacity(
            "run-fixture",
            work_item_id,
            capacity_evidence(&admission, &observation, &policy, &decision),
            at,
        )
        .unwrap()
}

fn resize_retained_capacity_event(
    connection: &Connection,
    kind: &str,
    target_bytes: usize,
) -> usize {
    let raw: Vec<u8> = connection
        .query_row(
            "SELECT raw_bytes FROM events WHERE kind = ?1 ORDER BY sequence ASC LIMIT 1",
            [kind],
            |row| row.get(0),
        )
        .unwrap();
    let mut event: Value = serde_json::from_slice(&raw).unwrap();
    let prior_id = event["event_id"].as_str().unwrap();
    let fixed_bytes = raw.len().checked_sub(prior_id.len()).unwrap();
    assert!(target_bytes > fixed_bytes);
    let event_id = "x".repeat(target_bytes - fixed_bytes);
    event["event_id"] = Value::String(event_id.clone());
    let resized = serde_jcs::to_vec(&event).unwrap();
    assert_eq!(resized.len(), target_bytes);
    connection
        .execute(
            "UPDATE events SET event_id = ?1, raw_bytes = ?2, raw_digest = ?3 WHERE kind = ?4",
            rusqlite::params![event_id, resized, retained_raw_digest(&resized), kind],
        )
        .unwrap();
    target_bytes
}

#[test]
fn capacity_policy_disposition_matrix_is_enforced_at_attempt_admission() {
    let (state, admission, result) = capacity_matrix_case("medium", 0.60, false, false);
    assert_eq!(state, CapacityState::Abundant);
    assert_eq!(admission, CapacityAdmissionDisposition::OrdinaryBounded);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("bounded", 0.60, false, false);
    assert_eq!(state, CapacityState::Abundant);
    assert_eq!(admission, CapacityAdmissionDisposition::OrdinaryBounded);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("medium", 0.30, false, false);
    assert_eq!(state, CapacityState::Normal);
    assert_eq!(admission, CapacityAdmissionDisposition::OrdinaryBounded);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("bounded", 0.30, false, false);
    assert_eq!(state, CapacityState::Normal);
    assert_eq!(admission, CapacityAdmissionDisposition::OrdinaryBounded);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("bounded", 0.15, false, false);
    assert_eq!(state, CapacityState::Conserve);
    assert_eq!(admission, CapacityAdmissionDisposition::CheapBoundedOnly);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("medium", 0.15, false, false);
    assert_eq!(state, CapacityState::Conserve);
    assert_eq!(admission, CapacityAdmissionDisposition::CheapBoundedOnly);
    assert!(matches!(
        result,
        Err(ForemanError::Transition(message))
            if message.contains("only closed cheap model classes")
    ));

    let (state, admission, result) = capacity_matrix_case("bounded", 0.60, true, true);
    assert_eq!(state, CapacityState::Unknown);
    assert_eq!(admission, CapacityAdmissionDisposition::CheapBoundedOnly);
    assert!(result.is_ok());

    let (state, admission, result) = capacity_matrix_case("medium", 0.60, true, true);
    assert_eq!(state, CapacityState::Unknown);
    assert_eq!(admission, CapacityAdmissionDisposition::CheapBoundedOnly);
    assert!(matches!(
        result,
        Err(ForemanError::Transition(message))
            if message.contains("only closed cheap model classes")
    ));

    let (state, admission, result) = capacity_matrix_case("bounded", 0.60, true, false);
    assert_eq!(state, CapacityState::Unknown);
    assert_eq!(admission, CapacityAdmissionDisposition::NoNewWork);
    assert!(matches!(
        result,
        Err(ForemanError::Transition(message)) if message.contains("admits no new work")
    ));

    let (state, admission, result) = capacity_matrix_case("medium", 0.60, true, false);
    assert_eq!(state, CapacityState::Unknown);
    assert_eq!(admission, CapacityAdmissionDisposition::NoNewWork);
    assert!(matches!(
        result,
        Err(ForemanError::Transition(message)) if message.contains("admits no new work")
    ));

    for model_class in ["bounded", "medium"] {
        let (state, admission, result) = capacity_matrix_case(model_class, 0.01, false, false);
        assert_eq!(state, CapacityState::Critical);
        assert_eq!(admission, CapacityAdmissionDisposition::NoNewWork);
        assert!(matches!(
            result,
            Err(ForemanError::Transition(message)) if message.contains("admits no new work")
        ));
    }
}

#[test]
fn shared_capacity_history_refuses_digest_consistent_restart_mutations() {
    {
        let fixture = capacity_run_fixture();
        let path = fixture.path.clone();
        drop(fixture.store);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("DROP TRIGGER events_no_update;")
            .unwrap();
        let raw: Vec<u8> = connection
            .query_row(
                "SELECT raw_bytes FROM events WHERE kind = 'capacity_requirement'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut event: Value = serde_json::from_slice(&raw).unwrap();
        let mut requirement: ForemanCapacityRequirementV1 =
            serde_json::from_value(event["payload"]["requirement"].clone()).unwrap();
        requirement.policy_id = "substituted-policy".into();
        requirement.seal().unwrap();
        let requirement_bytes = serde_jcs::to_vec(&requirement).unwrap();
        event["payload"]["requirement"] = serde_json::to_value(&requirement).unwrap();
        event["payload"]["requirement_bytes"] = serde_json::to_value(&requirement_bytes).unwrap();
        let substituted = serde_jcs::to_vec(&event).unwrap();
        connection
            .execute(
                "UPDATE events SET raw_bytes = ?1, raw_digest = ?2
                 WHERE kind = 'capacity_requirement'",
                rusqlite::params![substituted, retained_raw_digest(&substituted)],
            )
            .unwrap();
        drop(connection);
        assert!(ForemanStore::open(&path)
            .and_then(|store| store.projection("run-fixture"))
            .is_err());
    }

    {
        let fixture = capacity_run_fixture();
        prepare_capacity_fixture(&fixture, "root-a", instant(1));
        let (root_b_admission, root_b_observation, root_b_policy, root_b_decision, _) =
            capacity_bundle(
                CapacityFixtureOwner {
                    packet: &fixture.packet,
                    admission: &fixture.admission,
                    profile: &fixture.profile,
                    requirement: &fixture.requirement,
                    policy: &fixture.policy,
                },
                "root-b",
                instant(2),
                0.60,
            );
        let path = fixture.path.clone();
        drop(fixture.store);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("DROP TRIGGER events_no_update;")
            .unwrap();
        let raw: Vec<u8> = connection
            .query_row(
                "SELECT raw_bytes FROM events WHERE kind = 'capacity_admission'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut event: Value = serde_json::from_slice(&raw).unwrap();
        event["work_item_id"] = Value::String("root-b".into());
        let substituted = serde_jcs::to_vec(&event).unwrap();
        connection
            .execute(
                "UPDATE events SET work_item_id = 'root-b', raw_bytes = ?1, raw_digest = ?2
                 WHERE kind = 'capacity_admission'",
                rusqlite::params![substituted, retained_raw_digest(&substituted)],
            )
            .unwrap();
        let attempts_before: u64 = connection
            .query_row(
                "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        let store = ForemanStore::open(&path).unwrap();
        assert!(store
            .prepare_attempt_with_capacity(
                "run-fixture",
                "root-b",
                capacity_evidence(
                    &root_b_admission,
                    &root_b_observation,
                    &root_b_policy,
                    &root_b_decision,
                ),
                instant(2),
            )
            .is_err());
        drop(store);
        let connection = Connection::open(&path).unwrap();
        let attempts_after: u64 = connection
            .query_row(
                "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(attempts_after, attempts_before);
    }

    {
        let fixture = capacity_run_fixture();
        prepare_capacity_fixture(&fixture, "root-a", instant(1));
        let path = fixture.path.clone();
        drop(fixture.store);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "DROP TRIGGER events_no_update;
                 UPDATE events
                 SET sequence = (SELECT max(sequence) FROM events) + 100
                 WHERE event_id LIKE 'attempt-created-%';",
            )
            .unwrap();
        drop(connection);
        assert!(ForemanStore::open(&path)
            .and_then(|store| store.projection("run-fixture"))
            .is_err());
    }

    {
        let fixture = capacity_run_fixture();
        prepare_capacity_fixture(&fixture, "root-a", instant(1));
        let path = fixture.path.clone();
        drop(fixture.store);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "DROP TRIGGER events_no_delete;
                 DELETE FROM events WHERE kind = 'capacity_admission';",
            )
            .unwrap();
        drop(connection);
        assert!(ForemanStore::open(&path)
            .and_then(|store| store.projection("run-fixture"))
            .is_err());
    }
}

#[test]
fn cumulative_capacity_history_bound_refuses_two_subceiling_rows_on_reopen_and_mutation() {
    let fixture = capacity_run_fixture_with_event_maximum(MAXIMUM_CAPACITY_HISTORY_BYTES);
    prepare_capacity_fixture(&fixture, "root-a", instant(1));
    let (root_b_admission, root_b_observation, root_b_policy, root_b_decision, _) = capacity_bundle(
        CapacityFixtureOwner {
            packet: &fixture.packet,
            admission: &fixture.admission,
            profile: &fixture.profile,
            requirement: &fixture.requirement,
            policy: &fixture.policy,
        },
        "root-b",
        instant(2),
        0.60,
    );
    let path = fixture.path.clone();
    drop(fixture.store);

    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER events_no_update;")
        .unwrap();
    let each = usize::try_from(MAXIMUM_CAPACITY_HISTORY_BYTES / 2).unwrap() + 4_096;
    resize_retained_capacity_event(&connection, "capacity_requirement", each);
    resize_retained_capacity_event(&connection, "capacity_admission", each);
    let lengths = {
        let mut statement = connection
            .prepare(
                "SELECT length(raw_bytes) FROM events
                 WHERE kind IN ('capacity_requirement', 'capacity_admission')
                 ORDER BY sequence ASC",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, u64>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    assert_eq!(lengths, vec![each as u64, each as u64]);
    assert!(lengths
        .iter()
        .all(|length| *length <= fixture.profile.maximum_event_bytes));
    assert!(lengths.iter().sum::<u64>() > MAXIMUM_CAPACITY_HISTORY_BYTES);
    let attempts_before: u64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    let store = ForemanStore::open(&path).unwrap();
    assert!(matches!(
        store.projection("run-fixture"),
        Err(ForemanError::InputTooLarge("capacity journal history"))
    ));
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-b",
            capacity_evidence(
                &root_b_admission,
                &root_b_observation,
                &root_b_policy,
                &root_b_decision,
            ),
            instant(2),
        ),
        Err(ForemanError::InputTooLarge("capacity journal history"))
    ));
    drop(store);
    let connection = Connection::open(&path).unwrap();
    let attempts_after: u64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attempts_after, attempts_before);
}

#[test]
fn cumulative_capacity_history_is_checked_atomically_before_append() {
    let fixture = capacity_run_fixture_with_event_maximum(MAXIMUM_CAPACITY_HISTORY_BYTES);
    prepare_capacity_fixture(&fixture, "root-a", instant(1));
    let (root_b_admission, root_b_observation, root_b_policy, root_b_decision, _) = capacity_bundle(
        CapacityFixtureOwner {
            packet: &fixture.packet,
            admission: &fixture.admission,
            profile: &fixture.profile,
            requirement: &fixture.requirement,
            policy: &fixture.policy,
        },
        "root-b",
        instant(2),
        0.60,
    );
    let path = fixture.path.clone();
    drop(fixture.store);

    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER events_no_update;")
        .unwrap();
    let requirement_bytes: u64 = connection
        .query_row(
            "SELECT length(raw_bytes) FROM events WHERE kind = 'capacity_requirement'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let retained_allowance = MAXIMUM_CAPACITY_HISTORY_BYTES
        .checked_sub(requirement_bytes)
        .unwrap()
        .checked_sub(512)
        .unwrap();
    resize_retained_capacity_event(
        &connection,
        "capacity_admission",
        usize::try_from(retained_allowance).unwrap(),
    );
    let retained_total: u64 = connection
        .query_row(
            "SELECT sum(length(raw_bytes)) FROM events
             WHERE kind IN ('capacity_requirement', 'capacity_admission')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retained_total, MAXIMUM_CAPACITY_HISTORY_BYTES - 512);
    let attempts_before: u64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(connection);

    let store = ForemanStore::open(&path).unwrap();
    store.projection("run-fixture").unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-b",
            capacity_evidence(
                &root_b_admission,
                &root_b_observation,
                &root_b_policy,
                &root_b_decision,
            ),
            instant(2),
        ),
        Err(ForemanError::InputTooLarge("capacity journal history"))
    ));
    drop(store);
    let connection = Connection::open(&path).unwrap();
    let attempts_after: u64 = connection
        .query_row(
            "SELECT count(*) FROM events WHERE event_id LIKE 'attempt-created-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attempts_after, attempts_before);
}

#[test]
fn capacity_metadata_preflight_counts_rows_before_raw_materialization() {
    let fixture = capacity_run_fixture();
    let path = fixture.path.clone();
    let maximum_capacity_rows = fixture.packet.work_items.len().saturating_add(1);
    drop(fixture.store);

    let connection = Connection::open(&path).unwrap();
    for ordinal in 0..maximum_capacity_rows {
        connection
            .execute(
                "INSERT INTO events
                 (event_id, run_id, work_item_id, attempt_id, kind, recorded_at, raw_bytes, raw_digest)
                 VALUES (?1, 'run-fixture', NULL, NULL, 'capacity_admission', ?2,
                         zeroblob(1), ?3)",
                rusqlite::params![
                    format!("extra-capacity-{ordinal}"),
                    instant(1).to_rfc3339(),
                    format!("sha256:{}", "0".repeat(64)),
                ],
            )
            .unwrap();
    }
    drop(connection);

    let store = ForemanStore::open(&path).unwrap();
    assert!(matches!(
        store.projection("run-fixture"),
        Err(ForemanError::InputTooLarge("capacity journal history"))
    ));
    drop(store);
    assert!(matches!(
        read_only_run_snapshot(&path, "run-fixture"),
        Err(ForemanError::InputTooLarge("capacity journal history"))
    ));
}

#[test]
fn capacity_metadata_preflight_refuses_huge_blob_before_raw_materialization() {
    let fixture = capacity_run_fixture_with_event_maximum(MAXIMUM_CAPACITY_HISTORY_BYTES);
    prepare_capacity_fixture(&fixture, "root-a", instant(1));
    let path = fixture.path.clone();
    drop(fixture.store);

    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER events_no_update;")
        .unwrap();
    connection
        .execute(
            "UPDATE events SET raw_bytes = zeroblob(?1) WHERE kind = 'capacity_admission'",
            [MAXIMUM_CAPACITY_HISTORY_BYTES + 1],
        )
        .unwrap();
    drop(connection);

    let store = ForemanStore::open(&path).unwrap();
    assert!(matches!(
        store.projection("run-fixture"),
        Err(ForemanError::InputTooLarge("capacity journal event"))
    ));
    drop(store);
    assert!(matches!(
        read_only_run_snapshot(&path, "run-fixture"),
        Err(ForemanError::InputTooLarge("capacity journal event"))
    ));
}

#[test]
fn legacy_non_capacity_internal_event_above_capacity_ceiling_remains_readable() {
    let (directory, store, _, _, profile) = setup();
    let path = directory.path().join("foreman.sqlite");
    drop(store);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER events_no_update;")
        .unwrap();
    let raw: Vec<u8> = connection
        .query_row(
            "SELECT raw_bytes FROM events WHERE event_id LIKE 'run-admitted-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut event: Value = serde_json::from_slice(&raw).unwrap();
    let event_id = format!(
        "run-admitted-{}",
        "x".repeat(profile.maximum_event_bytes as usize)
    );
    event["event_id"] = Value::String(event_id.clone());
    let enlarged = serde_jcs::to_vec(&event).unwrap();
    assert!(enlarged.len() > profile.maximum_event_bytes as usize);
    connection
        .execute(
            "UPDATE events SET event_id = ?1, raw_bytes = ?2, raw_digest = ?3
             WHERE event_id LIKE 'run-admitted-%'",
            rusqlite::params![event_id, enlarged, retained_raw_digest(&enlarged)],
        )
        .unwrap();
    drop(connection);
    assert_eq!(
        ForemanStore::open(&path)
            .unwrap()
            .projection("run-fixture")
            .unwrap()
            .work_items
            .len(),
        4
    );
}

#[test]
fn capacity_required_run_is_atomic_restartable_and_legacy_path_cannot_win() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite");
    let packet = packet();
    let admission = admission(&packet);
    let policy = CapacityPolicyV1::default();
    let mut profile = profile(&packet, &admission);
    profile.budget_policy_ref = policy.policy_id.clone();
    profile.seal().unwrap();
    let requirement = capacity_requirement(&packet, &admission, &profile, &policy);
    let requirement_raw = serde_jcs::to_vec(&requirement).unwrap();
    let store = ForemanStore::open(&path).unwrap();
    store
        .admit_with_capacity_requirement(
            &packet.canonical_bytes().unwrap(),
            &serde_jcs::to_vec(&admission).unwrap(),
            &serde_jcs::to_vec(&profile).unwrap(),
            &requirement_raw,
            instant(0),
        )
        .unwrap();

    assert!(matches!(
        store.prepare_attempt("run-fixture", "root-a", instant(1)),
        Err(ForemanError::Transition(message))
            if message.contains("capacity-required run refuses legacy")
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .all(|work| work.active_attempt_id.is_none()));

    let (root_a_admission, root_a_observation, policy_raw, root_a_decision, decision) =
        capacity_bundle(
            CapacityFixtureOwner {
                packet: &packet,
                admission: &admission,
                profile: &profile,
                requirement: &requirement,
                policy: &policy,
            },
            "root-a",
            instant(1),
            0.60,
        );
    assert_eq!(
        decision.admission,
        CapacityAdmissionDisposition::OrdinaryBounded
    );
    let mut invalid_requirement = requirement.clone();
    invalid_requirement
        .model_cost_classes
        .insert("bounded".into(), CapacityCostClassV1::Expensive);
    assert!(matches!(
        invalid_requirement.seal(),
        Err(ContractError::InvalidField("capacity model-class map"))
    ));
    let mut speculative_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    speculative_admission.speculative_requested = true;
    assert!(matches!(
        speculative_admission.seal(),
        Err(ContractError::InvalidField(
            "speculative capacity admission is unbound"
        ))
    ));
    let mut noncanonical_timestamp: Value = serde_json::from_slice(&root_a_admission).unwrap();
    noncanonical_timestamp["evaluated_at"] = Value::String("2026-08-29T12:00:01-04:00".into());
    assert!(matches!(
        ForemanCapacityAdmissionV1::from_slice(
            &serde_json::to_vec(&noncanonical_timestamp).unwrap()
        ),
        Err(ContractError::InvalidField("evaluated_at"))
    ));

    let mut scoped_mismatch_observation: CapacityObservationV1 =
        serde_json::from_slice(&root_a_observation).unwrap();
    scoped_mismatch_observation.model_family = Some("medium".into());
    scoped_mismatch_observation.observation_digest =
        scoped_mismatch_observation.compute_digest().unwrap();
    let scoped_mismatch_decision =
        decide_capacity(&scoped_mismatch_observation, &policy, instant(1)).unwrap();
    let mut scoped_mismatch_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    scoped_mismatch_admission.observation_digest =
        scoped_mismatch_observation.observation_digest.clone();
    scoped_mismatch_admission.decision_digest = scoped_mismatch_decision.decision_digest.clone();
    scoped_mismatch_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&scoped_mismatch_admission).unwrap(),
                &serde_jcs::to_vec(&scoped_mismatch_observation).unwrap(),
                &policy_raw,
                &serde_jcs::to_vec(&scoped_mismatch_decision).unwrap(),
            ),
            instant(1),
        ),
        Err(ForemanError::IdentityMismatch(
            "capacity observation model family"
        ))
    ));

    let mut oversized_observation: CapacityObservationV1 =
        serde_json::from_slice(&root_a_observation).unwrap();
    oversized_observation.account_profile_locator = "x".repeat(30_000);
    oversized_observation.observation_digest = oversized_observation.compute_digest().unwrap();
    let oversized_decision = decide_capacity(&oversized_observation, &policy, instant(1)).unwrap();
    let mut oversized_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    oversized_admission.observation_digest = oversized_observation.observation_digest.clone();
    oversized_admission.decision_digest = oversized_decision.decision_digest.clone();
    oversized_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&oversized_admission).unwrap(),
                &serde_jcs::to_vec(&oversized_observation).unwrap(),
                &policy_raw,
                &serde_jcs::to_vec(&oversized_decision).unwrap(),
            ),
            instant(1),
        ),
        Err(ForemanError::InputTooLarge("capacity journal event"))
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .all(|work| work.active_attempt_id.is_none()));

    let exact_observation: CapacityObservationV1 =
        serde_json::from_slice(&root_a_observation).unwrap();
    let mut substituted_decision: CapacityDecisionV1 =
        serde_json::from_slice(&root_a_decision).unwrap();
    substituted_decision.state = CapacityState::Normal;
    substituted_decision.allow_new_speculative_work = false;
    substituted_decision.reason_codes = vec!["MINIMUM_REMAINING_WINDOW_NORMAL".into()];
    substituted_decision.decision_digest = substituted_decision.compute_digest().unwrap();
    substituted_decision.validate().unwrap();
    let mut substituted_outcome_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    substituted_outcome_admission.decision_digest = substituted_decision.decision_digest.clone();
    substituted_outcome_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&substituted_outcome_admission).unwrap(),
                &root_a_observation,
                &policy_raw,
                &serde_jcs::to_vec(&substituted_decision).unwrap(),
            ),
            instant(1),
        ),
        Err(ForemanError::Transition(message))
            if message.contains("not the exact deterministic FUEL outcome")
    ));

    let substituted_time_decision =
        decide_capacity(&exact_observation, &policy, instant(2)).unwrap();
    let mut substituted_time_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    substituted_time_admission.evaluated_at = instant(2);
    substituted_time_admission.decision_digest = substituted_time_decision.decision_digest.clone();
    substituted_time_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&substituted_time_admission).unwrap(),
                &root_a_observation,
                &policy_raw,
                &serde_jcs::to_vec(&substituted_time_decision).unwrap(),
            ),
            instant(1),
        ),
        Err(ForemanError::Transition(message)) if message.contains("not current")
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .all(|work| work.active_attempt_id.is_none()));

    let mut substituted_admission =
        ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    substituted_admission.provider_id = "provider:substituted".into();
    substituted_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&substituted_admission).unwrap(),
                &root_a_observation,
                &policy_raw,
                &root_a_decision,
            ),
            instant(1),
        ),
        Err(ForemanError::IdentityMismatch(
            "capacity requirement identity"
        ))
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .all(|work| work.active_attempt_id.is_none()));

    let mut stale_observation: CapacityObservationV1 =
        serde_json::from_slice(&root_a_observation).unwrap();
    stale_observation.expires_at = instant(1);
    stale_observation.observation_digest = stale_observation.compute_digest().unwrap();
    let stale_decision = decide_capacity(&stale_observation, &policy, instant(1)).unwrap();
    let mut stale_admission = ForemanCapacityAdmissionV1::from_slice(&root_a_admission).unwrap();
    stale_admission.observation_digest = stale_observation.observation_digest.clone();
    stale_admission.decision_digest = stale_decision.decision_digest.clone();
    stale_admission.seal().unwrap();
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &serde_jcs::to_vec(&stale_admission).unwrap(),
                &serde_jcs::to_vec(&stale_observation).unwrap(),
                &policy_raw,
                &serde_jcs::to_vec(&stale_decision).unwrap(),
            ),
            instant(1),
        ),
        Err(ForemanError::Transition(message)) if message.contains("not current")
    ));
    assert!(store
        .projection("run-fixture")
        .unwrap()
        .work_items
        .iter()
        .all(|work| work.active_attempt_id.is_none()));

    let root_a = store
        .prepare_attempt_with_capacity(
            "run-fixture",
            "root-a",
            capacity_evidence(
                &root_a_admission,
                &root_a_observation,
                &policy_raw,
                &root_a_decision,
            ),
            instant(1),
        )
        .unwrap();
    drop(store);

    let store = ForemanStore::open(&path).unwrap();
    assert_eq!(
        store
            .projection("run-fixture")
            .unwrap()
            .work_items
            .iter()
            .find(|work| work.work_item_id == "root-a")
            .unwrap()
            .active_attempt_id
            .as_deref(),
        Some(root_a.attempt_id.as_str())
    );

    let (critical_admission, critical_observation, critical_policy, critical_decision, decision) =
        capacity_bundle(
            CapacityFixtureOwner {
                packet: &packet,
                admission: &admission,
                profile: &profile,
                requirement: &requirement,
                policy: &policy,
            },
            "root-b",
            instant(2),
            0.01,
        );
    assert_eq!(decision.admission, CapacityAdmissionDisposition::NoNewWork);
    assert!(matches!(
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-b",
            capacity_evidence(
                &critical_admission,
                &critical_observation,
                &critical_policy,
                &critical_decision,
            ),
            instant(2),
        ),
        Err(ForemanError::Transition(message)) if message.contains("admits no new work")
    ));
    store
        .accept_terminal_receipt(
            &serde_jcs::to_vec(&terminal(
                &packet,
                &root_a,
                "EXACT-CUSTODY",
                "INDEPENDENT-CAPACITY-AWARE-FIXTURE",
            ))
            .unwrap(),
        )
        .unwrap();

    let (root_b_admission, root_b_observation, root_b_policy, root_b_decision, _) = capacity_bundle(
        CapacityFixtureOwner {
            packet: &packet,
            admission: &admission,
            profile: &profile,
            requirement: &requirement,
            policy: &policy,
        },
        "root-b",
        instant(6),
        0.60,
    );
    drop(store);
    let barrier = Arc::new(Barrier::new(2));
    let legacy_barrier = barrier.clone();
    let legacy_path = path.clone();
    let legacy = std::thread::spawn(move || {
        let store = ForemanStore::open(legacy_path).unwrap();
        legacy_barrier.wait();
        store.prepare_attempt("run-fixture", "root-b", instant(6))
    });
    let capacity_barrier = barrier;
    let capacity_path = path.clone();
    let capacity = std::thread::spawn(move || {
        let store = ForemanStore::open(capacity_path).unwrap();
        capacity_barrier.wait();
        store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-b",
            capacity_evidence(
                &root_b_admission,
                &root_b_observation,
                &root_b_policy,
                &root_b_decision,
            ),
            instant(6),
        )
    });
    assert!(legacy.join().unwrap().is_err());
    let root_b = capacity.join().unwrap().unwrap();

    let store = ForemanStore::open(&path).unwrap();
    assert_eq!(
        store
            .projection("run-fixture")
            .unwrap()
            .work_items
            .iter()
            .find(|work| work.work_item_id == "root-b")
            .unwrap()
            .active_attempt_id
            .as_deref(),
        Some(root_b.attempt_id.as_str())
    );
    drop(store);
    let snapshot = nightshift_foreman::read_only_run_snapshot(&path, "run-fixture").unwrap();
    let projected_requirement = snapshot.capacity_requirement.as_ref().unwrap();
    assert_eq!(projected_requirement.requirement, requirement);
    assert_eq!(projected_requirement.requirement_bytes, requirement_raw);
    assert_eq!(snapshot.capacity_admissions.len(), 2);
    let root_a_capacity = snapshot
        .capacity_admissions
        .iter()
        .find(|capacity| capacity.work_item_id == "root-a")
        .unwrap();
    assert_eq!(root_a_capacity.admission_bytes, root_a_admission);
    assert_eq!(root_a_capacity.observation_bytes, root_a_observation);
    assert_eq!(root_a_capacity.policy_bytes, policy_raw);
    assert_eq!(root_a_capacity.decision_bytes, root_a_decision);
    assert!(snapshot
        .events
        .iter()
        .all(|event| event.raw_digest.starts_with("sha256:")));
}
#[test]
fn midnight_rail_four_item_deterministic_dogfood_qualifies_without_provider() {
    fn event(
        packet: &NightshiftPacketV1,
        request: &WorkerStartRequestV2,
        event_id: &str,
        kind: AdapterEventKindV1,
        at: chrono::DateTime<Utc>,
    ) -> AdapterEventV1 {
        let mut event = AdapterEventV1 {
            schema: WORKER_ADAPTER_EVENT_SCHEMA_V1.into(),
            event_digest: format!("sha256:{}", "0".repeat(64)),
            event_id: event_id.into(),
            packet_digest: packet.packet_digest.clone(),
            run_id: request.run_id.clone(),
            work_item_id: request.work_item_id.clone(),
            attempt_id: request.attempt_id.clone(),
            adapter_id: request.adapter_id.clone(),
            adapter_version: request.adapter_version.clone(),
            occurred_at: at,
            kind,
            provider_identity: Some("provider:fixture".into()),
            model_identity: Some("model:fixture".into()),
            session_identity: Some("session:fixture".into()),
            thread_identity: None,
            turn_identity: None,
            queue_identity: None,
            message: None,
            human_question: None,
            extensions: BTreeMap::new(),
        };
        if !matches!(event.kind, AdapterEventKindV1::HumanQuestion) {
            event.seal().unwrap();
        }
        event
    }

    fn bound_terminal(
        packet: &NightshiftPacketV1,
        request: &WorkerStartRequestV2,
        state: &str,
        classification: &str,
        started_at: chrono::DateTime<Utc>,
        ended_at: chrono::DateTime<Utc>,
    ) -> TerminalReceiptV1 {
        let mut receipt = terminal(packet, request, state, classification);
        receipt.started_at = started_at;
        receipt.ended_at = ended_at;
        receipt.seal().unwrap();
        receipt
    }

    let fixture = capacity_run_fixture();
    let root_a = prepare_capacity_fixture(&fixture, "root-a", instant(1));
    let root_b = prepare_capacity_fixture(&fixture, "root-b", instant(2));
    assert_ne!(root_a.attempt_id, root_b.attempt_id);

    let root_c_bundle = capacity_bundle(
        CapacityFixtureOwner {
            packet: &fixture.packet,
            admission: &fixture.admission,
            profile: &fixture.profile,
            requirement: &fixture.requirement,
            policy: &fixture.policy,
        },
        "root-c",
        instant(3),
        0.60,
    );
    assert!(matches!(
        fixture.store.prepare_attempt_with_capacity(
            "run-fixture",
            "root-c",
            capacity_evidence(
                &root_c_bundle.0,
                &root_c_bundle.1,
                &root_c_bundle.2,
                &root_c_bundle.3,
            ),
            instant(3),
        ),
        Err(ForemanError::ResourceUnavailable(_))
    ));
    fixture
        .store
        .record_dispatch_requested("run-fixture", "root-a", &root_a.attempt_id, instant(2))
        .unwrap();
    fixture
        .store
        .record_dispatch_requested("run-fixture", "root-b", &root_b.attempt_id, instant(3))
        .unwrap();

    let before_restart =
        nightshift_casework::load_live_run_at(&fixture.path, "run-fixture", instant(3)).unwrap();
    assert_eq!(before_restart.projection.resource_claims.len(), 2);
    assert_eq!(
        before_restart.projection.provider_capacity.attempts.len(),
        2
    );
    assert_eq!(
        before_restart.projection.provider_capacity.status,
        "EXACT_RECORDED_BY_FOREMAN"
    );
    assert!(before_restart
        .projection
        .provider_capacity
        .attempts
        .windows(2)
        .all(|pair| pair[0].observed_at < pair[1].observed_at));

    let CapacityRunFixture {
        _directory: directory,
        path,
        store,
        packet,
        admission,
        profile,
        requirement,
        policy,
    } = fixture;
    drop(store);
    let store = ForemanStore::open(&path).unwrap();
    assert_eq!(
        store
            .projection("run-fixture")
            .unwrap()
            .work_items
            .iter()
            .find(|item| item.work_item_id == "root-a")
            .unwrap()
            .active_attempt_id
            .as_deref(),
        Some(root_a.attempt_id.as_str())
    );
    store
        .record_resume_requested("run-fixture", "root-a", &root_a.attempt_id, instant(4))
        .unwrap();
    store
        .accept_adapter_event(
            &serde_jcs::to_vec(&event(
                &packet,
                &root_a,
                "midnight-root-a-checkpoint",
                AdapterEventKindV1::Checkpoint,
                instant(4),
            ))
            .unwrap(),
        )
        .unwrap();
    store
        .accept_adapter_event(
            &serde_jcs::to_vec(&event(
                &packet,
                &root_b,
                "midnight-root-b-started",
                AdapterEventKindV1::WorkerStarted,
                instant(4),
            ))
            .unwrap(),
        )
        .unwrap();

    let root_a_receipt = bound_terminal(
        &packet,
        &root_a,
        "IMPLEMENTATION-EVIDENCE-RETAINED",
        "MIDNIGHT-DETERMINISTIC-IMPLEMENTATION-FIXTURE",
        instant(1),
        instant(5),
    );
    let root_a_raw = serde_jcs::to_vec(&root_a_receipt).unwrap();
    store.accept_terminal_receipt(&root_a_raw).unwrap();
    let root_b_receipt = bound_terminal(
        &packet,
        &root_b,
        "AUDIT-EVIDENCE-RETAINED",
        "MIDNIGHT-DETERMINISTIC-AUDIT-FIXTURE",
        instant(2),
        instant(6),
    );
    store
        .accept_terminal_receipt(&serde_jcs::to_vec(&root_b_receipt).unwrap())
        .unwrap();

    let dependent_bundle = capacity_bundle(
        CapacityFixtureOwner {
            packet: &packet,
            admission: &admission,
            profile: &profile,
            requirement: &requirement,
            policy: &policy,
        },
        "dependent",
        instant(8),
        0.30,
    );
    let dependent = store
        .prepare_attempt_with_capacity(
            "run-fixture",
            "dependent",
            capacity_evidence(
                &dependent_bundle.0,
                &dependent_bundle.1,
                &dependent_bundle.2,
                &dependent_bundle.3,
            ),
            instant(8),
        )
        .unwrap();
    let dependent_brief = store.worker_brief("run-fixture", "dependent").unwrap();
    WorkerBriefV2::from_slice_for_start(&dependent_brief, &dependent).unwrap();
    let brief: Value = serde_json::from_slice(&dependent_brief).unwrap();
    assert_eq!(
        hex::decode(
            brief["predecessor_receipts"]["root-a"]["bytes_hex"]
                .as_str()
                .unwrap()
        )
        .unwrap(),
        root_a_raw
    );
    store
        .accept_adapter_event(
            &serde_jcs::to_vec(&event(
                &packet,
                &dependent,
                "midnight-dependent-started",
                AdapterEventKindV1::WorkerStarted,
                instant(9),
            ))
            .unwrap(),
        )
        .unwrap();
    store
        .accept_terminal_receipt(
            &serde_jcs::to_vec(&bound_terminal(
                &packet,
                &dependent,
                "ENTRY-EVALUATED-EXACT-PREDECESSOR",
                "MIDNIGHT-DETERMINISTIC-DEPENDENT-FIXTURE",
                instant(8),
                instant(10),
            ))
            .unwrap(),
        )
        .unwrap();

    let root_c_bundle = capacity_bundle(
        CapacityFixtureOwner {
            packet: &packet,
            admission: &admission,
            profile: &profile,
            requirement: &requirement,
            policy: &policy,
        },
        "root-c",
        instant(11),
        0.60,
    );
    let root_c = store
        .prepare_attempt_with_capacity(
            "run-fixture",
            "root-c",
            capacity_evidence(
                &root_c_bundle.0,
                &root_c_bundle.1,
                &root_c_bundle.2,
                &root_c_bundle.3,
            ),
            instant(11),
        )
        .unwrap();
    store
        .record_dispatch_requested("run-fixture", "root-c", &root_c.attempt_id, instant(11))
        .unwrap();
    let question = HumanQuestionV1 {
        question_id: "midnight-fixture-authority".into(),
        question: "Is protected target-effect authority present for this fixture lane?".into(),
        exhausted_evidence: "The deterministic fixture carries no target-effect authority.".into(),
        safe_default: "Do not perform the protected effect.".into(),
        consequences: "Only this lane terminates blocked; independent lanes retain receipts."
            .into(),
        resume_point: "Create a successor occurrence after exact authority exists.".into(),
    };
    let mut question_event = event(
        &packet,
        &root_c,
        "midnight-root-c-human-question",
        AdapterEventKindV1::HumanQuestion,
        instant(12),
    );
    question_event.human_question = Some(question.clone());
    question_event.seal().unwrap();
    store
        .accept_adapter_event(&serde_jcs::to_vec(&question_event).unwrap())
        .unwrap();
    let mut lane_receipt = bound_terminal(
        &packet,
        &root_c,
        "BLOCKED-HUMAN-EXACT",
        "MIDNIGHT-LANE-LOCAL-QUESTION",
        instant(11),
        instant(13),
    );
    lane_receipt.human_questions = vec![question];
    lane_receipt.seal().unwrap();
    store
        .accept_terminal_receipt(&serde_jcs::to_vec(&lane_receipt).unwrap())
        .unwrap();

    let live = nightshift_casework::load_live_run_at(&path, "run-fixture", instant(14)).unwrap();
    assert_eq!(live.projection.work_items.len(), 4);
    assert_eq!(live.projection.provider_capacity.attempts.len(), 4);
    assert_eq!(
        live.projection
            .work_items
            .iter()
            .flat_map(|item| &item.human_questions)
            .count(),
        1
    );
    assert!(serde_json::to_value(&live.projection)
        .unwrap()
        .get("aggregate_result")
        .is_none());

    let final_receipts = store.close("run-fixture", instant(15)).unwrap();
    assert_eq!(
        final_receipts,
        store.close("run-fixture", instant(30)).unwrap()
    );
    let events = serde_jcs::to_vec(&store.export_events("run-fixture").unwrap()).unwrap();
    drop(store);
    let closed = nightshift_casework::load_live_run_at(&path, "run-fixture", instant(15)).unwrap();
    assert_eq!(
        closed.projection.foreman.lifecycle,
        "CLOSED_EXACT_FINAL_SNAPSHOT_RETAINED"
    );
    assert_eq!(closed.projection.foreman.terminal_receipt_count, 4);
    assert_eq!(closed.projection.foreman.not_started_receipt_count, 0);

    let sealed = directory.path().join("sealed-case");
    fs::create_dir(&sealed).unwrap();
    let packet_bytes = packet.canonical_bytes().unwrap();
    fs::write(sealed.join("packet.v1.json"), &packet_bytes).unwrap();
    fs::write(sealed.join("run-receipts.v1.json"), &final_receipts).unwrap();
    let final_case = nightshift_casework::load_run_at(&sealed, instant(12)).unwrap();
    assert_eq!(final_case.receipt_bytes, final_receipts);
    assert_eq!(final_case.projection.work_items.len(), 4);
    assert!(serde_json::to_value(&final_case.projection)
        .unwrap()
        .get("aggregate_result")
        .is_none());

    let approval = capacity_run_fixture();
    let waiting_request = prepare_capacity_fixture(&approval, "root-a", instant(1));
    let independent_request = prepare_capacity_fixture(&approval, "root-b", instant(2));
    approval
        .store
        .record_dispatch_requested(
            "run-fixture",
            "root-a",
            &waiting_request.attempt_id,
            instant(2),
        )
        .unwrap();
    approval
        .store
        .record_dispatch_requested(
            "run-fixture",
            "root-b",
            &independent_request.attempt_id,
            instant(2),
        )
        .unwrap();
    let mut waiting = event(
        &approval.packet,
        &waiting_request,
        "midnight-waiting-approval",
        AdapterEventKindV1::WaitingApproval,
        instant(3),
    );
    waiting.message = Some("Protected fixture effect requires authority; no response sent.".into());
    waiting.extensions = BTreeMap::from([
        ("approval_response_sent".into(), Value::Bool(false)),
        ("protected_effect_absent".into(), Value::Bool(true)),
    ]);
    waiting.seal().unwrap();
    let waiting_raw = serde_jcs::to_vec(&waiting).unwrap();
    approval.store.accept_adapter_event(&waiting_raw).unwrap();
    approval
        .store
        .accept_adapter_event(
            &serde_jcs::to_vec(&event(
                &approval.packet,
                &independent_request,
                "midnight-independent-started",
                AdapterEventKindV1::WorkerStarted,
                instant(3),
            ))
            .unwrap(),
        )
        .unwrap();
    approval
        .store
        .accept_terminal_receipt(
            &serde_jcs::to_vec(&bound_terminal(
                &approval.packet,
                &independent_request,
                "INDEPENDENT-LANE-COMPLETE",
                "MIDNIGHT-APPROVAL-INDEPENDENCE-FIXTURE",
                instant(2),
                instant(5),
            ))
            .unwrap(),
        )
        .unwrap();
    let approval_live =
        nightshift_casework::load_live_run_at(&approval.path, "run-fixture", instant(6)).unwrap();
    assert_eq!(
        approval_live
            .projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == "root-a")
            .unwrap()
            .scheduler_state,
        "WAITING_APPROVAL"
    );
    assert!(approval_live
        .projection
        .work_items
        .iter()
        .find(|item| item.work_item_id == "root-b")
        .unwrap()
        .accepted_outcome
        .is_some());
    assert!(matches!(
        approval.store.close("run-fixture", instant(7)),
        Err(ForemanError::IncompleteCloseout(_))
    ));
    let retained_waiting = approval
        .store
        .export_events("run-fixture")
        .unwrap()
        .into_iter()
        .any(|record| record == serde_json::from_slice::<Value>(&waiting_raw).unwrap());
    assert!(retained_waiting);
    let approval_events =
        serde_jcs::to_vec(&approval.store.export_events("run-fixture").unwrap()).unwrap();

    if let Some(output) = std::env::var_os("NIGHTSHIFT_MIDNIGHT_FIXTURE_DIR") {
        let output = PathBuf::from(output);
        assert!(output.is_dir());
        for (name, bytes) in [
            ("packet.v1.json", packet_bytes),
            (
                "foreman-admission.v1.json",
                serde_jcs::to_vec(&admission).unwrap(),
            ),
            (
                "foreman-execution-profile.v2.json",
                serde_jcs::to_vec(&profile).unwrap(),
            ),
            (
                "foreman-capacity-requirement.v1.json",
                serde_jcs::to_vec(&requirement).unwrap(),
            ),
            ("foreman-events.v1.json", events),
            (
                "live-before-close.v1.json",
                serde_jcs::to_vec(&live.projection).unwrap(),
            ),
            (
                "live-closed.v1.json",
                serde_jcs::to_vec(&closed.projection).unwrap(),
            ),
            ("run-receipts.v1.json", final_receipts),
            (
                "final-casework.v1.json",
                serde_jcs::to_vec(&final_case.projection).unwrap(),
            ),
            (
                "approval-waiting-live.v1.json",
                serde_jcs::to_vec(&approval_live.projection).unwrap(),
            ),
            ("approval-events.v1.json", approval_events),
            ("dependent-worker-brief.v2.json", dependent_brief),
        ] {
            fs::write(output.join(name), bytes).unwrap();
        }
    }
}
