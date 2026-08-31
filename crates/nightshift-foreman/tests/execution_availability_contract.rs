use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use nightshift_foreman::*;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

fn time(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}

fn placeholder() -> String {
    format!("sha256:{}", "0".repeat(64))
}

fn seal_value(mut value: Value, field: &str, domain: &[u8]) -> Value {
    value[field] = Value::String(placeholder());
    let mut basis = value.clone();
    basis.as_object_mut().unwrap().remove(field);
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(serde_jcs::to_vec(&basis).unwrap());
    value[field] = Value::String(format!("sha256:{:x}", hash.finalize()));
    value
}

fn canonical<T: Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).unwrap()
}

fn mapper_raw(value: Value) -> Value {
    let mut bytes = serde_json::to_vec(&value).unwrap();
    bytes.push(b'\n');
    json!({
        "representation": "EXACT_WIRE_BYTES_INCLUDING_LINE_TERMINATOR",
        "byte_length": bytes.len(),
        "sha256": format!("sha256:{:x}", Sha256::digest(&bytes)),
        "encoding": "hex",
        "bytes_hex": hex::encode(bytes),
    })
}

fn reseal_mapper_snapshot(mut snapshot: Value) -> Value {
    for record in snapshot["records"].as_array_mut().unwrap() {
        *record = seal_value(
            record.clone(),
            "evidence_digest",
            b"switchyard.codex-provider-admission-evidence.digest/v1\0",
        );
    }
    seal_value(
        snapshot,
        "snapshot_digest",
        b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
    )
}

