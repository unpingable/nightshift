//! Hostile vector matrix for the additive cross-vantage concordance surface.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use nightshiftd::diagnostic_concordance::*;
use nightshiftd::diagnostic_execution_v2::{
    AcquisitionInterval, DiagnosticClaim, DiagnosticExecution,
};
use nightshiftd::diagnostic_posture::*;
use nightshiftd::diagnostic_source::{
    NqPackagePin, NqSourceEntry, NqSourceImportReceipt, NqSourceManifest, NqSourceStatus,
    NQ_SOURCE_IMPORT_RECEIPT_SCHEMA, NQ_SOURCE_MANIFEST_SCHEMA,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const POSITIVE: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/positive.json");
const REFUSED: &[u8] = include_bytes!("fixtures/nq_diagnostic_execution/refused.json");
const PROVIDER_NO_RESPONSE: &[u8] =
    include_bytes!("fixtures/nq_diagnostic_execution/provider_no_response.json");

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn parse(bytes: &[u8]) -> DiagnosticExecutionV1 {
    let artifact: DiagnosticExecutionV1 = serde_json::from_slice(bytes).unwrap();
    artifact.validate().unwrap();
    assert_eq!(serde_jcs::to_vec(&artifact).unwrap(), bytes);
    artifact
}

fn reseal<T: Serialize>(value: &T, id_field: &str) -> String {
    let mut value = serde_json::to_value(value).unwrap();
    value.as_object_mut().unwrap().remove(id_field);
    format!(
        "sha256:{:x}",
        Sha256::digest(serde_jcs::to_vec(&value).unwrap())
    )
}

fn vantage(id: &str, byte: char) -> SemanticIdentityV1 {
    SemanticIdentityV1 {
        id: id.into(),
        version: "1".into(),
        digest: digest(byte),
    }
}

fn at_vantage(
    mut artifact: DiagnosticExecutionV1,
    id: &str,
    byte: char,
    ordinal: usize,
) -> DiagnosticExecutionV1 {
    artifact.vantage = vantage(id, byte);
    artifact.producer.node_id = format!("nq-node:{ordinal}");
    artifact.request_id = format!("request:{ordinal:03}");
    artifact.run_id = format!("run:{ordinal:03}");
    artifact.artifact_id = reseal(&artifact, "artifact_id");
    artifact.validate().unwrap();
    artifact
}

fn adverse(mut artifact: DiagnosticExecutionV1) -> DiagnosticExecutionV1 {
    let claim = artifact
        .claims
        .iter_mut()
        .find(|claim| claim.claim_id == artifact.primary_claim_id.as_deref().unwrap())
        .unwrap();
    claim.status = ClaimStatusV1::Established;
    claim.condition_effect = Some(ConditionV1::Present);
    artifact.outcome.condition = ConditionV1::Present;
    artifact.outcome.summary = "complete current testimony refutes absence of load pressure".into();
    artifact.artifact_id = reseal(&artifact, "artifact_id");
    artifact.validate().unwrap();
    artifact
}

fn wrong_profile(mut artifact: DiagnosticExecutionV1) -> DiagnosticExecutionV1 {
    artifact.profile = SemanticIdentityV1 {
        id: "profile:other".into(),
        version: "1".into(),
        digest: digest('f'),
    };
    artifact.artifact_id = reseal(&artifact, "artifact_id");
    artifact.validate().unwrap();
    artifact
}

fn with_optional_provider_no_response(
    mut artifact: DiagnosticExecutionV1,
) -> DiagnosticExecutionV1 {
    artifact.inputs.expected.push(ExpectedInputV1 {
        expectation_id: "expected:optional-provider".into(),
        role: "optional_provider".into(),
        required: false,
    });
    artifact.inputs.failed.push(FailedInputV1 {
        expectation_id: "expected:optional-provider".into(),
        failure_id: "failure:optional-provider".into(),
        kind: FailedInputKindV1::NoResponse,
        reason: "optional provider did not respond".into(),
    });
    artifact.artifact_id = reseal(&artifact, "artifact_id");
    artifact.validate().unwrap();
    artifact
}

fn key_for(
    artifact: &DiagnosticExecutionV1,
    expected_profile: Option<&SemanticIdentityV1>,
) -> DiagnosticKey {
    DiagnosticKey {
        question_id: artifact.question.id.clone(),
        subject_id: artifact.subject.id.clone(),
        profile_id: expected_profile.unwrap_or(&artifact.profile).id.clone(),
        vantage_id: artifact.vantage.id.clone(),
    }
}

fn binding_for(
    artifact: &DiagnosticExecutionV1,
    expected_profile: Option<&SemanticIdentityV1>,
) -> ContractBinding {
    ContractBinding {
        producer_node_id: artifact.producer.node_id.clone(),
        producer_build: artifact.producer.build.clone(),
        producer_cohort: artifact.producer.cohort.clone(),
        question: artifact.question.clone(),
        profile: expected_profile.unwrap_or(&artifact.profile).clone(),
        profile_semantic_id: None,
        vantage: artifact.vantage.clone(),
        state_model: artifact.state_model.clone(),
        evaluator: artifact.evaluator.clone(),
        threshold_policy: artifact.threshold_policy.clone(),
        projection: artifact.projection.clone(),
        subject: artifact.subject.clone(),
        claim_id: artifact
            .primary_claim_id
            .clone()
            .unwrap_or_else(|| "claim:load-pressure".into()),
    }
}

#[derive(Clone)]
struct InputSpec {
    artifact: DiagnosticExecutionV1,
    receiver_record: bool,
    input_status: Option<DiagnosticInputStatus>,
    expected_profile: Option<SemanticIdentityV1>,
    requirement: Requirement,
    recurrence: RecurrenceMode,
}

#[derive(Clone, Copy)]
enum RecurrenceMode {
    Current,
    WrongGeneration,
}

impl InputSpec {
    fn delivered(artifact: DiagnosticExecutionV1) -> Self {
        Self {
            artifact,
            receiver_record: true,
            input_status: None,
            expected_profile: None,
            requirement: Requirement::Mandatory,
            recurrence: RecurrenceMode::Current,
        }
    }
}

fn artifact_ref(artifact: &DiagnosticExecutionV1, key: DiagnosticKey) -> ArtifactRef {
    let claim = artifact
        .primary_claim_id
        .as_deref()
        .and_then(|id| artifact.claims.iter().find(|claim| claim.claim_id == id));
    let received: BTreeMap<_, _> = artifact
        .inputs
        .received
        .iter()
        .map(|input| (input.input_id.as_str(), &input.acquisition))
        .collect();
    ArtifactRef {
        contract_schema: Some(
            nightshiftd::diagnostic_posture::NQ_DIAGNOSTIC_EXECUTION_SCHEMA.into(),
        ),
        profile_semantic_id: None,
        artifact_id: artifact.artifact_id.clone(),
        request_id: artifact.request_id.clone(),
        run_id: artifact.run_id.clone(),
        attempt_interval: AcquisitionInterval::V1(artifact.attempt_interval.clone()),
        key,
        claim_id: claim.map(|claim| claim.claim_id.clone()),
        claim: claim.cloned().map(DiagnosticClaim::V1),
        dependency_acquisitions: claim
            .map(|claim| {
                claim
                    .dependency_input_ids
                    .iter()
                    .filter_map(|id| received.get(id.as_str()))
                    .map(|value| AcquisitionInterval::V1((*value).clone()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn current_schedule(key: &DiagnosticKey) -> SchedulePolicy {
    SchedulePolicy {
        schedule_id: format!("schedule:{}", key.vantage_id),
        first_due_at: "2026-07-27T20:00:00Z".into(),
        cadence_seconds: 60,
        jitter_bound_seconds: 0,
        max_execution_budget_seconds: 10,
        standing_window_seconds: 130,
    }
}

fn build_posture(specs: &[InputSpec]) -> OperationalPosture {
    let expected_contract = &specs[0].artifact;
    let mut inventory = Vec::new();
    let mut inputs = Vec::new();
    let mut obligations = Vec::new();
    let mut records = Vec::new();
    for spec in specs {
        let key = key_for(&spec.artifact, spec.expected_profile.as_ref());
        let mut required_state_bindings = vec![];
        if expected_contract.primary_claim_id.is_some()
            && expected_contract.state_bindings.len() == 1
        {
            required_state_bindings.push(RequiredStateBinding {
                kind: expected_contract.state_bindings[0].kind.clone(),
                value: expected_contract.state_bindings[0].value.clone(),
            });
        }
        let mut binding = binding_for(&spec.artifact, spec.expected_profile.as_ref());
        binding.claim_id = expected_contract
            .primary_claim_id
            .clone()
            .unwrap_or_else(|| "claim:load-pressure".into());
        inventory.push(InventoryEntry {
            binding,
            requirement: spec.requirement,
            required_state_bindings,
            max_age_seconds: 300,
        });
        if spec.receiver_record {
            inputs.push(DiagnosticInput {
                key: key.clone(),
                status: spec.input_status.clone().unwrap_or_else(|| {
                    DiagnosticInputStatus::Delivered {
                        artifact: Box::new(DiagnosticExecution::V1(spec.artifact.clone())),
                    }
                }),
            });
        }
        let schedule = current_schedule(&key);
        obligations.push(ScheduleObligation {
            key: key.clone(),
            policy: schedule.clone(),
        });
        match spec.recurrence {
            RecurrenceMode::Current => {
                if spec.receiver_record && spec.input_status.is_none() {
                    let slot = make_run_slot(&schedule, &key, 0).unwrap();
                    records.push(RecurrenceRecord {
                        key: key.clone(),
                        policy: schedule,
                        slot: slot.clone(),
                        evidence: RunSlotEvidence::Completed {
                            attempt: InvocationAttempt {
                                attempt_id: format!("attempt:{}", key.vantage_id),
                                slot_id: slot.slot_id,
                                request_id: spec.artifact.request_id.clone(),
                                started_at: "2026-07-27T20:00:00Z".into(),
                            },
                            completed_at: spec.artifact.completed_at.clone(),
                            artifact: Box::new(artifact_ref(&spec.artifact, key)),
                        },
                    });
                }
            }
            RecurrenceMode::WrongGeneration => {
                let old_schedule = SchedulePolicy {
                    first_due_at: "2026-07-27T19:59:00Z".into(),
                    ..schedule
                };
                let slot = make_run_slot(&old_schedule, &key, 1).unwrap();
                records.push(RecurrenceRecord {
                    key: key.clone(),
                    policy: old_schedule,
                    slot: slot.clone(),
                    evidence: RunSlotEvidence::Completed {
                        attempt: InvocationAttempt {
                            attempt_id: format!("attempt:old:{}", key.vantage_id),
                            slot_id: slot.slot_id,
                            request_id: spec.artifact.request_id.clone(),
                            started_at: "2026-07-27T20:00:00Z".into(),
                        },
                        completed_at: spec.artifact.completed_at.clone(),
                        artifact: Box::new(artifact_ref(&spec.artifact, key)),
                    },
                });
            }
        }
    }
    inventory.sort_by_key(|entry| {
        (
            entry.binding.question.id.clone(),
            entry.binding.subject.id.clone(),
            entry.binding.profile.id.clone(),
            entry.binding.vantage.id.clone(),
        )
    });
    inputs.sort_by_key(|input| input.key.clone());
    obligations.sort_by_key(|item| item.key.clone());
    records.sort_by_key(|item| item.key.clone());

    let first = expected_contract;
    let mut policy = PosturePolicy {
        schema: POSTURE_POLICY_SCHEMA.into(),
        policy_id: String::new(),
        generation: "posture-generation:fixture-1".into(),
        subject: first.subject.clone(),
        role: SemanticIdentityV1 {
            id: "nightshift-role:host".into(),
            version: "1".into(),
            digest: digest('b'),
        },
        delivery_required: false,
        inventory,
    };
    policy.policy_id = policy.computed_policy_id().unwrap();
    let mut inputs = DiagnosticInputs {
        schema: INPUTS_SCHEMA.into(),
        inputs_id: String::new(),
        inputs,
    };
    inputs.inputs_id = inputs.computed_inputs_id().unwrap();
    let mut recurrence = RecurrenceEvidence {
        schema: RECURRENCE_SCHEMA.into(),
        recurrence_id: String::new(),
        obligations,
        records,
        delivery: DeliveryStanding::NotRequired,
    };
    recurrence.recurrence_id = recurrence.computed_recurrence_id().unwrap();
    evaluate_posture(&policy, &inputs, &recurrence, evaluated_at()).unwrap()
}

fn evaluated_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-07-27T20:00:10Z")
        .unwrap()
        .with_timezone(&Utc)
}

fn concordance_policy(
    posture: &OperationalPosture,
    expected: &[DiagnosticExecutionV1],
) -> ConcordancePolicy {
    let first = &expected[0];
    let mut expected_vantages: Vec<_> = expected
        .iter()
        .map(|artifact| ExpectedVantage {
            vantage: artifact.vantage.clone(),
            key: key_for(artifact, Some(&first.profile)),
        })
        .collect();
    expected_vantages.sort_by(|left, right| {
        (left.vantage.id.as_bytes(), &left.key).cmp(&(right.vantage.id.as_bytes(), &right.key))
    });
    let state_bindings = first
        .state_bindings
        .iter()
        .map(|binding| ComparableStateBinding {
            kind: binding.kind.clone(),
            value: binding.value.clone(),
        })
        .collect();
    let mut policy = ConcordancePolicy {
        schema: CONCORDANCE_POLICY_SCHEMA.into(),
        policy_id: String::new(),
        posture_policy_id: posture.policy.policy_id.clone(),
        posture_generation: posture.policy.generation.clone(),
        comparison_set: Some(ComparisonSet {
            comparison_set_id: "comparison:host-load".into(),
            generation: "comparison-generation:fixture-1".into(),
            contract_schema: nightshiftd::diagnostic_concordance::NQ_DIAGNOSTIC_EXECUTION_SCHEMA
                .into(),
            subject: first.subject.clone(),
            question: first.question.clone(),
            profile: first.profile.clone(),
            profile_semantic_id: None,
            state_model: first.state_model.clone(),
            evaluator: first.evaluator.clone(),
            threshold_policy: first.threshold_policy.clone(),
            projection: first.projection.clone(),
            primary_claim_id: first
                .primary_claim_id
                .clone()
                .unwrap_or_else(|| "claim:load-pressure".into()),
            state_bindings,
            expected_vantages,
        }),
    };
    policy.policy_id = policy.computed_policy_id().unwrap();
    policy
}

fn evaluate(
    specs: &[InputSpec],
    expected: &[DiagnosticExecutionV1],
) -> OperationalPostureConcordance {
    let posture = build_posture(specs);
    let policy = concordance_policy(&posture, expected);
    evaluate_concordance(&posture, &policy).unwrap()
}

fn import_receipt(posture: &OperationalPosture) -> NqSourceImportReceipt {
    let mut sources: Vec<_> = posture
        .input_evidence
        .inputs
        .iter()
        .map(|input| NqSourceEntry {
            key: input.key.clone(),
            status: match &input.status {
                DiagnosticInputStatus::Delivered { artifact } => NqSourceStatus::Delivered {
                    artifact_path: format!("{}.json", input.key.vantage_id.replace(':', "_")),
                    artifact_sha256: format!(
                        "sha256:{:x}",
                        Sha256::digest(serde_jcs::to_vec(artifact).unwrap())
                    ),
                    artifact_id: artifact.artifact_id().to_owned(),
                },
                DiagnosticInputStatus::NoResponse => NqSourceStatus::NoResponse,
                DiagnosticInputStatus::AcquisitionFailed { reason } => {
                    NqSourceStatus::AcquisitionFailed {
                        reason: reason.clone(),
                    }
                }
                DiagnosticInputStatus::NotConfigured => NqSourceStatus::NotConfigured,
            },
        })
        .collect();
    sources.sort_by_key(|source| source.key.clone());
    let mut manifest = NqSourceManifest {
        schema: NQ_SOURCE_MANIFEST_SCHEMA.into(),
        source_manifest_id: String::new(),
        package: NqPackagePin {
            repository_identity: "nq-ng".into(),
            commit: "a".repeat(40),
            release_identity: "nq-ng:test-package".into(),
            contract_schema: nightshiftd::diagnostic_concordance::NQ_DIAGNOSTIC_EXECUTION_SCHEMA
                .into(),
            asset_root: "share/nq/diagnostic-contract".into(),
            asset_manifest_path: "share/nq/diagnostic-contract/manifest.json".into(),
            asset_manifest_sha256: digest('7'),
            payload_manifest_path: "share/nq/MANIFEST.sha256".into(),
            payload_manifest_sha256: digest('8'),
        },
        inputs: sources,
    };
    manifest.source_manifest_id = manifest.computed_source_manifest_id().unwrap();
    let mut receipt = NqSourceImportReceipt {
        schema: NQ_SOURCE_IMPORT_RECEIPT_SCHEMA.into(),
        receipt_id: String::new(),
        source_manifest: manifest,
        imported_inputs_id: posture.input_evidence.inputs_id.clone(),
    };
    receipt.receipt_id = reseal(&receipt, "receipt_id");
    receipt.validate_inputs(&posture.input_evidence).unwrap();
    receipt
}

fn reason_for(
    value: &OperationalPostureConcordance,
    vantage_id: &str,
) -> Option<NonContributionReason> {
    value
        .cross_vantage_concordance
        .members
        .iter()
        .find(|member| member.expected.vantage.id == vantage_id)
        .and_then(|member| match member.contribution {
            Contribution::NotContributing { reason, .. } => Some(reason),
            Contribution::Contributing { .. } => None,
        })
}

#[test]
fn v1_concordant_current_results_preserve_source_artifacts() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let before = serde_jcs::to_vec(&local).unwrap();
    let value = evaluate(
        &[
            InputSpec::delivered(local.clone()),
            InputSpec::delivered(remote.clone()),
        ],
        &[local.clone(), remote],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Concordant
    );
    assert_eq!(
        value
            .cross_vantage_concordance
            .contributing_artifact_ids
            .len(),
        2
    );
    assert_eq!(serde_jcs::to_vec(&local).unwrap(), before);
    assert_eq!(value.source_posture.headline, Headline::Clean);
}

#[test]
fn v2_discordant_current_results_select_no_winner() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = adverse(at_vantage(parse(POSITIVE), "vantage:b", 'b', 2));
    let value = evaluate(
        &[
            InputSpec::delivered(local.clone()),
            InputSpec::delivered(remote.clone()),
        ],
        &[local, remote],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Discordant
    );
    assert_eq!(value.cross_vantage_concordance.distinct_outcomes.len(), 2);
}

#[test]
fn v3_required_vantage_missing_is_insufficient_record_missing() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut remote_spec = InputSpec::delivered(remote.clone());
    remote_spec.receiver_record = false;
    let posture = build_posture(&[InputSpec::delivered(local.clone()), remote_spec]);
    let policy = concordance_policy(&posture, &[local, remote]);
    let value = evaluate_concordance(&posture, &policy).unwrap();
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Insufficient
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::RecordMissing)
    );
}

#[test]
fn v4_explicit_refusal_remains_distinct() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let refused = at_vantage(parse(REFUSED), "vantage:b", 'b', 2);
    let value = evaluate(
        &[
            InputSpec::delivered(local.clone()),
            InputSpec::delivered(refused.clone()),
        ],
        &[local, refused],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Insufficient
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::ExplicitRefusal)
    );
}

