//! Read-only, explicit cross-vantage concordance over an immutable
//! [`OperationalPosture`](crate::diagnostic_posture::OperationalPosture).
//!
//! Concordance is deliberately a companion artifact.  It does not change the
//! `nightshift.operational_posture.v1` wire contract, choose a winning
//! vantage, infer comparison groups, or carry authorization or actuation
//! semantics.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::diagnostic_posture::{
    ClaimStatusV1, CoherenceV1, ConditionV1, CoverageV1, DerivationV1, DiagnosticExecutionV1,
    DiagnosticInput, DiagnosticInputStatus, DiagnosticKey, InventoryEntry, OperationalPosture,
    ProjectionV1, RecurrenceAssessment, RecurrenceStanding, Requirement, SemanticIdentityV1,
    Standing, StateBindingV1, SubjectV1,
};
use crate::diagnostic_source::{NqSourceImportReceipt, NqSourceStatus};

pub const CONCORDANCE_POLICY_SCHEMA: &str = "nightshift.concordance_policy.v1";
pub const CONCORDANCE_SCHEMA: &str = "nightshift.operational_posture_concordance.v1";
pub const NQ_DIAGNOSTIC_EXECUTION_SCHEMA: &str = "nq.diagnostic_execution.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparableStateBinding {
    pub kind: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedVantage {
    pub vantage: SemanticIdentityV1,
    pub key: DiagnosticKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonSet {
    pub comparison_set_id: String,
    pub generation: String,
    pub contract_schema: String,
    pub subject: SubjectV1,
    pub question: SemanticIdentityV1,
    pub profile: SemanticIdentityV1,
    pub state_model: SemanticIdentityV1,
    pub evaluator: SemanticIdentityV1,
    pub threshold_policy: SemanticIdentityV1,
    pub projection: ProjectionV1,
    pub primary_claim_id: String,
    pub state_bindings: Vec<ComparableStateBinding>,
    pub expected_vantages: Vec<ExpectedVantage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConcordancePolicy {
    pub schema: String,
    pub policy_id: String,
    pub posture_policy_id: String,
    pub posture_generation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison_set: Option<ComparisonSet>,
}

impl ConcordancePolicy {
    pub fn computed_policy_id(&self) -> Result<String, String> {
        computed_object_id(self, "policy_id")
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CONCORDANCE_POLICY_SCHEMA {
            return Err(format!(
                "concordance policy schema must be {CONCORDANCE_POLICY_SCHEMA}, got {}",
                self.schema
            ));
        }
        validate_digest(&self.policy_id, "policy_id")?;
        validate_digest(&self.posture_policy_id, "posture_policy_id")?;
        require_token("posture_generation", &self.posture_generation)?;
        if let Some(set) = &self.comparison_set {
            set.validate()?;
        }
        if self.policy_id != self.computed_policy_id()? {
            return Err("policy_id does not match the canonical policy preimage".into());
        }
        Ok(())
    }
}

impl ComparisonSet {
    fn validate(&self) -> Result<(), String> {
        require_token("comparison_set_id", &self.comparison_set_id)?;
        require_token("comparison_set.generation", &self.generation)?;
        if self.contract_schema != NQ_DIAGNOSTIC_EXECUTION_SCHEMA {
            return Err(format!(
                "comparison set contract_schema must be {NQ_DIAGNOSTIC_EXECUTION_SCHEMA}"
            ));
        }
        require_token("comparison_set.subject.id", &self.subject.id)?;
        validate_semantic_identity(&self.subject.scope, "comparison_set.subject.scope")?;
        for (field, identity) in [
            ("comparison_set.question", &self.question),
            ("comparison_set.profile", &self.profile),
            ("comparison_set.state_model", &self.state_model),
            ("comparison_set.evaluator", &self.evaluator),
            ("comparison_set.threshold_policy", &self.threshold_policy),
            ("comparison_set.projection", &self.projection.identity),
        ] {
            validate_semantic_identity(identity, field)?;
        }
        let mut previous_omission: Option<&str> = None;
        for omission in &self.projection.omitted_distinctions {
            require_token("comparison_set.projection.omission.code", &omission.code)?;
            require_token(
                "comparison_set.projection.omission.detail",
                &omission.detail,
            )?;
            if previous_omission.is_some_and(|prior| prior.as_bytes() >= omission.code.as_bytes()) {
                return Err(
                    "comparison projection omissions must be strictly ordered and unique".into(),
                );
            }
            previous_omission = Some(&omission.code);
        }
        require_token("comparison_set.primary_claim_id", &self.primary_claim_id)?;

        let mut previous_state: Option<(&str, &str)> = None;
        for binding in &self.state_bindings {
            require_token("comparison_set.state_binding.kind", &binding.kind)?;
            require_token("comparison_set.state_binding.value", &binding.value)?;
            let current = (binding.kind.as_str(), binding.value.as_str());
            if previous_state.is_some_and(|previous| previous >= current) {
                return Err(
                    "comparison set state_bindings must be strictly ordered and unique".into(),
                );
            }
            previous_state = Some(current);
        }

        if self.expected_vantages.len() < 2 {
            return Err("a requested comparison set requires at least two vantages".into());
        }
        let mut previous: Option<(&str, &DiagnosticKey)> = None;
        let mut vantage_ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for expected in &self.expected_vantages {
            validate_semantic_identity(&expected.vantage, "expected_vantage.vantage")?;
            validate_key(&expected.key, "expected_vantage.key")?;
            if expected.key.question_id != self.question.id
                || expected.key.subject_id != self.subject.id
                || expected.key.profile_id != self.profile.id
                || expected.key.vantage_id != expected.vantage.id
            {
                return Err(
                    "expected vantage key does not bind the comparison subject/question/profile/vantage"
                        .into(),
                );
            }
            let current = (expected.vantage.id.as_str(), &expected.key);
            if previous.is_some_and(|prior| prior >= current) {
                return Err(
                    "expected_vantages must be strictly ordered by vantage id/key and unique"
                        .into(),
                );
            }
            previous = Some(current);
            if !vantage_ids.insert(expected.vantage.id.as_str()) {
                return Err("comparison set repeats a logical vantage identity".into());
            }
            if !keys.insert(&expected.key) {
                return Err("comparison set repeats a diagnostic key".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcordanceState {
    NotRequested,
    Insufficient,
    Concordant,
    Discordant,
    Uncomparable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NonContributionReason {
    RecordMissing,
    ExplicitRefusal,
    ProviderNoResponse,
    ReceiverSilence,
    AcquisitionFailed,
    NotConfigured,
    IncompatibleRecord,
    WrongGeneration,
    DuplicateExecution,
    PartialNqOutcome,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparableOutcome {
    pub outcome_id: String,
    pub claim_status: ClaimStatusV1,
    pub condition_effect: Option<ConditionV1>,
    pub derivation: DerivationV1,
    pub condition: ConditionV1,
    pub coherence: CoherenceV1,
    pub coverage: CoverageV1,
}

impl ComparableOutcome {
    fn from_artifact(artifact: &DiagnosticExecutionV1, claim_id: &str) -> Result<Self, String> {
        let claim = artifact
            .claims
            .iter()
            .find(|claim| claim.claim_id == claim_id)
            .ok_or_else(|| "comparison claim is not present in the NQ artifact".to_string())?;
        let mut outcome = Self {
            outcome_id: String::new(),
            claim_status: claim.status,
            condition_effect: claim.condition_effect,
            derivation: artifact.outcome.derivation,
            condition: artifact.outcome.condition,
            coherence: artifact.outcome.coherence,
            coverage: artifact.outcome.coverage,
        };
        outcome.outcome_id = computed_object_id(&outcome, "outcome_id")?;
        Ok(outcome)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Contribution {
    Contributing {
        comparable_outcome: ComparableOutcome,
    },
    NotContributing {
        reason: NonContributionReason,
        detail: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConcordanceMember {
    pub expected: ExpectedVantage,
    pub source_standing: Option<Standing>,
    pub recurrence_standing: Option<RecurrenceStanding>,
    pub artifact_id: Option<String>,
    pub request_id: Option<String>,
    pub run_id: Option<String>,
    pub contribution: Contribution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrossVantageConcordance {
    pub comparison_set_id: Option<String>,
    pub comparison_generation: Option<String>,
    pub state: ConcordanceState,
    pub expected_vantages: Vec<ExpectedVantage>,
    pub members: Vec<ConcordanceMember>,
    pub contributing_artifact_ids: Vec<String>,
    pub distinct_outcomes: Vec<ComparableOutcome>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationalPostureConcordance {
    pub schema: String,
    pub concordance_id: String,
    pub evaluator: SemanticIdentityV1,
    pub policy: ConcordancePolicy,
    pub source_posture_id: String,
    pub source_posture_generation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_import: Option<NqSourceImportReceipt>,
    pub source_posture: OperationalPosture,
    pub cross_vantage_concordance: CrossVantageConcordance,
}

impl OperationalPostureConcordance {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CONCORDANCE_SCHEMA {
            return Err(format!(
                "concordance schema must be {CONCORDANCE_SCHEMA}, got {}",
                self.schema
            ));
        }
        if self.evaluator != concordance_evaluator_identity() {
            return Err("concordance artifact binds an unknown evaluator identity".into());
        }
        self.policy.validate()?;
        if self.source_posture_id != self.source_posture.posture_id
            || self.source_posture_generation != self.source_posture.policy.generation
        {
            return Err("concordance artifact substitutes its source posture linkage".into());
        }
        if self.policy.posture_policy_id != self.source_posture.policy.policy_id
            || self.policy.posture_generation != self.source_posture.policy.generation
        {
            return Err("concordance policy does not bind its embedded source posture".into());
        }
        let evaluated_at = DateTime::parse_from_rfc3339(&self.source_posture.evaluated_at)
            .map_err(|error| format!("source posture evaluated_at is not RFC3339: {error}"))?
            .with_timezone(&Utc);
        let reopened_posture = crate::diagnostic_posture::evaluate_posture(
            &self.source_posture.policy,
            &self.source_posture.input_evidence,
            &self.source_posture.recurrence_evidence,
            evaluated_at,
        )?;
        if reopened_posture != self.source_posture {
            return Err(
                "embedded operational posture is not the exact deterministic v1 evaluation".into(),
            );
        }
        if let Some(receipt) = &self.source_import {
            receipt.validate_inputs(&self.source_posture.input_evidence)?;
        }
        let expected = match &self.policy.comparison_set {
            None => CrossVantageConcordance {
                comparison_set_id: None,
                comparison_generation: None,
                state: ConcordanceState::NotRequested,
                expected_vantages: vec![],
                members: vec![],
                contributing_artifact_ids: vec![],
                distinct_outcomes: vec![],
                warnings: vec![],
            },
            Some(set) => evaluate_set(&self.source_posture, set)?,
        };
        if expected != self.cross_vantage_concordance {
            return Err(
                "concordance state, members, outcomes, or warnings differ from evaluation".into(),
            );
        }
        validate_digest(&self.concordance_id, "concordance_id")?;
        if self.concordance_id != computed_object_id(self, "concordance_id")? {
            return Err("concordance_id does not match the canonical contract preimage".into());
        }
        Ok(())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, String> {
        let value: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("decode concordance contract: {error}"))?;
        value.validate()?;
        let canonical = serde_jcs::to_vec(&value).map_err(|error| error.to_string())?;
        if canonical != bytes {
            return Err("concordance contract bytes are not exact RFC 8785 canonical JSON".into());
        }
        Ok(value)
    }
}

pub fn concordance_evaluator_identity() -> SemanticIdentityV1 {
    SemanticIdentityV1 {
        id: "nightshift.cross_vantage_concordance_evaluator".into(),
        version: "1".into(),
        // sha256("nightshift.cross_vantage_concordance_evaluator.v1")
        digest: "sha256:e376d8f8cda9f1dfd906555a59d1c3a81e5643124beaa2959984c79848873af8".into(),
    }
}

pub fn evaluate_concordance(
    source: &OperationalPosture,
    policy: &ConcordancePolicy,
) -> Result<OperationalPostureConcordance, String> {
    evaluate_concordance_with_source(source, policy, None)
}

pub fn evaluate_concordance_with_source(
    source: &OperationalPosture,
    policy: &ConcordancePolicy,
    source_import: Option<NqSourceImportReceipt>,
) -> Result<OperationalPostureConcordance, String> {
    policy.validate()?;
    if source.policy.policy_id != policy.posture_policy_id {
        return Err("concordance policy does not bind the source posture policy".into());
    }
    if source.policy.generation != policy.posture_generation {
        return Err("concordance policy does not bind the source posture generation".into());
    }
    if let Some(receipt) = &source_import {
        receipt.validate()?;
        if receipt.imported_inputs_id != source.input_evidence.inputs_id {
            return Err("NQ source import receipt does not bind the source posture inputs".into());
        }
    }

    let concordance = match &policy.comparison_set {
        None => CrossVantageConcordance {
            comparison_set_id: None,
            comparison_generation: None,
            state: ConcordanceState::NotRequested,
            expected_vantages: vec![],
            members: vec![],
            contributing_artifact_ids: vec![],
            distinct_outcomes: vec![],
            warnings: vec![],
        },
        Some(set) => evaluate_set(source, set)?,
    };
    let mut result = OperationalPostureConcordance {
        schema: CONCORDANCE_SCHEMA.into(),
        concordance_id: String::new(),
        evaluator: concordance_evaluator_identity(),
        policy: policy.clone(),
        source_posture_id: source.posture_id.clone(),
        source_posture_generation: source.policy.generation.clone(),
        source_import,
        source_posture: source.clone(),
        cross_vantage_concordance: concordance,
    };
    result.concordance_id = computed_object_id(&result, "concordance_id")?;
    result.validate()?;
    Ok(result)
}

fn evaluate_set(
    source: &OperationalPosture,
    set: &ComparisonSet,
) -> Result<CrossVantageConcordance, String> {
    if source.policy.subject != set.subject {
        return Err("comparison set does not bind the source posture subject".into());
    }
    let inventory: BTreeMap<DiagnosticKey, _> = source
        .policy
        .inventory
        .iter()
        .map(|entry| {
            // ContractBinding::key is intentionally private to the posture
            // evaluator. Reconstruct only its public, exact four-field key.
            // This does not reinterpret the NQ result.
            let binding = &entry.binding;
            let key = DiagnosticKey {
                question_id: binding.question.id.clone(),
                subject_id: binding.subject.id.clone(),
                profile_id: binding.profile.id.clone(),
                vantage_id: binding.vantage.id.clone(),
            };
            (key, entry)
        })
        .collect();
    for expected in &set.expected_vantages {
        let Some(entry) = inventory.get(&expected.key) else {
            return Err(format!(
                "comparison member {:?} is outside the source closed inventory",
                expected.key
            ));
        };
        let binding = &entry.binding;
        let mut required_state_bindings: Vec<ComparableStateBinding> = entry
            .required_state_bindings
            .iter()
            .map(|required| ComparableStateBinding {
                kind: required.kind.clone(),
                value: required.value.clone(),
            })
            .collect();
        required_state_bindings.sort_by(|left, right| {
            (left.kind.as_bytes(), left.value.as_bytes())
                .cmp(&(right.kind.as_bytes(), right.value.as_bytes()))
        });
        if binding.subject != set.subject
            || binding.question != set.question
            || binding.profile != set.profile
            || binding.vantage != expected.vantage
            || binding.state_model != set.state_model
            || binding.evaluator != set.evaluator
            || binding.threshold_policy != set.threshold_policy
            || binding.projection != set.projection
            || binding.claim_id != set.primary_claim_id
            || required_state_bindings != set.state_bindings
            || entry.requirement == Requirement::Excluded
        {
            return Err(format!(
                "comparison member {:?} does not match the full closed-inventory semantic, claim, and state basis",
                expected.key
            ));
        }
    }

    let duplicates = duplicate_execution_keys(source, set);
    let mut members = Vec::with_capacity(set.expected_vantages.len());
    for expected in &set.expected_vantages {
        let entry = inventory
            .get(&expected.key)
            .expect("comparison inventory membership was checked above");
        members.push(evaluate_member(source, set, expected, entry, &duplicates)?);
    }

    let mut outcomes: Vec<ComparableOutcome> = members
        .iter()
        .filter_map(|member| match &member.contribution {
            Contribution::Contributing { comparable_outcome } => Some(comparable_outcome.clone()),
            Contribution::NotContributing { .. } => None,
        })
        .collect();
    outcomes.sort_by(|left, right| left.outcome_id.as_bytes().cmp(right.outcome_id.as_bytes()));
    outcomes.dedup_by(|left, right| left.outcome_id == right.outcome_id);

    let any_incompatible = members.iter().any(|member| {
        matches!(
            member.contribution,
            Contribution::NotContributing {
                reason: NonContributionReason::IncompatibleRecord
                    | NonContributionReason::WrongGeneration,
                ..
            }
        )
    });
    let all_contribute = members
        .iter()
        .all(|member| matches!(member.contribution, Contribution::Contributing { .. }));
    // Discordance wins over missing testimony and can never be majority
    // resolved. Two distinct admitted values are sufficient to preserve it.
    let state = if outcomes.len() >= 2 {
        ConcordanceState::Discordant
    } else if any_incompatible {
        ConcordanceState::Uncomparable
    } else if all_contribute && outcomes.len() == 1 {
        ConcordanceState::Concordant
    } else {
        ConcordanceState::Insufficient
    };

    let mut contributing_artifact_ids: Vec<String> = members
        .iter()
        .filter_map(|member| match member.contribution {
            Contribution::Contributing { .. } => member.artifact_id.clone(),
            Contribution::NotContributing { .. } => None,
        })
        .collect();
    contributing_artifact_ids.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let warnings = members
        .iter()
        .filter_map(|member| match &member.contribution {
            Contribution::NotContributing { reason, detail } => Some(format!(
                "{}: {:?}: {detail}",
                member.expected.vantage.id, reason
            )),
            Contribution::Contributing { .. } => None,
        })
        .collect();
    Ok(CrossVantageConcordance {
        comparison_set_id: Some(set.comparison_set_id.clone()),
        comparison_generation: Some(set.generation.clone()),
        state,
        expected_vantages: set.expected_vantages.clone(),
        members,
        contributing_artifact_ids,
        distinct_outcomes: outcomes,
        warnings,
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DuplicateKey {
    Artifact(String),
    Execution(String, String),
}

fn duplicate_execution_keys(
    source: &OperationalPosture,
    set: &ComparisonSet,
) -> BTreeSet<DuplicateKey> {
    let expected: BTreeSet<&DiagnosticKey> =
        set.expected_vantages.iter().map(|item| &item.key).collect();
    let mut counts: BTreeMap<DuplicateKey, usize> = BTreeMap::new();
    for input in &source.input_evidence.inputs {
        if !expected.contains(&input.key) {
            continue;
        }
        let DiagnosticInputStatus::Delivered { artifact } = &input.status else {
            continue;
        };
        *counts
            .entry(DuplicateKey::Artifact(artifact.artifact_id.clone()))
            .or_default() += 1;
        *counts
            .entry(DuplicateKey::Execution(
                artifact.producer.node_id.clone(),
                artifact.run_id.clone(),
            ))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect()
}

fn evaluate_member(
    source: &OperationalPosture,
    set: &ComparisonSet,
    expected: &ExpectedVantage,
    inventory_entry: &InventoryEntry,
    duplicates: &BTreeSet<DuplicateKey>,
) -> Result<ConcordanceMember, String> {
    let matching_inputs: Vec<&DiagnosticInput> = source
        .input_evidence
        .inputs
        .iter()
        .filter(|input| input.key == expected.key)
        .collect();
    let assessment = source
        .assessments
        .iter()
        .find(|assessment| assessment.key == expected.key);
    let recurrence = source
        .recurrence
        .iter()
        .find(|assessment| assessment.key == expected.key);
    let source_standing = assessment.map(|value| value.standing);
    let recurrence_standing = recurrence.map(|value| value.standing);
    let base = |artifact: Option<&DiagnosticExecutionV1>, contribution| ConcordanceMember {
        expected: expected.clone(),
        source_standing,
        recurrence_standing,
        artifact_id: artifact.map(|value| value.artifact_id.clone()),
        request_id: artifact.map(|value| value.request_id.clone()),
        run_id: artifact.map(|value| value.run_id.clone()),
        contribution,
    };

    let input = match matching_inputs.as_slice() {
        [] => {
            return Ok(base(
                None,
                not_contributing(
                    NonContributionReason::RecordMissing,
                    "no receiver record exists for the required vantage",
                ),
            ))
        }
        [input] => *input,
        _ => {
            return Ok(base(
                None,
                not_contributing(
                    NonContributionReason::DuplicateExecution,
                    "multiple receiver records bind the same required vantage",
                ),
            ))
        }
    };
    let artifact = match &input.status {
        DiagnosticInputStatus::NoResponse => {
            return Ok(base(
                None,
                not_contributing(
                    NonContributionReason::ReceiverSilence,
                    "Nightshift received no NQ artifact for this vantage",
                ),
            ))
        }
        DiagnosticInputStatus::AcquisitionFailed { reason } => {
            return Ok(base(
                None,
                not_contributing(NonContributionReason::AcquisitionFailed, reason),
            ))
        }
        DiagnosticInputStatus::NotConfigured => {
            return Ok(base(
                None,
                not_contributing(
                    NonContributionReason::NotConfigured,
                    "the required receiver path is not configured",
                ),
            ))
        }
        DiagnosticInputStatus::Delivered { artifact } => artifact.as_ref(),
    };

    if duplicates.contains(&DuplicateKey::Artifact(artifact.artifact_id.clone()))
        || duplicates.contains(&DuplicateKey::Execution(
            artifact.producer.node_id.clone(),
            artifact.run_id.clone(),
        ))
    {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::DuplicateExecution,
                "the same NQ artifact or producer/run identity occurs for multiple vantages",
            ),
        ));
    }

    // Establish that this is testimony for the declared comparison member
    // before interpreting any refusal or failure carried by the artifact.
    // Otherwise an artifact from a different profile, evaluator, or state
    // basis could be laundered into the declared member's failure reason.
    if !artifact_envelope_compatible(artifact, set, expected, inventory_entry)
        || matches!(
            source_standing,
            Some(Standing::FutureDated | Standing::DuplicateInput | Standing::Excluded)
        )
    {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::IncompatibleRecord,
                "the NQ contract/profile/semantic basis or source-posture binding does not match the declared comparison set",
            ),
        ));
    }
    if !recurrence_can_contribute(recurrence) {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::WrongGeneration,
                "the NQ execution is not bound to an admissible current recurrence generation",
            ),
        ));
    }

    if artifact.outcome.derivation == DerivationV1::Refused {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::ExplicitRefusal,
                "NQ explicitly refused the bounded diagnostic",
            ),
        ));
    }
    if artifact.outcome.derivation == DerivationV1::Unsupported {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::Unsupported,
                "NQ reports the bounded diagnostic as unsupported",
            ),
        ));
    }
    if artifact.outcome.derivation == DerivationV1::Partial
        && artifact
            .inputs
            .failed
            .iter()
            .any(|input| input.kind == crate::diagnostic_posture::FailedInputKindV1::NoResponse)
    {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::ProviderNoResponse,
                "the partial NQ artifact preserves provider no-response",
            ),
        ));
    }
    if artifact.outcome.derivation == DerivationV1::Partial {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::PartialNqOutcome,
                "the NQ derivation is partial",
            ),
        ));
    }

    if matches!(
        source_standing,
        Some(Standing::StateMismatch | Standing::BindingMismatch)
    ) {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::IncompatibleRecord,
                "the completed NQ result does not satisfy the source posture's required state basis",
            ),
        ));
    }

    if !artifact_comparable(artifact, set) {
        return Ok(base(
            Some(artifact),
            not_contributing(
                NonContributionReason::IncompatibleRecord,
                "the NQ primary claim or per-claim state basis does not match the declared comparison set",
            ),
        ));
    }

    let comparable = ComparableOutcome::from_artifact(artifact, &set.primary_claim_id)?;
    Ok(base(
        Some(artifact),
        Contribution::Contributing {
            comparable_outcome: comparable,
        },
    ))
}

