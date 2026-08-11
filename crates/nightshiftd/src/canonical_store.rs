//! Transactional store for canonical Nightshift recurrence and observation cycles.
//!
//! This is a temporal/run lifecycle, not an effect campaign FSM. AG remains the
//! sole owner of occurrence governance. The store retains complete evidence and
//! exact external references while deliberately retaining no reconstructible
//! live-currentness token.

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension as _, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::currentness::{
    QualifiedSupportV1, RecurrenceLatestAdmissibleV1, SupportStandingV1, TemporalHoldExpiryV1,
};
use crate::diagnostic_posture::{Headline, OperationalPosture};

pub const SLOT_SCHEMA_V1: &str = "nightshift.recurrence_slot.v1";
pub const CYCLE_SCHEMA_V1: &str = "nightshift.observation_cycle.v1";
pub const OBSERVATION_RECORD_SCHEMA_V1: &str = "nightshift.observation_record.v1";
pub const TYPED_INTENT_SCHEMA_V1: &str = "nightshift.typed_coarse_intent.v1";
pub const AG_REFERENCE_SCHEMA_V1: &str = "nightshift.ag_occurrence_reference.v1";
pub const AG_REFUSAL_SCHEMA_V1: &str = "nightshift.ag_refusal_reference.v1";
pub const PREPARED_AG_REQUEST_SCHEMA_V1: &str = "nightshift.prepared_ag_request.v1";

fn require_token(name: &str, value: &str) -> Result<(), CanonicalStoreError> {
    if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
        return Err(CanonicalStoreError::Invalid(format!(
            "{name} must be a non-empty token"
        )));
    }
    Ok(())
}

fn require_digest(name: &str, value: &str) -> Result<(), CanonicalStoreError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(CanonicalStoreError::Invalid(format!(
            "{name} must use sha256:<64 lowercase hex>"
        )));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CanonicalStoreError::Invalid(format!(
            "{name} must use sha256:<64 lowercase hex>"
        )));
    }
    Ok(())
}

fn object_id<T: Serialize>(value: &T, field: &str) -> Result<String, CanonicalStoreError> {
    let mut value = serde_json::to_value(value)?;
    value
        .as_object_mut()
        .ok_or_else(|| CanonicalStoreError::Invalid("identity preimage is not an object".into()))?
        .remove(field);
    let canonical = serde_jcs::to_vec(&value)
        .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<String, CanonicalStoreError> {
    String::from_utf8(
        serde_jcs::to_vec(value)
            .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))?,
    )
    .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

