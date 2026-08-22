//! Cross-repository conformance for the NQ-NG diagnostic execution contract
//! and Nightshift's read-only operational-posture foundation.

use std::{collections::BTreeSet, process::Command};

use chrono::{DateTime, Utc};
use nightshiftd::diagnostic_execution_v2::{
    AcquisitionInterval, DiagnosticClaim, DiagnosticExecution, DiagnosticInputFailureKind,
};
use nightshiftd::diagnostic_posture::*;

const POSITIVE: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/positive.json");
const REFUSED: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/refused.json");
const PROVIDER_NO_RESPONSE: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/provider_no_response.json");
const HOSTILE_MATCH: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/hostile_projection_collision_match.json");
const HOSTILE_MISMATCH: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/hostile_projection_collision_mismatch.json");
const SPECIMEN_POSITIVE: &[u8] =
    include_bytes!("../../../docs/operator/examples/diagnostic-posture-v1/nq-positive.json");

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn parse(bytes: &[u8]) -> DiagnosticExecutionV1 {
    let artifact: DiagnosticExecutionV1 = serde_json::from_slice(bytes).unwrap();
    artifact.validate().unwrap();
    assert_eq!(serde_jcs::to_vec(&artifact).unwrap(), bytes);
    artifact
}

fn key(artifact: &DiagnosticExecutionV1) -> DiagnosticKey {
    DiagnosticKey {
        question_id: artifact.question.id.clone(),
        subject_id: artifact.subject.id.clone(),
        profile_id: artifact.profile.id.clone(),
        vantage_id: artifact.vantage.id.clone(),
    }
}

fn policy(artifact: &DiagnosticExecutionV1) -> PosturePolicy {
    let mut policy = PosturePolicy {
        schema: POSTURE_POLICY_SCHEMA.into(),
        policy_id: String::new(),
        generation: "generation:fixture-1".into(),
        subject: artifact.subject.clone(),
        role: SemanticIdentityV1 {
            id: "nightshift-role:host".into(),
            version: "1".into(),
            digest: digest('b'),
        },
        delivery_required: false,
        inventory: vec![InventoryEntry {
            binding: ContractBinding {
                producer_node_id: artifact.producer.node_id.clone(),
                producer_build: artifact.producer.build.clone(),
                producer_cohort: artifact.producer.cohort.clone(),
                question: artifact.question.clone(),
                profile: artifact.profile.clone(),
                profile_semantic_id: None,
                vantage: artifact.vantage.clone(),
                state_model: artifact.state_model.clone(),
                evaluator: artifact.evaluator.clone(),
                threshold_policy: artifact.threshold_policy.clone(),
                projection: artifact.projection.clone(),
                subject: artifact.subject.clone(),
                claim_id: "claim:load-pressure".into(),
            },
            requirement: Requirement::Mandatory,
            required_state_bindings: vec![RequiredStateBinding {
                kind: "boot_epoch".into(),
                value: "boot-a".into(),
            }],
            max_age_seconds: 300,
        }],
    };
    policy.policy_id = policy.computed_policy_id().unwrap();
    policy
}

fn delivered(artifact: DiagnosticExecutionV1) -> DiagnosticInputs {
    let mut inputs = DiagnosticInputs {
        schema: INPUTS_SCHEMA.into(),
        inputs_id: String::new(),
        inputs: vec![DiagnosticInput {
            key: key(&artifact),
            status: DiagnosticInputStatus::Delivered {
                artifact: Box::new(DiagnosticExecution::V1(artifact)),
            },
        }],
    };
    inputs.inputs_id = inputs.computed_inputs_id().unwrap();
    inputs
}

fn artifact_ref(artifact: &DiagnosticExecutionV1) -> ArtifactRef {
    let claim = artifact
        .primary_claim_id
        .as_deref()
        .and_then(|id| artifact.claims.iter().find(|claim| claim.claim_id == id));
    let acquisitions = match claim {
        Some(claim) => claim
            .dependency_input_ids
            .iter()
            .map(|id| {
                artifact
                    .inputs
                    .received
                    .iter()
                    .find(|input| input.input_id == *id)
                    .unwrap()
                    .acquisition
                    .clone()
                    .into()
            })
            .collect(),
        None => vec![],
    };
    ArtifactRef {
        contract_schema: Some(NQ_DIAGNOSTIC_EXECUTION_SCHEMA.into()),
        profile_semantic_id: None,
        artifact_id: artifact.artifact_id.clone(),
        request_id: artifact.request_id.clone(),
        run_id: artifact.run_id.clone(),
        attempt_interval: AcquisitionInterval::V1(artifact.attempt_interval.clone()),
        key: key(artifact),
        claim_id: claim.map(|claim| claim.claim_id.clone()),
        claim: claim.cloned().map(DiagnosticClaim::V1),
        dependency_acquisitions: acquisitions,
    }
}