fn recurrence_can_contribute(recurrence: Option<&RecurrenceAssessment>) -> bool {
    recurrence.is_some_and(|assessment| {
        matches!(
            assessment.standing,
            RecurrenceStanding::Current | RecurrenceStanding::Overdue
        )
    })
}

fn artifact_envelope_compatible(
    artifact: &DiagnosticExecutionV1,
    set: &ComparisonSet,
    expected: &ExpectedVantage,
    inventory_entry: &InventoryEntry,
) -> bool {
    let binding = &inventory_entry.binding;
    artifact.producer.node_id == binding.producer_node_id
        && artifact.producer.build == binding.producer_build
        && artifact.producer.cohort == binding.producer_cohort
        && artifact.subject == set.subject
        && artifact.question == set.question
        && artifact.profile == set.profile
        && artifact.vantage == expected.vantage
        && artifact.state_model == set.state_model
        && artifact.evaluator == set.evaluator
        && artifact.threshold_policy == set.threshold_policy
        && artifact.projection == set.projection
}

fn artifact_comparable(artifact: &DiagnosticExecutionV1, set: &ComparisonSet) -> bool {
    if artifact.primary_claim_id.as_deref() != Some(set.primary_claim_id.as_str()) {
        return false;
    }
    let Some(claim) = artifact
        .claims
        .iter()
        .find(|claim| claim.claim_id == set.primary_claim_id)
    else {
        return false;
    };
    let bindings_by_id: BTreeMap<&str, &StateBindingV1> = artifact
        .state_bindings
        .iter()
        .map(|binding| (binding.binding_id.as_str(), binding))
        .collect();
    let mut bindings: Vec<ComparableStateBinding> = claim
        .state_binding_ids
        .iter()
        .filter_map(|binding_id| bindings_by_id.get(binding_id.as_str()))
        .map(|binding| ComparableStateBinding {
            kind: binding.kind.clone(),
            value: binding.value.clone(),
        })
        .collect();
    bindings.sort_by(|left, right| {
        (left.kind.as_bytes(), left.value.as_bytes())
            .cmp(&(right.kind.as_bytes(), right.value.as_bytes()))
    });
    bindings == set.state_bindings
}

