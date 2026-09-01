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
        allowed_adapter_ids: vec!["second-watch-fake-adapter".to_owned()],
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
                    adapter_id: "second-watch-fake-adapter".to_owned(),
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
            "second-watch-fake-adapter".to_owned(),
            AdapterRegistrationV2 {
                adapter_id: "second-watch-fake-adapter".to_owned(),
                protocol: "second-watch.deterministic-fake/v1".to_owned(),
                adapter_version: "v1".to_owned(),
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
    let adapter = &profile.adapters["second-watch-fake-adapter"];
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
}