fn recurrence(artifact: &DiagnosticExecutionV1) -> RecurrenceEvidence {
    let key = key(artifact);
    let schedule = SchedulePolicy {
        schedule_id: "schedule:fixture-host".into(),
        first_due_at: "2026-07-27T20:00:00Z".into(),
        cadence_seconds: 60,
        jitter_bound_seconds: 0,
        max_execution_budget_seconds: 10,
        standing_window_seconds: 130,
    };
    let slot = make_run_slot(&schedule, &key, 0).unwrap();
    let mut recurrence = RecurrenceEvidence {
        schema: RECURRENCE_SCHEMA.into(),
        recurrence_id: String::new(),
        obligations: vec![ScheduleObligation {
            key: key.clone(),
            policy: schedule.clone(),
        }],
        records: vec![RecurrenceRecord {
            key,
            policy: schedule,
            slot: slot.clone(),
            evidence: RunSlotEvidence::Completed {
                attempt: InvocationAttempt {
                    attempt_id: "attempt:fixture-1".into(),
                    slot_id: slot.slot_id,
                    request_id: artifact.request_id.clone(),
                    started_at: "2026-07-27T20:00:00Z".into(),
                },
                completed_at: artifact.completed_at.clone(),
                artifact: Box::new(artifact_ref(artifact)),
            },
        }],
        delivery: DeliveryStanding::NotRequired,
    };
    recurrence.recurrence_id = recurrence.computed_recurrence_id().unwrap();
    recurrence
}

fn evaluated_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-07-27T20:00:10Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn exact_nq_vectors_round_trip_and_keep_refusal_distinct_from_no_response() {
    let positive = parse(POSITIVE);
    let refused = parse(REFUSED);
    let provider_no_response = parse(PROVIDER_NO_RESPONSE);
    assert_eq!(
        provider_no_response.outcome.derivation,
        DerivationV1::Partial
    );
    assert_eq!(
        provider_no_response.inputs.failed[0].kind,
        FailedInputKindV1::NoResponse
    );
    let mut provider_policy = policy(&provider_no_response);
    provider_policy.inventory[0].binding.claim_id = "claim:provider-testimony".into();
    provider_policy.inventory[0].required_state_bindings.clear();
    provider_policy.policy_id.clear();
    provider_policy.policy_id = provider_policy.computed_policy_id().unwrap();
    let provider_posture = evaluate_posture(
        &provider_policy,
        &delivered(provider_no_response.clone()),
        &recurrence(&provider_no_response),
        evaluated_at(),
    )
    .unwrap();
    assert_eq!(
        provider_posture.assessments[0].status,
        OperatorStatus::PartialEvidence
    );
    assert_eq!(
        provider_posture.assessments[0]
            .nq_trace
            .as_ref()
            .unwrap()
            .input_failures[0]
            .kind,
        DiagnosticInputFailureKind::ProviderNoResponse
    );
    let mut nq_receiver_no_response = DiagnosticInputs {
        schema: INPUTS_SCHEMA.into(),
        inputs_id: String::new(),
        inputs: vec![DiagnosticInput {
            key: key(&provider_no_response),
            status: DiagnosticInputStatus::NoResponse,
        }],
    };
    nq_receiver_no_response.inputs_id = nq_receiver_no_response.computed_inputs_id().unwrap();
    let receiver_posture = evaluate_posture(
        &provider_policy,
        &nq_receiver_no_response,
        &recurrence(&provider_no_response),
        evaluated_at(),
    )
    .unwrap();
    assert_eq!(
        receiver_posture.assessments[0].status,
        OperatorStatus::NoResponse
    );
    assert_ne!(provider_posture.posture_id, receiver_posture.posture_id);

    let positive_posture = evaluate_posture(
        &policy(&positive),
        &delivered(positive.clone()),
        &recurrence(&positive),
        evaluated_at(),
    )
    .unwrap();
    assert_eq!(positive_posture.headline, Headline::Clean);

    let refused_posture = evaluate_posture(
        &policy(&refused),
        &delivered(refused.clone()),
        &recurrence(&refused),
        evaluated_at(),
    )
    .unwrap();
    assert_eq!(
        refused_posture.assessments[0].status,
        OperatorStatus::Refused
    );

    let mut no_response = DiagnosticInputs {
        schema: INPUTS_SCHEMA.into(),
        inputs_id: String::new(),
        inputs: vec![DiagnosticInput {
            key: key(&refused),
            status: DiagnosticInputStatus::NoResponse,
        }],
    };
    no_response.inputs_id = no_response.computed_inputs_id().unwrap();
    let no_response_posture = evaluate_posture(
        &policy(&refused),
        &no_response,
        &recurrence(&refused),
        evaluated_at(),
    )
    .unwrap();
    assert_eq!(
        no_response_posture.assessments[0].status,
        OperatorStatus::NoResponse
    );
    assert_ne!(refused_posture.posture_id, no_response_posture.posture_id);
}

