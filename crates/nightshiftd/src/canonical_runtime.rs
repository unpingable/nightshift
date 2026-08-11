//! One canonical Nightshift observation-cycle runtime.
//!
//! The runtime obtains qualified support, evaluates the complete NQ basis,
//! records non-authorizing posture/attention, and may ask AG to open one new
//! exact occurrence. It has no effect, standing, authorization, retry, Docket,
//! executor, or human-disposition API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::ag_port::{
    AgOccurrencePortV1, AgOpenModeV1, AgOpenOccurrenceRequestV1, AG_OPEN_REQUEST_SCHEMA_V1,
};
use crate::canonical_store::{
    AttentionClassV1, AttentionRecordV1, CanonicalStore, CanonicalStoreError, CycleStatusV1,
    ObservationCycleId, ObservationCycleV1, ObservationRecordV1, RecurrenceSlotV1, SlotTimingV1,
    TemporalDecisionV1, TemporalPostureV1, TypedCoarseIntentV1,
};
use crate::currentness::{
    delivered_artifact_ids, PresentEvidencePortV1, PresentEvidenceQueryV1, SupportStandingV1,
    TemporalHoldExpiryV1,
};
use crate::diagnostic_posture::{
    evaluate_posture_with_support, ConditionAxis, DiagnosticInputs, PosturePolicy,
    RecurrenceEvidence,
};

pub const CYCLE_REQUEST_SCHEMA_V1: &str = "nightshift.canonical_cycle_request.v1";
pub const PRECOMPILED_PROPOSAL_SCHEMA_V1: &str = "nightshift.precompiled_workflow_proposal.v1";

fn digest_value(value: &serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_jcs::to_vec(value).map_err(|error| error.to_string())?)
    ))
}

fn require_token(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(format!("{name} must be a non-empty token"));
    }
    Ok(())
}