#[test]
fn v5_provider_no_response_remains_distinct() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let no_response = at_vantage(parse(PROVIDER_NO_RESPONSE), "vantage:b", 'b', 2);
    let value = evaluate(
        &[
            InputSpec::delivered(local.clone()),
            InputSpec::delivered(no_response.clone()),
        ],
        &[local, no_response],
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::ProviderNoResponse),
        "{:#?}",
        value.cross_vantage_concordance
    );
}

#[test]
fn completed_result_with_optional_provider_no_response_still_contributes() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote =
        with_optional_provider_no_response(at_vantage(parse(POSITIVE), "vantage:b", 'b', 2));
    let value = evaluate(
        &[
            InputSpec::delivered(local.clone()),
            InputSpec::delivered(remote.clone()),
        ],
        &[local, remote],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Concordant
    );
    assert!(value
        .cross_vantage_concordance
        .members
        .iter()
        .all(|member| matches!(member.contribution, Contribution::Contributing { .. })));
}

#[test]
fn v6_nightshift_receiver_silence_remains_distinct() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut remote_spec = InputSpec::delivered(remote.clone());
    remote_spec.input_status = Some(DiagnosticInputStatus::NoResponse);
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), remote_spec],
        &[local, remote],
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::ReceiverSilence)
    );
}

