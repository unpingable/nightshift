use std::collections::BTreeMap;

use chrono::{DateTime, Duration, TimeZone as _, Utc};
use nightshift_foreman::*;
use nightshift_provider_capacity::{
    CapacityPolicyV1, Confidence, WindowType, CAPACITY_POLICY_SCHEMA_V1,
};
use nightshiftd::packet::{
    AuthoringIdentityV1, CampaignIdentityV1, CanonicalizationV1, ExactWorkRefV1,
    GlobalConstraintsV1, ModelRoutingV1, NightshiftPacketV1, RepositoryCustodyV1,
    SourceEvidenceRefV1, SwitchyardRegistrationV1, WorkItemV1, WorkerBudgetV1,
    EXACT_WORK_PROPOSAL_SCHEMA_V1, NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1,
    NIGHTSHIFT_PACKET_SCHEMA_V1,
};
use serde::Serialize;

fn time(second: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 31, 12, 0, second).unwrap()
}

fn placeholder() -> String {
    format!("sha256:{}", "0".repeat(64))
}

fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).unwrap()
}

fn work_item(id: &str, codename: &str, dependencies: Vec<&str>) -> WorkItemV1 {
    WorkItemV1 {
        id: id.to_owned(),
        track: "second-watch-fixture".to_owned(),
        campaign: CampaignIdentityV1 {
            codename: codename.to_owned(),
            canonical_slug: format!("second-watch-fixture-{id}"),
        },
        predecessor_lineage: vec![],
        dependencies: dependencies.into_iter().map(str::to_owned).collect(),
        exact_work_refs: vec![ExactWorkRefV1 {
            contract_kind: "exact_work_proposal_v1".to_owned(),
            contract_schema: EXACT_WORK_PROPOSAL_SCHEMA_V1.to_owned(),
            repository: "nightshift".to_owned(),
            branch: "campaign/second-watch-fixture".to_owned(),
            commit: "a".repeat(40),
            path: "qualification/second-watch/proposal.json".to_owned(),
            proposal_ref: format!("sha256:{}", "b".repeat(64)),
        }],
        entry_predicates: vec!["inspect exact retained inputs".to_owned()],
        allowed_mutation_surfaces: vec!["campaign-owned local fixture".to_owned()],
        forbidden_actions: vec![
            "target actuation".to_owned(),
            "approval response".to_owned(),
            "bootstrap recursion".to_owned(),
        ],
        acceptance_tests: vec!["exact terminal or not-started custody".to_owned()],
        stop_conditions: vec!["exact contract discrepancy".to_owned()],
        expected_receipts: vec!["terminal or not-started".to_owned()],
        closeout_requirements: vec!["local fixture teardown".to_owned()],
        model_routing: ModelRoutingV1 {
            class: "bounded".to_owned(),
            reason: "deterministic qualification".to_owned(),
            maximum_mutating_workers: 1,
        },
    }
}

fn packet() -> NightshiftPacketV1 {
    let mut value = NightshiftPacketV1 {
        schema: NIGHTSHIFT_PACKET_SCHEMA_V1.to_owned(),
        packet_id: "second-watch-fixture".to_owned(),
        packet_digest: String::new(),
        created_at: time(0),
        current_until: time(0) + Duration::hours(2),
        authoring: AuthoringIdentityV1 {
            agent: "second-watch-contract-test".to_owned(),
            session: "second-watch-contract-test".to_owned(),
            authority_basis: "direct operator successor authorization".to_owned(),
        },
        canonicalization: CanonicalizationV1 {
            algorithm: "RFC8785-JCS".to_owned(),
            digest_algorithm: "SHA-256".to_owned(),
            digest_preimage: NIGHTSHIFT_PACKET_DIGEST_PREIMAGE_V1.to_owned(),
        },
        source_evidence: vec![SourceEvidenceRefV1 {
            repository: "nightshift".to_owned(),
            branch: "campaign/holding-pattern".to_owned(),
            commit: SECOND_WATCH_HOLDING_RESULT_HEAD.to_owned(),
            path: "qualification/holding-pattern".to_owned(),
            file_digest: format!("sha256:{}", "c".repeat(64)),
            predecessor_classification: "EXACT-RESULT-ANCESTRY".to_owned(),
        }],
        repository_custody: vec![RepositoryCustodyV1 {
            repository: "nightshift".to_owned(),
            path: "/tmp/second-watch-fixture".to_owned(),
            branch: "campaign/second-watch-fixture".to_owned(),
            commit: "a".repeat(40),
            remote: None,
            remote_commit: None,
            worktree_clean: true,
            discrepancy: None,
        }],
        global_constraints: GlobalConstraintsV1 {
            allowed_actions: vec!["bounded local compute scheduling".to_owned()],
            forbidden_actions: vec!["target actuation".to_owned()],
            invariants: vec!["scheduler state is not authority".to_owned()],
        },
        work_items: vec![
            work_item("lane-a", "FIXTURE-LARK", vec![]),
            work_item("lane-b", "FIXTURE-TERN", vec![]),
            work_item("question", "FIXTURE-WREN", vec![]),
            work_item("closeout", "FIXTURE-OWL", vec!["lane-a", "lane-b"]),
        ],
        worker_budget: WorkerBudgetV1 {
            maximum_concurrent_mutating_workers: 2,
            recursive_worker_swarms_forbidden: true,
            reserve_posture: "bounded deterministic fixture".to_owned(),
        },
        human_question_criteria: vec!["presentation-only question".to_owned()],
        switchyard: SwitchyardRegistrationV1 {
            alias: "second-watch-fixture".to_owned(),
            plan_ref: String::new(),
            transport_fields: vec![
                "alias".to_owned(),
                "plan_ref".to_owned(),
                "nonce".to_owned(),
            ],
        },
    };
    value.seal().unwrap();
    value
}

