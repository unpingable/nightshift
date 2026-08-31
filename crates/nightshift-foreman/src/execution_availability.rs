//! Provider-neutral provider-execution availability and deferred-dispatch contracts.
//!
//! These records describe mechanism evidence only. They grant no target-effect,
//! approval-response, semantic-retry, provider-account, or production authority.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::contract::ContractError;

pub const EXECUTION_AVAILABILITY_OBSERVATION_SCHEMA_V1: &str =
    "nightshift.provider-execution-availability-observation/v1";
pub const EXECUTION_AVAILABILITY_POLICY_SCHEMA_V1: &str =
    "nightshift.provider-execution-availability-policy/v1";
pub const FOREMAN_EXECUTION_AVAILABILITY_REQUIREMENT_SCHEMA_V1: &str =
    "nightshift.foreman-execution-availability-requirement/v1";
pub const PROVIDER_DISPATCH_OCCURRENCE_SCHEMA_V1: &str =
    "nightshift.provider-dispatch-occurrence/v1";
pub const PROVIDER_ADMISSION_DISPOSITION_SCHEMA_V1: &str =
    "nightshift.provider-admission-disposition/v1";
pub const DEFERRED_PROVIDER_DISPATCH_SCHEMA_V1: &str = "nightshift.deferred-provider-dispatch/v1";

pub const ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD: &str =
    "c36a8137638decf8b04a49611354a90f32c5a945";
pub const ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_OWNER_HEAD: &str =
    "2ba25db66d8b29dd215bd87e05f4ea794024b3b7";
pub const ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_SHA256: &str =
    "sha256:131f1f6e0cf8cb0aea26ed225c584440c81ffedd443c68ace23adecbe493cf93";
pub const ACCEPTED_SWITCHYARD_DETERMINISTIC_FIXTURE_SHA256: &str =
    "sha256:cafa673ac58f60029fd6c1de229b4f57d9f42ba918b7ecb2a3bfb20cb2b41a31";
pub const MAXIMUM_AVAILABILITY_EVIDENCE_BYTES: usize = 16 * 1024;
pub const MAXIMUM_EXECUTION_AVAILABILITY_HISTORY_BYTES: u64 = 16 * 1024 * 1024;
pub const MAXIMUM_DISPATCH_OCCURRENCES: u16 = 16;
pub const MAXIMUM_TOTAL_DEFERRAL_SECONDS: u64 = 7 * 24 * 60 * 60;

const OBSERVATION_DOMAIN: &[u8] =
    b"nightshift.provider-execution-availability-observation.digest/v1\0";
const POLICY_DOMAIN: &[u8] = b"nightshift.provider-execution-availability-policy.digest/v1\0";
const REQUIREMENT_DOMAIN: &[u8] =
    b"nightshift.foreman-execution-availability-requirement.digest/v1\0";
const DISPATCH_DOMAIN: &[u8] = b"nightshift.provider-dispatch-occurrence.digest/v1\0";
const DISPOSITION_DOMAIN: &[u8] = b"nightshift.provider-admission-disposition.digest/v1\0";
const DEFERRED_DOMAIN: &[u8] = b"nightshift.deferred-provider-dispatch.digest/v1\0";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactAvailabilityEvidenceV1 {
    pub representation: String,
    pub byte_length: u64,
    pub sha256: String,
    pub encoding: String,
    pub bytes_hex: String,
}

