use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::{Duration, TimeZone as _, Utc};
use nightshift_foreman::{
    verify_adapter_contract, AdapterEventKindV1, AdapterEventV1, AdapterRegistrationV2,
    ContractError, ExecutionProfileV2, ForemanAdmissionV1, ForemanError, ForemanStore,
    HumanQuestionV1, NotStartedReceiptV1, ReceiptRepositoryV1, SchedulerStateV1,
    TeardownDeclarationV1, TerminalReceiptV1, WorkItemExecutionV1, WorkerAdapterCapabilitiesV1,
    WorkerBriefV2, WorkerStartRequestV2, FOREMAN_ADMISSION_SCHEMA_V1,
    FOREMAN_EXECUTION_PROFILE_SCHEMA_V2, MAXIMUM_WORKER_BRIEF_BYTES,
    WORKER_ADAPTER_CAPABILITIES_SCHEMA_V1, WORKER_ADAPTER_EVENT_SCHEMA_V1,
    WORKER_BRIEF_BASIS_SCHEMA_V2, WORKER_START_REQUEST_SCHEMA_V2,
    WORKER_TERMINAL_RECEIPT_SCHEMA_V1, WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1,
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
}

#[test]
fn checked_in_contract_schemas_are_closed_json_documents() {
    for bytes in [
        include_bytes!("../../../schemas/nightshift.foreman-admission.v1.schema.json").as_slice(),
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