type SnapshotMutation = (&'static str, fn(&mut Value));

fn policy() -> ExecutionAvailabilityPolicyV1 {
    let mut value = ExecutionAvailabilityPolicyV1 {
        schema: EXECUTION_AVAILABILITY_POLICY_SCHEMA_V1.to_owned(),
        policy_digest: placeholder(),
        policy_id: "holding-policy".to_owned(),
        maximum_dispatch_occurrences_per_attempt: 4,
        backoff_seconds: vec![5, 10, 20, 40],
        maximum_total_deferral_seconds: 600,
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

fn requirement(
    policy: &ExecutionAvailabilityPolicyV1,
) -> ForemanExecutionAvailabilityRequirementV1 {
    let mut selections = BTreeMap::new();
    selections.insert(
        "WORK-A".to_owned(),
        vec![
            ProviderModelSelectionV1 {
                provider_id: "openai".to_owned(),
                model_id: "gpt-5.6-sol".to_owned(),
                model_class: "large".to_owned(),
            },
            ProviderModelSelectionV1 {
                provider_id: "openai".to_owned(),
                model_id: "gpt-5.6-terra".to_owned(),
                model_class: "large".to_owned(),
            },
        ],
    );
    let mut value = ForemanExecutionAvailabilityRequirementV1 {
        schema: FOREMAN_EXECUTION_AVAILABILITY_REQUIREMENT_SCHEMA_V1.to_owned(),
        requirement_digest: placeholder(),
        packet_digest: placeholder(),
        admission_digest: placeholder(),
        profile_digest: placeholder(),
        run_id: "run-holding".to_owned(),
        adapter_id: "switchyard-codex".to_owned(),
        adapter_protocol: "switchyard.codex-app-server/v2".to_owned(),
        adapter_version: "2.0.0".to_owned(),
        adapter_executable_identity: placeholder(),
        owner_pins: ProviderAdmissionOwnerPinsV1::accepted(),
        policy_id: policy.policy_id.clone(),
        policy_digest: policy.policy_digest.clone(),
        work_item_model_selections: selections,
        admitted_at: time("2026-08-31T12:00:00Z"),
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn dispatch(
    requirement: &ForemanExecutionAvailabilityRequirementV1,
) -> ProviderDispatchOccurrenceV1 {
    let mut value = ProviderDispatchOccurrenceV1 {
        schema: PROVIDER_DISPATCH_OCCURRENCE_SCHEMA_V1.to_owned(),
        dispatch_digest: placeholder(),
        requirement_digest: requirement.requirement_digest.clone(),
        policy_digest: requirement.policy_digest.clone(),
        packet_digest: requirement.packet_digest.clone(),
        run_id: requirement.run_id.clone(),
        work_item_id: "WORK-A".to_owned(),
        work_attempt_id: "attempt-holding-1".to_owned(),
        dispatch_occurrence_id: "dispatch-holding-1".to_owned(),
        dispatch_ordinal: 1,
        selected_model_ordinal: 0,
        selection: requirement.work_item_model_selections["WORK-A"][0].clone(),
        adapter_id: requirement.adapter_id.clone(),
        adapter_version: requirement.adapter_version.clone(),
        adapter_protocol: requirement.adapter_protocol.clone(),
        adapter_process_occurrence_id: "adapter-process-holding-1".to_owned(),
        app_server_session_identity: "fixture-estate-holding-1".to_owned(),
        worker_start_request_schema: "nightshift.worker-start-request/v3".to_owned(),
        worker_start_request_digest: placeholder(),
        worker_brief_digest: placeholder(),
        opened_at: time("2026-08-31T12:01:00Z"),
        internal_provider_retry_count: 0,
        provider_execution_id: None,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn parked_snapshot() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-parked-not-admitted.snapshot.v1.json"
    )).unwrap()
}

fn disposition(
    requirement: &ForemanExecutionAvailabilityRequirementV1,
    dispatch: &ProviderDispatchOccurrenceV1,
) -> ProviderAdmissionDispositionV1 {
    let snapshot = parked_snapshot();
    let bytes = canonical(&snapshot);
    let mut value = ProviderAdmissionDispositionV1 {
        schema: PROVIDER_ADMISSION_DISPOSITION_SCHEMA_V1.to_owned(),
        disposition_digest: placeholder(),
        dispatch_digest: dispatch.dispatch_digest.clone(),
        requirement_digest: requirement.requirement_digest.clone(),
        policy_digest: requirement.policy_digest.clone(),
        packet_digest: requirement.packet_digest.clone(),
        run_id: requirement.run_id.clone(),
        work_item_id: dispatch.work_item_id.clone(),
        work_attempt_id: dispatch.work_attempt_id.clone(),
        dispatch_occurrence_id: dispatch.dispatch_occurrence_id.clone(),
        provider_id: dispatch.selection.provider_id.clone(),
        model_id: dispatch.selection.model_id.clone(),
        provider_request_occurrence_id: "request-0".to_owned(),
        adapter_process_occurrence_id: dispatch.adapter_process_occurrence_id.clone(),
        app_server_session_identity: dispatch.app_server_session_identity.clone(),
        thread_id: "thread-holding-1".to_owned(),
        turn_id: "turn-holding-1".to_owned(),
        disposition: ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity,
        mechanism_state: ProviderMechanismStateV1::ParkedNotAdmitted,
        received_at: time("2026-08-31T12:01:02Z"),
        response_created: false,
        will_retry: false,
        acquisition_complete: true,
        provider_retry_after: Some(time("2026-08-31T12:01:07Z")),
        provider_execution: None,
        mapper_snapshot_schema: "switchyard.codex-provider-admission-snapshot/v1".to_owned(),
        mapper_snapshot_digest: snapshot["snapshot_digest"].as_str().unwrap().to_owned(),
        mapper_snapshot: ExactMapperSnapshotV1::from_bytes(&bytes).unwrap(),
        approval_response_sent: false,
        protected_effect_absent: true,
        authority_effect: "SCHEDULING_MECHANISM_EVIDENCE_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

#[test]
fn closed_contracts_bind_independent_quota_execution_and_dispatch_identities() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let disposition = disposition(&requirement, &dispatch);

    assert!(
        disposition.mapper_snapshot.byte_length as usize
            <= MAXIMUM_SWITCHYARD_MAPPER_SNAPSHOT_BYTES
    );
    ExecutionAvailabilityPolicyV1::from_slice(&canonical(&policy))
        .unwrap()
        .validate()
        .unwrap();
    ForemanExecutionAvailabilityRequirementV1::from_slice(&canonical(&requirement))
        .unwrap()
        .validate()
        .unwrap();
    ProviderDispatchOccurrenceV1::from_slice(&canonical(&dispatch))
        .unwrap()
        .validate()
        .unwrap();
    ProviderAdmissionDispositionV1::from_slice(&canonical(&disposition))
        .unwrap()
        .validate()
        .unwrap();
    assert_ne!(dispatch.work_attempt_id, dispatch.dispatch_occurrence_id);
    assert!(dispatch.provider_execution_id.is_none());
    assert!(disposition.disposition.permits_automatic_park());

    let mut observation = ExecutionAvailabilityObservationV1 {
        schema: EXECUTION_AVAILABILITY_OBSERVATION_SCHEMA_V1.to_owned(),
        observation_digest: placeholder(),
        provider_id: "openai".to_owned(),
        model_id: "gpt-5.6-sol".to_owned(),
        model_class: "large".to_owned(),
        observed_at: time("2026-08-31T08:00:00.100Z"),
        received_at: time("2026-08-31T12:01:02Z"),
        expires_at: time("2026-08-31T12:02:02Z"),
        state: ExecutionAvailabilityStateV1::ModelAtCapacity,
        source_identity: "switchyard:provider-admission".to_owned(),
        source_version: "v1".to_owned(),
        provider_retry_after: disposition.provider_retry_after,
        exact_evidence: Some(
            serde_json::from_value(parked_snapshot()["records"][1]["raw"].clone()).unwrap(),
        ),
        authority_effect: "SCHEDULING_MECHANISM_EVIDENCE_ONLY".to_owned(),
    };
    observation.seal().unwrap();
    assert!(observation.is_current_at(time("2026-08-31T12:01:30Z")));
    assert!(!observation.is_current_at(time("2026-08-31T12:02:02Z")));

    let mut deferred = DeferredProviderDispatchV1 {
        schema: DEFERRED_PROVIDER_DISPATCH_SCHEMA_V1.to_owned(),
        deferred_dispatch_digest: placeholder(),
        requirement_digest: requirement.requirement_digest.clone(),
        policy_digest: policy.policy_digest.clone(),
        disposition_digest: disposition.disposition_digest.clone(),
        packet_digest: requirement.packet_digest.clone(),
        run_id: requirement.run_id.clone(),
        work_item_id: dispatch.work_item_id.clone(),
        work_attempt_id: dispatch.work_attempt_id.clone(),
        last_dispatch_occurrence_id: dispatch.dispatch_occurrence_id.clone(),
        provider_id: dispatch.selection.provider_id.clone(),
        model_id: dispatch.selection.model_id.clone(),
        selected_model_ordinal: 0,
        remaining_model_ordinals: vec![1],
        refusal_received_at: disposition.received_at,
        wake_basis: DeferredWakeBasisV1::ProviderRetryAfter,
        backoff_ordinal: 0,
        backoff_seconds: 5,
        provider_retry_after: disposition.provider_retry_after,
        wake_at: disposition.provider_retry_after.unwrap(),
        parked_resource_lock_policy: policy.parked_resource_lock_policy,
        provider_capacity_released: true,
        semantic_retry: false,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    deferred.seal().unwrap();
    DeferredProviderDispatchV1::from_slice(&canonical(&deferred))
        .unwrap()
        .validate()
        .unwrap();
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &dispatch,
        &observation,
        &disposition,
        &[],
        Some(&deferred),
    )
    .unwrap();
}

#[test]
fn owner_pin_mapper_and_transition_substitutions_fail_closed() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let disposition = disposition(&requirement, &dispatch);

    let mut substituted_requirement = requirement.clone();
    substituted_requirement.owner_pins.codex_owner_head = "0".repeat(40);
    substituted_requirement.seal().unwrap_err();

    let mut retrying = dispatch.clone();
    retrying.internal_provider_retry_count = 1;
    retrying.seal().unwrap_err();

    let mut legacy_start = dispatch.clone();
    legacy_start.worker_start_request_schema = "nightshift.worker-start-request/v2".to_owned();
    legacy_start.seal().unwrap_err();

    let mut snapshot: Value =
        serde_json::from_slice(&disposition.mapper_snapshot.validate().unwrap()).unwrap();
    snapshot["binding"]["provider"] = json!("substituted");
    snapshot["binding"] = seal_value(
        snapshot["binding"].clone(),
        "binding_digest",
        b"switchyard.codex-provider-admission-binding.digest/v1\0",
    );
    let binding_digest = snapshot["binding"]["binding_digest"].clone();
    for record in snapshot["records"].as_array_mut().unwrap() {
        record["binding_digest"] = binding_digest.clone();
        *record = seal_value(
            record.clone(),
            "evidence_digest",
            b"switchyard.codex-provider-admission-evidence.digest/v1\0",
        );
    }
    snapshot = seal_value(
        snapshot,
        "snapshot_digest",
        b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
    );
    let mut substituted = disposition.clone();
    substituted.mapper_snapshot_digest = snapshot["snapshot_digest"].as_str().unwrap().to_owned();
    substituted.mapper_snapshot = ExactMapperSnapshotV1::from_bytes(&canonical(&snapshot)).unwrap();
    substituted.seal().unwrap_err();

    let mut unknown = ExecutionAvailabilityObservationV1 {
        schema: EXECUTION_AVAILABILITY_OBSERVATION_SCHEMA_V1.to_owned(),
        observation_digest: placeholder(),
        provider_id: "openai".to_owned(),
        model_id: "gpt-5.6-sol".to_owned(),
        model_class: "large".to_owned(),
        observed_at: time("2026-08-31T12:00:00Z"),
        received_at: time("2026-08-31T12:00:00Z"),
        expires_at: time("2026-08-31T12:01:00Z"),
        state: ExecutionAvailabilityStateV1::Unknown,
        source_identity: "nightshift:absence".to_owned(),
        source_version: "v1".to_owned(),
        provider_retry_after: None,
        exact_evidence: None,
        authority_effect: "SCHEDULING_MECHANISM_EVIDENCE_ONLY".to_owned(),
    };
    unknown.seal().unwrap();
    unknown.state = ExecutionAvailabilityStateV1::Available;
    unknown.seal().unwrap_err();

    let mut wrong_wake = DeferredProviderDispatchV1 {
        schema: DEFERRED_PROVIDER_DISPATCH_SCHEMA_V1.to_owned(),
        deferred_dispatch_digest: placeholder(),
        requirement_digest: requirement.requirement_digest,
        policy_digest: policy.policy_digest,
        disposition_digest: disposition.disposition_digest,
        packet_digest: requirement.packet_digest,
        run_id: requirement.run_id,
        work_item_id: dispatch.work_item_id,
        work_attempt_id: dispatch.work_attempt_id,
        last_dispatch_occurrence_id: dispatch.dispatch_occurrence_id,
        provider_id: dispatch.selection.provider_id,
        model_id: dispatch.selection.model_id,
        selected_model_ordinal: 0,
        remaining_model_ordinals: vec![1],
        refusal_received_at: disposition.received_at,
        wake_basis: DeferredWakeBasisV1::PolicyBackoff,
        backoff_ordinal: 0,
        backoff_seconds: 5,
        provider_retry_after: None,
        wake_at: disposition.received_at + Duration::seconds(6),
        parked_resource_lock_policy: policy.parked_resource_lock_policy,
        provider_capacity_released: true,
        semantic_retry: false,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    wrong_wake.seal().unwrap_err();
}

fn disposition_from_exact_snapshot(
    requirement: &ForemanExecutionAvailabilityRequirementV1,
    dispatch: &ProviderDispatchOccurrenceV1,
    raw: &[u8],
    received_at: DateTime<Utc>,
) -> ProviderAdmissionDispositionV1 {
    let snapshot: Value = serde_json::from_slice(raw).unwrap();
    let execution = snapshot["provider_execution_identity"]
        .as_object()
        .map(|identity| ProviderExecutionIdentityV1 {
            provider_id: identity["provider"].as_str().unwrap().to_owned(),
            model_id: identity["model"].as_str().unwrap().to_owned(),
            app_server_session_identity: identity["app_server_session_identity"]
                .as_str()
                .unwrap()
                .to_owned(),
            thread_id: identity["thread_id"].as_str().unwrap().to_owned(),
            turn_id: identity["turn_id"].as_str().unwrap().to_owned(),
            first_response_id: identity["first_response_id"].as_str().unwrap().to_owned(),
        });
    let disposition_kind = match snapshot["admission_disposition"].as_str().unwrap() {
        "EXECUTION_ADMITTED" => ProviderAdmissionDispositionKindV1::ExecutionAdmitted,
        "NOT_ADMITTED_MODEL_AT_CAPACITY" => {
            ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity
        }
        "ADMISSION_INDETERMINATE" => ProviderAdmissionDispositionKindV1::AdmissionIndeterminate,
        value => panic!("unexpected fixture disposition {value}"),
    };
    let mechanism_state = match snapshot["mechanism_state"].as_str().unwrap() {
        "PARKED_NOT_ADMITTED" => ProviderMechanismStateV1::ParkedNotAdmitted,
        "ADMISSION_INDETERMINATE" => ProviderMechanismStateV1::AdmissionIndeterminate,
        "POST_ADMISSION_INTERRUPTED" => ProviderMechanismStateV1::PostAdmissionInterrupted,
        "PROVIDER_COMPLETED" => ProviderMechanismStateV1::ProviderCompleted,
        value => panic!("unexpected fixture mechanism state {value}"),
    };
    let request_occurrence = snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|record| {
            record["normalized"]["request_occurrence_id"]
                .as_str()
                .map(str::to_owned)
        })
        .unwrap();
    let retry_after_ms = snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|record| record["normalized"]["retry_after_ms"].as_i64());
    let mut value = ProviderAdmissionDispositionV1 {
        schema: PROVIDER_ADMISSION_DISPOSITION_SCHEMA_V1.to_owned(),
        disposition_digest: placeholder(),
        dispatch_digest: dispatch.dispatch_digest.clone(),
        requirement_digest: requirement.requirement_digest.clone(),
        policy_digest: requirement.policy_digest.clone(),
        packet_digest: requirement.packet_digest.clone(),
        run_id: requirement.run_id.clone(),
        work_item_id: dispatch.work_item_id.clone(),
        work_attempt_id: dispatch.work_attempt_id.clone(),
        dispatch_occurrence_id: dispatch.dispatch_occurrence_id.clone(),
        provider_id: dispatch.selection.provider_id.clone(),
        model_id: dispatch.selection.model_id.clone(),
        provider_request_occurrence_id: request_occurrence,
        adapter_process_occurrence_id: dispatch.adapter_process_occurrence_id.clone(),
        app_server_session_identity: dispatch.app_server_session_identity.clone(),
        thread_id: "thread-holding-1".to_owned(),
        turn_id: "turn-holding-1".to_owned(),
        disposition: disposition_kind,
        mechanism_state,
        received_at,
        response_created: execution.is_some(),
        will_retry: false,
        acquisition_complete: snapshot["acquisition_cut"]["clean"]
            .as_bool()
            .unwrap_or(false),
        provider_retry_after: retry_after_ms.map(|ms| received_at + Duration::milliseconds(ms)),
        provider_execution: execution,
        mapper_snapshot_schema: "switchyard.codex-provider-admission-snapshot/v1".to_owned(),
        mapper_snapshot_digest: snapshot["snapshot_digest"].as_str().unwrap().to_owned(),
        mapper_snapshot: ExactMapperSnapshotV1::from_bytes(raw).unwrap(),
        approval_response_sent: false,
        protected_effect_absent: true,
        authority_effect: "SCHEDULING_MECHANISM_EVIDENCE_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn observation_for_disposition(
    disposition: &ProviderAdmissionDispositionV1,
) -> ExecutionAvailabilityObservationV1 {
    let raw = disposition.mapper_snapshot.validate().unwrap();
    let snapshot: Value = serde_json::from_slice(&raw).unwrap();
    let source_kind = match disposition.disposition {
        ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity => {
            "PROVIDER_ADMISSION_REFUSED"
        }
        ProviderAdmissionDispositionKindV1::ExecutionAdmitted => "PROVIDER_EXECUTION_STEP",
        ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => "ADMISSION_DISCREPANCY",
        _ => unreachable!(),
    };
    let exact_evidence = snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["kind"] == source_kind)
        .and_then(|record| {
            if record["raw"].is_null() {
                None
            } else {
                Some(serde_json::from_value(record["raw"].clone()).unwrap())
            }
        });
    let observed_at = snapshot["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["kind"] == source_kind)
        .and_then(|record| record["normalized"]["observed_at_ms"].as_i64())
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(disposition.received_at);
    let state = match disposition.disposition {
        ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity => {
            ExecutionAvailabilityStateV1::ModelAtCapacity
        }
        ProviderAdmissionDispositionKindV1::ExecutionAdmitted => {
            ExecutionAvailabilityStateV1::Available
        }
        ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => {
            ExecutionAvailabilityStateV1::Unknown
        }
        _ => unreachable!(),
    };
    let mut value = ExecutionAvailabilityObservationV1 {
        schema: EXECUTION_AVAILABILITY_OBSERVATION_SCHEMA_V1.to_owned(),
        observation_digest: placeholder(),
        provider_id: disposition.provider_id.clone(),
        model_id: disposition.model_id.clone(),
        model_class: "large".to_owned(),
        observed_at,
        received_at: disposition.received_at,
        expires_at: disposition.received_at + Duration::seconds(60),
        state,
        source_identity: "switchyard:provider-admission".to_owned(),
        source_version: "v1".to_owned(),
        provider_retry_after: disposition.provider_retry_after,
        exact_evidence,
        authority_effect: "SCHEDULING_MECHANISM_EVIDENCE_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

fn deferred_for(
    requirement: &ForemanExecutionAvailabilityRequirementV1,
    policy: &ExecutionAvailabilityPolicyV1,
    dispatch: &ProviderDispatchOccurrenceV1,
    disposition: &ProviderAdmissionDispositionV1,
) -> DeferredProviderDispatchV1 {
    let mut value = DeferredProviderDispatchV1 {
        schema: DEFERRED_PROVIDER_DISPATCH_SCHEMA_V1.to_owned(),
        deferred_dispatch_digest: placeholder(),
        requirement_digest: requirement.requirement_digest.clone(),
        policy_digest: policy.policy_digest.clone(),
        disposition_digest: disposition.disposition_digest.clone(),
        packet_digest: requirement.packet_digest.clone(),
        run_id: requirement.run_id.clone(),
        work_item_id: dispatch.work_item_id.clone(),
        work_attempt_id: dispatch.work_attempt_id.clone(),
        last_dispatch_occurrence_id: dispatch.dispatch_occurrence_id.clone(),
        provider_id: dispatch.selection.provider_id.clone(),
        model_id: dispatch.selection.model_id.clone(),
        selected_model_ordinal: dispatch.selected_model_ordinal,
        remaining_model_ordinals: vec![1],
        refusal_received_at: disposition.received_at,
        wake_basis: DeferredWakeBasisV1::ProviderRetryAfter,
        backoff_ordinal: 0,
        backoff_seconds: 5,
        provider_retry_after: disposition.provider_retry_after,
        wake_at: disposition.provider_retry_after.unwrap(),
        parked_resource_lock_policy: policy.parked_resource_lock_policy,
        provider_capacity_released: true,
        semantic_retry: false,
        authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY".to_owned(),
    };
    value.seal().unwrap();
    value
}

#[test]
fn graph_currentness_backoff_fallback_and_lock_substitutions_fail_closed() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let disposition = disposition(&requirement, &dispatch);
    let observation = observation_for_disposition(&disposition);
    let deferred = deferred_for(&requirement, &policy, &dispatch, &disposition);
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &dispatch,
        &observation,
        &disposition,
        &[],
        Some(&deferred),
    )
    .unwrap();

    let mut substituted_observation = observation.clone();
    substituted_observation.received_at += Duration::seconds(1);
    substituted_observation.seal().unwrap();
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &dispatch,
        &substituted_observation,
        &disposition,
        &[],
        Some(&deferred),
    )
    .unwrap_err();

    for mut substituted in [
        {
            let mut value = observation.clone();
            value.source_identity = "switchyard:other-source".to_owned();
            value
        },
        {
            let mut value = observation.clone();
            value.observed_at += Duration::milliseconds(1);
            value
        },
        {
            let mut value = observation.clone();
            value.provider_retry_after = None;
            value
        },
    ] {
        substituted.seal().unwrap();
        validate_execution_availability_graph(
            &requirement,
            &policy,
            &dispatch,
            &substituted,
            &disposition,
            &[],
            Some(&deferred),
        )
        .unwrap_err();
    }

    let mut stale = observation.clone();
    stale.expires_at = disposition.received_at;
    stale.seal().unwrap_err();

    for mut substituted in [
        {
            let mut value = deferred.clone();
            value.remaining_model_ordinals.clear();
            value
        },
        {
            let mut value = deferred.clone();
            value.refusal_received_at += Duration::seconds(1);
            value
        },
        {
            let mut value = deferred.clone();
            value.backoff_ordinal = 1;
            value
        },
        {
            let mut value = deferred.clone();
            value.parked_resource_lock_policy = ParkedResourceLockPolicyV1::RetainWhileParked;
            value
        },
        {
            let mut value = deferred.clone();
            value.backoff_seconds += 1;
            value.provider_retry_after = value
                .provider_retry_after
                .map(|time| time + Duration::seconds(1));
            value.wake_at += Duration::seconds(1);
            value
        },
    ] {
        substituted.seal().unwrap();
        validate_execution_availability_graph(
            &requirement,
            &policy,
            &dispatch,
            &observation,
            &disposition,
            &[],
            Some(&substituted),
        )
        .unwrap_err();
    }

    let completed = disposition_from_exact_snapshot(
        &requirement,
        &dispatch,
        include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-provider-completed.snapshot.v1.json"),
        disposition.received_at,
    );
    let available = observation_for_disposition(&completed);
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &dispatch,
        &available,
        &completed,
        &[],
        Some(&deferred),
    )
    .unwrap_err();
}

#[test]
fn cumulative_multi_deferral_history_is_explicit_and_bounded() {
    let mut policy = policy();
    policy.maximum_total_deferral_seconds = 75;
    policy.seal().unwrap();
    let mut requirement = requirement(&policy);
    requirement.policy_digest = policy.policy_digest.clone();
    requirement.seal().unwrap();

    let first_dispatch = dispatch(&requirement);
    let first_disposition = disposition(&requirement, &first_dispatch);
    let first_deferred = deferred_for(&requirement, &policy, &first_dispatch, &first_disposition);

    let mut second_dispatch = first_dispatch.clone();
    second_dispatch.dispatch_occurrence_id = "dispatch-holding-2".to_owned();
    second_dispatch.dispatch_ordinal = 2;
    second_dispatch.opened_at = first_deferred.wake_at;
    second_dispatch.seal().unwrap();

    let mut snapshot = parked_snapshot();
    snapshot["binding"]["dispatch_occurrence_id"] = json!(second_dispatch.dispatch_occurrence_id);
    snapshot["binding"] = seal_value(
        snapshot["binding"].clone(),
        "binding_digest",
        b"switchyard.codex-provider-admission-binding.digest/v1\0",
    );
    let binding_digest = snapshot["binding"]["binding_digest"].clone();
    for record in snapshot["records"].as_array_mut().unwrap() {
        record["dispatch_occurrence_id"] = json!(second_dispatch.dispatch_occurrence_id);
        record["binding_digest"] = binding_digest.clone();
        *record = seal_value(
            record.clone(),
            "evidence_digest",
            b"switchyard.codex-provider-admission-evidence.digest/v1\0",
        );
    }
    snapshot = seal_value(
        snapshot,
        "snapshot_digest",
        b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
    );
    let raw = canonical(&snapshot);
    let second_disposition = disposition_from_exact_snapshot(
        &requirement,
        &second_dispatch,
        &raw,
        time("2026-08-31T12:02:00Z"),
    );
    let second_observation = observation_for_disposition(&second_disposition);
    let mut second_deferred =
        deferred_for(&requirement, &policy, &second_dispatch, &second_disposition);
    second_deferred.backoff_ordinal = 1;
    second_deferred.seal().unwrap();
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &second_dispatch,
        &second_observation,
        &second_disposition,
        std::slice::from_ref(&first_deferred),
        Some(&second_deferred),
    )
    .unwrap();

    let mut reversed_model = first_deferred.clone();
    reversed_model.selected_model_ordinal = 1;
    reversed_model.model_id = "gpt-5.6-terra".to_owned();
    reversed_model.remaining_model_ordinals.clear();
    reversed_model.seal().unwrap();
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &second_dispatch,
        &second_observation,
        &second_disposition,
        &[reversed_model],
        Some(&second_deferred),
    )
    .unwrap_err();

    let mut over_budget_snapshot = snapshot;
    let refusal = &mut over_budget_snapshot["records"][1];
    refusal["normalized"]["retry_after_ms"] = json!(71_000);
    let raw_hex = refusal["raw"]["bytes_hex"].as_str().unwrap();
    let mut raw_value: Value = serde_json::from_slice(&hex::decode(raw_hex).unwrap()).unwrap();
    raw_value["params"]["retryAfterMs"] = json!(71_000);
    let mut wire = serde_json::to_vec(&raw_value).unwrap();
    wire.push(b'\n');
    refusal["raw"]["byte_length"] = json!(wire.len());
    refusal["raw"]["bytes_hex"] = json!(hex::encode(&wire));
    refusal["raw"]["sha256"] = json!(format!("sha256:{:x}", Sha256::digest(&wire)));
    *refusal = seal_value(
        refusal.clone(),
        "evidence_digest",
        b"switchyard.codex-provider-admission-evidence.digest/v1\0",
    );
    over_budget_snapshot = seal_value(
        over_budget_snapshot,
        "snapshot_digest",
        b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
    );
    let over_budget_disposition = disposition_from_exact_snapshot(
        &requirement,
        &second_dispatch,
        &canonical(&over_budget_snapshot),
        time("2026-08-31T12:02:00Z"),
    );
    let over_budget_observation = observation_for_disposition(&over_budget_disposition);
    let mut current_deferred = deferred_for(
        &requirement,
        &policy,
        &second_dispatch,
        &over_budget_disposition,
    );
    current_deferred.backoff_ordinal = 1;
    current_deferred.seal().unwrap();
    validate_execution_availability_graph(
        &requirement,
        &policy,
        &second_dispatch,
        &over_budget_observation,
        &over_budget_disposition,
        &[first_deferred],
        Some(&current_deferred),
    )
    .unwrap_err();
}