fn admission(packet: &NightshiftPacketV1) -> ForemanAdmissionV1 {
    let mut value = ForemanAdmissionV1 {
        schema: FOREMAN_ADMISSION_SCHEMA_V1.to_owned(),
        admission_digest: placeholder(),
        run_id: "second-watch-run".to_owned(),
        packet_digest: packet.packet_digest.clone(),
        operator_basis_digest: format!("sha256:{}", "d".repeat(64)),
        admitted_at: time(0),
        expires_at: time(0) + Duration::hours(1),
        local_runtime_identity: "second-watch-local-runtime".to_owned(),
        maximum_concurrent_workers: 2,
        allowed_adapter_ids: vec![HOLDING_QUALIFICATION_PRODUCER_ID.to_owned()],
        allowed_provider_model_classes: vec!["bounded".to_owned()],
        maximum_new_attempts_per_work_item: 1,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
        target_effects_authorized: false,
    };
    value.seal().unwrap();
    value
}

fn capacity_policy() -> CapacityPolicyV1 {
    let mut value = CapacityPolicyV1 {
        schema: CAPACITY_POLICY_SCHEMA_V1.to_owned(),
        policy_id: "second-watch-budget".to_owned(),
        abundant_min_remaining: 0.8,
        normal_min_remaining: 0.5,
        conserve_min_remaining: 0.2,
        minimum_confidence: Confidence::High,
        required_window_types: vec![WindowType::FiveHour],
        unknown_allows_new_cheap_work: true,
        policy_digest: placeholder(),
    };
    value.policy_digest = value.compute_digest().unwrap();
    value.validate().unwrap();
    value
}