fn require_digest(name: &str, value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!("{name} must use sha256:<64 lowercase hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{name} must use sha256:<64 lowercase hex>"));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum CanonicalRuntimeError {
    #[error(transparent)]
    Store(#[from] CanonicalStoreError),
    #[error("canonical cycle input is invalid: {0}")]
    Invalid(String),
    #[error("present-evidence authority refused or was unavailable: {0}")]
    PresentEvidence(String),
    #[error("diagnostic posture evaluation refused: {0}")]
    Diagnostic(String),
    #[error("AG boundary refused or was unavailable: {0}")]
    Ag(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalPolicyRequestV1 {
    pub policy_id: String,
    pub basis_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_expiry: Option<TemporalHoldExpiryV1>,
}

/// Already-compiled workflow-specific exact work. This adapter binds the
/// complete live Nightshift basis; it never interprets free text.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrecompiledWorkflowProposalV1 {
    pub schema: String,
    pub workflow_id: String,
    pub intent_kind: String,
    pub subject_digest: String,
    pub immutable_parameters: serde_json::Value,
    pub campaign_id: String,
    pub occurrence_id: String,
    pub mode: AgOpenModeV1,
    pub proposal_input: serde_json::Value,
}

impl PrecompiledWorkflowProposalV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != PRECOMPILED_PROPOSAL_SCHEMA_V1 {
            return Err("unsupported precompiled workflow proposal schema".into());
        }
        require_token("workflow_id", &self.workflow_id)?;
        require_token("intent_kind", &self.intent_kind)?;
        require_digest("subject_digest", &self.subject_digest)?;
        require_digest("campaign_id", &self.campaign_id)?;
        uuid::Uuid::parse_str(&self.occurrence_id)
            .map_err(|_| "occurrence_id must be an independently allocated UUID".to_string())?;
        if !self.immutable_parameters.is_object() {
            return Err("immutable_parameters must be an exact typed object".into());
        }
        if !self.proposal_input.is_object() {
            return Err("proposal_input must be an exact typed object".into());
        }
        let proposal = self
            .proposal_input
            .get("proposal")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "proposal_input must contain one exact proposal object".to_string())?;
        let work_schema = proposal
            .get("work_schema")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "exact proposal work_schema must be a string".to_string())?;
        let work = proposal
            .get("work")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "exact proposal work must be a digest".to_string())?;
        let compiled_payload = serde_json::json!({
            "parameters": &self.immutable_parameters,
            "schema": work_schema,
        });
        if work != digest_value(&compiled_payload)? {
            return Err(
                "exact proposal work digest does not bind the immutable compiled payload".into(),
            );
        }
        Ok(())
    }

    fn compile(
        &self,
        observation: &ObservationRecordV1,
    ) -> Result<(TypedCoarseIntentV1, AgOpenOccurrenceRequestV1), String> {
        self.validate()?;
        let intent = TypedCoarseIntentV1 {
            schema: String::new(),
            intent_id: String::new(),
            workflow_id: self.workflow_id.clone(),
            intent_kind: self.intent_kind.clone(),
            subject_id: observation.posture.policy.subject.id.clone(),
            subject_digest: self.subject_digest.clone(),
            scope_id: observation.posture.policy.subject.scope.digest.clone(),
            source_observation_id: observation.observation_id.clone(),
            source_support_id: observation.support.support_id.clone(),
            source_posture_id: observation.posture.posture_id.clone(),
            immutable_parameters: self.immutable_parameters.clone(),
        }
        .seal()
        .map_err(|error| error.to_string())?;
        let request = AgOpenOccurrenceRequestV1 {
            schema: AG_OPEN_REQUEST_SCHEMA_V1.into(),
            request_id: String::new(),
            campaign_id: self.campaign_id.clone(),
            occurrence_id: self.occurrence_id.clone(),
            subject_digest: self.subject_digest.clone(),
            scope_digest: observation.posture.policy.subject.scope.digest.clone(),
            source_observation_id: observation.observation_id.clone(),
            source_support_id: observation.support.support_id.clone(),
            source_posture_id: observation.posture.posture_id.clone(),
            source_intent_id: intent.intent_id.clone(),
            mode: self.mode.clone(),
            proposal_input: self.proposal_input.clone(),
        }
        .seal()?;
        request.validate_for(observation, &intent)?;
        Ok((intent, request))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCycleRequestV1 {
    pub schema: String,
    pub request_id: String,
    pub slot: RecurrenceSlotV1,
    pub scheduler_clock_id: String,
    pub evaluated_at: DateTime<Utc>,
    pub observation_id: String,
    pub policy: PosturePolicy,
    pub inputs: DiagnosticInputs,
    pub recurrence: RecurrenceEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_policy: Option<TemporalPolicyRequestV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<PrecompiledWorkflowProposalV1>,
}

impl CanonicalCycleRequestV1 {
    pub fn seal(mut self) -> Result<Self, String> {
        self.schema = CYCLE_REQUEST_SCHEMA_V1.into();
        self.request_id.clear();
        let mut value = serde_json::to_value(&self).map_err(|error| error.to_string())?;
        value
            .as_object_mut()
            .expect("cycle request is an object")
            .remove("request_id");
        self.request_id = digest_value(&value)?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CYCLE_REQUEST_SCHEMA_V1 {
            return Err("unsupported canonical cycle request schema".into());
        }
        require_digest("request_id", &self.request_id)?;
        require_digest("observation_id", &self.observation_id)?;
        require_token("scheduler_clock_id", &self.scheduler_clock_id)?;
        self.slot.validate().map_err(|error| error.to_string())?;
        self.policy.validate()?;
        self.inputs.validate()?;
        self.recurrence.validate()?;
        if self.slot.scheduler_clock_id != self.scheduler_clock_id
            || self.slot.subject_id != self.policy.subject.id
            || self.slot.scope_id != self.policy.subject.scope.digest
            || self.slot.policy_id != self.policy.policy_id
        {
            return Err("cycle slot does not exactly bind scheduler/policy/subject/scope".into());
        }
        if let Some(proposal) = &self.proposal {
            proposal.validate()?;
        }
        let mut value = serde_json::to_value(self).map_err(|error| error.to_string())?;
        value
            .as_object_mut()
            .expect("cycle request is an object")
            .remove("request_id");
        if self.request_id != digest_value(&value)? {
            return Err("request_id does not match exact cycle preimage".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CycleRunOutcomeV1 {
    Missed { cycle: ObservationCycleV1 },
    PostureOnly { cycle: ObservationCycleV1 },
    AgOccurrenceOpened { cycle: ObservationCycleV1 },
}

pub struct CanonicalRuntime<'a, P, A>
where
    P: PresentEvidencePortV1,
    A: AgOccurrencePortV1,
{
    store: &'a mut CanonicalStore,
    present_evidence: &'a mut P,
    ag: &'a mut A,
}

impl<'a, P, A> CanonicalRuntime<'a, P, A>
where
    P: PresentEvidencePortV1,
    A: AgOccurrencePortV1,
{
    pub fn new(store: &'a mut CanonicalStore, present_evidence: &'a mut P, ag: &'a mut A) -> Self {
        Self {
            store,
            present_evidence,
            ag,
        }
    }

    pub fn run_cycle(
        &mut self,
        request: CanonicalCycleRequestV1,
    ) -> Result<CycleRunOutcomeV1, CanonicalRuntimeError> {
        request.validate().map_err(CanonicalRuntimeError::Invalid)?;
        if request
            .slot
            .timing_at(&request.scheduler_clock_id, request.evaluated_at)?
            == SlotTimingV1::Missed
        {
            let cycle = self.store.record_missed(
                request.slot,
                &request.scheduler_clock_id,
                request.evaluated_at,
                "slot passed its exact latest-admissible instant".into(),
            )?;
            return Ok(CycleRunOutcomeV1::Missed { cycle });
        }
        let (claimed, lease) = self.store.claim_slot(
            request.slot,
            &request.scheduler_clock_id,
            request.evaluated_at,
        )?;
        let query = PresentEvidenceQueryV1 {
            schema: String::new(),
            query_id: String::new(),
            observation_cycle_id: claimed.cycle_id.as_str().into(),
            request_nonce: format!("support-query:{}", uuid::Uuid::new_v4()),
            observation_id: request.observation_id.clone(),
            diagnostic_inputs_id: request.inputs.inputs_id.clone(),
            subject_id: request.policy.subject.id.clone(),
            scope_id: request.policy.subject.scope.digest.clone(),
            artifact_ids: delivered_artifact_ids(&request.inputs),
        }
        .seal()
        .map_err(CanonicalRuntimeError::Invalid)?;
        let support = match self.present_evidence.resolve(&query) {
            Ok(value) => value,
            Err(error) => {
                let _ = self.store.mark_recovery_required(
                    &claimed.cycle_id,
                    &claimed.state_digest,
                    "present_evidence_unavailable".into(),
                    request.evaluated_at,
                );
                return Err(CanonicalRuntimeError::PresentEvidence(error));
            }
        };
        let posture = match evaluate_posture_with_support(
            &request.policy,
            &request.inputs,
            &request.recurrence,
            request.evaluated_at,
            &support,
        ) {
            Ok(value) => value,
            Err(error) => {
                let _ = self.store.mark_recovery_required(
                    &claimed.cycle_id,
                    &claimed.state_digest,
                    "diagnostic_posture_refused".into(),
                    request.evaluated_at,
                );
                return Err(CanonicalRuntimeError::Diagnostic(error));
            }
        };
        let observation = ObservationRecordV1 {
            schema: crate::canonical_store::OBSERVATION_RECORD_SCHEMA_V1.into(),
            observation_id: request.observation_id,
            support,
            posture,
        };
        let temporal = request
            .temporal_policy
            .map(|policy| {
                TemporalPostureV1::evaluate(
                    policy.policy_id,
                    policy.basis_digest,
                    policy.hold_expiry,
                    &request.scheduler_clock_id,
                    request.evaluated_at,
                )
            })
            .transpose()?;
        let attention = attention_for(&observation, temporal.as_ref());
        let temporal_hold = temporal
            .as_ref()
            .is_some_and(|value| value.decision == TemporalDecisionV1::Hold);
        let recorded = self.store.record_observation(
            &lease,
            &claimed.state_digest,
            observation.clone(),
            attention,
            temporal,
            request.evaluated_at,
        )?;
        let Some(precompiled) = request.proposal else {
            let cycle = self.store.close_without_proposal(
                &lease,
                &recorded.state_digest,
                request.evaluated_at,
            )?;
            return Ok(CycleRunOutcomeV1::PostureOnly { cycle });
        };
        if temporal_hold
            || !observation.posture.current
            || observation.support.standing != SupportStandingV1::Current
        {
            let cycle = self.store.close_without_proposal(
                &lease,
                &recorded.state_digest,
                request.evaluated_at,
            )?;
            return Ok(CycleRunOutcomeV1::PostureOnly { cycle });
        }
        let (intent, ag_request) = precompiled
            .compile(&observation)
            .map_err(CanonicalRuntimeError::Invalid)?;
        let prepared = ag_request.prepared()?;
        let pending = self.store.prepare_ag_occurrence(
            &lease,
            &recorded.state_digest,
            intent,
            prepared,
            request.evaluated_at,
        )?;
        let ag = match self.ag.open_occurrence(&ag_request) {
            Ok(value) => value,
            Err(error) => {
                let _ = self.store.mark_recovery_required(
                    &claimed.cycle_id,
                    &pending.state_digest,
                    "ag_status_required_no_resubmit".into(),
                    request.evaluated_at,
                );
                return Err(CanonicalRuntimeError::Ag(error));
            }
        };
        let cycle = self.store.attach_ag_occurrence(
            &lease,
            &pending.state_digest,
            ag,
            request.evaluated_at,
        )?;
        Ok(CycleRunOutcomeV1::AgOccurrenceOpened { cycle })
    }

    /// Read AG status only. A missing local AG response is reconciled by exact
    /// campaign/occurrence identity; the stored request is never resubmitted.
    pub fn sync_ag(
        &mut self,
        cycle_id: &ObservationCycleId,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalRuntimeError> {
        let cycle = self.store.get_cycle(cycle_id)?;
        let request = cycle.prepared_ag_request.as_ref().ok_or_else(|| {
            CanonicalRuntimeError::Invalid("cycle has no prepared AG request".into())
        })?;
        let status = match self.ag.status(&request.campaign_id, &request.occurrence_id) {
            Ok(value) => value,
            Err(error) => {
                if cycle.status != CycleStatusV1::RecoveryRequired {
                    let _ = self.store.mark_recovery_required(
                        cycle_id,
                        &cycle.state_digest,
                        "ag_status_unavailable_no_resubmit".into(),
                        now,
                    );
                }
                return Err(CanonicalRuntimeError::Ag(error));
            }
        };
        if cycle.ag.is_some() {
            Ok(self
                .store
                .record_ag_status(cycle_id, &cycle.state_digest, status, now)?)
        } else {
            Ok(self
                .store
                .recover_ag_occurrence(cycle_id, &cycle.state_digest, status, now)?)
        }
    }
}

fn attention_for(
    observation: &ObservationRecordV1,
    temporal: Option<&TemporalPostureV1>,
) -> AttentionRecordV1 {
    let (class, reason_code) = if matches!(
        observation.support.standing,
        SupportStandingV1::Contradictory | SupportStandingV1::Blind
    ) {
        (
            AttentionClassV1::EscalationRequested,
            "support_not_decidable",
        )
    } else if observation.support.standing != SupportStandingV1::Current {
        (AttentionClassV1::AttentionRequired, "support_not_current")
    } else if temporal.is_some_and(|value| {
        value.decision == crate::canonical_store::TemporalDecisionV1::Attention
    }) {
        (AttentionClassV1::AttentionRequired, "temporal_hold_expired")
    } else if observation.posture.condition == ConditionAxis::ConditionPresent {
        (AttentionClassV1::AttentionRequired, "condition_present")
    } else {
        (AttentionClassV1::Display, "posture_observed")
    };
    AttentionRecordV1 {
        class,
        source_posture_id: observation.posture.posture_id.clone(),
        reason_code: reason_code.into(),
        display_text: Some(format!("{:?}", observation.posture.headline)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ag_port::parse_ag_refusal;
    use crate::canonical_store::{
        AgOccurrenceReferenceV1, AgProgramCounterV1, RecurrenceTriggerV1, AG_REFERENCE_SCHEMA_V1,
    };
    use crate::currentness::{
        PresentEvidenceQueryV1, QualifiedSupportV1, SupportExpiryV1, SupportReceiverInstantV1,
    };
    use crate::diagnostic_posture::Headline;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn time(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn policy_inputs_recurrence() -> (PosturePolicy, DiagnosticInputs, RecurrenceEvidence) {
        (
            serde_json::from_str(include_str!(
                "../../../docs/operator/examples/diagnostic-posture-v1/policy.json"
            ))
            .unwrap(),
            serde_json::from_str(include_str!(
                "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
            ))
            .unwrap(),
            serde_json::from_str(include_str!(
                "../../../docs/operator/examples/diagnostic-posture-v1/recurrence.json"
            ))
            .unwrap(),
        )
    }

    fn cycle_request(occurrence: u64, proposal: bool) -> CanonicalCycleRequestV1 {
        let (policy, inputs, recurrence) = policy_inputs_recurrence();
        let campaign = digest('a');
        let scope = policy.subject.scope.digest.clone();
        let occurrence_id = format!("00000000-0000-4000-8000-{occurrence:012}");
        let slot = RecurrenceSlotV1::new(
            policy.policy_id.clone(),
            "config-v1".into(),
            policy.subject.id.clone(),
            policy.subject.scope.digest.clone(),
            "nightshift-scheduler-1".into(),
            time("2026-07-27T20:00:00Z") + chrono::Duration::minutes(occurrence as i64),
            time("2026-07-27T20:00:30Z") + chrono::Duration::minutes(occurrence as i64),
            occurrence,
            RecurrenceTriggerV1::Scheduled,
            None,
        )
        .unwrap();
        let immutable_parameters = serde_json::json!({"resource_id":"resource-1"});
        let work_schema = "example.exact-work/v1";
        let work = digest_value(&serde_json::json!({
            "parameters": &immutable_parameters,
            "schema": work_schema,
        }))
        .unwrap();
        CanonicalCycleRequestV1 {
            schema: String::new(),
            request_id: String::new(),
            slot,
            scheduler_clock_id: "nightshift-scheduler-1".into(),
            evaluated_at: time("2026-07-27T20:00:10Z")
                + chrono::Duration::minutes(occurrence as i64),
            observation_id: digest(char::from(b'd' + occurrence as u8)),
            policy,
            inputs,
            recurrence,
            temporal_policy: None,
            proposal: proposal.then(|| PrecompiledWorkflowProposalV1 {
                schema: PRECOMPILED_PROPOSAL_SCHEMA_V1.into(),
                workflow_id: "workflow:host-care".into(),
                intent_kind: "inspect_exact_resource".into(),
                subject_digest: digest('b'),
                immutable_parameters,
                campaign_id: campaign.clone(),
                occurrence_id: occurrence_id.clone(),
                mode: AgOpenModeV1::Genesis {
                    genesis: serde_json::json!({
                        "campaign": campaign.clone(),
                        "occurrence": occurrence_id,
                        "program": digest('2'),
                        "residuals": [],
                        "budget": {"retry_limit":1,"retries_used":0,"probe_limit":1,"probes_used":0,"escalation_limit":1,"escalations_used":0}
                    }),
                },
                proposal_input: serde_json::json!({
                    "observation": digest(char::from(b'd' + occurrence as u8)),
                    "proposal": {
                        "schema":"ag.governed-loop.exact-work-proposal/v1",
                        "campaign":campaign,
                        "subject":digest('b'),
                        "scope":scope,
                        "work_schema":work_schema,
                        "work":work,
                        "repair":null
                    },
                    "class":"initial"
                }),
            }),
        }
        .seal()
        .unwrap()
    }

    struct CurrentSupportPort {
        standing: SupportStandingV1,
    }

    struct UnavailableSupportPort;

    impl PresentEvidencePortV1 for UnavailableSupportPort {
        fn resolve(&mut self, _: &PresentEvidenceQueryV1) -> Result<QualifiedSupportV1, String> {
            Err("qualified currentness unavailable".into())
        }
    }

    impl Default for CurrentSupportPort {
        fn default() -> Self {
            Self {
                standing: SupportStandingV1::Current,
            }
        }
    }

    impl PresentEvidencePortV1 for CurrentSupportPort {
        fn resolve(
            &mut self,
            query: &PresentEvidenceQueryV1,
        ) -> Result<QualifiedSupportV1, String> {
            let mut support = QualifiedSupportV1 {
                schema: crate::currentness::QUALIFIED_SUPPORT_SCHEMA_V1.into(),
                support_id: String::new(),
                authority_id: "pulse-receiver-1".into(),
                query_id: query.query_id.clone(),
                observation_cycle_id: query.observation_cycle_id.clone(),
                request_nonce: query.request_nonce.clone(),
                observation_id: query.observation_id.clone(),
                diagnostic_inputs_id: query.diagnostic_inputs_id.clone(),
                subject_id: query.subject_id.clone(),
                scope_id: query.scope_id.clone(),
                artifact_ids: query.artifact_ids.clone(),
                evaluated_at: SupportReceiverInstantV1 {
                    clock_id: "pulse-receiver-clock-1".into(),
                    tick: 100,
                },
                expiry: (self.standing == SupportStandingV1::Current).then(|| SupportExpiryV1 {
                    clock_id: "pulse-receiver-clock-1".into(),
                    tick: 101,
                }),
                standing: self.standing,
                evidence_refs: vec![digest('9')],
                contradiction_refs: Vec::new(),
            };
            support.support_id = support.computed_support_id()?;
            support.validate_for(query)?;
            Ok(support)
        }
    }

    struct FakeAg {
        open_count: usize,
        lose_open_response: bool,
        status_pc: AgProgramCounterV1,
        request: Option<AgOpenOccurrenceRequestV1>,
    }

    impl Default for FakeAg {
        fn default() -> Self {
            Self {
                open_count: 0,
                lose_open_response: false,
                status_pc: AgProgramCounterV1::ProposalRecorded,
                request: None,
            }
        }
    }

    fn fake_reference(
        campaign: &str,
        occurrence: &str,
        pc: AgProgramCounterV1,
    ) -> AgOccurrenceReferenceV1 {
        let docket_attempt_id = matches!(
            pc,
            AgProgramCounterV1::Dispatched
                | AgProgramCounterV1::ReconciliationRequired
                | AgProgramCounterV1::SettledObservationRequired
        )
        .then(|| digest('8'));
        let settlement_id =
            (pc == AgProgramCounterV1::SettledObservationRequired).then(|| digest('9'));
        let exact_snapshot = serde_json::json!({
            "fake_canonical_ag_snapshot": true,
            "campaign": campaign,
            "occurrence": occurrence,
            "program_counter": pc,
            "docket_attempt": docket_attempt_id,
            "settlement": settlement_id,
        });
        AgOccurrenceReferenceV1 {
            schema: AG_REFERENCE_SCHEMA_V1.into(),
            campaign_id: campaign.into(),
            occurrence_id: occurrence.into(),
            state_digest: digest('7'),
            snapshot_digest: digest_value(&exact_snapshot).unwrap(),
            program_counter: pc,
            docket_attempt_id,
            settlement_id,
            external_decision_request_id: None,
            exact_snapshot,
        }
    }

    impl AgOccurrencePortV1 for FakeAg {
        fn open_occurrence(
            &mut self,
            request: &AgOpenOccurrenceRequestV1,
        ) -> Result<AgOccurrenceReferenceV1, String> {
            self.open_count += 1;
            self.request = Some(request.clone());
            if self.lose_open_response {
                return Err("AG response lost after exact occurrence creation".into());
            }
            Ok(fake_reference(
                &request.campaign_id,
                &request.occurrence_id,
                AgProgramCounterV1::ProposalRecorded,
            ))
        }

        fn status(
            &mut self,
            campaign_id: &str,
            occurrence_id: &str,
        ) -> Result<AgOccurrenceReferenceV1, String> {
            Ok(fake_reference(campaign_id, occurrence_id, self.status_pc))
        }
    }

    #[test]
    fn one_cycle_preserves_complete_basis_and_closes_without_authority() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let outcome = CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(cycle_request(0, false))
            .unwrap();
        let CycleRunOutcomeV1::PostureOnly { cycle } = outcome else {
            panic!("expected posture-only cycle")
        };
        assert_eq!(cycle.status, CycleStatusV1::Closed);
        assert!(cycle.observation.unwrap().posture.present_support.is_some());
        assert_eq!(ag.open_count, 0);
    }

    #[test]
    fn same_nq_generation_never_suppresses_a_distinct_slot() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        runtime.run_cycle(cycle_request(0, false)).unwrap();
        runtime.run_cycle(cycle_request(1, false)).unwrap();
        assert_eq!(runtime.store.list_cycles().unwrap().len(), 2);
    }

    #[test]
    fn recurrence_cannot_reuse_an_existing_ag_occurrence() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        runtime.run_cycle(cycle_request(0, true)).unwrap();

        let mut second = cycle_request(1, true);
        second.slot = RecurrenceSlotV1::new(
            second.policy.policy_id.clone(),
            "config-v1".into(),
            second.policy.subject.id.clone(),
            second.policy.subject.scope.digest.clone(),
            "nightshift-scheduler-1".into(),
            time("2026-07-27T20:00:00Z"),
            time("2026-07-27T20:00:30Z"),
            1,
            RecurrenceTriggerV1::Manual,
            None,
        )
        .unwrap();
        second.evaluated_at = time("2026-07-27T20:00:10Z");
        let first_occurrence = "00000000-0000-4000-8000-000000000000";
        let proposal = second.proposal.as_mut().unwrap();
        proposal.occurrence_id = first_occurrence.into();
        proposal.mode = AgOpenModeV1::Genesis {
            genesis: serde_json::json!({
                "campaign": proposal.campaign_id,
                "occurrence": first_occurrence,
                "program": digest('2'),
                "residuals": [],
                "budget": {"retry_limit":1,"retries_used":0,"probe_limit":1,"probes_used":0,"escalation_limit":1,"escalations_used":0}
            }),
        };
        second = second.seal().unwrap();

        assert!(runtime.run_cycle(second).is_err());
        assert_eq!(runtime.ag.open_count, 1);
    }

    #[test]
    fn settlement_changes_only_external_status_and_requires_reobservation() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        let outcome = runtime.run_cycle(cycle_request(0, true)).unwrap();
        let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } = outcome else {
            panic!("expected AG occurrence")
        };
        let observation = cycle.observation.clone();
        runtime.ag.status_pc = AgProgramCounterV1::SettledObservationRequired;
        let settled = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
            .unwrap();
        assert_eq!(settled.status, CycleStatusV1::ObservationRequired);
        assert_eq!(settled.observation, observation);
        assert_eq!(
            settled.ag.as_ref().unwrap().docket_attempt_id,
            Some(digest('8'))
        );
        assert_eq!(
            settled.ag.as_ref().unwrap().settlement_id,
            Some(digest('9'))
        );
        let replayed = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:21Z"))
            .unwrap();
        assert_eq!(replayed.state_digest, settled.state_digest);
        assert_eq!(replayed.version, settled.version);
        assert_eq!(runtime.ag.open_count, 1);
    }

    #[test]
    fn indeterminate_status_never_reopens_or_repeats() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        let outcome = runtime.run_cycle(cycle_request(0, true)).unwrap();
        let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } = outcome else {
            panic!("expected AG occurrence")
        };
        runtime.ag.status_pc = AgProgramCounterV1::ReconciliationRequired;
        let first = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
            .unwrap();
        let second = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:21Z"))
            .unwrap();
        assert_eq!(first.status, CycleStatusV1::AwaitingAgReconciliation);
        assert_eq!(second.status, CycleStatusV1::AwaitingAgReconciliation);
        assert_eq!(ag.open_count, 1);
    }

    #[test]
    fn stale_ag_program_counter_cannot_replace_a_settlement() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        let outcome = runtime.run_cycle(cycle_request(0, true)).unwrap();
        let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } = outcome else {
            panic!("expected AG occurrence")
        };
        runtime.ag.status_pc = AgProgramCounterV1::SettledObservationRequired;
        runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
            .unwrap();
        runtime.ag.status_pc = AgProgramCounterV1::ProposalRecorded;
        assert!(runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:21Z"))
            .is_err());
    }

    #[test]
    fn lost_ag_open_response_recovers_by_status_without_resubmission() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("ns.sqlite");
        let cycle_id = {
            let mut store = CanonicalStore::open(&database).unwrap();
            let mut support = CurrentSupportPort::default();
            let mut ag = FakeAg {
                lose_open_response: true,
                ..FakeAg::default()
            };
            assert!(CanonicalRuntime::new(&mut store, &mut support, &mut ag)
                .run_cycle(cycle_request(0, true))
                .is_err());
            assert_eq!(ag.open_count, 1);
            let cycle = store.list_cycles().unwrap().pop().unwrap();
            assert_eq!(cycle.status, CycleStatusV1::RecoveryRequired);
            assert!(cycle.prepared_ag_request.is_some());
            assert!(cycle.ag.is_none());
            cycle.cycle_id
        };

        let mut restarted = CanonicalStore::open(&database).unwrap();
        let candidates = restarted
            .recover_after_restart(time("2026-07-27T20:00:20Z"))
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].cycle_id, cycle_id);
        let mut support = UnavailableSupportPort;
        let mut ag = FakeAg::default();
        let recovered = CanonicalRuntime::new(&mut restarted, &mut support, &mut ag)
            .sync_ag(&cycle_id, time("2026-07-27T20:00:21Z"))
            .unwrap();
        assert_eq!(recovered.status, CycleStatusV1::AwaitingAg);
        assert!(recovered.ag.is_some());
        assert_eq!(ag.open_count, 0);
    }

    #[test]
    fn restart_after_posture_recording_erases_live_proposal_eligibility() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("ns.sqlite");
        let cycle_id = {
            let request = cycle_request(0, true);
            let mut store = CanonicalStore::open(&database).unwrap();
            let (claimed, lease) = store
                .claim_slot(
                    request.slot.clone(),
                    &request.scheduler_clock_id,
                    request.evaluated_at,
                )
                .unwrap();
            let query = PresentEvidenceQueryV1 {
                schema: String::new(),
                query_id: String::new(),
                observation_cycle_id: claimed.cycle_id.as_str().into(),
                request_nonce: "support-query:crash-cut".into(),
                observation_id: request.observation_id.clone(),
                diagnostic_inputs_id: request.inputs.inputs_id.clone(),
                subject_id: request.policy.subject.id.clone(),
                scope_id: request.policy.subject.scope.digest.clone(),
                artifact_ids: delivered_artifact_ids(&request.inputs),
            }
            .seal()
            .unwrap();
            let mut port = CurrentSupportPort::default();
            let support = port.resolve(&query).unwrap();
            let posture = evaluate_posture_with_support(
                &request.policy,
                &request.inputs,
                &request.recurrence,
                request.evaluated_at,
                &support,
            )
            .unwrap();
            assert!(posture.current);
            let observation = ObservationRecordV1 {
                schema: crate::canonical_store::OBSERVATION_RECORD_SCHEMA_V1.into(),
                observation_id: request.observation_id,
                support,
                posture,
            };
            let recorded = store
                .record_observation(
                    &lease,
                    &claimed.state_digest,
                    observation.clone(),
                    attention_for(&observation, None),
                    None,
                    request.evaluated_at,
                )
                .unwrap();
            assert_eq!(recorded.status, CycleStatusV1::PostureRecorded);
            recorded.cycle_id
        };

        let mut restarted = CanonicalStore::open(&database).unwrap();
        let recovered = restarted
            .recover_after_restart(time("2026-07-27T20:00:20Z"))
            .unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].cycle_id, cycle_id);
        assert_eq!(recovered[0].status, CycleStatusV1::RecoveryRequired);
        assert!(recovered[0].prepared_ag_request.is_none());
        assert!(recovered[0].intent.is_none());
    }

    #[test]
    fn restart_preserves_reconciliation_without_repeat() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("ns.sqlite");
        let cycle_id = {
            let mut store = CanonicalStore::open(&database).unwrap();
            let mut support = CurrentSupportPort::default();
            let mut ag = FakeAg::default();
            let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
            let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } =
                runtime.run_cycle(cycle_request(0, true)).unwrap()
            else {
                panic!("expected AG occurrence")
            };
            runtime.ag.status_pc = AgProgramCounterV1::ReconciliationRequired;
            let reconciliating = runtime
                .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
                .unwrap();
            assert_eq!(
                reconciliating.status,
                CycleStatusV1::AwaitingAgReconciliation
            );
            reconciliating.cycle_id
        };

        let mut restarted = CanonicalStore::open(&database).unwrap();
        let candidates = restarted
            .recover_after_restart(time("2026-07-27T20:00:21Z"))
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].status,
            CycleStatusV1::AwaitingAgReconciliation
        );
        let mut support = UnavailableSupportPort;
        let mut ag = FakeAg {
            status_pc: AgProgramCounterV1::ReconciliationRequired,
            ..FakeAg::default()
        };
        let unchanged = CanonicalRuntime::new(&mut restarted, &mut support, &mut ag)
            .sync_ag(&cycle_id, time("2026-07-27T20:00:22Z"))
            .unwrap();
        assert_eq!(unchanged.status, CycleStatusV1::AwaitingAgReconciliation);
        assert_eq!(ag.open_count, 0);
    }

    #[test]
    fn settled_restart_requires_a_new_observation_cycle() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("ns.sqlite");
        let cycle_id = {
            let mut store = CanonicalStore::open(&database).unwrap();
            let mut support = CurrentSupportPort::default();
            let mut ag = FakeAg::default();
            let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
            let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } =
                runtime.run_cycle(cycle_request(0, true)).unwrap()
            else {
                panic!("expected AG occurrence")
            };
            runtime.ag.status_pc = AgProgramCounterV1::SettledObservationRequired;
            runtime
                .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
                .unwrap()
                .cycle_id
        };

        let mut restarted = CanonicalStore::open(&database).unwrap();
        assert!(restarted
            .recover_after_restart(time("2026-07-27T20:00:21Z"))
            .unwrap()
            .is_empty());
        let historical = restarted.get_cycle(&cycle_id).unwrap();
        assert_eq!(historical.status, CycleStatusV1::ObservationRequired);
        assert!(historical.ag.as_ref().unwrap().settlement_id.is_some());
    }

    #[test]
    fn halted_and_completed_ag_states_remain_external_and_non_effecting() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut runtime = CanonicalRuntime::new(&mut store, &mut support, &mut ag);
        let outcome = runtime.run_cycle(cycle_request(0, true)).unwrap();
        let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } = outcome else {
            panic!("expected AG occurrence")
        };
        let observation = cycle.observation.clone();

        runtime.ag.status_pc = AgProgramCounterV1::Halted;
        let halted = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:20Z"))
            .unwrap();
        assert_eq!(halted.status, CycleStatusV1::Halted);
        assert_eq!(halted.observation.as_ref(), observation.as_ref());

        runtime.ag.status_pc = AgProgramCounterV1::Completed;
        let completed = runtime
            .sync_ag(&cycle.cycle_id, time("2026-07-27T20:00:21Z"))
            .unwrap();
        assert_eq!(completed.status, CycleStatusV1::Closed);
        assert_eq!(completed.observation.as_ref(), observation.as_ref());
        assert_eq!(runtime.ag.open_count, 1);
    }

    #[test]
    fn ag_refusal_requires_the_exact_observed_predecessor() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let outcome = CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(cycle_request(0, true))
            .unwrap();
        let CycleRunOutcomeV1::AgOccurrenceOpened { cycle } = outcome else {
            panic!("expected AG occurrence")
        };
        let ag_reference = cycle.ag.as_ref().unwrap();
        let campaign_id = ag_reference.campaign_id.clone();
        let occurrence_id = ag_reference.occurrence_id.clone();
        let exact = serde_json::json!({
            "key": {
                "campaign": campaign_id,
                "occurrence": occurrence_id,
            },
            "at_state_digest": ag_reference.state_digest,
            "code": "stale_observation",
            "evidence": null,
        });
        let refusal = parse_ag_refusal(
            exact.clone(),
            &ag_reference.campaign_id,
            &ag_reference.occurrence_id,
        )
        .unwrap();
        let mut substituted_exact = exact;
        substituted_exact["at_state_digest"] = serde_json::json!(digest('6'));
        let substituted = parse_ag_refusal(
            substituted_exact,
            &ag_reference.campaign_id,
            &ag_reference.occurrence_id,
        )
        .unwrap();
        assert!(store
            .record_ag_refusal(
                &cycle.cycle_id,
                &cycle.state_digest,
                substituted,
                time("2026-07-27T20:00:20Z"),
            )
            .is_err());
        let closed = store
            .record_ag_refusal(
                &cycle.cycle_id,
                &cycle.state_digest,
                refusal,
                time("2026-07-27T20:00:21Z"),
            )
            .unwrap();
        assert_eq!(closed.status, CycleStatusV1::Closed);
        assert!(closed.ag_refusal.is_some());
    }

    #[test]
    fn attention_and_headline_never_construct_an_ag_request() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let outcome = CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(cycle_request(0, false))
            .unwrap();
        let CycleRunOutcomeV1::PostureOnly { cycle } = outcome else {
            panic!("expected posture-only cycle")
        };
        assert_eq!(cycle.headline(), Some(Headline::Clean));
        assert!(cycle.intent.is_none());
        assert!(cycle.prepared_ag_request.is_none());
        assert!(cycle.ag.is_none());
    }

    #[test]
    fn immutable_work_parameters_cannot_be_substituted_after_compilation() {
        let mut request = cycle_request(0, true);
        request.proposal.as_mut().unwrap().immutable_parameters["resource_id"] =
            serde_json::json!("different-resource");
        assert!(request.seal().is_err());
    }

    #[test]
    fn active_temporal_hold_suppresses_work_without_minting_authority() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort::default();
        let mut ag = FakeAg::default();
        let mut request = cycle_request(0, true);
        request.temporal_policy = Some(TemporalPolicyRequestV1 {
            policy_id: "temporal:host-care".into(),
            basis_digest: digest('5'),
            hold_expiry: Some(TemporalHoldExpiryV1 {
                scheduler_clock_id: request.scheduler_clock_id.clone(),
                at: time("2026-07-27T20:00:11Z"),
            }),
        });
        request = request.seal().unwrap();
        let outcome = CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(request)
            .unwrap();
        let CycleRunOutcomeV1::PostureOnly { cycle } = outcome else {
            panic!("active temporal hold must remain posture-only")
        };
        assert_eq!(
            cycle.temporal_posture.unwrap().decision,
            TemporalDecisionV1::Hold
        );
        assert!(cycle.intent.is_none());
        assert_eq!(ag.open_count, 0);
    }

    #[test]
    fn authority_owned_unknown_support_suppresses_precompiled_work() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = CurrentSupportPort {
            standing: SupportStandingV1::Unknown,
        };
        let mut ag = FakeAg::default();
        let outcome = CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(cycle_request(0, true))
            .unwrap();
        let CycleRunOutcomeV1::PostureOnly { cycle } = outcome else {
            panic!("unknown support must produce posture only")
        };
        assert_eq!(cycle.status, CycleStatusV1::Closed);
        assert!(!cycle.observation.as_ref().unwrap().posture.current);
        assert!(cycle.intent.is_none());
        assert_eq!(ag.open_count, 0);
    }

    #[test]
    fn historical_timestamps_cannot_substitute_for_qualified_currentness() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = CanonicalStore::open(directory.path().join("ns.sqlite")).unwrap();
        let mut support = UnavailableSupportPort;
        let mut ag = FakeAg::default();
        assert!(CanonicalRuntime::new(&mut store, &mut support, &mut ag)
            .run_cycle(cycle_request(0, true))
            .is_err());
        let cycles = store.list_cycles().unwrap();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].status, CycleStatusV1::RecoveryRequired);
        assert!(cycles[0].observation.is_none());
        assert_eq!(ag.open_count, 0);
    }
}