fn not_contributing(reason: NonContributionReason, detail: impl Into<String>) -> Contribution {
    Contribution::NotContributing {
        reason,
        detail: detail.into(),
    }
}

pub fn render_text(value: &OperationalPostureConcordance) -> String {
    let result = &value.cross_vantage_concordance;
    let mut output = String::new();
    output.push_str(&format!("concordance: {}\n", quoted(&value.concordance_id)));
    output.push_str(&format!(
        "source_posture: {}\n",
        quoted(&value.source_posture_id)
    ));
    output.push_str(&format!(
        "source_generation: {}\n",
        quoted(&value.source_posture_generation)
    ));
    if let Some(import) = &value.source_import {
        let package = &import.source_manifest.package;
        output.push_str(&format!(
            "source_manifest: {} receipt={}\n",
            quoted(&import.source_manifest.source_manifest_id),
            quoted(&import.receipt_id)
        ));
        output.push_str(&format!(
            "nq_package: repository={} commit={} release={} contract={} asset_manifest={} payload_manifest={}\n",
            quoted(&package.repository_identity),
            quoted(&package.commit),
            quoted(&package.release_identity),
            quoted(&package.contract_schema),
            quoted(&package.asset_manifest_sha256),
            quoted(&package.payload_manifest_sha256)
        ));
    }
    output.push_str(&format!(
        "comparison_set: {}\n",
        quoted(
            result
                .comparison_set_id
                .as_deref()
                .unwrap_or("not_requested")
        )
    ));
    output.push_str(&format!(
        "comparison_generation: {}\n",
        quoted(
            result
                .comparison_generation
                .as_deref()
                .unwrap_or("not_requested")
        )
    ));
    output.push_str(&format!("state: {:?}\n", result.state));
    output.push_str(&format!(
        "source_axes: headline={:?} completeness={:?} condition={:?} coverage={:?} recurrence={:?} current={}\n",
        value.source_posture.headline,
        value.source_posture.completeness,
        value.source_posture.condition,
        value.source_posture.coverage,
        value.source_posture.recurrence_axis,
        value.source_posture.current
    ));
    for member in &result.members {
        output.push_str(&format!(
            "vantage: {} key={}/{}/{}/{} source={:?} recurrence={:?}\n",
            quoted(&member.expected.vantage.id),
            quoted(&member.expected.key.question_id),
            quoted(&member.expected.key.subject_id),
            quoted(&member.expected.key.profile_id),
            quoted(&member.expected.key.vantage_id),
            member.source_standing,
            member.recurrence_standing
        ));
        if let Some(artifact_id) = &member.artifact_id {
            output.push_str(&format!(
                "  artifact: {} request={} run={}\n",
                quoted(artifact_id),
                quoted(member.request_id.as_deref().unwrap_or("none")),
                quoted(member.run_id.as_deref().unwrap_or("none"))
            ));
            if let Some(import) = &value.source_import {
                if let Some(source) = import
                    .source_manifest
                    .inputs
                    .iter()
                    .find(|source| source.key == member.expected.key)
                {
                    if let NqSourceStatus::Delivered {
                        artifact_sha256,
                        artifact_path,
                        ..
                    } = &source.status
                    {
                        output.push_str(&format!(
                            "  source_bytes: digest={} path={}\n",
                            quoted(artifact_sha256),
                            quoted(artifact_path)
                        ));
                    }
                }
            }
        }
        match &member.contribution {
            Contribution::Contributing {
                comparable_outcome,
            } => output.push_str(&format!(
                "  contribution: outcome={} claim={:?} condition_effect={:?} derivation={:?} condition={:?} coherence={:?} coverage={:?}\n",
                comparable_outcome.outcome_id,
                comparable_outcome.claim_status,
                comparable_outcome.condition_effect,
                comparable_outcome.derivation,
                comparable_outcome.condition,
                comparable_outcome.coherence,
                comparable_outcome.coverage
            )),
            Contribution::NotContributing { reason, detail } => {
                output.push_str(&format!(
                    "  noncontribution: reason={reason:?} detail={}\n",
                    quoted(detail)
                ));
            }
        }
    }
    output
}