fn profile(
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    capacity_policy: &CapacityPolicyV1,
) -> ExecutionProfileV2 {
    let work_items = packet
        .work_items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                WorkItemExecutionV1 {
                    adapter_id: HOLDING_QUALIFICATION_PRODUCER_ID.to_owned(),
                    workspace_identity: format!("workspace:{}", item.id),
                    resource_lock_keys: vec![format!("fixture:{}", item.id)],
                    provider_model_class: "bounded".to_owned(),
                },
            )
        })
        .collect();
    let mut value = ExecutionProfileV2 {
        schema: FOREMAN_EXECUTION_PROFILE_SCHEMA_V2.to_owned(),
        profile_digest: placeholder(),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        adapters: BTreeMap::from([(
            HOLDING_QUALIFICATION_PRODUCER_ID.to_owned(),
            AdapterRegistrationV2 {
                adapter_id: HOLDING_QUALIFICATION_PRODUCER_ID.to_owned(),
                protocol: DETERMINISTIC_PROVIDER_ADMISSION_EVIDENCE_SCHEMA_V1.to_owned(),
                adapter_version: HOLDING_QUALIFICATION_PRODUCER_VERSION.to_owned(),
                executable_identity: HOLDING_QUALIFICATION_EXECUTABLE_SHA256.to_owned(),
                bounded_arguments: vec![],
            },
        )]),
        work_items,
        budget_policy_ref: capacity_policy.policy_id.clone(),
        log_custody_root: "/tmp/second-watch-fixture/log".to_owned(),
        receipt_custody_root: "/tmp/second-watch-fixture/receipts".to_owned(),
        maximum_event_bytes: 65_536,
        maximum_receipt_bytes: 131_072,
        adapter_timeout_seconds: 60,
        closeout_policy: "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn capacity_requirement(
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
    policy: &CapacityPolicyV1,
) -> ForemanCapacityRequirementV1 {
    let mut value = ForemanCapacityRequirementV1 {
        schema: FOREMAN_CAPACITY_REQUIREMENT_SCHEMA_V1.to_owned(),
        capacity_requirement_digest: placeholder(),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        profile_digest: profile.profile_digest.clone(),
        run_id: admission.run_id.clone(),
        policy_id: policy.policy_id.clone(),
        provider_id: "qualification-provider".to_owned(),
        model_cost_classes: BTreeMap::from([("bounded".to_owned(), CapacityCostClassV1::Cheap)]),
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn availability_policy() -> ExecutionAvailabilityPolicyV1 {
    let mut value = ExecutionAvailabilityPolicyV1 {
        schema: EXECUTION_AVAILABILITY_POLICY_SCHEMA_V1.to_owned(),
        policy_digest: placeholder(),
        policy_id: "second-watch-availability".to_owned(),
        maximum_dispatch_occurrences_per_attempt: 3,
        backoff_seconds: vec![5, 10, 20],
        maximum_total_deferral_seconds: 60,
        parked_resource_lock_policy: ParkedResourceLockPolicyV1::ReleaseAndReacquire,
        provider_capacity_released_while_parked: true,
        reconcile_indeterminate: true,
        allow_ordered_model_fallback: true,
        automatic_semantic_retry: false,
        approval_response_authorized: false,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn availability_requirement(
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
    policy: &ExecutionAvailabilityPolicyV1,
) -> ForemanExecutionAvailabilityRequirementV1 {
    let adapter = &profile.adapters[HOLDING_QUALIFICATION_PRODUCER_ID];
    let selections = packet
        .work_items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                vec![ProviderModelSelectionV1 {
                    provider_id: "qualification-provider".to_owned(),
                    model_id: "deterministic-model".to_owned(),
                    model_class: "bounded".to_owned(),
                }],
            )
        })
        .collect();
    let mut value = ForemanExecutionAvailabilityRequirementV1 {
        schema: FOREMAN_EXECUTION_AVAILABILITY_REQUIREMENT_SCHEMA_V1.to_owned(),
        requirement_digest: placeholder(),
        packet_digest: packet.packet_digest.clone(),
        admission_digest: admission.admission_digest.clone(),
        profile_digest: profile.profile_digest.clone(),
        run_id: admission.run_id.clone(),
        adapter_id: adapter.adapter_id.clone(),
        adapter_protocol: adapter.protocol.clone(),
        adapter_version: adapter.adapter_version.clone(),
        adapter_executable_identity: adapter.executable_identity.clone(),
        owner_pins: ProviderAdmissionOwnerPinsV1::accepted(),
        policy_id: policy.policy_id.clone(),
        policy_digest: policy.policy_digest.clone(),
        work_item_model_selections: selections,
        admitted_at: admission.admitted_at,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

struct Fixture {
    plan: SelfHostedForemanBootstrapV1,
    packet: NightshiftPacketV1,
    admission: ForemanAdmissionV1,
    profile: ExecutionProfileV2,
    capacity_requirement: ForemanCapacityRequirementV1,
    capacity_policy: CapacityPolicyV1,
    availability_requirement: ForemanExecutionAvailabilityRequirementV1,
    availability_policy: ExecutionAvailabilityPolicyV1,
}

impl Fixture {
    fn validate(&self) -> Result<(), ContractError> {
        self.plan.validate_graph(
            &canonical(&self.packet),
            &canonical(&self.admission),
            &canonical(&self.profile),
            &canonical(&self.capacity_requirement),
            &canonical(&self.capacity_policy),
            &canonical(&self.availability_requirement),
            &canonical(&self.availability_policy),
        )
    }
}

fn reseal_adapter_graph(value: &mut Fixture, adapter: AdapterRegistrationV2) {
    let adapter_id = adapter.adapter_id.clone();
    value.admission.allowed_adapter_ids = vec![adapter_id.clone()];
    value.admission.seal().unwrap();

    value.profile.admission_digest = value.admission.admission_digest.clone();
    for work in value.profile.work_items.values_mut() {
        work.adapter_id = adapter_id.clone();
    }
    value.profile.adapters = BTreeMap::from([(adapter_id.clone(), adapter.clone())]);
    value.profile.seal().unwrap();

    value.capacity_requirement.admission_digest = value.admission.admission_digest.clone();
    value.capacity_requirement.profile_digest = value.profile.profile_digest.clone();
    value.capacity_requirement.seal().unwrap();

    value.availability_requirement.admission_digest = value.admission.admission_digest.clone();
    value.availability_requirement.profile_digest = value.profile.profile_digest.clone();
    value.availability_requirement.adapter_id = adapter_id;
    value.availability_requirement.adapter_protocol = adapter.protocol;
    value.availability_requirement.adapter_version = adapter.adapter_version;
    value.availability_requirement.adapter_executable_identity = adapter.executable_identity;
    value.availability_requirement.seal().unwrap();

    value.plan.admission_digest = value.admission.admission_digest.clone();
    value.plan.profile_digest = value.profile.profile_digest.clone();
    value.plan.capacity_requirement_digest = value
        .capacity_requirement
        .capacity_requirement_digest
        .clone();
    value.plan.execution_availability_requirement_digest =
        value.availability_requirement.requirement_digest.clone();
    value.plan.seal().unwrap();
}

fn reseal_admission_graph(value: &mut Fixture) {
    value.admission.seal().unwrap();

    value.profile.admission_digest = value.admission.admission_digest.clone();
    value.profile.seal().unwrap();

    value.capacity_requirement.admission_digest = value.admission.admission_digest.clone();
    value.capacity_requirement.profile_digest = value.profile.profile_digest.clone();
    value.capacity_requirement.seal().unwrap();

    value.availability_requirement.admission_digest = value.admission.admission_digest.clone();
    value.availability_requirement.profile_digest = value.profile.profile_digest.clone();
    value.availability_requirement.seal().unwrap();

    value.plan.admission_digest = value.admission.admission_digest.clone();
    value.plan.profile_digest = value.profile.profile_digest.clone();
    value.plan.capacity_requirement_digest = value
        .capacity_requirement
        .capacity_requirement_digest
        .clone();
    value.plan.execution_availability_requirement_digest =
        value.availability_requirement.requirement_digest.clone();
    value.plan.seal().unwrap();
}

fn fixture() -> Fixture {
    let packet = packet();
    let admission = admission(&packet);
    let capacity_policy = capacity_policy();
    let profile = profile(&packet, &admission, &capacity_policy);
    let capacity_requirement =
        capacity_requirement(&packet, &admission, &profile, &capacity_policy);
    let availability_policy = availability_policy();
    let availability_requirement =
        availability_requirement(&packet, &admission, &profile, &availability_policy);
    let mut plan = SelfHostedForemanBootstrapV1 {
        schema: SELF_HOSTED_FOREMAN_BOOTSTRAP_SCHEMA_V1.to_owned(),
        bootstrap_digest: placeholder(),
        digest_preimage: SELF_HOSTED_FOREMAN_BOOTSTRAP_DIGEST_PREIMAGE_V1.to_owned(),
        campaign_codename: "SECOND-WATCH".to_owned(),
        canonical_slug: SECOND_WATCH_CANONICAL_SLUG.to_owned(),
        track: "nightshift-self-hosting".to_owned(),
        holding_result_head: SECOND_WATCH_HOLDING_RESULT_HEAD.to_owned(),
        holding_qualified_subject: SECOND_WATCH_HOLDING_QUALIFIED_SUBJECT.to_owned(),
        durable_roadmap_head: SECOND_WATCH_DURABLE_ROADMAP_HEAD.to_owned(),
        midnight_result_head: SECOND_WATCH_MIDNIGHT_RESULT_HEAD.to_owned(),
        silicon_result_head: SECOND_WATCH_SILICON_RESULT_HEAD.to_owned(),
        codex_owner_head: ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD.to_owned(),
        switchyard_owner_head: ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_OWNER_HEAD.to_owned(),
        bootstrap_occurrence_id: "bootstrap-second-watch-1".to_owned(),
        run_id: admission.run_id.clone(),
        packet_id: packet.packet_id.clone(),
        packet_digest: packet.packet_digest.clone(),
        predecessor_v2_packet_digest: SECOND_WATCH_PREDECESSOR_V2_PACKET_DIGEST.to_owned(),
        admission_digest: admission.admission_digest.clone(),
        profile_digest: profile.profile_digest.clone(),
        capacity_requirement_digest: capacity_requirement.capacity_requirement_digest.clone(),
        capacity_policy_digest: capacity_policy.policy_digest.clone(),
        execution_availability_requirement_digest: availability_requirement
            .requirement_digest
            .clone(),
        execution_availability_policy_digest: availability_policy.policy_digest.clone(),
        local_runtime_identity: admission.local_runtime_identity.clone(),
        evaluated_at: time(1),
        expected_work_item_count: packet.work_items.len().try_into().unwrap(),
        initially_runnable_lane_count: 3,
        presentation_only_question_work_item_id: "question".to_owned(),
        maximum_driver_steps: 100,
        maximum_wall_seconds: 600,
        bootstrap_depth: 0,
        parent_bootstrap_occurrence_id: None,
        scheduler_owner: "NIGHTSHIFT_DURABLE_FOREMAN".to_owned(),
        worker_adapter_mode: "CAMPAIGN_QUALIFICATION_DETERMINISTIC_FAKE".to_owned(),
        wake_source_policy: "QUALIFIED_LOCAL_REEVALUATION_NO_EVIDENCE_OR_AUTHORITY".to_owned(),
        closeout_policy: "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED".to_owned(),
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
        target_effects_authorized: false,
        approval_response_authorized: false,
        protected_effect_authorized: false,
        semantic_retry_authorized: false,
        bootstrap_may_nest: false,
        worker_may_invoke_bootstrap: false,
        outer_conversation_scheduler: false,
        timer_or_service_activation_authorized: false,
        production_activation_authorized: false,
        aggregate_result_created: false,
    };
    plan.seal().unwrap();
    Fixture {
        plan,
        packet,
        admission,
        profile,
        capacity_requirement,
        capacity_policy,
        availability_requirement,
        availability_policy,
    }
}

#[test]
fn closed_bootstrap_plan_and_exact_graph_validate() {
    let value = fixture();
    value.validate().unwrap();
    let adapter = value.profile.adapters.values().next().unwrap();
    assert_eq!(adapter.adapter_id, HOLDING_QUALIFICATION_PRODUCER_ID);
    assert_eq!(
        adapter.protocol,
        DETERMINISTIC_PROVIDER_ADMISSION_EVIDENCE_SCHEMA_V1
    );
    assert_eq!(
        adapter.adapter_version,
        HOLDING_QUALIFICATION_PRODUCER_VERSION
    );
    assert_eq!(
        adapter.executable_identity,
        HOLDING_QUALIFICATION_EXECUTABLE_SHA256
    );
    assert!(adapter.bounded_arguments.is_empty());
    let raw = canonical(&value.plan);
    let reopened = SelfHostedForemanBootstrapV1::from_slice(&raw).unwrap();
    assert_eq!(reopened, value.plan);
    reopened.validate().unwrap();
}

#[test]
fn authority_recursion_and_predecessor_substitutions_refuse() {
    for mutate in [
        |plan: &mut SelfHostedForemanBootstrapV1| plan.approval_response_authorized = true,
        |plan: &mut SelfHostedForemanBootstrapV1| plan.bootstrap_may_nest = true,
        |plan: &mut SelfHostedForemanBootstrapV1| plan.bootstrap_depth = 1,
        |plan: &mut SelfHostedForemanBootstrapV1| plan.holding_result_head = "f".repeat(40),
        |plan: &mut SelfHostedForemanBootstrapV1| {
            plan.packet_digest = SECOND_WATCH_PREDECESSOR_V2_PACKET_DIGEST.to_owned()
        },
    ] {
        let mut value = fixture().plan;
        mutate(&mut value);
        value.bootstrap_digest = placeholder();
        assert!(value.seal().is_err());
    }
}

#[test]
fn graph_substitution_and_topology_cases_fail_closed() {
    let mut wrong_profile = fixture();
    wrong_profile.plan.profile_digest = format!("sha256:{}", "f".repeat(64));
    wrong_profile.plan.seal().unwrap();
    assert!(wrong_profile.validate().is_err());

    let mut wrong_policy = fixture();
    wrong_policy.capacity_requirement.policy_id = "substituted-policy".to_owned();
    wrong_policy.capacity_requirement.seal().unwrap();
    assert!(wrong_policy.validate().is_err());

    let mut wrong_count = fixture();
    wrong_count.plan.initially_runnable_lane_count = 2;
    wrong_count.plan.seal().unwrap();
    assert!(wrong_count.validate().is_err());

    let mut recursive = fixture();
    recursive.packet.work_items[0].campaign.canonical_slug = SECOND_WATCH_CANONICAL_SLUG.to_owned();
    recursive.packet.seal().unwrap();
    assert!(recursive.validate().is_err());

    let mut wrong_provider = fixture();
    for selections in wrong_provider
        .availability_requirement
        .work_item_model_selections
        .values_mut()
    {
        for selection in selections {
            selection.provider_id = "substituted-provider".to_owned();
        }
    }
    wrong_provider.availability_requirement.seal().unwrap();
    wrong_provider
        .plan
        .execution_availability_requirement_digest = wrong_provider
        .availability_requirement
        .requirement_digest
        .clone();
    wrong_provider.plan.seal().unwrap();
    assert!(wrong_provider.validate().is_err());

    let mut wrong_admission_time = fixture();
    wrong_admission_time.availability_requirement.admitted_at = time(2);
    wrong_admission_time
        .availability_requirement
        .seal()
        .unwrap();
    wrong_admission_time
        .plan
        .execution_availability_requirement_digest = wrong_admission_time
        .availability_requirement
        .requirement_digest
        .clone();
    wrong_admission_time.plan.seal().unwrap();
    assert!(wrong_admission_time.validate().is_err());

    let exact = fixture().profile.adapters.values().next().unwrap().clone();
    let mut substituted_adapters = Vec::new();
    let mut substituted = exact.clone();
    substituted.adapter_id = "substituted-adapter".to_owned();
    substituted_adapters.push(substituted);
    let mut substituted = exact.clone();
    substituted.protocol = "substituted.protocol/v1".to_owned();
    substituted_adapters.push(substituted);
    let mut substituted = exact.clone();
    substituted.adapter_version = "v2".to_owned();
    substituted_adapters.push(substituted);
    let mut substituted = exact.clone();
    substituted.executable_identity = format!("sha256:{}", "f".repeat(64));
    substituted_adapters.push(substituted);
    let mut substituted = exact;
    substituted.bounded_arguments = vec!["--substituted".to_owned()];
    substituted_adapters.push(substituted);

    for adapter in substituted_adapters {
        let mut coherent = fixture();
        reseal_adapter_graph(&mut coherent, adapter);
        assert!(coherent.validate().is_err());
    }
}

#[test]
fn noncanonical_time_and_unknown_fields_refuse() {
    let value = fixture();
    let mut json = serde_json::to_value(&value.plan).unwrap();
    json["evaluated_at"] = serde_json::json!("2026-08-31T08:00:01-04:00");
    assert!(SelfHostedForemanBootstrapV1::from_slice(&serde_json::to_vec(&json).unwrap()).is_err());
    json = serde_json::to_value(&value.plan).unwrap();
    json["approve"] = serde_json::json!(true);
    assert!(SelfHostedForemanBootstrapV1::from_slice(&serde_json::to_vec(&json).unwrap()).is_err());

    let mut noncanonical_plan = canonical(&value.plan);
    noncanonical_plan.push(b'\n');
    assert!(SelfHostedForemanBootstrapV1::from_slice(&noncanonical_plan).is_err());

    let noncanonical_packet = serde_json::to_vec_pretty(&value.packet).unwrap();
    assert!(value
        .plan
        .validate_graph(
            &noncanonical_packet,
            &canonical(&value.admission),
            &canonical(&value.profile),
            &canonical(&value.capacity_requirement),
            &canonical(&value.capacity_policy),
            &canonical(&value.availability_requirement),
            &canonical(&value.availability_policy),
        )
        .is_err());
}
fn fixture_inputs(value: &Fixture) -> [Vec<u8>; 8] {
    [
        canonical(&value.plan),
        canonical(&value.packet),
        canonical(&value.admission),
        canonical(&value.profile),
        canonical(&value.capacity_requirement),
        canonical(&value.capacity_policy),
        canonical(&value.availability_requirement),
        canonical(&value.availability_policy),
    ]
}

fn admit_fixture(path: &std::path::Path, value: &Fixture) -> String {
    let bytes = fixture_inputs(value);
    ForemanStore::admit_self_hosted_at_path(
        path,
        SelfHostedBootstrapInputsV1 {
            bootstrap_bytes: &bytes[0],
            packet_bytes: &bytes[1],
            admission_bytes: &bytes[2],
            profile_bytes: &bytes[3],
            capacity_requirement_bytes: &bytes[4],
            capacity_policy_bytes: &bytes[5],
            execution_availability_requirement_bytes: &bytes[6],
            execution_availability_policy_bytes: &bytes[7],
        },
    )
    .unwrap()
}

#[test]
fn driver_step_contract_is_closed_bounded_and_non_authorizing() {
    let value = fixture();
    let mut step = SelfHostedForemanDriverStepV1 {
        schema: SELF_HOSTED_FOREMAN_DRIVER_STEP_SCHEMA_V1.to_owned(),
        step_digest: placeholder(),
        bootstrap_digest: value.plan.bootstrap_digest.clone(),
        bootstrap_occurrence_id: value.plan.bootstrap_occurrence_id.clone(),
        run_id: value.plan.run_id.clone(),
        step_ordinal: 1,
        scheduler_process_occurrence_id: "scheduler-process-1".to_owned(),
        observed_projection_digest: format!("sha256:{}", "1".repeat(64)),
        disposition: SelfHostedDriverDispositionV1::ReadyWorkPresent,
        recorded_at: time(2),
        worker_dispatch_authorized: false,
        approval_response_authorized: false,
        protected_effect_authorized: false,
        semantic_retry_authorized: false,
        aggregate_result_created: false,
    };
    step.seal().unwrap();
    assert_eq!(
        SelfHostedForemanDriverStepV1::from_slice(&canonical(&step)).unwrap(),
        step
    );

    for ordinal in [0, 1_000_001] {
        let mut substituted = step.clone();
        substituted.step_ordinal = ordinal;
        substituted.step_digest = placeholder();
        assert!(substituted.seal().is_err());
    }
    let mut widened = step.clone();
    widened.worker_dispatch_authorized = true;
    widened.step_digest = placeholder();
    assert!(widened.seal().is_err());
}

#[test]
fn self_hosted_admission_preflights_before_store_creation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid.sqlite3");
    let mut value = fixture();
    value.plan.profile_digest = format!("sha256:{}", "f".repeat(64));
    value.plan.seal().unwrap();
    let bytes = fixture_inputs(&value);
    assert!(ForemanStore::admit_self_hosted_at_path(
        &path,
        SelfHostedBootstrapInputsV1 {
            bootstrap_bytes: &bytes[0],
            packet_bytes: &bytes[1],
            admission_bytes: &bytes[2],
            profile_bytes: &bytes[3],
            capacity_requirement_bytes: &bytes[4],
            capacity_policy_bytes: &bytes[5],
            execution_availability_requirement_bytes: &bytes[6],
            execution_availability_policy_bytes: &bytes[7],
        },
    )
    .is_err());
    assert!(!path.exists());
}

#[test]
fn self_hosted_store_reopens_and_driver_step_is_idempotent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    assert_eq!(run_id, value.plan.run_id);

    let store = ForemanStore::open(&path).unwrap();
    assert!(store.self_hosted_bootstrap(&run_id).is_err());
    let first = store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-1",
            time(2),
        )
        .unwrap();
    assert_eq!(first.step_ordinal, 1);
    assert_eq!(
        first.disposition,
        SelfHostedDriverDispositionV1::ReadyWorkPresent
    );
    assert!(!first.worker_dispatch_authorized);

    let duplicate = store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-losing-writer",
            time(3),
        )
        .unwrap();
    assert_eq!(duplicate, first);
    assert!(store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            3,
            "scheduler-process-3",
            time(3),
        )
        .is_err());

    drop(store);
    let casework_snapshot = read_only_run_snapshot(&path, &run_id).unwrap();
    assert_eq!(casework_snapshot.run_id, run_id);
    assert_eq!(casework_snapshot.packet_bytes, canonical(&value.packet));
    let reopened = ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&run_id)
        .unwrap();
    assert_eq!(reopened.bootstrap, value.plan);
    assert_eq!(reopened.bootstrap_bytes, canonical(&value.plan));
    assert_eq!(
        reopened.capacity_policy_bytes,
        canonical(&value.capacity_policy)
    );
    assert_eq!(reopened.steps, vec![first.clone()]);
    assert_eq!(reopened.step_bytes, vec![canonical(&first)]);
}
#[test]
fn concurrent_driver_writers_converge_on_one_custody_record() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut handles = Vec::new();
    for process in ["scheduler-process-a", "scheduler-process-b"] {
        let path = path.clone();
        let run_id = run_id.clone();
        let digest = value.plan.bootstrap_digest.clone();
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            let store = ForemanStore::open(path).unwrap();
            barrier.wait();
            store
                .advance_self_hosted_driver(&run_id, &digest, 1, process, time(2))
                .unwrap()
        }));
    }
    let first = handles.remove(0).join().unwrap();
    let second = handles.remove(0).join().unwrap();
    assert_eq!(first, second);
    let reopened = ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&run_id)
        .unwrap();
    assert_eq!(reopened.steps, vec![first]);
}