#[test]
fn mapper_cut_ordinal_representation_number_and_receipt_time_substitutions_fail_closed() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let original = disposition(&requirement, &dispatch);
    let raw = original.mapper_snapshot.validate().unwrap();

    let mutations: &[SnapshotMutation] = &[
        ("cut-session", |snapshot| {
            snapshot["acquisition_cut"]["app_server_session_identity"] = json!("other-session")
        }),
        ("ordinal-gap", |snapshot| {
            snapshot["records"][1]["acquisition_ordinal"] = json!(7)
        }),
        ("representation", |snapshot| {
            snapshot["records"][1]["raw"]["representation"] =
                json!("EXACT_PROVIDER_AVAILABILITY_SOURCE_BYTES")
        }),
        ("unsafe-number", |snapshot| {
            snapshot["records"][0]["normalized"]["started_at_ms"] = json!(9_007_199_254_740_992_i64)
        }),
        ("schema-string-bound", |snapshot| {
            snapshot["records"][1]["normalized"]["diagnostic"] = json!("x".repeat(4097))
        }),
    ];
    for (name, mutate) in mutations {
        let mut snapshot: Value = serde_json::from_slice(&raw).unwrap();
        mutate(&mut snapshot);
        for record in snapshot["records"].as_array_mut().unwrap() {
            *record = seal_value(
                record.clone(),
                "evidence_digest",
                b"switchyard.codex-provider-admission-evidence.digest/v1\0",
            );
        }
        snapshot = seal_value(
            snapshot,
            "snapshot_digest",
            b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
        );
        let mut substituted = original.clone();
        substituted.mapper_snapshot_digest =
            snapshot["snapshot_digest"].as_str().unwrap().to_owned();
        substituted.mapper_snapshot =
            ExactMapperSnapshotV1::from_bytes(&canonical(&snapshot)).unwrap();
        assert!(substituted.seal().is_err(), "accepted {name} substitution");
    }

    let mut pre_receipt = original;
    pre_receipt.received_at = time("2026-08-31T08:00:00Z");
    pre_receipt.provider_retry_after = Some(time("2026-08-31T08:00:05Z"));
    pre_receipt.seal().unwrap_err();
}