#[derive(Debug, thiserror::Error)]
pub enum CanonicalStoreError {
    #[error("canonical store SQL error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("canonical store JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid canonical record: {0}")]
    Invalid(String),
    #[error("recurrence slot already has an authoritative attempt: {0}")]
    DuplicateSlot(String),
    #[error("AG campaign/occurrence is already bound to another observation cycle: {0}/{1}")]
    DuplicateAgOccurrence(String, String),
    #[error("recurrence slot is not due: {0}")]
    NotDue(String),
    #[error("recurrence slot is outside its latest-admissible instant: {0}")]
    Missed(String),
    #[error("observation cycle not found: {0}")]
    CycleNotFound(String),
    #[error("stale predecessor digest")]
    StalePredecessor,
    #[error("illegal observation-cycle transition: {0}")]
    IllegalTransition(String),
    #[error("live cycle lease does not bind this cycle")]
    WrongLiveLease,
    #[error("deterministic replay failed: {0}")]
    Replay(String),
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ObservationCycleId(String);

impl ObservationCycleId {
    pub fn fresh() -> Self {
        Self(format!("cycle:{}", Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(value: String) -> Result<Self, CanonicalStoreError> {
        require_token("observation_cycle_id", &value)?;
        let Some(uuid) = value.strip_prefix("cycle:") else {
            return Err(CanonicalStoreError::Invalid(
                "observation_cycle_id must use the cycle:<uuid> namespace".into(),
            ));
        };
        uuid::Uuid::parse_str(uuid).map_err(|_| {
            CanonicalStoreError::Invalid("observation_cycle_id contains an invalid UUID".into())
        })?;
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RecurrenceSlotId(String);

impl RecurrenceSlotId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceTriggerV1 {
    Scheduled,
    Manual,
    Event,
    CatchUp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotTimingV1 {
    OnTime,
    Late,
    CatchUp,
    Missed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecurrenceSlotV1 {
    pub schema: String,
    pub slot_id: RecurrenceSlotId,
    pub policy_id: String,
    pub configuration_version: String,
    pub subject_id: String,
    pub scope_id: String,
    pub scheduler_clock_id: String,
    pub nominal_due_at: DateTime<Utc>,
    pub latest_admissible: RecurrenceLatestAdmissibleV1,
    pub occurrence: u64,
    pub trigger: RecurrenceTriggerV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catch_up_of: Option<RecurrenceSlotId>,
}

impl RecurrenceSlotV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy_id: String,
        configuration_version: String,
        subject_id: String,
        scope_id: String,
        scheduler_clock_id: String,
        nominal_due_at: DateTime<Utc>,
        latest_admissible: DateTime<Utc>,
        occurrence: u64,
        trigger: RecurrenceTriggerV1,
        catch_up_of: Option<RecurrenceSlotId>,
    ) -> Result<Self, CanonicalStoreError> {
        let mut value = Self {
            schema: SLOT_SCHEMA_V1.into(),
            slot_id: RecurrenceSlotId(String::new()),
            policy_id,
            configuration_version,
            subject_id,
            scope_id,
            scheduler_clock_id: scheduler_clock_id.clone(),
            nominal_due_at,
            latest_admissible: RecurrenceLatestAdmissibleV1 {
                scheduler_clock_id,
                at: latest_admissible,
            },
            occurrence,
            trigger,
            catch_up_of,
        };
        value.slot_id = RecurrenceSlotId(object_id(&value, "slot_id")?);
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != SLOT_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(format!(
                "unsupported recurrence slot schema {}",
                self.schema
            )));
        }
        require_digest("slot_id", self.slot_id.as_str())?;
        for (name, value) in [
            ("policy_id", &self.policy_id),
            ("configuration_version", &self.configuration_version),
            ("subject_id", &self.subject_id),
            ("scope_id", &self.scope_id),
            ("scheduler_clock_id", &self.scheduler_clock_id),
        ] {
            require_token(name, value)?;
        }
        if self.latest_admissible.scheduler_clock_id != self.scheduler_clock_id {
            return Err(CanonicalStoreError::Invalid(
                "slot due/latest instants use different scheduler clocks".into(),
            ));
        }
        if self.latest_admissible.at < self.nominal_due_at {
            return Err(CanonicalStoreError::Invalid(
                "latest-admissible instant precedes nominal due instant".into(),
            ));
        }
        if matches!(self.trigger, RecurrenceTriggerV1::CatchUp) != self.catch_up_of.is_some() {
            return Err(CanonicalStoreError::Invalid(
                "catch-up trigger and catch_up_of linkage must appear together".into(),
            ));
        }
        if self.slot_id.0 != object_id(self, "slot_id")? {
            return Err(CanonicalStoreError::Invalid(
                "slot_id does not match exact recurrence-slot basis".into(),
            ));
        }
        Ok(())
    }

    pub fn timing_at(
        &self,
        scheduler_clock_id: &str,
        now: DateTime<Utc>,
    ) -> Result<SlotTimingV1, CanonicalStoreError> {
        self.validate()?;
        if scheduler_clock_id != self.scheduler_clock_id {
            return Err(CanonicalStoreError::Invalid(
                "runtime scheduler clock does not match slot clock".into(),
            ));
        }
        if now < self.nominal_due_at {
            return Err(CanonicalStoreError::NotDue(self.slot_id.0.clone()));
        }
        if !self
            .latest_admissible
            .admits(scheduler_clock_id, now)
            .map_err(CanonicalStoreError::Invalid)?
        {
            return Ok(SlotTimingV1::Missed);
        }
        if self.trigger == RecurrenceTriggerV1::CatchUp {
            Ok(SlotTimingV1::CatchUp)
        } else if now == self.nominal_due_at {
            Ok(SlotTimingV1::OnTime)
        } else {
            Ok(SlotTimingV1::Late)
        }
    }
}

/// Deliberately non-serializable process-local witness returned by the unique
/// slot claim. A restart cannot reconstruct it from database representation.
#[derive(Debug)]
pub struct LiveCycleLeaseV1 {
    cycle_id: ObservationCycleId,
    slot_id: RecurrenceSlotId,
    process_nonce: Uuid,
}

impl LiveCycleLeaseV1 {
    pub fn cycle_id(&self) -> &ObservationCycleId {
        &self.cycle_id
    }

    pub fn slot_id(&self) -> &RecurrenceSlotId {
        &self.slot_id
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleStatusV1 {
    Observing,
    PostureRecorded,
    AwaitingAg,
    AwaitingAgReconciliation,
    ObservationRequired,
    Halted,
    Closed,
    Missed,
    RecoveryRequired,
}

impl CycleStatusV1 {
    fn as_db(self) -> &'static str {
        match self {
            Self::Observing => "observing",
            Self::PostureRecorded => "posture_recorded",
            Self::AwaitingAg => "awaiting_ag",
            Self::AwaitingAgReconciliation => "awaiting_ag_reconciliation",
            Self::ObservationRequired => "observation_required",
            Self::Halted => "halted",
            Self::Closed => "closed",
            Self::Missed => "missed",
            Self::RecoveryRequired => "recovery_required",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRecordV1 {
    pub schema: String,
    pub observation_id: String,
    pub support: QualifiedSupportV1,
    pub posture: OperationalPosture,
}

impl ObservationRecordV1 {
    pub fn validate_for_cycle(
        &self,
        cycle: &ObservationCycleId,
    ) -> Result<(), CanonicalStoreError> {
        if self.schema != OBSERVATION_RECORD_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported observation record schema".into(),
            ));
        }
        require_digest("observation_id", &self.observation_id)?;
        self.support
            .validate_shape()
            .map_err(CanonicalStoreError::Invalid)?;
        if self.support.observation_cycle_id != cycle.as_str()
            || self.support.observation_id != self.observation_id
            || self.support.diagnostic_inputs_id != self.posture.input_evidence.inputs_id
            || self.posture.present_support.as_ref() != Some(&self.support)
        {
            return Err(CanonicalStoreError::Invalid(
                "observation/support/posture basis is not exact".into(),
            ));
        }
        if self.posture.current
            != (self.support.standing == SupportStandingV1::Current
                && self.posture.completeness
                    == crate::diagnostic_posture::CompletenessAxis::Complete
                && self.posture.coverage == crate::diagnostic_posture::CoverageAxis::Complete
                && self.posture.recurrence_axis
                    == crate::diagnostic_posture::RecurrenceAxis::Current)
        {
            return Err(CanonicalStoreError::Invalid(
                "posture currentness does not correspond to exact support and retained axes".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionClassV1 {
    Display,
    AttentionRequired,
    EscalationRequested,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionRecordV1 {
    pub class: AttentionClassV1,
    pub source_posture_id: String,
    pub reason_code: String,
    /// Presentation only. It is excluded from every proposal/authority type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_text: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TemporalDecisionV1 {
    Observe,
    Hold,
    Attention,
}

/// Durable non-authorizing Nightshift temporal/tolerability posture.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalPostureV1 {
    pub policy_id: String,
    pub basis_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_expiry: Option<TemporalHoldExpiryV1>,
    pub evaluated_at: DateTime<Utc>,
    pub decision: TemporalDecisionV1,
}

impl TemporalPostureV1 {
    pub fn evaluate(
        policy_id: String,
        basis_digest: String,
        hold_expiry: Option<TemporalHoldExpiryV1>,
        scheduler_clock_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Self, CanonicalStoreError> {
        require_token("temporal policy_id", &policy_id)?;
        require_digest("temporal basis_digest", &basis_digest)?;
        let decision = match &hold_expiry {
            Some(expiry)
                if expiry
                    .is_active(scheduler_clock_id, now)
                    .map_err(CanonicalStoreError::Invalid)? =>
            {
                TemporalDecisionV1::Hold
            }
            Some(_) => TemporalDecisionV1::Attention,
            None => TemporalDecisionV1::Observe,
        };
        Ok(Self {
            policy_id,
            basis_digest,
            hold_expiry,
            evaluated_at: now,
            decision,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypedCoarseIntentV1 {
    pub schema: String,
    pub intent_id: String,
    pub workflow_id: String,
    pub intent_kind: String,
    pub subject_id: String,
    /// Workflow-supplied exact AG subject digest. This is never inferred by
    /// hashing the display subject identifier.
    pub subject_digest: String,
    pub scope_id: String,
    pub source_observation_id: String,
    pub source_support_id: String,
    pub source_posture_id: String,
    pub immutable_parameters: serde_json::Value,
}

impl TypedCoarseIntentV1 {
    pub fn seal(mut self) -> Result<Self, CanonicalStoreError> {
        self.schema = TYPED_INTENT_SCHEMA_V1.into();
        self.intent_id.clear();
        self.intent_id = object_id(&self, "intent_id")?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != TYPED_INTENT_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported typed intent schema".into(),
            ));
        }
        require_digest("intent_id", &self.intent_id)?;
        for (name, value) in [
            ("workflow_id", &self.workflow_id),
            ("intent_kind", &self.intent_kind),
            ("subject_id", &self.subject_id),
            ("source_observation_id", &self.source_observation_id),
            ("source_support_id", &self.source_support_id),
            ("source_posture_id", &self.source_posture_id),
        ] {
            require_token(name, value)?;
        }
        require_digest("subject_digest", &self.subject_digest)?;
        require_digest("scope_id", &self.scope_id)?;
        if !self.immutable_parameters.is_object() {
            return Err(CanonicalStoreError::Invalid(
                "immutable_parameters must be a typed JSON object, never prose".into(),
            ));
        }
        if self.intent_id != object_id(self, "intent_id")? {
            return Err(CanonicalStoreError::Invalid(
                "intent_id does not match typed intent preimage".into(),
            ));
        }
        Ok(())
    }

    pub fn validate_for_observation(
        &self,
        observation: &ObservationRecordV1,
    ) -> Result<(), CanonicalStoreError> {
        self.validate()?;
        if self.subject_id != observation.posture.policy.subject.id
            || self.scope_id != observation.posture.policy.subject.scope.digest
            || self.source_observation_id != observation.observation_id
            || self.source_support_id != observation.support.support_id
            || self.source_posture_id != observation.posture.posture_id
        {
            return Err(CanonicalStoreError::Invalid(
                "typed intent does not exactly bind the complete live posture basis".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgProgramCounterV1 {
    ObservationRequired,
    ProposalRecorded,
    StandingRequired,
    AdmissiblePendingAuthorization,
    AuthorizationConsumed,
    Dispatched,
    ReconciliationRequired,
    SettledObservationRequired,
    Halted,
    Completed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgOccurrenceReferenceV1 {
    pub schema: String,
    pub campaign_id: String,
    pub occurrence_id: String,
    pub state_digest: String,
    pub snapshot_digest: String,
    pub program_counter: AgProgramCounterV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docket_attempt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_decision_request_id: Option<String>,
    pub exact_snapshot: serde_json::Value,
}

/// Exact durable AG refusal evidence. It is not an AG program-counter state
/// and grants no Nightshift or AG authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgRefusalReferenceV1 {
    pub schema: String,
    pub refusal_digest: String,
    pub campaign_id: String,
    pub occurrence_id: String,
    pub at_state_digest: String,
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    pub exact_outcome: serde_json::Value,
}

impl AgRefusalReferenceV1 {
    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != AG_REFUSAL_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported AG refusal reference schema".into(),
            ));
        }
        require_digest("refusal_digest", &self.refusal_digest)?;
        require_digest("campaign_id", &self.campaign_id)?;
        require_token("occurrence_id", &self.occurrence_id)?;
        require_digest("at_state_digest", &self.at_state_digest)?;
        require_token("refusal code", &self.code)?;
        if let Some(evidence) = &self.evidence {
            require_digest("refusal evidence", evidence)?;
        }
        if !self.exact_outcome.is_object() {
            return Err(CanonicalStoreError::Invalid(
                "AG refusal exact outcome must be an object".into(),
            ));
        }
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(
                serde_jcs::to_vec(&self.exact_outcome)
                    .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))?
            )
        );
        if digest != self.refusal_digest {
            return Err(CanonicalStoreError::Invalid(
                "AG refusal digest does not bind the exact outcome".into(),
            ));
        }
        Ok(())
    }
}

/// Exact, non-authorizing request made durable before calling AG. Its bytes
/// are retained for identity and audit after a crash, but Nightshift never
/// treats them as permission to resubmit, spend AG authorization, or call
/// Docket.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAgRequestV1 {
    pub schema: String,
    pub request_digest: String,
    pub campaign_id: String,
    pub occurrence_id: String,
    pub source_intent_id: String,
    pub exact_request: serde_json::Value,
}

impl PreparedAgRequestV1 {
    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != PREPARED_AG_REQUEST_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported prepared AG request schema".into(),
            ));
        }
        require_digest("request_digest", &self.request_digest)?;
        require_digest("campaign_id", &self.campaign_id)?;
        require_digest("source_intent_id", &self.source_intent_id)?;
        uuid::Uuid::parse_str(&self.occurrence_id).map_err(|_| {
            CanonicalStoreError::Invalid(
                "prepared AG occurrence_id must be an independently allocated UUID".into(),
            )
        })?;
        if !self.exact_request.is_object() {
            return Err(CanonicalStoreError::Invalid(
                "prepared AG request must retain an exact object".into(),
            ));
        }
        let expected = format!(
            "sha256:{:x}",
            Sha256::digest(
                serde_jcs::to_vec(&self.exact_request)
                    .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))?
            )
        );
        if self.request_digest != expected {
            return Err(CanonicalStoreError::Invalid(
                "prepared AG request digest mismatch".into(),
            ));
        }
        Ok(())
    }
}

impl AgOccurrenceReferenceV1 {
    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != AG_REFERENCE_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported AG occurrence reference schema".into(),
            ));
        }
        require_digest("campaign_id", &self.campaign_id)?;
        require_digest("state_digest", &self.state_digest)?;
        require_digest("snapshot_digest", &self.snapshot_digest)?;
        uuid::Uuid::parse_str(&self.occurrence_id).map_err(|_| {
            CanonicalStoreError::Invalid(
                "AG occurrence_id must be an independently allocated UUID".into(),
            )
        })?;
        for (name, value) in [
            ("docket_attempt_id", &self.docket_attempt_id),
            ("settlement_id", &self.settlement_id),
            (
                "external_decision_request_id",
                &self.external_decision_request_id,
            ),
        ] {
            if let Some(value) = value {
                require_token(name, value)?;
            }
        }
        if !self.exact_snapshot.is_object() {
            return Err(CanonicalStoreError::Invalid(
                "AG exact snapshot must be an object".into(),
            ));
        }
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(
                serde_jcs::to_vec(&self.exact_snapshot)
                    .map_err(|error| CanonicalStoreError::Invalid(error.to_string()))?
            )
        );
        if digest != self.snapshot_digest {
            return Err(CanonicalStoreError::Invalid(
                "AG snapshot digest does not match exact snapshot".into(),
            ));
        }
        Ok(())
    }
}