#[test]
fn v7_incompatible_profile_is_uncomparable_not_compared() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote_expected = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let remote = wrong_profile(remote_expected.clone());
    let mut remote_spec = InputSpec::delivered(remote);
    remote_spec.expected_profile = Some(local.profile.clone());
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), remote_spec],
        &[local, remote_expected],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Uncomparable
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::IncompatibleRecord)
    );
}

#[test]
fn hostile_wrong_profile_refusal_is_incompatible_not_a_declared_member_refusal() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote_expected = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let remote = wrong_profile(at_vantage(parse(REFUSED), "vantage:b", 'b', 2));
    let mut remote_spec = InputSpec::delivered(remote);
    remote_spec.expected_profile = Some(local.profile.clone());
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), remote_spec],
        &[local, remote_expected],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Uncomparable
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::IncompatibleRecord)
    );
}

#[test]
fn hostile_wrong_profile_provider_failure_is_incompatible_not_provider_no_response() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote_expected = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let remote = wrong_profile(at_vantage(parse(PROVIDER_NO_RESPONSE), "vantage:b", 'b', 2));
    let mut remote_spec = InputSpec::delivered(remote);
    remote_spec.expected_profile = Some(local.profile.clone());
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), remote_spec],
        &[local, remote_expected],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Uncomparable
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::IncompatibleRecord)
    );
}

