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
        work_attempt_id: "attempt-a".to_owned(),
        dispatch_occurrence_id: "dispatch-a-1".to_owned(),
        dispatch_ordinal: 1,
        selected_model_ordinal: 0,
        selection: requirement.work_item_model_selections["WORK-A"][0].clone(),
        adapter_id: requirement.adapter_id.clone(),
        adapter_version: requirement.adapter_version.clone(),
        adapter_protocol: requirement.adapter_protocol.clone(),
        adapter_process_occurrence_id: "adapter-process-a".to_owned(),
        app_server_session_identity: "session-a".to_owned(),
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

fn evidence_record(
    sequence: u64,
    binding: &Value,
    kind: &str,
    method: &str,
    normalized: Value,
) -> Value {
    seal_value(
        json!({
            "schema": "switchyard.codex-provider-admission-evidence/v1",
            "evidence_digest": placeholder(),
            "sequence": sequence,
            "acquisition_ordinal": sequence,
            "acquisition_kind": "NOTIFICATION",
            "binding_digest": binding["binding_digest"],
            "work_attempt_id": "attempt-a",
            "dispatch_occurrence_id": "dispatch-a-1",
            "adapter_process_occurrence_id": "adapter-process-a",
            "app_server_session_identity": "session-a",
            "thread_id": "thread-a",
            "turn_id": "turn-a",
            "provider": "openai",
            "model": "gpt-5.6-sol",
            "kind": kind,
            "method": method,
            "normalized": normalized,
            "raw": null
        }),
        "evidence_digest",
        b"switchyard.codex-provider-admission-evidence.digest/v1\0",
    )
}

fn parked_snapshot() -> Value {
    let binding = seal_value(
        json!({
            "schema": "switchyard.codex-provider-admission-binding/v1",
            "binding_digest": placeholder(),
            "work_attempt_id": "attempt-a",
            "dispatch_occurrence_id": "dispatch-a-1",
            "adapter_process_occurrence_id": "adapter-process-a",
            "app_server_session_identity": "session-a",
            "thread_id": "thread-a",
            "turn_id": "turn-a",
            "provider": "openai",
            "model": "gpt-5.6-sol",
            "codex_source_head": ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD,
            "executable_kind": "DETERMINISTIC_FIXTURE",
            "app_server_executable_identity": "fixture-holding",
            "app_server_executable_sha256": ACCEPTED_SWITCHYARD_DETERMINISTIC_FIXTURE_SHA256,
            "internal_provider_request_retries": 0
        }),
        "binding_digest",
        b"switchyard.codex-provider-admission-binding.digest/v1\0",
    );
    let cut = json!({
        "adapter_process_occurrence_id": "adapter-process-a",
        "app_server_session_identity": "session-a",
        "stream_quiesced": true,
        "loss_generation": 0,
        "process_disposition": "EXITED",
        "ordered_high_water": 3,
        "consumed_ordinal_count": 3,
        "outstanding_client_request_count": 0,
        "clean": true
    });
    let records = vec![
        evidence_record(
            0,
            &binding,
            "PROVIDER_REQUEST_STARTED",
            "providerRequest/started",
            json!({
                "request_occurrence_id": "request-a",
                "sampling_ordinal": 0,
                "request_order": 0,
                "started_at_ms": 1,
                "proves_provider_admission": false
            }),
        ),
        evidence_record(
            1,
            &binding,
            "PROVIDER_ADMISSION_REFUSED",
            "providerAdmission/refused",
            json!({
                "request_occurrence_id": "request-a",
                "sampling_ordinal": 0,
                "request_order": 0,
                "response_created": false,
                "will_retry": false,
                "refusal_kind": "MODEL_AT_CAPACITY",
                "codex_error_info": "serverOverloaded",
                "retry_after_ms": 5000,
                "diagnostic": "typed fixture",
                "observed_at_ms": 2,
                "provider_execution_identity": null
            }),
        ),
        evidence_record(
            2,
            &binding,
            "ACQUISITION_CUT",
            "adapter/acquisition-cut",
            cut.clone(),
        ),
    ];
    seal_value(
        json!({
            "schema": "switchyard.codex-provider-admission-snapshot/v1",
            "snapshot_digest": placeholder(),
            "binding": binding,
            "admission_disposition": "NOT_ADMITTED_MODEL_AT_CAPACITY",
            "mechanism_state": "PARKED_NOT_ADMITTED",
            "provider_execution_identity": null,
            "acquisition_cut": cut,
            "records": records
        }),
        "snapshot_digest",
        b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
    )
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
        provider_request_occurrence_id: "request-a".to_owned(),
        adapter_process_occurrence_id: dispatch.adapter_process_occurrence_id.clone(),
        app_server_session_identity: dispatch.app_server_session_identity.clone(),
        thread_id: "thread-a".to_owned(),
        turn_id: "turn-a".to_owned(),
        disposition: ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity,
        received_at: time("2026-08-31T12:01:02Z"),
        response_created: false,
        will_retry: false,
        acquisition_complete: true,
        provider_retry_after: Some(time("2026-08-31T12:01:07Z")),
        provider_execution: None,
        mapper_snapshot_schema: "switchyard.codex-provider-admission-snapshot/v1".to_owned(),
        mapper_snapshot_digest: snapshot["snapshot_digest"].as_str().unwrap().to_owned(),
        mapper_snapshot: ExactAvailabilityEvidenceV1::from_bytes(
            "RFC8785_SWITCHYARD_MAPPER_SNAPSHOT",
            &bytes,
        )
        .unwrap(),
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

    for bytes in [
        canonical(&policy),
        canonical(&requirement),
        canonical(&dispatch),
        canonical(&disposition),
    ] {
        assert!(bytes.len() < MAXIMUM_EXECUTION_AVAILABILITY_HISTORY_BYTES as usize);
    }
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
        observed_at: time("2026-08-31T12:01:01Z"),
        received_at: time("2026-08-31T12:01:02Z"),
        expires_at: time("2026-08-31T12:02:02Z"),
        state: ExecutionAvailabilityStateV1::ModelAtCapacity,
        source_identity: "switchyard:provider-admission".to_owned(),
        source_version: "v1".to_owned(),
        provider_retry_after: disposition.provider_retry_after,
        exact_evidence: Some(disposition.mapper_snapshot.clone()),
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
    substituted.mapper_snapshot = ExactAvailabilityEvidenceV1::from_bytes(
        "RFC8785_SWITCHYARD_MAPPER_SNAPSHOT",
        &canonical(&snapshot),
    )
    .unwrap();
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