#[test]
fn semantic_execution_identity_substitutions_fail_closed() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let completed_raw = include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-provider-completed.snapshot.v1.json");
    let completed = disposition_from_exact_snapshot(
        &requirement,
        &dispatch,
        completed_raw,
        time("2026-08-31T12:01:02Z"),
    );
    let substitutions: &[SnapshotMutation] = &[
        ("top-execution", |snapshot| {
            snapshot["provider_execution_identity"]["first_response_id"] = json!("substituted")
        }),
        ("step-request-order", |snapshot| {
            snapshot["records"][1]["normalized"]["provider_execution_step_identity"]
                ["request_order"] = json!(1)
        }),
    ];
    for (name, mutate) in substitutions {
        let mut snapshot: Value = serde_json::from_slice(completed_raw).unwrap();
        mutate(&mut snapshot);
        snapshot = reseal_mapper_snapshot(snapshot);
        let mut substituted = completed.clone();
        substituted.mapper_snapshot_digest =
            snapshot["snapshot_digest"].as_str().unwrap().to_owned();
        substituted.mapper_snapshot =
            ExactMapperSnapshotV1::from_bytes(&canonical(&snapshot)).unwrap();
        assert!(substituted.seal().is_err(), "accepted {name} substitution");
    }

    let mut response_created = completed;
    response_created.response_created = false;
    response_created.seal().unwrap_err();
}