#[test]
fn failed_driver_append_rolls_back_and_restart_recovers_exact_ordinal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    let fixture_connection = rusqlite::Connection::open(&path).unwrap();
    fixture_connection
        .execute_batch(
            "CREATE TRIGGER qualification_driver_insert_refusal
             BEFORE INSERT ON self_hosted_driver_steps
             BEGIN SELECT RAISE(ABORT, 'qualification fixture'); END;",
        )
        .unwrap();
    let store = ForemanStore::open(&path).unwrap();
    assert!(store.self_hosted_bootstrap(&run_id).is_err());
    assert!(store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-crash",
            time(2),
        )
        .is_err());
    assert!(ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&run_id)
        .unwrap()
        .steps
        .is_empty());
    fixture_connection
        .execute_batch("DROP TRIGGER qualification_driver_insert_refusal;")
        .unwrap();
    drop(store);

    let restarted = ForemanStore::open(&path).unwrap();
    let retained = restarted
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-restarted",
            time(2),
        )
        .unwrap();
    assert_eq!(retained.step_ordinal, 1);
    assert_eq!(
        ForemanStore::open_read_only(&path)
            .unwrap()
            .self_hosted_bootstrap(&run_id)
            .unwrap()
            .steps,
        vec![retained]
    );
}

#[test]
fn append_only_triggers_and_content_mutation_refuse_before_next_step() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    let store = ForemanStore::open(&path).unwrap();
    assert!(store.self_hosted_bootstrap(&run_id).is_err());
    let first = store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-1",
            time(2),
        )
        .unwrap();
    drop(store);

    let fixture_connection = rusqlite::Connection::open(&path).unwrap();
    assert!(fixture_connection
        .execute(
            "UPDATE self_hosted_driver_steps SET recorded_at = recorded_at
             WHERE run_id = ?1 AND step_ordinal = 1",
            [&run_id],
        )
        .is_err());

    fixture_connection
        .execute_batch("DROP TRIGGER self_hosted_driver_steps_no_update;")
        .unwrap();
    let mut substituted = first;
    substituted.scheduler_process_occurrence_id = "scheduler-process-substituted".to_owned();
    substituted.step_digest = placeholder();
    substituted.seal().unwrap();
    fixture_connection
        .execute(
            "UPDATE self_hosted_driver_steps SET raw_bytes = ?1
             WHERE run_id = ?2 AND step_ordinal = 1",
            rusqlite::params![canonical(&substituted), run_id],
        )
        .unwrap();
    drop(fixture_connection);

    assert!(ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&value.plan.run_id)
        .is_err());
    let writable = ForemanStore::open(&path).unwrap();
    assert!(writable
        .advance_self_hosted_driver(
            &value.plan.run_id,
            &value.plan.bootstrap_digest,
            2,
            "scheduler-process-2",
            time(3),
        )
        .is_err());
    let fixture_connection = rusqlite::Connection::open(&path).unwrap();
    let count: i64 = fixture_connection
        .query_row(
            "SELECT COUNT(*) FROM self_hosted_driver_steps WHERE run_id = ?1",
            [&value.plan.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
#[test]
fn oversized_driver_history_refuses_at_metadata_preflight() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    let store = ForemanStore::open(&path).unwrap();
    store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-1",
            time(2),
        )
        .unwrap();
    drop(store);

    let fixture_connection = rusqlite::Connection::open(&path).unwrap();
    fixture_connection
        .execute_batch("DROP TRIGGER self_hosted_driver_steps_no_update;")
        .unwrap();
    fixture_connection
        .execute(
            "UPDATE self_hosted_driver_steps SET raw_bytes = zeroblob(?1)
             WHERE run_id = ?2 AND step_ordinal = 1",
            rusqlite::params![i64::from(16 * 1024 + 1), run_id],
        )
        .unwrap();
    drop(fixture_connection);

    let error = ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&value.plan.run_id)
        .unwrap_err();
    assert!(matches!(
        error,
        ForemanError::InputTooLarge("self-hosted driver history")
    ));
}
#[test]
fn packet_budget_caps_admission_concurrency_before_store_creation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-concurrency.sqlite3");
    let mut value = fixture();
    assert_eq!(
        value
            .packet
            .worker_budget
            .maximum_concurrent_mutating_workers,
        2
    );
    value.admission.maximum_concurrent_workers = 3;
    reseal_admission_graph(&mut value);
    value.admission.validate().unwrap();
    value.profile.validate().unwrap();
    value.capacity_requirement.validate().unwrap();
    value.availability_requirement.validate().unwrap();
    assert!(value.validate().is_err());

    let bytes = fixture_inputs(&value);
    assert!(ForemanStore::admit_self_hosted_at_path(
        &path,
        SelfHostedBootstrapInputsV1 {
            bootstrap_bytes: &bytes[0],
            packet_bytes: &bytes[1],
            admission_bytes: &bytes[2],
            profile_bytes: &bytes[3],
            capacity_requirement_bytes: &bytes[4],
            capacity_policy_bytes: &bytes[5],
            execution_availability_requirement_bytes: &bytes[6],
            execution_availability_policy_bytes: &bytes[7],
        },
    )
    .is_err());
    assert!(!path.exists());
}