#[test]
fn comparison_policy_must_match_full_closed_inventory_before_receiver_silence() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut remote_spec = InputSpec::delivered(remote.clone());
    remote_spec.input_status = Some(DiagnosticInputStatus::NoResponse);
    let posture = build_posture(&[InputSpec::delivered(local.clone()), remote_spec]);
    let base = concordance_policy(&posture, &[local, remote]);

    let mut wrong_claim = base.clone();
    wrong_claim
        .comparison_set
        .as_mut()
        .unwrap()
        .primary_claim_id = "claim:counterfeit".into();
    wrong_claim.policy_id = wrong_claim.computed_policy_id().unwrap();
    assert!(evaluate_concordance(&posture, &wrong_claim)
        .unwrap_err()
        .contains("full closed-inventory"));

    let mut wrong_state = base.clone();
    wrong_state.comparison_set.as_mut().unwrap().state_bindings[0].value =
        "boot:counterfeit".into();
    wrong_state.policy_id = wrong_state.computed_policy_id().unwrap();
    assert!(evaluate_concordance(&posture, &wrong_state)
        .unwrap_err()
        .contains("full closed-inventory"));

    let mut wrong_profile_digest = base;
    wrong_profile_digest
        .comparison_set
        .as_mut()
        .unwrap()
        .profile
        .digest = digest('0');
    wrong_profile_digest.policy_id = wrong_profile_digest.computed_policy_id().unwrap();
    assert!(evaluate_concordance(&posture, &wrong_profile_digest)
        .unwrap_err()
        .contains("full closed-inventory"));
}