#[test]
fn client_request_response_pairing_is_exact() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let completed_raw = include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-provider-completed.snapshot.v1.json");
    let mut snapshot: Value = serde_json::from_slice(completed_raw).unwrap();
    for record in snapshot["records"].as_array_mut().unwrap() {
        record["sequence"] = json!(record["sequence"].as_u64().unwrap() + 2);
        if let Some(ordinal) = record["acquisition_ordinal"].as_u64() {
            record["acquisition_ordinal"] = json!(ordinal + 2);
        }
    }
    snapshot["acquisition_cut"]["ordered_high_water"] = json!(6);
    snapshot["acquisition_cut"]["consumed_ordinal_count"] = json!(6);
    let last = snapshot["records"].as_array().unwrap().len() - 1;
    snapshot["records"][last]["normalized"] = snapshot["acquisition_cut"].clone();
    let mut request = snapshot["records"][0].clone();
    request["sequence"] = json!(0);
    request["acquisition_ordinal"] = json!(0);
    request["acquisition_kind"] = json!("CLIENT_REQUEST");
    request["kind"] = json!("CLIENT_REQUEST_ISSUED");
    request["method"] = json!("client-request/thread/read");
    request["normalized"] = json!({"request_id":7,"request_method":"thread/read","params_sha256":placeholder(),"proves_provider_admission":false});
    request["raw"] = mapper_raw(json!({"id":7,"method":"thread/read","params":{}}));
    let mut response = request.clone();
    response["sequence"] = json!(1);
    response["acquisition_ordinal"] = json!(1);
    response["acquisition_kind"] = json!("CLIENT_RESPONSE");
    response["kind"] = json!("CLIENT_RESPONSE_RETAINED");
    response["method"] = json!("client-response/thread/read");
    response["normalized"] = json!({"request_id":7,"request_method":"thread/read","params_sha256":placeholder(),"result_sha256":placeholder(),"proves_provider_admission":false});
    response["raw"] = mapper_raw(json!({"id":7,"result":{"thread":"thread-holding-1"}}));
    snapshot["records"]
        .as_array_mut()
        .unwrap()
        .splice(0..0, [request, response]);
    snapshot = reseal_mapper_snapshot(snapshot);
    let disposition = disposition_from_exact_snapshot(
        &requirement,
        &dispatch,
        &canonical(&snapshot),
        time("2026-08-31T12:01:02Z"),
    );
    disposition.validate().unwrap();

    snapshot["records"][1]["normalized"]["params_sha256"] =
        json!(format!("sha256:{}", "1".repeat(64)));
    snapshot = reseal_mapper_snapshot(snapshot);
    let mut substituted = disposition;
    substituted.mapper_snapshot_digest = snapshot["snapshot_digest"].as_str().unwrap().to_owned();
    substituted.mapper_snapshot = ExactMapperSnapshotV1::from_bytes(&canonical(&snapshot)).unwrap();
    substituted.seal().unwrap_err();
}