fn ag_status_can_follow(prior: AgProgramCounterV1, next: AgProgramCounterV1) -> bool {
    use AgProgramCounterV1 as Pc;
    if prior == next {
        return true;
    }
    match prior {
        Pc::ObservationRequired => !matches!(next, Pc::ObservationRequired),
        Pc::ProposalRecorded => !matches!(next, Pc::ObservationRequired | Pc::ProposalRecorded),
        Pc::StandingRequired => !matches!(
            next,
            Pc::ObservationRequired | Pc::ProposalRecorded | Pc::StandingRequired
        ),
        Pc::AdmissiblePendingAuthorization => matches!(
            next,
            Pc::AuthorizationConsumed
                | Pc::Dispatched
                | Pc::ReconciliationRequired
                | Pc::SettledObservationRequired
                | Pc::Halted
                | Pc::Completed
        ),
        Pc::AuthorizationConsumed => matches!(
            next,
            Pc::Dispatched
                | Pc::ReconciliationRequired
                | Pc::SettledObservationRequired
                | Pc::Halted
                | Pc::Completed
        ),
        Pc::Dispatched => matches!(
            next,
            Pc::ReconciliationRequired
                | Pc::SettledObservationRequired
                | Pc::Halted
                | Pc::Completed
        ),
        Pc::ReconciliationRequired => matches!(
            next,
            Pc::SettledObservationRequired | Pc::Halted | Pc::Completed
        ),
        Pc::SettledObservationRequired => matches!(next, Pc::Halted | Pc::Completed),
        Pc::Halted => matches!(next, Pc::Completed),
        Pc::Completed => false,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationCycleV1 {
    pub schema: String,
    pub cycle_id: ObservationCycleId,
    pub slot: RecurrenceSlotV1,
    pub timing: SlotTimingV1,
    pub status: CycleStatusV1,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_state_digest: Option<String>,
    pub state_digest: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<ObservationRecordV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<AttentionRecordV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_posture: Option<TemporalPostureV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<TypedCoarseIntentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepared_ag_request: Option<PreparedAgRequestV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ag: Option<AgOccurrenceReferenceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ag_refusal: Option<AgRefusalReferenceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_reason: Option<String>,
}

impl ObservationCycleV1 {
    fn seal(&mut self) -> Result<(), CanonicalStoreError> {
        self.state_digest.clear();
        self.state_digest = object_id(self, "state_digest")?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), CanonicalStoreError> {
        if self.schema != CYCLE_SCHEMA_V1 {
            return Err(CanonicalStoreError::Invalid(
                "unsupported observation-cycle schema".into(),
            ));
        }
        require_token("cycle_id", self.cycle_id.as_str())?;
        self.slot.validate()?;
        require_digest("state_digest", &self.state_digest)?;
        if let Some(prior) = &self.prior_state_digest {
            require_digest("prior_state_digest", prior)?;
        }
        if let Some(observation) = &self.observation {
            observation.validate_for_cycle(&self.cycle_id)?;
        }
        if let Some(intent) = &self.intent {
            let observation = self.observation.as_ref().ok_or_else(|| {
                CanonicalStoreError::Invalid("intent exists without observation basis".into())
            })?;
            intent.validate_for_observation(observation)?;
        }
        if let Some(temporal) = &self.temporal_posture {
            require_token("temporal policy_id", &temporal.policy_id)?;
            require_digest("temporal basis_digest", &temporal.basis_digest)?;
            let expected = match &temporal.hold_expiry {
                Some(expiry)
                    if expiry
                        .is_active(&expiry.scheduler_clock_id, temporal.evaluated_at)
                        .map_err(CanonicalStoreError::Invalid)? =>
                {
                    TemporalDecisionV1::Hold
                }
                Some(_) => TemporalDecisionV1::Attention,
                None => TemporalDecisionV1::Observe,
            };
            if temporal.decision != expected {
                return Err(CanonicalStoreError::Invalid(
                    "temporal posture does not match its distinct hold-expiry law".into(),
                ));
            }
        }
        if let Some(ag) = &self.ag {
            ag.validate()?;
            if self.intent.is_none() {
                return Err(CanonicalStoreError::Invalid(
                    "AG occurrence exists without an exact typed intent".into(),
                ));
            }
        }
        if let Some(refusal) = &self.ag_refusal {
            refusal.validate()?;
            let request = self.prepared_ag_request.as_ref().ok_or_else(|| {
                CanonicalStoreError::Invalid("AG refusal exists without prepared request".into())
            })?;
            if request.campaign_id != refusal.campaign_id
                || request.occurrence_id != refusal.occurrence_id
            {
                return Err(CanonicalStoreError::Invalid(
                    "AG refusal names the wrong prepared occurrence".into(),
                ));
            }
        }
        if let Some(request) = &self.prepared_ag_request {
            request.validate()?;
            let intent = self.intent.as_ref().ok_or_else(|| {
                CanonicalStoreError::Invalid("prepared AG request exists without intent".into())
            })?;
            if request.source_intent_id != intent.intent_id {
                return Err(CanonicalStoreError::Invalid(
                    "prepared AG request does not bind the exact intent".into(),
                ));
            }
        }
        let structural = match self.status {
            CycleStatusV1::Observing => {
                self.observation.is_none()
                    && self.intent.is_none()
                    && self.prepared_ag_request.is_none()
                    && self.ag.is_none()
            }
            CycleStatusV1::PostureRecorded => {
                self.observation.is_some()
                    && self.intent.is_none()
                    && self.prepared_ag_request.is_none()
                    && self.ag.is_none()
            }
            CycleStatusV1::AwaitingAg => {
                self.observation.is_some()
                    && self.intent.is_some()
                    && self.prepared_ag_request.is_some()
            }
            CycleStatusV1::AwaitingAgReconciliation => self
                .ag
                .as_ref()
                .is_some_and(|ag| ag.program_counter == AgProgramCounterV1::ReconciliationRequired),
            CycleStatusV1::ObservationRequired => self.ag.as_ref().is_some_and(|ag| {
                ag.program_counter == AgProgramCounterV1::SettledObservationRequired
            }),
            CycleStatusV1::Halted => self
                .ag
                .as_ref()
                .is_some_and(|ag| ag.program_counter == AgProgramCounterV1::Halted),
            CycleStatusV1::Closed => {
                (self.observation.is_some()
                    && self.intent.is_none()
                    && self.prepared_ag_request.is_none()
                    && self.ag.is_none())
                    || self
                        .ag
                        .as_ref()
                        .is_some_and(|ag| ag.program_counter == AgProgramCounterV1::Completed)
                    || self.ag_refusal.is_some()
            }
            CycleStatusV1::Missed => {
                self.timing == SlotTimingV1::Missed
                    && self.observation.is_none()
                    && self.intent.is_none()
                    && self.ag.is_none()
            }
            CycleStatusV1::RecoveryRequired => match &self.prepared_ag_request {
                Some(_) => self.observation.is_some() && self.intent.is_some(),
                None => self.intent.is_none() && self.ag.is_none() && self.ag_refusal.is_none(),
            },
        };
        if !structural {
            return Err(CanonicalStoreError::Invalid(
                "cycle status and retained semantic facts are inconsistent".into(),
            ));
        }
        if self.state_digest != object_id(self, "state_digest")? {
            return Err(CanonicalStoreError::Invalid(
                "observation-cycle state digest mismatch".into(),
            ));
        }
        Ok(())
    }

    pub fn headline(&self) -> Option<Headline> {
        self.observation
            .as_ref()
            .map(|value| value.posture.headline)
    }
}

pub struct CanonicalStore {
    connection: Connection,
}

impl CanonicalStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CanonicalStoreError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS canonical_recurrence_slots (
                slot_id TEXT PRIMARY KEY,
                cycle_id TEXT NOT NULL UNIQUE,
                status TEXT NOT NULL,
                basis_json TEXT NOT NULL,
                state_digest TEXT NOT NULL,
                updated_at TEXT NOT NULL
            ) STRICT;
            CREATE TABLE IF NOT EXISTS canonical_observation_cycles (
                cycle_id TEXT PRIMARY KEY,
                slot_id TEXT NOT NULL UNIQUE,
                version INTEGER NOT NULL,
                status TEXT NOT NULL,
                state_digest TEXT NOT NULL,
                snapshot_json TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(slot_id) REFERENCES canonical_recurrence_slots(slot_id)
            ) STRICT;
            CREATE TABLE IF NOT EXISTS canonical_cycle_events (
                cycle_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                event_kind TEXT NOT NULL,
                prior_state_digest TEXT,
                resulting_state_digest TEXT NOT NULL,
                snapshot_json TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                PRIMARY KEY(cycle_id, sequence),
                UNIQUE(cycle_id, resulting_state_digest),
                FOREIGN KEY(cycle_id) REFERENCES canonical_observation_cycles(cycle_id)
            ) STRICT;
            CREATE UNIQUE INDEX IF NOT EXISTS canonical_one_successor_per_predecessor
            ON canonical_cycle_events(cycle_id, prior_state_digest)
            WHERE prior_state_digest IS NOT NULL;
            CREATE TABLE IF NOT EXISTS canonical_ag_occurrence_claims (
                campaign_id TEXT NOT NULL,
                occurrence_id TEXT NOT NULL,
                cycle_id TEXT NOT NULL UNIQUE,
                request_digest TEXT NOT NULL,
                claimed_at TEXT NOT NULL,
                PRIMARY KEY(campaign_id, occurrence_id),
                FOREIGN KEY(cycle_id) REFERENCES canonical_observation_cycles(cycle_id)
            ) STRICT;
            ",
        )?;
        Ok(Self { connection })
    }

    pub fn claim_slot(
        &mut self,
        slot: RecurrenceSlotV1,
        scheduler_clock_id: &str,
        now: DateTime<Utc>,
    ) -> Result<(ObservationCycleV1, LiveCycleLeaseV1), CanonicalStoreError> {
        let timing = slot.timing_at(scheduler_clock_id, now)?;
        if timing == SlotTimingV1::Missed {
            return Err(CanonicalStoreError::Missed(slot.slot_id.0));
        }
        let cycle_id = ObservationCycleId::fresh();
        let now_text = timestamp(now);
        let mut cycle = ObservationCycleV1 {
            schema: CYCLE_SCHEMA_V1.into(),
            cycle_id: cycle_id.clone(),
            slot: slot.clone(),
            timing,
            status: CycleStatusV1::Observing,
            version: 0,
            prior_state_digest: None,
            state_digest: String::new(),
            created_at: now_text.clone(),
            updated_at: now_text.clone(),
            observation: None,
            attention: None,
            temporal_posture: None,
            intent: None,
            prepared_ag_request: None,
            ag: None,
            ag_refusal: None,
            recovery_reason: None,
        };
        cycle.seal()?;
        cycle.validate()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO canonical_recurrence_slots
             (slot_id, cycle_id, status, basis_json, state_digest, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                slot.slot_id.as_str(),
                cycle_id.as_str(),
                CycleStatusV1::Observing.as_db(),
                canonical_json(&slot)?,
                &cycle.state_digest,
                &now_text,
            ],
        )?;
        if inserted != 1 {
            return Err(CanonicalStoreError::DuplicateSlot(slot.slot_id.0));
        }
        insert_initial_cycle(&tx, &cycle, "slot_claimed")?;
        tx.commit()?;
        let lease = LiveCycleLeaseV1 {
            cycle_id,
            slot_id: slot.slot_id,
            process_nonce: Uuid::new_v4(),
        };
        Ok((cycle, lease))
    }

    pub fn record_missed(
        &mut self,
        slot: RecurrenceSlotV1,
        scheduler_clock_id: &str,
        now: DateTime<Utc>,
        reason: String,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        if slot.timing_at(scheduler_clock_id, now)? != SlotTimingV1::Missed {
            return Err(CanonicalStoreError::IllegalTransition(
                "only an actually missed slot may be recorded missed".into(),
            ));
        }
        require_token("missed reason", &reason)?;
        let cycle_id = ObservationCycleId::fresh();
        let now_text = timestamp(now);
        let mut cycle = ObservationCycleV1 {
            schema: CYCLE_SCHEMA_V1.into(),
            cycle_id: cycle_id.clone(),
            slot: slot.clone(),
            timing: SlotTimingV1::Missed,
            status: CycleStatusV1::Missed,
            version: 0,
            prior_state_digest: None,
            state_digest: String::new(),
            created_at: now_text.clone(),
            updated_at: now_text.clone(),
            observation: None,
            attention: None,
            temporal_posture: None,
            intent: None,
            prepared_ag_request: None,
            ag: None,
            ag_refusal: None,
            recovery_reason: Some(reason),
        };
        cycle.seal()?;
        cycle.validate()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO canonical_recurrence_slots
             (slot_id, cycle_id, status, basis_json, state_digest, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                slot.slot_id.as_str(),
                cycle_id.as_str(),
                CycleStatusV1::Missed.as_db(),
                canonical_json(&slot)?,
                &cycle.state_digest,
                &now_text,
            ],
        )?;
        if inserted != 1 {
            return Err(CanonicalStoreError::DuplicateSlot(slot.slot_id.0));
        }
        insert_initial_cycle(&tx, &cycle, "slot_missed")?;
        tx.commit()?;
        Ok(cycle)
    }

    pub fn record_observation(
        &mut self,
        lease: &LiveCycleLeaseV1,
        expected_digest: &str,
        observation: ObservationRecordV1,
        attention: AttentionRecordV1,
        temporal_posture: Option<TemporalPostureV1>,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        observation.validate_for_cycle(&lease.cycle_id)?;
        if attention.source_posture_id != observation.posture.posture_id {
            return Err(CanonicalStoreError::Invalid(
                "attention record does not bind the exact posture".into(),
            ));
        }
        self.transition(
            &lease.cycle_id,
            Some(lease),
            expected_digest,
            "observation_recorded",
            now,
            move |cycle| {
                if cycle.status != CycleStatusV1::Observing {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "observation may be recorded only while observing".into(),
                    ));
                }
                cycle.observation = Some(observation);
                cycle.attention = Some(attention);
                cycle.temporal_posture = temporal_posture;
                cycle.status = CycleStatusV1::PostureRecorded;
                Ok(())
            },
        )
    }

    pub fn prepare_ag_occurrence(
        &mut self,
        lease: &LiveCycleLeaseV1,
        expected_digest: &str,
        intent: TypedCoarseIntentV1,
        request: PreparedAgRequestV1,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        request.validate()?;
        self.transition(
            &lease.cycle_id,
            Some(lease),
            expected_digest,
            "ag_occurrence_prepared",
            now,
            move |cycle| {
                if cycle.status != CycleStatusV1::PostureRecorded {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "AG occurrence may be attached only after exact posture recording".into(),
                    ));
                }
                let observation = cycle.observation.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition("observation basis is absent".into())
                })?;
                if !observation.posture.current
                    || observation.support.standing != SupportStandingV1::Current
                {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "non-current posture cannot originate an AG proposal".into(),
                    ));
                }
                intent.validate_for_observation(observation)?;
                if request.source_intent_id != intent.intent_id {
                    return Err(CanonicalStoreError::Invalid(
                        "prepared AG request does not bind the exact intent".into(),
                    ));
                }
                cycle.intent = Some(intent);
                cycle.prepared_ag_request = Some(request);
                cycle.status = CycleStatusV1::AwaitingAg;
                Ok(())
            },
        )
    }

    pub fn attach_ag_occurrence(
        &mut self,
        lease: &LiveCycleLeaseV1,
        expected_digest: &str,
        ag: AgOccurrenceReferenceV1,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        ag.validate()?;
        self.transition(
            &lease.cycle_id,
            Some(lease),
            expected_digest,
            "ag_occurrence_attached",
            now,
            move |cycle| {
                if cycle.status != CycleStatusV1::AwaitingAg || cycle.ag.is_some() {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "AG occurrence may attach only to one prepared request".into(),
                    ));
                }
                let request = cycle.prepared_ag_request.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition("prepared AG request is absent".into())
                })?;
                if request.campaign_id != ag.campaign_id
                    || request.occurrence_id != ag.occurrence_id
                {
                    return Err(CanonicalStoreError::Invalid(
                        "AG response names the wrong prepared occurrence".into(),
                    ));
                }
                if matches!(
                    ag.program_counter,
                    AgProgramCounterV1::AuthorizationConsumed
                        | AgProgramCounterV1::Dispatched
                        | AgProgramCounterV1::ReconciliationRequired
                        | AgProgramCounterV1::SettledObservationRequired
                        | AgProgramCounterV1::Halted
                        | AgProgramCounterV1::Completed
                ) {
                    return Err(CanonicalStoreError::Invalid(
                        "Nightshift adapter crossed an AG consequence boundary".into(),
                    ));
                }
                cycle.ag = Some(ag);
                Ok(())
            },
        )
    }

    /// Consume a read-only AG status. This never changes diagnostic posture and
    /// can never create a new intent or occurrence.
    pub fn record_ag_status(
        &mut self,
        cycle_id: &ObservationCycleId,
        expected_digest: &str,
        ag: AgOccurrenceReferenceV1,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        ag.validate()?;
        let current = self.get_cycle(cycle_id)?;
        if current.state_digest != expected_digest {
            return Err(CanonicalStoreError::StalePredecessor);
        }
        if current.ag.as_ref() == Some(&ag) {
            return Ok(current);
        }
        self.transition(
            cycle_id,
            None,
            expected_digest,
            "ag_status_observed",
            now,
            move |cycle| {
                if !matches!(
                    cycle.status,
                    CycleStatusV1::AwaitingAg
                        | CycleStatusV1::AwaitingAgReconciliation
                        | CycleStatusV1::ObservationRequired
                        | CycleStatusV1::Halted
                        | CycleStatusV1::RecoveryRequired
                ) {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "AG status cannot be attached from this cycle state".into(),
                    ));
                }
                let prior = cycle.ag.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition("cycle has no AG occurrence".into())
                })?;
                if prior.campaign_id != ag.campaign_id || prior.occurrence_id != ag.occurrence_id {
                    return Err(CanonicalStoreError::Invalid(
                        "AG status names the wrong campaign or occurrence".into(),
                    ));
                }
                if !ag_status_can_follow(prior.program_counter, ag.program_counter) {
                    return Err(CanonicalStoreError::Invalid(
                        "stale or regressive AG program-counter observation".into(),
                    ));
                }
                cycle.status = match ag.program_counter {
                    AgProgramCounterV1::ReconciliationRequired => {
                        CycleStatusV1::AwaitingAgReconciliation
                    }
                    AgProgramCounterV1::SettledObservationRequired => {
                        CycleStatusV1::ObservationRequired
                    }
                    AgProgramCounterV1::Halted => CycleStatusV1::Halted,
                    AgProgramCounterV1::Completed => CycleStatusV1::Closed,
                    _ => CycleStatusV1::AwaitingAg,
                };
                cycle.ag = Some(ag);
                Ok(())
            },
        )
    }

    pub fn record_ag_refusal(
        &mut self,
        cycle_id: &ObservationCycleId,
        expected_digest: &str,
        refusal: AgRefusalReferenceV1,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        refusal.validate()?;
        self.transition(
            cycle_id,
            None,
            expected_digest,
            "ag_refusal_observed",
            now,
            move |cycle| {
                if !matches!(
                    cycle.status,
                    CycleStatusV1::AwaitingAg | CycleStatusV1::RecoveryRequired
                ) {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "AG refusal can close only its exact pending cycle".into(),
                    ));
                }
                let request = cycle.prepared_ag_request.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition("prepared AG request is absent".into())
                })?;
                if request.campaign_id != refusal.campaign_id
                    || request.occurrence_id != refusal.occurrence_id
                {
                    return Err(CanonicalStoreError::Invalid(
                        "AG refusal names the wrong campaign or occurrence".into(),
                    ));
                }
                let ag = cycle.ag.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition(
                        "AG refusal requires the exact observed AG predecessor state".into(),
                    )
                })?;
                if ag.state_digest != refusal.at_state_digest {
                    return Err(CanonicalStoreError::Invalid(
                        "AG refusal names a substituted predecessor state".into(),
                    ));
                }
                cycle.ag_refusal = Some(refusal);
                cycle.status = CycleStatusV1::Closed;
                Ok(())
            },
        )
    }

    /// Reattach an AG occurrence discovered by a read-only status query after
    /// a crash between AG acceptance and local reference persistence. This is
    /// evidence reconciliation only; it cannot call AG or create an occurrence.
    pub fn recover_ag_occurrence(
        &mut self,
        cycle_id: &ObservationCycleId,
        expected_digest: &str,
        ag: AgOccurrenceReferenceV1,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        ag.validate()?;
        self.transition(
            cycle_id,
            None,
            expected_digest,
            "ag_occurrence_recovered_read_only",
            now,
            move |cycle| {
                if !matches!(
                    cycle.status,
                    CycleStatusV1::AwaitingAg | CycleStatusV1::RecoveryRequired
                ) || cycle.ag.is_some()
                {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "AG occurrence recovery requires an unresolved prepared request".into(),
                    ));
                }
                let request = cycle.prepared_ag_request.as_ref().ok_or_else(|| {
                    CanonicalStoreError::IllegalTransition("prepared AG request is absent".into())
                })?;
                if request.campaign_id != ag.campaign_id
                    || request.occurrence_id != ag.occurrence_id
                {
                    return Err(CanonicalStoreError::Invalid(
                        "recovered AG response names the wrong occurrence".into(),
                    ));
                }
                cycle.status = match ag.program_counter {
                    AgProgramCounterV1::ReconciliationRequired => {
                        CycleStatusV1::AwaitingAgReconciliation
                    }
                    AgProgramCounterV1::SettledObservationRequired => {
                        CycleStatusV1::ObservationRequired
                    }
                    AgProgramCounterV1::Halted => CycleStatusV1::Halted,
                    AgProgramCounterV1::Completed => CycleStatusV1::Closed,
                    _ => CycleStatusV1::AwaitingAg,
                };
                cycle.ag = Some(ag);
                cycle.recovery_reason = None;
                Ok(())
            },
        )
    }

    pub fn mark_recovery_required(
        &mut self,
        cycle_id: &ObservationCycleId,
        expected_digest: &str,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        require_token("recovery reason", &reason)?;
        self.transition(
            cycle_id,
            None,
            expected_digest,
            "recovery_required",
            now,
            move |cycle| {
                if matches!(cycle.status, CycleStatusV1::Closed | CycleStatusV1::Missed) {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "terminal cycle cannot enter recovery".into(),
                    ));
                }
                cycle.status = CycleStatusV1::RecoveryRequired;
                cycle.recovery_reason = Some(reason);
                Ok(())
            },
        )
    }

    pub fn close_without_proposal(
        &mut self,
        lease: &LiveCycleLeaseV1,
        expected_digest: &str,
        now: DateTime<Utc>,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        self.transition(
            &lease.cycle_id,
            Some(lease),
            expected_digest,
            "cycle_closed_without_proposal",
            now,
            |cycle| {
                if cycle.status != CycleStatusV1::PostureRecorded || cycle.intent.is_some() {
                    return Err(CanonicalStoreError::IllegalTransition(
                        "only a posture-only cycle may close without a proposal".into(),
                    ));
                }
                cycle.status = CycleStatusV1::Closed;
                Ok(())
            },
        )
    }

    pub fn get_cycle(
        &self,
        cycle_id: &ObservationCycleId,
    ) -> Result<ObservationCycleV1, CanonicalStoreError> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot_json FROM canonical_observation_cycles WHERE cycle_id=?1",
                [cycle_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let cycle: ObservationCycleV1 = serde_json::from_str(
            &json.ok_or_else(|| CanonicalStoreError::CycleNotFound(cycle_id.0.clone()))?,
        )?;
        cycle.validate()?;
        self.validate_ag_occurrence_claim(&cycle)?;
        Ok(cycle)
    }

    pub fn list_cycles(&self) -> Result<Vec<ObservationCycleV1>, CanonicalStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT snapshot_json FROM canonical_observation_cycles ORDER BY updated_at, cycle_id",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut values = Vec::new();
        for row in rows {
            let cycle: ObservationCycleV1 = serde_json::from_str(&row?)?;
            cycle.validate()?;
            values.push(cycle);
        }
        drop(statement);
        for cycle in &values {
            self.validate_ag_occurrence_claim(cycle)?;
        }
        Ok(values)
    }

    /// Classify restart without recreating freshness. Local in-flight cycles
    /// become `RecoveryRequired`; AG-bound cycles remain exact references that
    /// must be queried through AG.
    pub fn recover_after_restart(
        &mut self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ObservationCycleV1>, CanonicalStoreError> {
        let cycles = self.list_cycles()?;
        let mut recovered = Vec::new();
        for cycle in cycles {
            match cycle.status {
                CycleStatusV1::Observing | CycleStatusV1::PostureRecorded => {
                    let id = cycle.cycle_id.clone();
                    recovered.push(self.transition(
                        &id,
                        None,
                        &cycle.state_digest,
                        "restart_local_currentness_erased",
                        now,
                        |value| {
                            value.status = CycleStatusV1::RecoveryRequired;
                            value.recovery_reason = Some(
                                "restart erased live observation support; start a fresh cycle"
                                    .into(),
                            );
                            Ok(())
                        },
                    )?);
                }
                CycleStatusV1::AwaitingAg | CycleStatusV1::AwaitingAgReconciliation => {
                    // Exact AG reference is returned for a read-only AG status query.
                    recovered.push(cycle);
                }
                CycleStatusV1::RecoveryRequired if cycle.prepared_ag_request.is_some() => {
                    // A crash or lost response after durable preparation is resolved only
                    // by querying the exact occurrence through AG. Never resubmit here.
                    recovered.push(cycle);
                }
                _ => {}
            }
        }
        Ok(recovered)
    }

    pub fn replay(
        &self,
        cycle_id: &ObservationCycleId,
    ) -> Result<Vec<ObservationCycleV1>, CanonicalStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT sequence, prior_state_digest, resulting_state_digest, snapshot_json
             FROM canonical_cycle_events WHERE cycle_id=?1 ORDER BY sequence",
        )?;
        let rows = statement.query_map([cycle_id.as_str()], |row| {
            Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut result = Vec::new();
        let mut expected_prior: Option<String> = None;
        for row in rows {
            let (sequence, prior, resulting, json) = row?;
            if sequence != result.len() as u64 || prior != expected_prior {
                return Err(CanonicalStoreError::Replay(
                    "event sequence or predecessor chain diverged".into(),
                ));
            }
            let cycle: ObservationCycleV1 = serde_json::from_str(&json)?;
            cycle.validate()?;
            if cycle.state_digest != resulting || cycle.prior_state_digest != prior {
                return Err(CanonicalStoreError::Replay(
                    "event digest does not bind stored snapshot".into(),
                ));
            }
            expected_prior = Some(resulting);
            result.push(cycle);
        }
        let current = self.get_cycle(cycle_id)?;
        if result.last() != Some(&current) {
            return Err(CanonicalStoreError::Replay(
                "authoritative row differs from replay tail".into(),
            ));
        }
        Ok(result)
    }

    fn transition<F>(
        &mut self,
        cycle_id: &ObservationCycleId,
        lease: Option<&LiveCycleLeaseV1>,
        expected_digest: &str,
        event_kind: &str,
        now: DateTime<Utc>,
        mutate: F,
    ) -> Result<ObservationCycleV1, CanonicalStoreError>
    where
        F: FnOnce(&mut ObservationCycleV1) -> Result<(), CanonicalStoreError>,
    {
        if let Some(lease) = lease {
            if lease.cycle_id != *cycle_id || lease.process_nonce.is_nil() {
                return Err(CanonicalStoreError::WrongLiveLease);
            }
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let json: Option<String> = tx
            .query_row(
                "SELECT snapshot_json FROM canonical_observation_cycles WHERE cycle_id=?1",
                [cycle_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        let mut cycle: ObservationCycleV1 = serde_json::from_str(
            &json.ok_or_else(|| CanonicalStoreError::CycleNotFound(cycle_id.0.clone()))?,
        )?;
        cycle.validate()?;
        let durable_claim: Option<(String, String, String)> = tx
            .query_row(
                "SELECT campaign_id, occurrence_id, request_digest
                 FROM canonical_ag_occurrence_claims WHERE cycle_id=?1",
                [cycle_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match (&cycle.prepared_ag_request, durable_claim) {
            (None, None) => {}
            (Some(request), Some((campaign, occurrence, digest)))
                if campaign == request.campaign_id
                    && occurrence == request.occurrence_id
                    && digest == request.request_digest => {}
            _ => {
                return Err(CanonicalStoreError::Replay(
                    "AG occurrence claim diverged before transition".into(),
                ));
            }
        }
        if cycle.state_digest != expected_digest {
            return Err(CanonicalStoreError::StalePredecessor);
        }
        let prior = cycle.state_digest.clone();
        let had_prepared_ag_request = cycle.prepared_ag_request.is_some();
        mutate(&mut cycle)?;
        cycle.version = cycle
            .version
            .checked_add(1)
            .ok_or_else(|| CanonicalStoreError::Invalid("cycle version overflow".into()))?;
        cycle.prior_state_digest = Some(prior.clone());
        cycle.updated_at = timestamp(now);
        cycle.seal()?;
        cycle.validate()?;
        if !had_prepared_ag_request {
            if let Some(request) = &cycle.prepared_ag_request {
                let inserted = tx.execute(
                    "INSERT OR IGNORE INTO canonical_ag_occurrence_claims
                     (campaign_id, occurrence_id, cycle_id, request_digest, claimed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        &request.campaign_id,
                        &request.occurrence_id,
                        cycle_id.as_str(),
                        &request.request_digest,
                        &cycle.updated_at,
                    ],
                )?;
                if inserted != 1 {
                    return Err(CanonicalStoreError::DuplicateAgOccurrence(
                        request.campaign_id.clone(),
                        request.occurrence_id.clone(),
                    ));
                }
            }
        }
        let changed = tx.execute(
            "UPDATE canonical_observation_cycles
             SET version=?1, status=?2, state_digest=?3, snapshot_json=?4, updated_at=?5
             WHERE cycle_id=?6 AND state_digest=?7",
            params![
                cycle.version,
                cycle.status.as_db(),
                &cycle.state_digest,
                canonical_json(&cycle)?,
                &cycle.updated_at,
                cycle_id.as_str(),
                &prior,
            ],
        )?;
        if changed != 1 {
            return Err(CanonicalStoreError::StalePredecessor);
        }
        let slot_changed = tx.execute(
            "UPDATE canonical_recurrence_slots
             SET status=?1, state_digest=?2, updated_at=?3
             WHERE slot_id=?4 AND state_digest=?5",
            params![
                cycle.status.as_db(),
                &cycle.state_digest,
                &cycle.updated_at,
                cycle.slot.slot_id.as_str(),
                &prior,
            ],
        )?;
        if slot_changed != 1 {
            return Err(CanonicalStoreError::StalePredecessor);
        }
        tx.execute(
            "INSERT INTO canonical_cycle_events
             (cycle_id, sequence, event_kind, prior_state_digest, resulting_state_digest,
              snapshot_json, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                cycle_id.as_str(),
                cycle.version,
                event_kind,
                &prior,
                &cycle.state_digest,
                canonical_json(&cycle)?,
                &cycle.updated_at,
            ],
        )?;
        tx.commit()?;
        Ok(cycle)
    }

    fn validate_ag_occurrence_claim(
        &self,
        cycle: &ObservationCycleV1,
    ) -> Result<(), CanonicalStoreError> {
        let claim: Option<(String, String, String)> = self
            .connection
            .query_row(
                "SELECT campaign_id, occurrence_id, request_digest
                 FROM canonical_ag_occurrence_claims WHERE cycle_id=?1",
                [cycle.cycle_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        match (&cycle.prepared_ag_request, claim) {
            (None, None) => Ok(()),
            (Some(request), Some((campaign, occurrence, digest)))
                if campaign == request.campaign_id
                    && occurrence == request.occurrence_id
                    && digest == request.request_digest =>
            {
                Ok(())
            }
            _ => Err(CanonicalStoreError::Replay(
                "AG occurrence claim does not match the authoritative cycle snapshot".into(),
            )),
        }
    }
}

fn insert_initial_cycle(
    tx: &Transaction<'_>,
    cycle: &ObservationCycleV1,
    event_kind: &str,
) -> Result<(), CanonicalStoreError> {
    let json = canonical_json(cycle)?;
    tx.execute(
        "INSERT INTO canonical_observation_cycles
         (cycle_id, slot_id, version, status, state_digest, snapshot_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            cycle.cycle_id.as_str(),
            cycle.slot.slot_id.as_str(),
            cycle.version,
            cycle.status.as_db(),
            &cycle.state_digest,
            &json,
            &cycle.updated_at,
        ],
    )?;
    tx.execute(
        "INSERT INTO canonical_cycle_events
         (cycle_id, sequence, event_kind, prior_state_digest, resulting_state_digest,
          snapshot_json, recorded_at)
         VALUES (?1, 0, ?2, NULL, ?3, ?4, ?5)",
        params![
            cycle.cycle_id.as_str(),
            event_kind,
            &cycle.state_digest,
            &json,
            &cycle.updated_at,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::*;

    fn time(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn slot(trigger: RecurrenceTriggerV1) -> RecurrenceSlotV1 {
        RecurrenceSlotV1::new(
            "policy-1".into(),
            "config-1".into(),
            "subject-1".into(),
            "scope-1".into(),
            "nightshift-scheduler-1".into(),
            time("2026-08-11T12:00:00Z"),
            time("2026-08-11T12:05:00Z"),
            7,
            trigger,
            None,
        )
        .unwrap()
    }

    #[test]
    fn exact_slot_identity_does_not_use_nq_generation() {
        let first = slot(RecurrenceTriggerV1::Scheduled);
        let mut second = first.clone();
        second.occurrence += 1;
        second.slot_id = RecurrenceSlotId(object_id(&second, "slot_id").unwrap());
        assert_ne!(first.slot_id, second.slot_id);
    }

    #[test]
    fn duplicate_slot_claim_fails_closed_across_connections() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        let mut first = CanonicalStore::open(&database).unwrap();
        first
            .claim_slot(
                slot(RecurrenceTriggerV1::Scheduled),
                "nightshift-scheduler-1",
                time("2026-08-11T12:00:00Z"),
            )
            .unwrap();
        let mut second = CanonicalStore::open(&database).unwrap();
        assert!(matches!(
            second.claim_slot(
                slot(RecurrenceTriggerV1::Scheduled),
                "nightshift-scheduler-1",
                time("2026-08-11T12:00:01Z")
            ),
            Err(CanonicalStoreError::DuplicateSlot(_))
        ));
    }

    #[test]
    fn concurrent_slot_claim_has_one_winner() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        CanonicalStore::open(&database).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let database = database.clone();
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    let mut store = CanonicalStore::open(database).unwrap();
                    barrier.wait();
                    store
                        .claim_slot(
                            slot(RecurrenceTriggerV1::Scheduled),
                            "nightshift-scheduler-1",
                            time("2026-08-11T12:00:00Z"),
                        )
                        .is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|won| *won)
                .count(),
            1
        );
    }

    #[test]
    fn equality_is_slot_valid_and_after_latest_is_missed() {
        let value = slot(RecurrenceTriggerV1::Scheduled);
        assert_eq!(
            value
                .timing_at("nightshift-scheduler-1", time("2026-08-11T12:05:00Z"))
                .unwrap(),
            SlotTimingV1::Late
        );
        assert_eq!(
            value
                .timing_at("nightshift-scheduler-1", time("2026-08-11T12:05:01Z"))
                .unwrap(),
            SlotTimingV1::Missed
        );
    }

    #[test]
    fn missed_slot_is_durable_and_catch_up_is_a_distinct_slot() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        let mut store = CanonicalStore::open(&database).unwrap();
        let missed_slot = slot(RecurrenceTriggerV1::Scheduled);
        let missed = store
            .record_missed(
                missed_slot.clone(),
                "nightshift-scheduler-1",
                time("2026-08-11T12:05:01Z"),
                "scheduler_observed_missed_slot".into(),
            )
            .unwrap();
        assert_eq!(missed.status, CycleStatusV1::Missed);
        let catch_up = RecurrenceSlotV1::new(
            "policy-1".into(),
            "config-1".into(),
            "subject-1".into(),
            "scope-1".into(),
            "nightshift-scheduler-1".into(),
            time("2026-08-11T12:06:00Z"),
            time("2026-08-11T12:07:00Z"),
            8,
            RecurrenceTriggerV1::CatchUp,
            Some(missed_slot.slot_id.clone()),
        )
        .unwrap();
        assert_ne!(catch_up.slot_id, missed_slot.slot_id);
        let (caught_up, _) = store
            .claim_slot(
                catch_up,
                "nightshift-scheduler-1",
                time("2026-08-11T12:06:00Z"),
            )
            .unwrap();
        assert_eq!(caught_up.timing, SlotTimingV1::CatchUp);
    }

    #[test]
    fn restart_erases_local_currentness_and_replay_is_exact() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        let cycle_id = {
            let mut store = CanonicalStore::open(&database).unwrap();
            let (cycle, _lease) = store
                .claim_slot(
                    slot(RecurrenceTriggerV1::Scheduled),
                    "nightshift-scheduler-1",
                    time("2026-08-11T12:00:00Z"),
                )
                .unwrap();
            cycle.cycle_id
        };
        let mut restarted = CanonicalStore::open(&database).unwrap();
        let recovered = restarted
            .recover_after_restart(time("2026-08-11T12:01:00Z"))
            .unwrap();
        assert_eq!(recovered[0].status, CycleStatusV1::RecoveryRequired);
        assert_eq!(restarted.replay(&cycle_id).unwrap().len(), 2);
    }

    #[test]
    fn stale_predecessor_cannot_create_two_successors() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        let mut store = CanonicalStore::open(&database).unwrap();
        let (cycle, _lease) = store
            .claim_slot(
                slot(RecurrenceTriggerV1::Scheduled),
                "nightshift-scheduler-1",
                time("2026-08-11T12:00:00Z"),
            )
            .unwrap();
        let first = store
            .transition(
                &cycle.cycle_id,
                None,
                &cycle.state_digest,
                "test_recovery",
                time("2026-08-11T12:01:00Z"),
                |value| {
                    value.status = CycleStatusV1::RecoveryRequired;
                    Ok(())
                },
            )
            .unwrap();
        assert!(matches!(
            store.transition(
                &cycle.cycle_id,
                None,
                &cycle.state_digest,
                "stale",
                time("2026-08-11T12:02:00Z"),
                |_| Ok(())
            ),
            Err(CanonicalStoreError::StalePredecessor)
        ));
        assert_eq!(store.replay(&cycle.cycle_id).unwrap().last(), Some(&first));
    }

    #[test]
    fn concurrent_reconcilers_have_one_cas_winner() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("nightshift.sqlite");
        let cycle = {
            let mut store = CanonicalStore::open(&database).unwrap();
            store
                .claim_slot(
                    slot(RecurrenceTriggerV1::Scheduled),
                    "nightshift-scheduler-1",
                    time("2026-08-11T12:00:00Z"),
                )
                .unwrap()
                .0
        };
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2)
            .map(|index| {
                let database = database.clone();
                let barrier = Arc::clone(&barrier);
                let cycle_id = cycle.cycle_id.clone();
                let digest = cycle.state_digest.clone();
                thread::spawn(move || {
                    let mut store = CanonicalStore::open(database).unwrap();
                    barrier.wait();
                    store
                        .mark_recovery_required(
                            &cycle_id,
                            &digest,
                            format!("reconciler_{index}"),
                            time("2026-08-11T12:01:00Z"),
                        )
                        .is_ok()
                })
            })
            .collect();
        assert_eq!(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .filter(|won| *won)
                .count(),
            1
        );
    }
}