#[test]
fn excluded_inventory_member_cannot_be_requested_for_concordance() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut excluded = InputSpec::delivered(remote.clone());
    excluded.requirement = Requirement::Excluded;
    excluded.receiver_record = false;
    let posture = build_posture(&[InputSpec::delivered(local.clone()), excluded]);
    let policy = concordance_policy(&posture, &[local, remote]);
    assert!(evaluate_concordance(&posture, &policy)
        .unwrap_err()
        .contains("full closed-inventory"));
}

#[test]
fn v8_different_recurrence_generation_is_not_concordance() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut remote_spec = InputSpec::delivered(remote.clone());
    remote_spec.recurrence = RecurrenceMode::WrongGeneration;
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), remote_spec],
        &[local, remote],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Uncomparable
    );
    assert_eq!(
        reason_for(&value, "vantage:b"),
        Some(NonContributionReason::WrongGeneration)
    );
}

#[test]
fn v9_duplicate_execution_under_two_labels_counts_neither_copy() {
    let local = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let remote_expected = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let mut duplicate_spec = InputSpec::delivered(local.clone());
    duplicate_spec.artifact = local.clone();
    duplicate_spec.artifact.vantage = remote_expected.vantage.clone();
    // Preserve the same artifact and producer/run identity deliberately:
    // only the receiver key/binding label changes.
    duplicate_spec.input_status = Some(DiagnosticInputStatus::Delivered {
        artifact: Box::new(DiagnosticExecution::V1(local.clone())),
    });
    let value = evaluate(
        &[InputSpec::delivered(local.clone()), duplicate_spec],
        &[local, remote_expected],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Insufficient
    );
    assert!(value
        .cross_vantage_concordance
        .members
        .iter()
        .all(|member| matches!(
            member.contribution,
            Contribution::NotContributing {
                reason: NonContributionReason::DuplicateExecution,
                ..
            }
        )));
}

