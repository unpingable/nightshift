use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::{Duration, TimeZone as _, Utc};
use nightshift_foreman::{
    AdapterEventKindV1, AdapterEventV1, AdapterRegistrationV2, ExecutionProfileV2,
    ForemanAdmissionV1, ForemanError, ForemanStore, HumanQuestionV1, NotStartedReceiptV1,
    ReceiptRepositoryV1, SchedulerStateV1, TeardownDeclarationV1, TerminalReceiptV1,
    WorkItemExecutionV1, FOREMAN_ADMISSION_SCHEMA_V1, FOREMAN_EXECUTION_PROFILE_SCHEMA_V2,
    WORKER_ADAPTER_EVENT_SCHEMA_V1, WORKER_TERMINAL_RECEIPT_SCHEMA_V1,
    WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1,
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
    request: &nightshift_foreman::WorkerStartRequestV1,
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
}

#[test]
fn wal_journal_locks_restart_and_classification_separation_qualify() {
    let (directory, store, packet, _, _) = setup();
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
    let mut receipt = terminal(&packet, &request, "EXACT-STATE", "EXACT-CLASSIFICATION");
    receipt.thread_identity = Some("thread:fixture".into());
    receipt.turn_identity = Some("turn:fixture".into());
    receipt.queue_identity = Some("queue:fixture".into());
    receipt.seal().unwrap();
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
        .execute_batch("DROP TRIGGER final_snapshots_no_update;")
        .unwrap();
    let substituted = b"{}".to_vec();
    connection
        .execute(
            "UPDATE final_snapshots SET raw_digest = ?1, raw_bytes = ?2
             WHERE run_id = 'run-fixture'",
            rusqlite::params![retained_raw_digest(&substituted), substituted],
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
fn checked_in_contract_schemas_are_closed_json_documents() {
    for bytes in [
        include_bytes!("../../../schemas/nightshift.foreman-admission.v1.schema.json").as_slice(),
        include_bytes!("../../../schemas/nightshift.foreman-execution-profile.v2.schema.json")
            .as_slice(),
        include_bytes!("../../../schemas/nightshift.worker-adapter.v1.schema.json").as_slice(),
    ] {
        let schema: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(schema.get("$id").is_some());
    }
}