fn quoted(value: &str) -> String {
    // JSON string syntax provides deterministic escaping for the human
    // projection, so admitted identifiers, paths, and failure details cannot
    // forge additional operator-output lines.
    serde_json::to_string(value).expect("serializing a Rust string cannot fail")
}

fn computed_object_id<T: Serialize>(value: &T, identity_field: &str) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    value
        .as_object_mut()
        .ok_or_else(|| "self-identified contract must serialize as an object".to_string())?
        .remove(identity_field);
    let canonical = serde_jcs::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn validate_key(key: &DiagnosticKey, field: &str) -> Result<(), String> {
    require_token(&format!("{field}.question_id"), &key.question_id)?;
    require_token(&format!("{field}.subject_id"), &key.subject_id)?;
    require_token(&format!("{field}.profile_id"), &key.profile_id)?;
    require_token(&format!("{field}.vantage_id"), &key.vantage_id)
}

fn validate_semantic_identity(identity: &SemanticIdentityV1, field: &str) -> Result<(), String> {
    require_token(&format!("{field}.id"), &identity.id)?;
    require_token(&format!("{field}.version"), &identity.version)?;
    validate_digest(&identity.digest, &format!("{field}.digest"))
}

fn validate_digest(value: &str, field: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!("{field} must use sha256:<64 lowercase hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must use sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn require_token(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} is empty"))
    } else {
        Ok(())
    }
}