#[test]
fn v10_majority_never_resolves_disagreement() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = adverse(at_vantage(parse(POSITIVE), "vantage:b", 'b', 2));
    let c = at_vantage(parse(POSITIVE), "vantage:c", 'c', 3);
    let value = evaluate(
        &[
            InputSpec::delivered(a.clone()),
            InputSpec::delivered(b.clone()),
            InputSpec::delivered(c.clone()),
        ],
        &[a, b, c],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Discordant
    );
    assert_eq!(value.cross_vantage_concordance.distinct_outcomes.len(), 2);
}

#[test]
fn v11_out_of_set_record_is_irrelevant_and_retained_in_source_posture() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let c = adverse(at_vantage(parse(POSITIVE), "vantage:c", 'c', 3));
    let mut c_spec = InputSpec::delivered(c);
    c_spec.requirement = Requirement::Optional;
    let value = evaluate(
        &[
            InputSpec::delivered(a.clone()),
            InputSpec::delivered(b.clone()),
            c_spec,
        ],
        &[a, b],
    );
    assert_eq!(
        value.cross_vantage_concordance.state,
        ConcordanceState::Concordant
    );
    assert_eq!(value.source_posture.assessments.len(), 3);
    assert_eq!(value.cross_vantage_concordance.members.len(), 2);
}