#[test]
fn cross_repo_projection_collision_vectors_are_self_identified_but_structurally_refused() {
    let match_artifact: DiagnosticExecutionV1 = serde_json::from_slice(HOSTILE_MATCH).unwrap();
    let mismatch_artifact: DiagnosticExecutionV1 =
        serde_json::from_slice(HOSTILE_MISMATCH).unwrap();
    assert_eq!(serde_jcs::to_vec(&match_artifact).unwrap(), HOSTILE_MATCH);
    assert_eq!(
        serde_jcs::to_vec(&mismatch_artifact).unwrap(),
        HOSTILE_MISMATCH
    );
    assert_ne!(match_artifact.artifact_id, mismatch_artifact.artifact_id);
    assert_eq!(
        match_artifact.inputs.admitted[0].projected_artifact_id,
        mismatch_artifact.inputs.admitted[0].projected_artifact_id
    );
    for artifact in [match_artifact, mismatch_artifact] {
        let error = artifact.validate().unwrap_err();
        assert!(
            error.contains("claim requires a distinction omitted by the projection"),
            "{error}"
        );
    }
}

#[test]
fn operator_projection_retains_the_item_and_refuses_clean_when_hidden() {
    let artifact = parse(POSITIVE);
    let posture = evaluate_posture(
        &policy(&artifact),
        &delivered(artifact.clone()),
        &recurrence(&artifact),
        evaluated_at(),
    )
    .unwrap();
    let projected = posture.project(&BTreeSet::new());
    assert_eq!(projected.slots.len(), posture.assessments.len());
    assert_eq!(
        projected.slots[0].visibility,
        nightshiftd::diagnostic_posture::ProjectionVisibility::Omitted
    );
    assert_eq!(projected.headline, Headline::Incomplete);
    assert_eq!(projected.source_posture_id, posture.posture_id);
}

#[test]
fn recurrence_loss_changes_current_posture_without_mutating_exact_nq_bytes() {
    let artifact = parse(POSITIVE);
    let source_before = serde_jcs::to_vec(&artifact).unwrap();
    let policy = policy(&artifact);
    let inputs = delivered(artifact.clone());
    let recurrence = recurrence(&artifact);

    let current = evaluate_posture(
        &policy,
        &inputs,
        &recurrence,
        DateTime::parse_from_rfc3339("2026-07-27T20:00:10Z")
            .unwrap()
            .with_timezone(&Utc),
    )
    .unwrap();
    let later = evaluate_posture(
        &policy,
        &inputs,
        &recurrence,
        DateTime::parse_from_rfc3339("2026-07-27T20:01:10Z")
            .unwrap()
            .with_timezone(&Utc),
    )
    .unwrap();

    assert_eq!(source_before, POSITIVE);
    assert_eq!(serde_jcs::to_vec(&artifact).unwrap(), POSITIVE);
    assert_eq!(current.recurrence_axis, RecurrenceAxis::Current);
    assert_eq!(current.headline, Headline::Clean);
    assert_eq!(later.recurrence_axis, RecurrenceAxis::Incomplete);
    assert_eq!(
        later.recurrence[0].standing,
        RecurrenceStanding::RecordMissing
    );
    assert_eq!(later.headline, Headline::Incomplete);
    assert_ne!(current.posture_id, later.posture_id);
}

#[test]
fn production_cli_exposes_only_the_canonical_cycle_surface() {
    assert_eq!(SPECIMEN_POSITIVE, POSITIVE);
    let output = Command::new(env!("CARGO_BIN_EXE_nightshift"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("cycle"));
    for retired in ["watchbill", "governor", "wicket", "wlp", "drill"] {
        assert!(!help.to_ascii_lowercase().contains(retired), "{help}");
    }

    let run_help = Command::new(env!("CARGO_BIN_EXE_nightshift"))
        .args(["cycle", "run", "--help"])
        .output()
        .unwrap();
    assert!(run_help.status.success());
    let run_help = String::from_utf8(run_help.stdout).unwrap();
    for required in ["--nq-program", "--nq-config", "--nq-source-id"] {
        assert!(run_help.contains(required), "{run_help}");
    }
}