#[test]
fn exact_switchyard_vectors_reopen_with_distinct_terminal_mechanism_states() {
    let policy = policy();
    let requirement = requirement(&policy);
    let dispatch = dispatch(&requirement);
    let received = time("2026-08-31T12:01:02Z");
    let cases: &[(&[u8], ProviderMechanismStateV1, bool, bool)] = &[
        (
            include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-provider-completed.snapshot.v1.json"),
            ProviderMechanismStateV1::ProviderCompleted,
            true,
            true,
        ),
        (
            include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-post-admission-interrupted.snapshot.v1.json"),
            ProviderMechanismStateV1::PostAdmissionInterrupted,
            false,
            true,
        ),
        (
            include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-approval-interrupted.snapshot.v1.json"),
            ProviderMechanismStateV1::PostAdmissionInterrupted,
            false,
            true,
        ),
        (
            include_bytes!("../../../qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/switchyard-admission-indeterminate.snapshot.v1.json"),
            ProviderMechanismStateV1::AdmissionIndeterminate,
            false,
            false,
        ),
    ];
    for (raw, expected_state, clean, admitted) in cases {
        let disposition = disposition_from_exact_snapshot(&requirement, &dispatch, raw, received);
        assert_eq!(disposition.mechanism_state, *expected_state);
        assert_eq!(disposition.acquisition_complete, *clean);
        assert_eq!(disposition.provider_execution.is_some(), *admitted);
        let observation = observation_for_disposition(&disposition);
        validate_execution_availability_graph(
            &requirement,
            &policy,
            &dispatch,
            &observation,
            &disposition,
            &[],
            None,
        )
        .unwrap();
        if matches!(
            expected_state,
            ProviderMechanismStateV1::PostAdmissionInterrupted
        ) {
            assert!(disposition.provider_execution.is_some());
            assert!(!disposition.disposition.permits_automatic_park());
        }
    }
}