#[test]
fn v12_hostile_unknown_contract_input_is_rejected_before_comparison() {
    let mut value: serde_json::Value = serde_json::from_slice(POSITIVE).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unknown_field".into(), serde_json::json!("hostile"));
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(serde_json::from_slice::<DiagnosticExecutionV1>(&bytes).is_err());
    assert_eq!(
        Sha256::digest(&bytes),
        Sha256::digest(&bytes),
        "rejection does not mutate the supplied bytes"
    );

    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let posture = build_posture(&[
        InputSpec::delivered(a.clone()),
        InputSpec::delivered(b.clone()),
    ]);
    let mut unknown_policy = concordance_policy(&posture, &[a.clone(), b.clone()]);
    unknown_policy.schema = "nightshift.concordance_policy.v99".into();
    unknown_policy.policy_id.clear();
    unknown_policy.policy_id = unknown_policy.computed_policy_id().unwrap();
    assert!(unknown_policy.validate().is_err());

    let mut legacy_policy = concordance_policy(&posture, &[a, b]);
    legacy_policy.schema = CONCORDANCE_POLICY_SCHEMA_V1.into();
    legacy_policy
        .comparison_set
        .as_mut()
        .unwrap()
        .contract_schema = "nq.diagnostic_execution.v2".into();
    legacy_policy.policy_id.clear();
    legacy_policy.policy_id = legacy_policy.computed_policy_id().unwrap();
    assert!(legacy_policy.validate().is_err());

    let mut missing_profile_semantic = concordance_policy(
        &posture,
        &[
            at_vantage(parse(POSITIVE), "vantage:a", 'a', 1),
            at_vantage(parse(POSITIVE), "vantage:b", 'b', 2),
        ],
    );
    missing_profile_semantic
        .comparison_set
        .as_mut()
        .unwrap()
        .contract_schema = "nq.diagnostic_execution.v2".into();
    missing_profile_semantic.policy_id.clear();
    missing_profile_semantic.policy_id = missing_profile_semantic.computed_policy_id().unwrap();
    assert!(
        missing_profile_semantic.validate().is_err(),
        "v2 comparison cannot infer the compiled profile semantic identity"
    );
}

#[test]
fn not_requested_and_permutation_invariance_are_explicit() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let posture = build_posture(&[
        InputSpec::delivered(a.clone()),
        InputSpec::delivered(b.clone()),
    ]);
    let mut not_requested = ConcordancePolicy {
        schema: CONCORDANCE_POLICY_SCHEMA.into(),
        policy_id: String::new(),
        posture_policy_id: posture.policy.policy_id.clone(),
        posture_generation: posture.policy.generation.clone(),
        comparison_set: None,
    };
    not_requested.policy_id = not_requested.computed_policy_id().unwrap();
    assert_eq!(
        evaluate_concordance(&posture, &not_requested)
            .unwrap()
            .cross_vantage_concordance
            .state,
        ConcordanceState::NotRequested
    );

    let first = evaluate_concordance(
        &posture,
        &concordance_policy(&posture, &[a.clone(), b.clone()]),
    )
    .unwrap();
    let reversed_posture = build_posture(&[
        InputSpec::delivered(b.clone()),
        InputSpec::delivered(a.clone()),
    ]);
    let second = evaluate_concordance(
        &reversed_posture,
        &concordance_policy(&reversed_posture, &[b, a]),
    )
    .unwrap();
    assert_eq!(
        first.cross_vantage_concordance.state,
        second.cross_vantage_concordance.state
    );
    assert_eq!(
        first.cross_vantage_concordance.distinct_outcomes,
        second.cross_vantage_concordance.distinct_outcomes
    );
}