impl ExactAvailabilityEvidenceV1 {
    pub fn from_bytes(
        representation: impl Into<String>,
        raw: &[u8],
    ) -> Result<Self, ContractError> {
        if raw.is_empty() || raw.len() > MAXIMUM_AVAILABILITY_EVIDENCE_BYTES {
            return Err(ContractError::InvalidField("availability evidence bytes"));
        }
        let value = Self {
            representation: representation.into(),
            byte_length: raw.len() as u64,
            sha256: plain_sha256(raw),
            encoding: "hex".to_owned(),
            bytes_hex: hex::encode(raw),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<Vec<u8>, ContractError> {
        id("evidence representation", &self.representation)?;
        digest("evidence sha256", &self.sha256)?;
        if self.encoding != "hex"
            || self.byte_length == 0
            || self.byte_length as usize > MAXIMUM_AVAILABILITY_EVIDENCE_BYTES
            || self.bytes_hex.len() != self.byte_length as usize * 2
        {
            return Err(ContractError::InvalidField(
                "availability evidence encoding",
            ));
        }
        let raw = hex::decode(&self.bytes_hex)
            .map_err(|_| ContractError::InvalidField("availability evidence hex"))?;
        if plain_sha256(&raw) != self.sha256 {
            return Err(ContractError::DigestMismatch(
                "availability evidence sha256",
            ));
        }
        Ok(raw)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionAvailabilityStateV1 {
    Available,
    ModelAtCapacity,
    ProviderUnavailable,
    RateLimited,
    AuthenticationRefused,
    TransportError,
    ProtocolError,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAvailabilityObservationV1 {
    pub schema: String,
    pub observation_digest: String,
    pub provider_id: String,
    pub model_id: String,
    pub model_class: String,
    pub observed_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub state: ExecutionAvailabilityStateV1,
    pub source_identity: String,
    pub source_version: String,
    pub provider_retry_after: Option<DateTime<Utc>>,
    pub exact_evidence: Option<ExactAvailabilityEvidenceV1>,
    pub authority_effect: String,
}

impl ExecutionAvailabilityObservationV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Value = parse(bytes)?;
        for field in ["observed_at", "received_at", "expires_at"] {
            canonical_timestamp(&value, field)?;
        }
        canonical_optional_timestamp(&value, "provider_retry_after")?;
        serde_json::from_value(value).map_err(json_error)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.observation_digest = digest_without(self, "observation_digest", OBSERVATION_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(&self.schema, EXECUTION_AVAILABILITY_OBSERVATION_SCHEMA_V1)?;
        sealed_digest(
            self,
            "observation_digest",
            &self.observation_digest,
            OBSERVATION_DOMAIN,
        )?;
        for (field, value) in [
            ("provider_id", &self.provider_id),
            ("model_id", &self.model_id),
            ("model_class", &self.model_class),
            ("source_identity", &self.source_identity),
            ("source_version", &self.source_version),
        ] {
            id(field, value)?;
        }
        if self.observed_at > self.received_at || self.received_at >= self.expires_at {
            return Err(ContractError::InvalidField(
                "availability observation time order",
            ));
        }
        if self
            .provider_retry_after
            .is_some_and(|retry_after| retry_after < self.received_at)
        {
            return Err(ContractError::InvalidField("provider_retry_after"));
        }
        match (&self.state, &self.exact_evidence) {
            (ExecutionAvailabilityStateV1::Unknown, None) => {}
            (_, Some(evidence)) => {
                evidence.validate()?;
            }
            _ => return Err(ContractError::InvalidField("availability evidence absence")),
        }
        if self.authority_effect != "SCHEDULING_MECHANISM_EVIDENCE_ONLY" {
            return Err(ContractError::InvalidField("availability authority effect"));
        }
        Ok(())
    }
    pub fn is_current_at(&self, evaluated_at: DateTime<Utc>) -> bool {
        self.observed_at <= evaluated_at && evaluated_at < self.expires_at
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ParkedResourceLockPolicyV1 {
    ReleaseAndReacquire,
    RetainWhileParked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAvailabilityPolicyV1 {
    pub schema: String,
    pub policy_digest: String,
    pub policy_id: String,
    pub maximum_dispatch_occurrences_per_attempt: u16,
    pub backoff_seconds: Vec<u64>,
    pub maximum_total_deferral_seconds: u64,
    pub parked_resource_lock_policy: ParkedResourceLockPolicyV1,
    pub provider_capacity_released_while_parked: bool,
    pub reconcile_indeterminate: bool,
    pub allow_ordered_model_fallback: bool,
    pub automatic_semantic_retry: bool,
    pub approval_response_authorized: bool,
    pub authority_effect: String,
}

impl ExecutionAvailabilityPolicyV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        parse(bytes)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.policy_digest = digest_without(self, "policy_digest", POLICY_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(&self.schema, EXECUTION_AVAILABILITY_POLICY_SCHEMA_V1)?;
        sealed_digest(self, "policy_digest", &self.policy_digest, POLICY_DOMAIN)?;
        id("policy_id", &self.policy_id)?;
        let total = self
            .backoff_seconds
            .iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value));
        if !(1..=MAXIMUM_DISPATCH_OCCURRENCES)
            .contains(&self.maximum_dispatch_occurrences_per_attempt)
            || self.backoff_seconds.is_empty()
            || self.backoff_seconds.len()
                > usize::from(self.maximum_dispatch_occurrences_per_attempt)
            || self
                .backoff_seconds
                .iter()
                .any(|seconds| *seconds == 0 || *seconds > 86_400)
            || self.maximum_total_deferral_seconds == 0
            || self.maximum_total_deferral_seconds > MAXIMUM_TOTAL_DEFERRAL_SECONDS
            || total.is_none_or(|sum| sum > self.maximum_total_deferral_seconds)
            || !self.provider_capacity_released_while_parked
            || !self.reconcile_indeterminate
            || self.automatic_semantic_retry
            || self.approval_response_authorized
            || self.authority_effect != "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY"
        {
            return Err(ContractError::InvalidField("availability policy boundary"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAdmissionOwnerPinsV1 {
    pub codex_owner_head: String,
    pub switchyard_owner_head: String,
    pub switchyard_schema_sha256: String,
    pub deterministic_fixture_sha256: String,
}

impl ProviderAdmissionOwnerPinsV1 {
    pub fn accepted() -> Self {
        Self {
            codex_owner_head: ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD.to_owned(),
            switchyard_owner_head: ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_OWNER_HEAD.to_owned(),
            switchyard_schema_sha256: ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_SHA256
                .to_owned(),
            deterministic_fixture_sha256: ACCEPTED_SWITCHYARD_DETERMINISTIC_FIXTURE_SHA256
                .to_owned(),
        }
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        if self != &Self::accepted() {
            return Err(ContractError::InvalidField("provider admission owner pins"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderModelSelectionV1 {
    pub provider_id: String,
    pub model_id: String,
    pub model_class: String,
}

impl ProviderModelSelectionV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        id("selection provider_id", &self.provider_id)?;
        id("selection model_id", &self.model_id)?;
        id("selection model_class", &self.model_class)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForemanExecutionAvailabilityRequirementV1 {
    pub schema: String,
    pub requirement_digest: String,
    pub packet_digest: String,
    pub admission_digest: String,
    pub profile_digest: String,
    pub run_id: String,
    pub adapter_id: String,
    pub adapter_protocol: String,
    pub adapter_version: String,
    pub adapter_executable_identity: String,
    pub owner_pins: ProviderAdmissionOwnerPinsV1,
    pub policy_id: String,
    pub policy_digest: String,
    pub work_item_model_selections: BTreeMap<String, Vec<ProviderModelSelectionV1>>,
    pub admitted_at: DateTime<Utc>,
    pub authority_effect: String,
}

impl ForemanExecutionAvailabilityRequirementV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Value = parse(bytes)?;
        canonical_timestamp(&value, "admitted_at")?;
        serde_json::from_value(value).map_err(json_error)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.requirement_digest = digest_without(self, "requirement_digest", REQUIREMENT_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(
            &self.schema,
            FOREMAN_EXECUTION_AVAILABILITY_REQUIREMENT_SCHEMA_V1,
        )?;
        sealed_digest(
            self,
            "requirement_digest",
            &self.requirement_digest,
            REQUIREMENT_DOMAIN,
        )?;
        for (field, value) in [
            ("packet_digest", &self.packet_digest),
            ("admission_digest", &self.admission_digest),
            ("profile_digest", &self.profile_digest),
            (
                "adapter_executable_identity",
                &self.adapter_executable_identity,
            ),
            ("policy_digest", &self.policy_digest),
        ] {
            digest(field, value)?;
        }
        for (field, value) in [
            ("run_id", &self.run_id),
            ("adapter_id", &self.adapter_id),
            ("adapter_protocol", &self.adapter_protocol),
            ("adapter_version", &self.adapter_version),
            ("policy_id", &self.policy_id),
        ] {
            id(field, value)?;
        }
        self.owner_pins.validate()?;
        if self.work_item_model_selections.is_empty()
            || self.work_item_model_selections.len() > 1024
        {
            return Err(ContractError::InvalidField("work-item model selections"));
        }
        for (work_item, selections) in &self.work_item_model_selections {
            id("work_item_id", work_item)?;
            if selections.is_empty() || selections.len() > 16 {
                return Err(ContractError::InvalidField("model selection count"));
            }
            let mut exact = BTreeSet::new();
            for selection in selections {
                selection.validate()?;
                if !exact.insert((
                    selection.provider_id.as_str(),
                    selection.model_id.as_str(),
                    selection.model_class.as_str(),
                )) {
                    return Err(ContractError::InvalidField("duplicate model selection"));
                }
            }
        }
        if self.authority_effect != "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY" {
            return Err(ContractError::InvalidField("requirement authority effect"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderDispatchOccurrenceV1 {
    pub schema: String,
    pub dispatch_digest: String,
    pub requirement_digest: String,
    pub policy_digest: String,
    pub packet_digest: String,
    pub run_id: String,
    pub work_item_id: String,
    pub work_attempt_id: String,
    pub dispatch_occurrence_id: String,
    pub dispatch_ordinal: u16,
    pub selected_model_ordinal: u16,
    pub selection: ProviderModelSelectionV1,
    pub adapter_id: String,
    pub adapter_version: String,
    pub adapter_protocol: String,
    pub adapter_process_occurrence_id: String,
    pub app_server_session_identity: String,
    pub worker_start_request_schema: String,
    pub worker_start_request_digest: String,
    pub worker_brief_digest: String,
    pub opened_at: DateTime<Utc>,
    pub internal_provider_retry_count: u16,
    pub provider_execution_id: Option<String>,
    pub authority_effect: String,
}

impl ProviderDispatchOccurrenceV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Value = parse(bytes)?;
        canonical_timestamp(&value, "opened_at")?;
        serde_json::from_value(value).map_err(json_error)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.dispatch_digest = digest_without(self, "dispatch_digest", DISPATCH_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(&self.schema, PROVIDER_DISPATCH_OCCURRENCE_SCHEMA_V1)?;
        sealed_digest(
            self,
            "dispatch_digest",
            &self.dispatch_digest,
            DISPATCH_DOMAIN,
        )?;
        for (field, value) in [
            ("requirement_digest", &self.requirement_digest),
            ("policy_digest", &self.policy_digest),
            ("packet_digest", &self.packet_digest),
            (
                "worker_start_request_digest",
                &self.worker_start_request_digest,
            ),
            ("worker_brief_digest", &self.worker_brief_digest),
        ] {
            digest(field, value)?;
        }
        for (field, value) in [
            ("run_id", &self.run_id),
            ("work_item_id", &self.work_item_id),
            ("work_attempt_id", &self.work_attempt_id),
            ("dispatch_occurrence_id", &self.dispatch_occurrence_id),
            ("adapter_id", &self.adapter_id),
            ("adapter_version", &self.adapter_version),
            ("adapter_protocol", &self.adapter_protocol),
            (
                "adapter_process_occurrence_id",
                &self.adapter_process_occurrence_id,
            ),
            (
                "app_server_session_identity",
                &self.app_server_session_identity,
            ),
        ] {
            id(field, value)?;
        }
        self.selection.validate()?;
        if self.dispatch_ordinal == 0
            || self.dispatch_ordinal > MAXIMUM_DISPATCH_OCCURRENCES
            || self.selected_model_ordinal >= 16
            || self.worker_start_request_schema != "nightshift.worker-start-request/v3"
            || self.internal_provider_retry_count != 0
            || self.provider_execution_id.is_some()
            || self.authority_effect != "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY"
        {
            return Err(ContractError::InvalidField("dispatch occurrence boundary"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderAdmissionDispositionKindV1 {
    NotAdmittedModelAtCapacity,
    NotAdmittedProviderUnavailable,
    NotAdmittedRateLimited,
    AuthenticationRefused,
    QuotaExhaustedFuelOwned,
    AdmissionIndeterminate,
    ExecutionAdmitted,
}

impl ProviderAdmissionDispositionKindV1 {
    pub fn permits_automatic_park(self) -> bool {
        matches!(
            self,
            Self::NotAdmittedModelAtCapacity
                | Self::NotAdmittedProviderUnavailable
                | Self::NotAdmittedRateLimited
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderExecutionIdentityV1 {
    pub provider_id: String,
    pub model_id: String,
    pub app_server_session_identity: String,
    pub thread_id: String,
    pub turn_id: String,
    pub first_response_id: String,
}

impl ProviderExecutionIdentityV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        for (field, value) in [
            ("execution provider_id", &self.provider_id),
            ("execution model_id", &self.model_id),
            ("execution session", &self.app_server_session_identity),
            ("execution thread", &self.thread_id),
            ("execution turn", &self.turn_id),
            ("execution first_response_id", &self.first_response_id),
        ] {
            id(field, value)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderAdmissionDispositionV1 {
    pub schema: String,
    pub disposition_digest: String,
    pub dispatch_digest: String,
    pub requirement_digest: String,
    pub policy_digest: String,
    pub packet_digest: String,
    pub run_id: String,
    pub work_item_id: String,
    pub work_attempt_id: String,
    pub dispatch_occurrence_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub provider_request_occurrence_id: String,
    pub adapter_process_occurrence_id: String,
    pub app_server_session_identity: String,
    pub thread_id: String,
    pub turn_id: String,
    pub disposition: ProviderAdmissionDispositionKindV1,
    pub received_at: DateTime<Utc>,
    pub response_created: bool,
    pub will_retry: bool,
    pub acquisition_complete: bool,
    pub provider_retry_after: Option<DateTime<Utc>>,
    pub provider_execution: Option<ProviderExecutionIdentityV1>,
    pub mapper_snapshot_schema: String,
    pub mapper_snapshot_digest: String,
    pub mapper_snapshot: ExactAvailabilityEvidenceV1,
    pub approval_response_sent: bool,
    pub protected_effect_absent: bool,
    pub authority_effect: String,
}

impl ProviderAdmissionDispositionV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Value = parse(bytes)?;
        canonical_timestamp(&value, "received_at")?;
        canonical_optional_timestamp(&value, "provider_retry_after")?;
        serde_json::from_value(value).map_err(json_error)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.disposition_digest = digest_without(self, "disposition_digest", DISPOSITION_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(&self.schema, PROVIDER_ADMISSION_DISPOSITION_SCHEMA_V1)?;
        sealed_digest(
            self,
            "disposition_digest",
            &self.disposition_digest,
            DISPOSITION_DOMAIN,
        )?;
        for (field, value) in [
            ("dispatch_digest", &self.dispatch_digest),
            ("requirement_digest", &self.requirement_digest),
            ("policy_digest", &self.policy_digest),
            ("packet_digest", &self.packet_digest),
            ("mapper_snapshot_digest", &self.mapper_snapshot_digest),
        ] {
            digest(field, value)?;
        }
        for (field, value) in [
            ("run_id", &self.run_id),
            ("work_item_id", &self.work_item_id),
            ("work_attempt_id", &self.work_attempt_id),
            ("dispatch_occurrence_id", &self.dispatch_occurrence_id),
            ("provider_id", &self.provider_id),
            ("model_id", &self.model_id),
            (
                "provider_request_occurrence_id",
                &self.provider_request_occurrence_id,
            ),
            (
                "adapter_process_occurrence_id",
                &self.adapter_process_occurrence_id,
            ),
            (
                "app_server_session_identity",
                &self.app_server_session_identity,
            ),
            ("thread_id", &self.thread_id),
            ("turn_id", &self.turn_id),
        ] {
            id(field, value)?;
        }
        if self.mapper_snapshot_schema != "switchyard.codex-provider-admission-snapshot/v1" {
            return Err(ContractError::InvalidField("mapper snapshot schema"));
        }
        let raw = self.mapper_snapshot.validate()?;
        validate_switchyard_snapshot(self, &raw)?;
        if self.will_retry || self.approval_response_sent || !self.protected_effect_absent {
            return Err(ContractError::InvalidField(
                "provider admission authority boundary",
            ));
        }
        match self.disposition {
            ProviderAdmissionDispositionKindV1::ExecutionAdmitted => {
                let execution =
                    self.provider_execution
                        .as_ref()
                        .ok_or(ContractError::InvalidField(
                            "provider execution identity absence",
                        ))?;
                execution.validate()?;
                if !self.response_created
                    || !self.acquisition_complete
                    || execution.provider_id != self.provider_id
                    || execution.model_id != self.model_id
                    || execution.app_server_session_identity != self.app_server_session_identity
                    || execution.thread_id != self.thread_id
                    || execution.turn_id != self.turn_id
                {
                    return Err(ContractError::InvalidField("execution admission binding"));
                }
            }
            ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity
            | ProviderAdmissionDispositionKindV1::NotAdmittedProviderUnavailable
            | ProviderAdmissionDispositionKindV1::NotAdmittedRateLimited
            | ProviderAdmissionDispositionKindV1::AuthenticationRefused => {
                if self.provider_execution.is_some()
                    || self.response_created
                    || !self.acquisition_complete
                {
                    return Err(ContractError::InvalidField("pre-admission refusal binding"));
                }
            }
            ProviderAdmissionDispositionKindV1::QuotaExhaustedFuelOwned => {
                return Err(ContractError::InvalidField("FUEL-owned quota disposition"));
            }
            ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => {
                if self.provider_execution.is_some() && !self.response_created {
                    return Err(ContractError::InvalidField(
                        "indeterminate execution binding",
                    ));
                }
            }
        }
        if self
            .provider_retry_after
            .is_some_and(|retry_after| retry_after < self.received_at)
            || self.authority_effect != "SCHEDULING_MECHANISM_EVIDENCE_ONLY"
        {
            return Err(ContractError::InvalidField(
                "admission disposition boundary",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeferredWakeBasisV1 {
    ProviderRetryAfter,
    PolicyBackoff,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredProviderDispatchV1 {
    pub schema: String,
    pub deferred_dispatch_digest: String,
    pub requirement_digest: String,
    pub policy_digest: String,
    pub disposition_digest: String,
    pub packet_digest: String,
    pub run_id: String,
    pub work_item_id: String,
    pub work_attempt_id: String,
    pub last_dispatch_occurrence_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub selected_model_ordinal: u16,
    pub remaining_model_ordinals: Vec<u16>,
    pub refusal_received_at: DateTime<Utc>,
    pub wake_basis: DeferredWakeBasisV1,
    pub backoff_ordinal: u16,
    pub backoff_seconds: u64,
    pub provider_retry_after: Option<DateTime<Utc>>,
    pub wake_at: DateTime<Utc>,
    pub parked_resource_lock_policy: ParkedResourceLockPolicyV1,
    pub provider_capacity_released: bool,
    pub semantic_retry: bool,
    pub authority_effect: String,
}

impl DeferredProviderDispatchV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ContractError> {
        let value: Value = parse(bytes)?;
        for field in ["refusal_received_at", "wake_at"] {
            canonical_timestamp(&value, field)?;
        }
        canonical_optional_timestamp(&value, "provider_retry_after")?;
        serde_json::from_value(value).map_err(json_error)
    }
    pub fn seal(&mut self) -> Result<(), ContractError> {
        self.deferred_dispatch_digest =
            digest_without(self, "deferred_dispatch_digest", DEFERRED_DOMAIN)?;
        self.validate()
    }
    pub fn validate(&self) -> Result<(), ContractError> {
        schema(&self.schema, DEFERRED_PROVIDER_DISPATCH_SCHEMA_V1)?;
        sealed_digest(
            self,
            "deferred_dispatch_digest",
            &self.deferred_dispatch_digest,
            DEFERRED_DOMAIN,
        )?;
        for (field, value) in [
            ("requirement_digest", &self.requirement_digest),
            ("policy_digest", &self.policy_digest),
            ("disposition_digest", &self.disposition_digest),
            ("packet_digest", &self.packet_digest),
        ] {
            digest(field, value)?;
        }
        for (field, value) in [
            ("run_id", &self.run_id),
            ("work_item_id", &self.work_item_id),
            ("work_attempt_id", &self.work_attempt_id),
            (
                "last_dispatch_occurrence_id",
                &self.last_dispatch_occurrence_id,
            ),
            ("provider_id", &self.provider_id),
            ("model_id", &self.model_id),
        ] {
            id(field, value)?;
        }
        if self.selected_model_ordinal >= 16
            || self.backoff_ordinal >= MAXIMUM_DISPATCH_OCCURRENCES
            || self.backoff_seconds == 0
            || self.backoff_seconds > 86_400
            || self.remaining_model_ordinals.len() > 16
            || self
                .remaining_model_ordinals
                .iter()
                .any(|ordinal| *ordinal >= 16 || *ordinal <= self.selected_model_ordinal)
            || self
                .remaining_model_ordinals
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || !self.provider_capacity_released
            || self.semantic_retry
            || self.authority_effect != "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY"
        {
            return Err(ContractError::InvalidField("deferred dispatch boundary"));
        }
        let expected_wake = match self.wake_basis {
            DeferredWakeBasisV1::ProviderRetryAfter => self
                .provider_retry_after
                .ok_or(ContractError::InvalidField("provider_retry_after"))?,
            DeferredWakeBasisV1::PolicyBackoff => {
                if self.provider_retry_after.is_some() {
                    return Err(ContractError::InvalidField(
                        "unexpected provider_retry_after",
                    ));
                }
                self.refusal_received_at
                    .checked_add_signed(Duration::seconds(self.backoff_seconds as i64))
                    .ok_or(ContractError::InvalidField("wake_at"))?
            }
        };
        if self.wake_at != expected_wake || self.wake_at <= self.refusal_received_at {
            return Err(ContractError::InvalidField("wake_at"));
        }
        Ok(())
    }
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ContractError> {
    serde_json::from_slice(bytes).map_err(json_error)
}

fn digest_without<T: Serialize>(
    record: &T,
    field: &'static str,
    domain: &[u8],
) -> Result<String, ContractError> {
    let mut value = serde_json::to_value(record).map_err(json_error)?;
    value
        .as_object_mut()
        .ok_or(ContractError::InvalidField("record"))?
        .remove(field);
    let canonical = serde_jcs::to_vec(&value).map_err(json_error)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(canonical);
    Ok(format!("sha256:{:x}", digest.finalize()))
}

fn sealed_digest<T: Serialize>(
    value: &T,
    field: &'static str,
    observed: &str,
    domain: &[u8],
) -> Result<(), ContractError> {
    digest(field, observed)?;
    if digest_without(value, field, domain)? != observed {
        return Err(ContractError::DigestMismatch(field));
    }
    Ok(())
}

fn plain_sha256(raw: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(raw))
}

fn schema(actual: &str, expected: &str) -> Result<(), ContractError> {
    if actual != expected {
        return Err(ContractError::ForeignSchema(actual.to_owned()));
    }
    Ok(())
}

fn digest(field: &'static str, value: &str) -> Result<(), ContractError> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ContractError::InvalidField(field));
    }
    Ok(())
}

fn id(field: &'static str, value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > 512
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/')
        })
    {
        return Err(ContractError::InvalidField(field));
    }
    Ok(())
}

fn canonical_timestamp(value: &Value, field: &'static str) -> Result<(), ContractError> {
    let raw = value
        .as_object()
        .and_then(|object| object.get(field))
        .and_then(Value::as_str)
        .ok_or(ContractError::InvalidField(field))?;
    let parsed = DateTime::parse_from_rfc3339(raw)
        .map_err(|_| ContractError::InvalidField(field))?
        .with_timezone(&Utc);
    if serde_json::to_value(parsed).map_err(json_error)?.as_str() != Some(raw) {
        return Err(ContractError::InvalidField(field));
    }
    Ok(())
}

fn canonical_optional_timestamp(value: &Value, field: &'static str) -> Result<(), ContractError> {
    match value.as_object().and_then(|object| object.get(field)) {
        Some(Value::Null) => Ok(()),
        Some(Value::String(_)) => canonical_timestamp(value, field),
        _ => Err(ContractError::InvalidField(field)),
    }
}

fn json_error(error: impl std::fmt::Display) -> ContractError {
    ContractError::Json(error.to_string())
}

fn validate_switchyard_snapshot(
    disposition: &ProviderAdmissionDispositionV1,
    raw: &[u8],
) -> Result<(), ContractError> {
    let snapshot: Value = serde_json::from_slice(raw).map_err(json_error)?;
    if serde_jcs::to_vec(&snapshot).map_err(json_error)? != raw {
        return Err(ContractError::InvalidField(
            "mapper snapshot canonical bytes",
        ));
    }
    exact_object_keys(
        &snapshot,
        &[
            "schema",
            "snapshot_digest",
            "binding",
            "admission_disposition",
            "mechanism_state",
            "provider_execution_identity",
            "acquisition_cut",
            "records",
        ],
        "mapper snapshot closure",
    )?;
    if string(&snapshot, "schema")? != disposition.mapper_snapshot_schema
        || string(&snapshot, "snapshot_digest")? != disposition.mapper_snapshot_digest
        || canonical_value_digest(
            &snapshot,
            "snapshot_digest",
            b"switchyard.codex-provider-admission-snapshot.digest/v1\0",
        )? != disposition.mapper_snapshot_digest
    {
        return Err(ContractError::DigestMismatch("mapper snapshot digest"));
    }

    let binding = snapshot
        .get("binding")
        .ok_or(ContractError::InvalidField("mapper binding"))?;
    exact_object_keys(
        binding,
        &[
            "schema",
            "binding_digest",
            "work_attempt_id",
            "dispatch_occurrence_id",
            "adapter_process_occurrence_id",
            "app_server_session_identity",
            "thread_id",
            "turn_id",
            "provider",
            "model",
            "codex_source_head",
            "executable_kind",
            "app_server_executable_identity",
            "app_server_executable_sha256",
            "internal_provider_request_retries",
        ],
        "mapper binding closure",
    )?;
    if string(binding, "schema")? != "switchyard.codex-provider-admission-binding/v1"
        || string(binding, "codex_source_head")? != ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD
        || integer(binding, "internal_provider_request_retries")? != 0
        || canonical_value_digest(
            binding,
            "binding_digest",
            b"switchyard.codex-provider-admission-binding.digest/v1\0",
        )? != string(binding, "binding_digest")?
    {
        return Err(ContractError::InvalidField("mapper binding owner law"));
    }
    for (field, expected) in [
        ("work_attempt_id", disposition.work_attempt_id.as_str()),
        (
            "dispatch_occurrence_id",
            disposition.dispatch_occurrence_id.as_str(),
        ),
        (
            "adapter_process_occurrence_id",
            disposition.adapter_process_occurrence_id.as_str(),
        ),
        (
            "app_server_session_identity",
            disposition.app_server_session_identity.as_str(),
        ),
        ("thread_id", disposition.thread_id.as_str()),
        ("turn_id", disposition.turn_id.as_str()),
        ("provider", disposition.provider_id.as_str()),
        ("model", disposition.model_id.as_str()),
    ] {
        if string(binding, field)? != expected {
            return Err(ContractError::InvalidField("mapper binding substitution"));
        }
    }
    digest(
        "mapper executable sha256",
        string(binding, "app_server_executable_sha256")?,
    )?;

    let records = snapshot
        .get("records")
        .and_then(Value::as_array)
        .ok_or(ContractError::InvalidField("mapper records"))?;
    if records.len() > 4096 {
        return Err(ContractError::InvalidField("mapper record count"));
    }
    let mut response_identity: Option<ProviderExecutionIdentityV1> = None;
    let mut refusal_occurrence: Option<String> = None;
    let mut saw_waiting_approval = false;
    for (sequence, record) in records.iter().enumerate() {
        exact_object_keys(
            record,
            &[
                "schema",
                "evidence_digest",
                "sequence",
                "acquisition_ordinal",
                "acquisition_kind",
                "binding_digest",
                "work_attempt_id",
                "dispatch_occurrence_id",
                "adapter_process_occurrence_id",
                "app_server_session_identity",
                "thread_id",
                "turn_id",
                "provider",
                "model",
                "kind",
                "method",
                "normalized",
                "raw",
            ],
            "mapper evidence closure",
        )?;
        if string(record, "schema")? != "switchyard.codex-provider-admission-evidence/v1"
            || integer(record, "sequence")? != sequence as i64
            || string(record, "binding_digest")? != string(binding, "binding_digest")?
            || canonical_value_digest(
                record,
                "evidence_digest",
                b"switchyard.codex-provider-admission-evidence.digest/v1\0",
            )? != string(record, "evidence_digest")?
        {
            return Err(ContractError::InvalidField("mapper evidence owner law"));
        }
        for (field, expected) in [
            ("work_attempt_id", disposition.work_attempt_id.as_str()),
            (
                "dispatch_occurrence_id",
                disposition.dispatch_occurrence_id.as_str(),
            ),
            (
                "adapter_process_occurrence_id",
                disposition.adapter_process_occurrence_id.as_str(),
            ),
            (
                "app_server_session_identity",
                disposition.app_server_session_identity.as_str(),
            ),
            ("thread_id", disposition.thread_id.as_str()),
            ("turn_id", disposition.turn_id.as_str()),
            ("provider", disposition.provider_id.as_str()),
            ("model", disposition.model_id.as_str()),
        ] {
            if string(record, field)? != expected {
                return Err(ContractError::InvalidField("mapper evidence substitution"));
            }
        }
        if let Some(raw_evidence) = record.get("raw").filter(|raw_value| !raw_value.is_null()) {
            validate_switchyard_raw(raw_evidence)?;
        }
        let normalized = record
            .get("normalized")
            .ok_or(ContractError::InvalidField("mapper normalized evidence"))?;
        match string(record, "kind")? {
            "PROVIDER_EXECUTION_STEP" => {
                let identity = normalized
                    .get("provider_execution_identity")
                    .ok_or(ContractError::InvalidField("mapper execution identity"))?;
                let observed = ProviderExecutionIdentityV1 {
                    provider_id: string(identity, "provider")?.to_owned(),
                    model_id: string(identity, "model")?.to_owned(),
                    app_server_session_identity: string(identity, "app_server_session_identity")?
                        .to_owned(),
                    thread_id: string(identity, "thread_id")?.to_owned(),
                    turn_id: string(identity, "turn_id")?.to_owned(),
                    first_response_id: string(identity, "first_response_id")?.to_owned(),
                };
                observed.validate()?;
                if response_identity
                    .as_ref()
                    .is_some_and(|prior| prior != &observed)
                {
                    return Err(ContractError::InvalidField("mapper execution substitution"));
                }
                response_identity = Some(observed);
            }
            "PROVIDER_ADMISSION_REFUSED" => {
                if normalized.get("response_created").and_then(Value::as_bool) != Some(false)
                    || normalized.get("will_retry").and_then(Value::as_bool) != Some(false)
                    || string(normalized, "refusal_kind")? != "MODEL_AT_CAPACITY"
                    || string(normalized, "codex_error_info")? != "serverOverloaded"
                    || !normalized
                        .get("provider_execution_identity")
                        .is_some_and(Value::is_null)
                {
                    return Err(ContractError::InvalidField("mapper refusal owner law"));
                }
                refusal_occurrence = Some(string(normalized, "request_occurrence_id")?.to_owned());
            }
            "WAITING_APPROVAL" => {
                if normalized
                    .get("approval_response_sent")
                    .and_then(Value::as_bool)
                    != Some(false)
                    || normalized
                        .get("protected_effect_absent")
                        .and_then(Value::as_bool)
                        != Some(true)
                {
                    return Err(ContractError::InvalidField("mapper approval boundary"));
                }
                saw_waiting_approval = true;
            }
            _ => {}
        }
    }

    let snapshot_execution = snapshot.get("provider_execution_identity");
    let cut = snapshot.get("acquisition_cut");
    let clean_cut = cut
        .filter(|value| !value.is_null())
        .and_then(|value| value.get("clean"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if clean_cut != disposition.acquisition_complete {
        return Err(ContractError::InvalidField(
            "mapper acquisition-cut binding",
        ));
    }
    if saw_waiting_approval
        && (disposition.approval_response_sent || !disposition.protected_effect_absent)
    {
        return Err(ContractError::InvalidField("mapper approval substitution"));
    }
    match disposition.disposition {
        ProviderAdmissionDispositionKindV1::ExecutionAdmitted => {
            if string(&snapshot, "admission_disposition")? != "EXECUTION_ADMITTED"
                || response_identity.as_ref() != disposition.provider_execution.as_ref()
                || snapshot_execution.is_none_or(Value::is_null)
            {
                return Err(ContractError::InvalidField("mapper execution disposition"));
            }
        }
        ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity => {
            if string(&snapshot, "admission_disposition")? != "NOT_ADMITTED_MODEL_AT_CAPACITY"
                || refusal_occurrence.as_deref()
                    != Some(disposition.provider_request_occurrence_id.as_str())
                || response_identity.is_some()
                || snapshot_execution.is_some_and(|value| !value.is_null())
            {
                return Err(ContractError::InvalidField("mapper refusal disposition"));
            }
        }
        ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => {
            if string(&snapshot, "admission_disposition")? != "ADMISSION_INDETERMINATE" {
                return Err(ContractError::InvalidField(
                    "mapper indeterminate disposition",
                ));
            }
        }
        _ => {
            return Err(ContractError::InvalidField(
                "disposition is not emitted by accepted Switchyard V1",
            ));
        }
    }
    Ok(())
}

fn validate_switchyard_raw(value: &Value) -> Result<(), ContractError> {
    exact_object_keys(
        value,
        &[
            "representation",
            "byte_length",
            "sha256",
            "encoding",
            "bytes_hex",
        ],
        "mapper raw closure",
    )?;
    let representation = string(value, "representation")?;
    if !matches!(
        representation,
        "EXACT_WIRE_BYTES_INCLUDING_LINE_TERMINATOR"
            | "EXACT_ACQUIRED_FRAME_BYTES_INCLUDING_LINE_TERMINATOR"
    ) || string(value, "encoding")? != "hex"
    {
        return Err(ContractError::InvalidField("mapper raw representation"));
    }
    let bytes = hex::decode(string(value, "bytes_hex")?)
        .map_err(|_| ContractError::InvalidField("mapper raw hex"))?;
    if bytes.is_empty()
        || bytes.len() > MAXIMUM_AVAILABILITY_EVIDENCE_BYTES
        || bytes.last() != Some(&b'\n')
        || integer(value, "byte_length")? != bytes.len() as i64
        || string(value, "sha256")? != plain_sha256(&bytes)
    {
        return Err(ContractError::InvalidField("mapper raw custody"));
    }
    Ok(())
}

fn exact_object_keys(
    value: &Value,
    expected: &[&str],
    field: &'static str,
) -> Result<(), ContractError> {
    let object = value
        .as_object()
        .ok_or(ContractError::InvalidField(field))?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(ContractError::InvalidField(field));
    }
    Ok(())
}

fn string<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, ContractError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or(ContractError::InvalidField(field))
}

fn integer(value: &Value, field: &'static str) -> Result<i64, ContractError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .ok_or(ContractError::InvalidField(field))
}

fn canonical_value_digest(
    value: &Value,
    field: &'static str,
    domain: &[u8],
) -> Result<String, ContractError> {
    let mut basis = value.clone();
    basis
        .as_object_mut()
        .ok_or(ContractError::InvalidField(field))?
        .remove(field);
    let canonical = serde_jcs::to_vec(&basis).map_err(json_error)?;
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(canonical);
    Ok(format!("sha256:{:x}", hash.finalize()))
}