#[test]
fn self_hosted_query_reopens_every_authoritative_run_column() {
    let substitutions = [
        format!(
            "UPDATE runs SET packet_digest = 'sha256:{}' WHERE run_id = 'second-watch-run'",
            "f".repeat(64)
        ),
        "UPDATE runs SET admitted_at = '2026-08-31T12:00:01Z'
         WHERE run_id = 'second-watch-run'"
            .to_owned(),
        "UPDATE runs SET maximum_concurrent_workers = 1
         WHERE run_id = 'second-watch-run'"
            .to_owned(),
    ];
    for statement in substitutions {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("foreman.sqlite3");
        let value = fixture();
        let run_id = admit_fixture(&path, &value);
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("DROP TRIGGER runs_no_update;")
            .unwrap();
        connection.execute(&statement, []).unwrap();
        drop(connection);

        assert!(ForemanStore::open_read_only(&path)
            .unwrap()
            .self_hosted_bootstrap(&run_id)
            .is_err());
    }
}

#[test]
fn driver_time_is_nondecreasing_in_mutation_restart_and_replay() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("foreman.sqlite3");
    let value = fixture();
    let run_id = admit_fixture(&path, &value);
    let store = ForemanStore::open(&path).unwrap();
    let first = store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            1,
            "scheduler-process-1",
            time(3),
        )
        .unwrap();
    assert!(store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            2,
            "scheduler-process-time-inversion",
            time(2),
        )
        .is_err());
    let second = store
        .advance_self_hosted_driver(
            &run_id,
            &value.plan.bootstrap_digest,
            2,
            "scheduler-process-2",
            time(4),
        )
        .unwrap();
    drop(store);

    let reopened = ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&run_id)
        .unwrap();
    assert_eq!(reopened.steps, vec![first, second.clone()]);

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch("DROP TRIGGER self_hosted_driver_steps_no_update;")
        .unwrap();
    let mut substituted = second;
    substituted.recorded_at = time(2);
    substituted.step_digest = placeholder();
    substituted.seal().unwrap();
    connection
        .execute(
            "UPDATE self_hosted_driver_steps
             SET step_digest = ?1, recorded_at = ?2, raw_bytes = ?3
             WHERE run_id = ?4 AND step_ordinal = 2",
            rusqlite::params![
                substituted.step_digest,
                substituted.recorded_at.to_rfc3339(),
                canonical(&substituted),
                run_id
            ],
        )
        .unwrap();
    drop(connection);

    assert!(ForemanStore::open_read_only(&path)
        .unwrap()
        .self_hosted_bootstrap(&value.plan.run_id)
        .is_err());
    assert!(ForemanStore::open(&path)
        .unwrap()
        .advance_self_hosted_driver(
            &value.plan.run_id,
            &value.plan.bootstrap_digest,
            3,
            "scheduler-process-after-inversion",
            time(5),
        )
        .is_err());
}