#[test]
fn companion_artifact_has_no_authority_or_action_fields_and_leaves_v1_unchanged() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let value = evaluate(
        &[
            InputSpec::delivered(a.clone()),
            InputSpec::delivered(b.clone()),
        ],
        &[a, b],
    );
    assert_eq!(value.source_posture.schema, POSTURE_SCHEMA);
    fn assert_no_authority_keys(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, child) in map {
                    assert!(
                        !matches!(
                            key.as_str(),
                            "authorization" | "action" | "dispatch" | "permission"
                        ),
                        "forbidden structural field: {key}"
                    );
                    assert_no_authority_keys(child);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    assert_no_authority_keys(child);
                }
            }
            _ => {}
        }
    }
    assert_no_authority_keys(&serde_json::to_value(&value).unwrap());
    assert!(nightshiftd::diagnostic_concordance::render_text(&value).contains("state: Concordant"));
}

#[test]
fn machine_contract_reopens_exact_canonical_bytes_and_refuses_substitution() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let posture = build_posture(&[
        InputSpec::delivered(a.clone()),
        InputSpec::delivered(b.clone()),
    ]);
    let policy = concordance_policy(&posture, &[a, b]);
    let value = evaluate_concordance_with_source(&posture, &policy, Some(import_receipt(&posture)))
        .unwrap();
    let bytes = serde_jcs::to_vec(&value).unwrap();
    assert_eq!(
        OperationalPostureConcordance::from_canonical_bytes(&bytes).unwrap(),
        value
    );

    let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown["invented_authority"] = serde_json::json!(true);
    assert!(
        serde_json::from_value::<OperationalPostureConcordance>(unknown).is_err(),
        "the companion contract must be closed to unknown fields"
    );

    let mut source_substitution = value.clone();
    source_substitution.source_posture_id = digest('9');
    source_substitution.concordance_id = reseal(&source_substitution, "concordance_id");
    assert!(source_substitution.validate().is_err());

    let mut member_substitution = value.clone();
    member_substitution.cross_vantage_concordance.state = ConcordanceState::Discordant;
    member_substitution.concordance_id = reseal(&member_substitution, "concordance_id");
    assert!(member_substitution.validate().is_err());

    let mut import_substitution = value.clone();
    let receipt = import_substitution.source_import.as_mut().unwrap();
    let NqSourceStatus::Delivered {
        artifact_sha256, ..
    } = &mut receipt.source_manifest.inputs[0].status
    else {
        panic!("fixture source must be delivered");
    };
    *artifact_sha256 = digest('0');
    receipt.source_manifest.source_manifest_id = receipt
        .source_manifest
        .computed_source_manifest_id()
        .unwrap();
    receipt.receipt_id = reseal(receipt, "receipt_id");
    import_substitution.concordance_id = reseal(&import_substitution, "concordance_id");
    assert!(import_substitution.validate().is_err());

    let mut evaluator_substitution = value;
    evaluator_substitution.evaluator = SemanticIdentityV1 {
        id: "nightshift.other_evaluator".into(),
        version: "1".into(),
        digest: digest('e'),
    };
    evaluator_substitution.concordance_id = reseal(&evaluator_substitution, "concordance_id");
    assert!(evaluator_substitution.validate().is_err());
}

#[test]
fn human_projection_quotes_untrusted_strings_instead_of_forging_lines() {
    let a = at_vantage(parse(POSITIVE), "vantage:a", 'a', 1);
    let b = at_vantage(parse(POSITIVE), "vantage:b", 'b', 2);
    let posture = build_posture(&[
        InputSpec::delivered(a.clone()),
        InputSpec::delivered(b.clone()),
    ]);
    let policy = concordance_policy(&posture, &[a, b]);
    let mut receipt = import_receipt(&posture);
    receipt.source_manifest.package.repository_identity = "nq-ng\nstate: forged-by-input".into();
    receipt.source_manifest.source_manifest_id = receipt
        .source_manifest
        .computed_source_manifest_id()
        .unwrap();
    receipt.receipt_id = reseal(&receipt, "receipt_id");
    let value = evaluate_concordance_with_source(&posture, &policy, Some(receipt)).unwrap();
    let rendered = nightshiftd::diagnostic_concordance::render_text(&value);
    assert!(!rendered.contains("repository=nq-ng\nstate: forged-by-input"));
    assert!(rendered.contains("repository=\"nq-ng\\nstate: forged-by-input\""));
    assert!(rendered.contains("nq_source_declaration:"));
    assert!(rendered.contains("attestation=unverified"));
    assert!(rendered.contains("nq_package_bytes:"));
    assert!(rendered.contains("source_posture_detail:"));
    assert!(rendered.contains("diagnostic:"));
    assert!(rendered.contains("comparison_generation:"));
}