#[test]
fn driver_terminal_dispositions_close_append_and_replay() {
    let bound_directory = tempfile::tempdir().unwrap();
    let bound_path = bound_directory.path().join("bound.sqlite3");
    let mut bounded = fixture();
    bounded.plan.maximum_wall_seconds = 1;
    bounded.plan.seal().unwrap();
    let bound_run_id = admit_fixture(&bound_path, &bounded);
    let bound_store = ForemanStore::open(&bound_path).unwrap();
    let bound = bound_store
        .advance_self_hosted_driver(
            &bound_run_id,
            &bounded.plan.bootstrap_digest,
            1,
            "scheduler-process-bound",
            time(2),
        )
        .unwrap();
    assert_eq!(
        bound.disposition,
        SelfHostedDriverDispositionV1::BoundReached
    );
    assert!(bound_store
        .advance_self_hosted_driver(
            &bound_run_id,
            &bounded.plan.bootstrap_digest,
            2,
            "scheduler-process-after-bound",
            time(3),
        )
        .is_err());
    drop(bound_store);
    assert_eq!(
        ForemanStore::open_read_only(&bound_path)
            .unwrap()
            .self_hosted_bootstrap(&bound_run_id)
            .unwrap()
            .steps,
        vec![bound]
    );

    let terminal_directory = tempfile::tempdir().unwrap();
    let terminal_path = terminal_directory.path().join("terminal.sqlite3");
    let terminal_fixture = fixture();
    let terminal_run_id = admit_fixture(&terminal_path, &terminal_fixture);
    let terminal_store = ForemanStore::open(&terminal_path).unwrap();
    let mut first = terminal_store
        .advance_self_hosted_driver(
            &terminal_run_id,
            &terminal_fixture.plan.bootstrap_digest,
            1,
            "scheduler-process-1",
            time(2),
        )
        .unwrap();
    terminal_store
        .advance_self_hosted_driver(
            &terminal_run_id,
            &terminal_fixture.plan.bootstrap_digest,
            2,
            "scheduler-process-2",
            time(3),
        )
        .unwrap();
    drop(terminal_store);

    first.disposition = SelfHostedDriverDispositionV1::AllItemsExplicitTerminal;
    first.step_digest = placeholder();
    first.seal().unwrap();
    let connection = rusqlite::Connection::open(&terminal_path).unwrap();
    connection
        .execute_batch("DROP TRIGGER self_hosted_driver_steps_no_update;")
        .unwrap();
    connection
        .execute(
            "UPDATE self_hosted_driver_steps
             SET step_digest = ?1, raw_bytes = ?2
             WHERE run_id = ?3 AND step_ordinal = 1",
            rusqlite::params![first.step_digest, canonical(&first), terminal_run_id],
        )
        .unwrap();
    drop(connection);
    assert!(ForemanStore::open_read_only(&terminal_path)
        .unwrap()
        .self_hosted_bootstrap(&terminal_fixture.plan.run_id)
        .is_err());
}
