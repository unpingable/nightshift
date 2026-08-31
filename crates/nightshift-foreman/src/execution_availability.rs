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
pub const MAXIMUM_SWITCHYARD_MAPPER_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
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
const SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../schemas/vendor/switchyard.codex-provider-admission.v1.schema.json");

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
        if !matches!(
            self.representation.as_str(),
            "EXACT_PROVIDER_AVAILABILITY_SOURCE_BYTES"
                | "EXACT_WIRE_BYTES_INCLUDING_LINE_TERMINATOR"
                | "EXACT_ACQUIRED_FRAME_BYTES_INCLUDING_LINE_TERMINATOR"
        ) {
            return Err(ContractError::InvalidField(
                "availability evidence representation",
            ));
        }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactMapperSnapshotV1 {
    pub representation: String,
    pub byte_length: u64,
    pub sha256: String,
    pub encoding: String,
    pub bytes_hex: String,
}

impl ExactMapperSnapshotV1 {
    pub fn from_bytes(raw: &[u8]) -> Result<Self, ContractError> {
        if raw.is_empty() || raw.len() > MAXIMUM_SWITCHYARD_MAPPER_SNAPSHOT_BYTES {
            return Err(ContractError::InvalidField("mapper snapshot bytes"));
        }
        let value = Self {
            representation: "RFC8785_SWITCHYARD_MAPPER_SNAPSHOT".to_owned(),
            byte_length: raw.len() as u64,
            sha256: plain_sha256(raw),
            encoding: "hex".to_owned(),
            bytes_hex: hex::encode(raw),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<Vec<u8>, ContractError> {
        digest("mapper snapshot sha256", &self.sha256)?;
        if self.representation != "RFC8785_SWITCHYARD_MAPPER_SNAPSHOT"
            || self.encoding != "hex"
            || self.byte_length == 0
            || self.byte_length as usize > MAXIMUM_SWITCHYARD_MAPPER_SNAPSHOT_BYTES
            || self.bytes_hex.len() != self.byte_length as usize * 2
        {
            return Err(ContractError::InvalidField("mapper snapshot encoding"));
        }
        let raw = hex::decode(&self.bytes_hex)
            .map_err(|_| ContractError::InvalidField("mapper snapshot hex"))?;
        if plain_sha256(&raw) != self.sha256 {
            return Err(ContractError::DigestMismatch("mapper snapshot sha256"));
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
        self.received_at <= evaluated_at && evaluated_at < self.expires_at
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderMechanismStateV1 {
    ParkedNotAdmitted,
    AdmissionIndeterminate,
    ExecutionAdmitted,
    PostAdmissionInterrupted,
    WaitingApproval,
    ProviderCompleted,
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
    pub mechanism_state: ProviderMechanismStateV1,
    pub received_at: DateTime<Utc>,
    pub response_created: bool,
    pub will_retry: bool,
    pub acquisition_complete: bool,
    pub provider_retry_after: Option<DateTime<Utc>>,
    pub provider_execution: Option<ProviderExecutionIdentityV1>,
    pub mapper_snapshot_schema: String,
    pub mapper_snapshot_digest: String,
    pub mapper_snapshot: ExactMapperSnapshotV1,
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
        validate_switchyard_snapshot_complete(self, &raw)?;
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
        || !value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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
    validate_vendored_switchyard_schema(&snapshot)?;
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

fn validate_vendored_switchyard_schema(instance: &Value) -> Result<(), ContractError> {
    if plain_sha256(SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_BYTES)
        != ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_SHA256
    {
        return Err(ContractError::DigestMismatch(
            "vendored Switchyard schema sha256",
        ));
    }
    let root: Value =
        serde_json::from_slice(SWITCHYARD_PROVIDER_ADMISSION_SCHEMA_BYTES).map_err(json_error)?;
    validate_schema_node(&root, &root, instance, 0)?;
    Ok(())
}

fn validate_schema_node(
    root: &Value,
    node: &Value,
    instance: &Value,
    depth: usize,
) -> Result<BTreeSet<String>, ContractError> {
    if depth > 64 {
        return Err(ContractError::InvalidField("Switchyard schema depth"));
    }
    let schema = node
        .as_object()
        .ok_or(ContractError::InvalidField("Switchyard schema node"))?;
    let mut evaluated = BTreeSet::new();

    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let name = reference
            .strip_prefix("#/$defs/")
            .ok_or(ContractError::InvalidField("Switchyard schema ref"))?;
        let target = root
            .get("$defs")
            .and_then(|defs| defs.get(name))
            .ok_or(ContractError::InvalidField("Switchyard schema ref"))?;
        evaluated.extend(validate_schema_node(root, target, instance, depth + 1)?);
    }
    if let Some(all) = schema.get("allOf").and_then(Value::as_array) {
        for branch in all {
            evaluated.extend(validate_schema_node(root, branch, instance, depth + 1)?);
        }
    }
    if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
        let matches: Vec<_> = branches
            .iter()
            .filter_map(|branch| validate_schema_node(root, branch, instance, depth + 1).ok())
            .collect();
        if matches.len() != 1 {
            return Err(ContractError::InvalidField("Switchyard schema oneOf"));
        }
        evaluated.extend(matches.into_iter().next().expect("one exact match"));
    }
    if let Some(expected) = schema.get("const") {
        if expected != instance {
            return Err(ContractError::InvalidField("Switchyard schema const"));
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        if !values.contains(instance) {
            return Err(ContractError::InvalidField("Switchyard schema enum"));
        }
    }
    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        let type_matches = match expected_type {
            "null" => instance.is_null(),
            "boolean" => instance.is_boolean(),
            "string" => instance.is_string(),
            "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
            "object" => instance.is_object(),
            "array" => instance.is_array(),
            _ => false,
        };
        if !type_matches {
            return Err(ContractError::InvalidField("Switchyard schema type"));
        }
    }

    if let Some(text) = instance.as_str() {
        let length = text.chars().count() as u64;
        if schema
            .get("minLength")
            .and_then(Value::as_u64)
            .is_some_and(|minimum| length < minimum)
            || schema
                .get("maxLength")
                .and_then(Value::as_u64)
                .is_some_and(|maximum| length > maximum)
        {
            return Err(ContractError::InvalidField(
                "Switchyard schema string bound",
            ));
        }
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            let matches = match pattern {
                "^[0-9a-f]+$" => {
                    !text.is_empty()
                        && text
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                }
                "^sha256:[0-9a-f]{64}$" => {
                    text.len() == 71
                        && text.starts_with("sha256:")
                        && text[7..]
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                }
                _ => false,
            };
            if !matches {
                return Err(ContractError::InvalidField("Switchyard schema pattern"));
            }
        }
    }
    if instance.as_i64().is_some() || instance.as_u64().is_some() {
        let value = instance
            .as_i64()
            .map(|value| value as i128)
            .or_else(|| instance.as_u64().map(|value| value as i128))
            .ok_or(ContractError::InvalidField("Switchyard schema integer"))?;
        if schema
            .get("minimum")
            .and_then(Value::as_i64)
            .is_some_and(|minimum| value < minimum as i128)
            || schema
                .get("maximum")
                .and_then(Value::as_i64)
                .is_some_and(|maximum| value > maximum as i128)
        {
            return Err(ContractError::InvalidField(
                "Switchyard schema numeric bound",
            ));
        }
    }
    if let Some(items) = instance.as_array() {
        if schema
            .get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|maximum| items.len() as u64 > maximum)
        {
            return Err(ContractError::InvalidField("Switchyard schema array bound"));
        }
        if let Some(item_schema) = schema.get("items") {
            for item in items {
                validate_schema_node(root, item_schema, item, depth + 1)?;
            }
        }
    }
    if let Some(object) = instance.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            if required.iter().any(|field| {
                field
                    .as_str()
                    .is_none_or(|field| !object.contains_key(field))
            }) {
                return Err(ContractError::InvalidField("Switchyard schema required"));
            }
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (name, property_schema) in properties {
                evaluated.insert(name.clone());
                if let Some(value) = object.get(name) {
                    validate_schema_node(root, property_schema, value, depth + 1)?;
                }
            }
            if schema.get("additionalProperties").and_then(Value::as_bool) == Some(false)
                && object.keys().any(|name| !properties.contains_key(name))
            {
                return Err(ContractError::InvalidField(
                    "Switchyard schema additional property",
                ));
            }
        }
        if schema.get("unevaluatedProperties").and_then(Value::as_bool) == Some(false)
            && object.keys().any(|name| !evaluated.contains(name))
        {
            return Err(ContractError::InvalidField(
                "Switchyard schema unevaluated property",
            ));
        }
    }
    Ok(evaluated)
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

fn validate_switchyard_snapshot_complete(
    disposition: &ProviderAdmissionDispositionV1,
    raw: &[u8],
) -> Result<(), ContractError> {
    validate_switchyard_snapshot(disposition, raw)?;
    let snapshot: Value = serde_json::from_slice(raw).map_err(json_error)?;
    validate_safe_json(&snapshot)?;
    let records = snapshot["records"]
        .as_array()
        .ok_or(ContractError::InvalidField("mapper records"))?;
    if records.is_empty() {
        return Err(ContractError::InvalidField("empty mapper evidence"));
    }

    let mut next_acquisition_ordinal = 0_i64;
    let mut pending_request: Option<(String, i64, i64)> = None;
    let mut client_requests: BTreeMap<i64, (String, String)> = BTreeMap::new();
    let mut execution: Option<ProviderExecutionIdentityV1> = None;
    let mut open_response: Option<String> = None;
    let mut completed_responses = BTreeSet::new();
    let mut refusal: Option<(String, Option<i64>, i64)> = None;
    let mut saw_turn_completed = false;
    let mut saw_approval = false;
    let mut saw_discrepancy = false;
    let mut cut_value: Option<&Value> = None;

    for (index, record) in records.iter().enumerate() {
        let kind = string(record, "kind")?;
        let method = string(record, "method")?;
        let acquisition_ordinal = record
            .get("acquisition_ordinal")
            .ok_or(ContractError::InvalidField("acquisition ordinal"))?;
        let acquisition_kind = record
            .get("acquisition_kind")
            .ok_or(ContractError::InvalidField("acquisition kind"))?;
        let normalized = record
            .get("normalized")
            .ok_or(ContractError::InvalidField("mapper normalized evidence"))?;
        let raw_evidence = record
            .get("raw")
            .ok_or(ContractError::InvalidField("mapper raw evidence"))?;

        if kind == "ACQUISITION_CUT" {
            if index + 1 != records.len()
                || !acquisition_ordinal.is_null()
                || !acquisition_kind.is_null()
                || !raw_evidence.is_null()
                || method != "adapter/acquisition-cut"
            {
                return Err(ContractError::InvalidField(
                    "terminal acquisition-cut order",
                ));
            }
            exact_object_keys(
                normalized,
                &[
                    "adapter_process_occurrence_id",
                    "app_server_session_identity",
                    "stream_quiesced",
                    "loss_generation",
                    "process_disposition",
                    "ordered_high_water",
                    "consumed_ordinal_count",
                    "outstanding_client_request_count",
                    "clean",
                ],
                "acquisition-cut closure",
            )?;
            cut_value = Some(normalized);
            continue;
        }
        if cut_value.is_some() {
            return Err(ContractError::InvalidField("evidence after terminal cut"));
        }

        if let Some(ordinal) = acquisition_ordinal.as_i64() {
            if ordinal != next_acquisition_ordinal {
                return Err(ContractError::InvalidField(
                    "acquisition ordinal continuity",
                ));
            }
            next_acquisition_ordinal += 1;
            let lane = acquisition_kind
                .as_str()
                .ok_or(ContractError::InvalidField("acquisition kind"))?;
            if !matches!(
                lane,
                "LOSS"
                    | "NOTIFICATION"
                    | "SERVER_REQUEST"
                    | "CLIENT_REQUEST"
                    | "CLIENT_RESPONSE"
                    | "UNKNOWN"
            ) {
                return Err(ContractError::InvalidField("acquisition kind"));
            }
            if let Some(raw_value) = raw_evidence.as_object() {
                let representation = raw_value
                    .get("representation")
                    .and_then(Value::as_str)
                    .ok_or(ContractError::InvalidField("mapper raw representation"))?;
                let expected = if lane == "LOSS" {
                    "EXACT_ACQUIRED_FRAME_BYTES_INCLUDING_LINE_TERMINATOR"
                } else {
                    "EXACT_WIRE_BYTES_INCLUDING_LINE_TERMINATOR"
                };
                if representation != expected {
                    return Err(ContractError::InvalidField(
                        "mapper raw lane representation",
                    ));
                }
            } else if lane != "LOSS" {
                return Err(ContractError::InvalidField("mapper raw evidence absence"));
            }
            validate_kind_lane(kind, method, lane)?;
        } else if !acquisition_kind.is_null() {
            return Err(ContractError::InvalidField("partial acquisition identity"));
        }

        match kind {
            "PROVIDER_REQUEST_STARTED" => {
                exact_object_keys(
                    normalized,
                    &[
                        "request_occurrence_id",
                        "sampling_ordinal",
                        "request_order",
                        "started_at_ms",
                        "proves_provider_admission",
                    ],
                    "provider request normalized closure",
                )?;
                if normalized
                    .get("proves_provider_admission")
                    .and_then(Value::as_bool)
                    != Some(false)
                    || pending_request.is_some()
                    || saw_approval
                {
                    return Err(ContractError::InvalidField("provider request transition"));
                }
                pending_request = Some((
                    string(normalized, "request_occurrence_id")?.to_owned(),
                    integer(normalized, "sampling_ordinal")?,
                    integer(normalized, "request_order")?,
                ));
            }
            "PROVIDER_EXECUTION_STEP" => {
                exact_object_keys(
                    normalized,
                    &[
                        "provider_execution_identity",
                        "provider_execution_step_identity",
                        "first_admission_boundary",
                        "observed_at_ms",
                    ],
                    "provider step normalized closure",
                )?;
                let step = normalized
                    .get("provider_execution_step_identity")
                    .ok_or(ContractError::InvalidField("provider step identity"))?;
                let (pending, sampling_ordinal, request_order) = pending_request.take().ok_or(
                    ContractError::InvalidField("response without provider request"),
                )?;
                if string(step, "request_occurrence_id")? != pending
                    || string(step, "provider")? != disposition.provider_id
                    || string(step, "model")? != disposition.model_id
                    || string(step, "thread_id")? != disposition.thread_id
                    || string(step, "turn_id")? != disposition.turn_id
                    || integer(step, "sampling_ordinal")? != sampling_ordinal
                    || integer(step, "request_order")? != request_order
                {
                    return Err(ContractError::InvalidField("provider step request binding"));
                }
                if execution.is_none() && pending != disposition.provider_request_occurrence_id {
                    return Err(ContractError::InvalidField(
                        "admitted provider request occurrence",
                    ));
                }
                let identity = normalized
                    .get("provider_execution_identity")
                    .ok_or(ContractError::InvalidField("provider execution identity"))?;
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
                if provider_execution_from_switchyard(identity)? != observed
                    || provider_execution_from_switchyard(
                        normalized
                            .get("provider_execution_identity")
                            .ok_or(ContractError::InvalidField("provider execution identity"))?,
                    )? != observed
                {
                    return Err(ContractError::InvalidField(
                        "nested provider execution identity",
                    ));
                }
                if execution.as_ref().is_some_and(|prior| prior != &observed)
                    || open_response.is_some()
                {
                    return Err(ContractError::InvalidField("provider execution continuity"));
                }
                let response_id = string(step, "response_id")?.to_owned();
                if response_id != observed.first_response_id && execution.is_none() {
                    return Err(ContractError::InvalidField("first response identity"));
                }
                execution = Some(observed);
                open_response = Some(response_id);
            }
            "PROVIDER_ADMISSION_REFUSED" => {
                if refusal.is_some() || execution.is_some() || pending_request.is_none() {
                    return Err(ContractError::InvalidField("refusal transition"));
                }
                let occurrence = string(normalized, "request_occurrence_id")?.to_owned();
                let pending = pending_request.take();
                if pending.as_ref().map(|value| value.0.as_str()) != Some(occurrence.as_str())
                    || pending.as_ref().map(|value| value.1)
                        != Some(integer(normalized, "sampling_ordinal")?)
                    || pending.as_ref().map(|value| value.2)
                        != Some(integer(normalized, "request_order")?)
                {
                    return Err(ContractError::InvalidField("refusal request binding"));
                }
                let retry_after = match normalized.get("retry_after_ms") {
                    Some(Value::Null) => None,
                    Some(value) => value.as_i64(),
                    None => return Err(ContractError::InvalidField("retry_after_ms")),
                };
                refusal = Some((
                    occurrence,
                    retry_after,
                    integer(normalized, "observed_at_ms")?,
                ));
            }
            "PROVIDER_RESPONSE_COMPLETED" => {
                exact_object_keys(
                    normalized,
                    &["response_id", "proves_new_admission"],
                    "provider completion normalized closure",
                )?;
                if normalized
                    .get("proves_new_admission")
                    .and_then(Value::as_bool)
                    != Some(false)
                {
                    return Err(ContractError::InvalidField(
                        "completion admission substitution",
                    ));
                }
                let response_id = string(normalized, "response_id")?;
                if open_response.as_deref() != Some(response_id)
                    || !completed_responses.insert(response_id.to_owned())
                {
                    return Err(ContractError::InvalidField("response completion order"));
                }
                open_response = None;
            }
            "WAITING_APPROVAL" => {
                exact_object_keys(
                    normalized,
                    &[
                        "approval_response_sent",
                        "protected_effect_absent",
                        "provider_execution_identity",
                    ],
                    "approval normalized closure",
                )?;
                if execution.is_none()
                    || open_response.is_some()
                    || saw_approval
                    || normalized
                        .get("approval_response_sent")
                        .and_then(Value::as_bool)
                        != Some(false)
                    || normalized
                        .get("protected_effect_absent")
                        .and_then(Value::as_bool)
                        != Some(true)
                {
                    return Err(ContractError::InvalidField("approval transition"));
                }
                let approval_execution = provider_execution_from_switchyard(
                    normalized
                        .get("provider_execution_identity")
                        .ok_or(ContractError::InvalidField("approval execution identity"))?,
                )?;
                if execution.as_ref() != Some(&approval_execution) {
                    return Err(ContractError::InvalidField(
                        "approval execution identity binding",
                    ));
                }
                saw_approval = true;
            }
            "LOCAL_TURN_FACT" => {
                exact_object_keys(
                    normalized,
                    &["proves_provider_admission"],
                    "local fact normalized closure",
                )?;
                if normalized
                    .get("proves_provider_admission")
                    .and_then(Value::as_bool)
                    != Some(false)
                {
                    return Err(ContractError::InvalidField("local fact admission widening"));
                }
                if method == "turn/completed" {
                    if open_response.is_some() || pending_request.is_some() {
                        return Err(ContractError::InvalidField("premature turn completion"));
                    }
                    saw_turn_completed = true;
                }
            }
            "ADMISSION_DISCREPANCY" => {
                exact_object_keys(
                    normalized,
                    &[
                        "detail",
                        "provider_execution_identity",
                        "resume_same_attempt_only",
                    ],
                    "discrepancy normalized closure",
                )?;
                if normalized
                    .get("resume_same_attempt_only")
                    .and_then(Value::as_bool)
                    != Some(execution.is_some())
                {
                    return Err(ContractError::InvalidField("discrepancy resume law"));
                }
                let discrepancy_execution = match normalized.get("provider_execution_identity") {
                    Some(Value::Null) => None,
                    Some(value) => Some(provider_execution_from_switchyard(value)?),
                    None => {
                        return Err(ContractError::InvalidField(
                            "discrepancy execution identity",
                        ));
                    }
                };
                if discrepancy_execution.as_ref() != execution.as_ref() {
                    return Err(ContractError::InvalidField(
                        "discrepancy execution identity binding",
                    ));
                }
                saw_discrepancy = true;
            }
            "CLIENT_REQUEST_ISSUED" => {
                exact_object_keys(
                    normalized,
                    &[
                        "request_id",
                        "request_method",
                        "params_sha256",
                        "proves_provider_admission",
                    ],
                    "client request normalized closure",
                )?;
                digest("client params sha256", string(normalized, "params_sha256")?)?;
                let request_id = integer(normalized, "request_id")?;
                if client_requests
                    .insert(
                        request_id,
                        (
                            string(normalized, "request_method")?.to_owned(),
                            string(normalized, "params_sha256")?.to_owned(),
                        ),
                    )
                    .is_some()
                {
                    return Err(ContractError::InvalidField("duplicate client request"));
                }
            }
            "CLIENT_RESPONSE_RETAINED" => {
                exact_object_keys(
                    normalized,
                    &[
                        "request_id",
                        "request_method",
                        "params_sha256",
                        "result_sha256",
                        "proves_provider_admission",
                    ],
                    "client response normalized closure",
                )?;
                digest("client params sha256", string(normalized, "params_sha256")?)?;
                digest("client result sha256", string(normalized, "result_sha256")?)?;
                let request_id = integer(normalized, "request_id")?;
                let expected = client_requests
                    .remove(&request_id)
                    .ok_or(ContractError::InvalidField("unmatched client response"))?;
                if expected.0 != string(normalized, "request_method")?
                    || expected.1 != string(normalized, "params_sha256")?
                {
                    return Err(ContractError::InvalidField(
                        "client request response binding",
                    ));
                }
            }
            "ACQUISITION_WATERMARK" => {
                exact_object_keys(
                    normalized,
                    &["proves_provider_admission"],
                    "watermark normalized closure",
                )?;
            }
            "ACQUISITION_CUT" => unreachable!(),
            _ => return Err(ContractError::InvalidField("mapper evidence kind")),
        }
    }

    let cut = cut_value.ok_or(ContractError::InvalidField(
        "terminal acquisition cut absence",
    ))?;
    if snapshot.get("acquisition_cut") != Some(cut) {
        return Err(ContractError::InvalidField(
            "snapshot acquisition-cut record binding",
        ));
    }
    let process = string(cut, "process_disposition")?;
    let process_closed = matches!(
        process,
        "EXITED" | "EXITED_AFTER_TERMINATE" | "EXITED_AFTER_KILL"
    );
    if !matches!(
        process,
        "UNKNOWN"
            | "ABSENT"
            | "RUNNING"
            | "EXITED"
            | "EXITED_AFTER_TERMINATE"
            | "EXITED_AFTER_KILL"
            | "EXIT_UNCONFIRMED"
    ) || string(cut, "adapter_process_occurrence_id")?
        != disposition.adapter_process_occurrence_id
        || string(cut, "app_server_session_identity")? != disposition.app_server_session_identity
        || integer(cut, "ordered_high_water")? != next_acquisition_ordinal
        || integer(cut, "consumed_ordinal_count")? != next_acquisition_ordinal
    {
        return Err(ContractError::InvalidField("acquisition-cut identity"));
    }
    let stream_quiesced = cut
        .get("stream_quiesced")
        .and_then(Value::as_bool)
        .ok_or(ContractError::InvalidField("stream_quiesced"))?;
    let loss_generation = integer(cut, "loss_generation")?;
    let outstanding = integer(cut, "outstanding_client_request_count")?;
    if outstanding != client_requests.len() as i64 {
        return Err(ContractError::InvalidField(
            "client request cut outstanding binding",
        ));
    }
    let semantic_closed = (refusal.is_some() && saw_turn_completed)
        || (execution.is_some()
            && open_response.is_none()
            && saw_turn_completed
            && !saw_approval
            && !saw_discrepancy);
    let expected_clean = stream_quiesced
        && process_closed
        && loss_generation == 0
        && outstanding == 0
        && semantic_closed;
    if cut.get("clean").and_then(Value::as_bool) != Some(expected_clean)
        || disposition.acquisition_complete != expected_clean
    {
        return Err(ContractError::InvalidField("acquisition-cut clean law"));
    }

    let snapshot_state = string(&snapshot, "mechanism_state")?;
    let expected_state = match disposition.mechanism_state {
        ProviderMechanismStateV1::ParkedNotAdmitted => "PARKED_NOT_ADMITTED",
        ProviderMechanismStateV1::AdmissionIndeterminate => "ADMISSION_INDETERMINATE",
        ProviderMechanismStateV1::ExecutionAdmitted => "EXECUTION_ADMITTED",
        ProviderMechanismStateV1::PostAdmissionInterrupted => "POST_ADMISSION_INTERRUPTED",
        ProviderMechanismStateV1::WaitingApproval => "WAITING_APPROVAL",
        ProviderMechanismStateV1::ProviderCompleted => "PROVIDER_COMPLETED",
    };
    if snapshot_state != expected_state {
        return Err(ContractError::InvalidField(
            "mapper mechanism-state binding",
        ));
    }
    let snapshot_execution = match snapshot.get("provider_execution_identity") {
        Some(Value::Null) => None,
        Some(value) => Some(provider_execution_from_switchyard(value)?),
        None => return Err(ContractError::InvalidField("snapshot execution identity")),
    };
    if snapshot_execution != execution
        || snapshot_execution != disposition.provider_execution
        || disposition.response_created != snapshot_execution.is_some()
    {
        return Err(ContractError::InvalidField(
            "snapshot disposition execution binding",
        ));
    }
    match disposition.mechanism_state {
        ProviderMechanismStateV1::ParkedNotAdmitted => {
            if refusal.is_none() || !expected_clean || execution.is_some() {
                return Err(ContractError::InvalidField("parked mechanism state"));
            }
        }
        ProviderMechanismStateV1::ProviderCompleted => {
            if execution.is_none() || !expected_clean {
                return Err(ContractError::InvalidField("completed mechanism state"));
            }
        }
        ProviderMechanismStateV1::PostAdmissionInterrupted => {
            if execution.is_none() || expected_clean {
                return Err(ContractError::InvalidField(
                    "post-admission interruption state",
                ));
            }
        }
        ProviderMechanismStateV1::WaitingApproval => {
            if !saw_approval || cut_value.is_some() {
                return Err(ContractError::InvalidField("waiting approval terminal cut"));
            }
        }
        ProviderMechanismStateV1::AdmissionIndeterminate => {
            if execution.is_some() {
                return Err(ContractError::InvalidField(
                    "indeterminate execution identity",
                ));
            }
        }
        ProviderMechanismStateV1::ExecutionAdmitted => {
            if execution.is_none() || cut_value.is_some() {
                return Err(ContractError::InvalidField("pre-cut admitted state"));
            }
        }
    }
    if let Some((occurrence, retry_after_ms, observed_at_ms)) = refusal {
        if occurrence != disposition.provider_request_occurrence_id {
            return Err(ContractError::InvalidField("refusal occurrence binding"));
        }
        let received_ms = disposition.received_at.timestamp_millis();
        if received_ms < observed_at_ms {
            return Err(ContractError::InvalidField("pre-receipt disposition time"));
        }
        let expected_retry =
            retry_after_ms.map(|delay| disposition.received_at + Duration::milliseconds(delay));
        if expected_retry != disposition.provider_retry_after {
            return Err(ContractError::InvalidField("provider retry-after binding"));
        }
    } else if disposition.provider_retry_after.is_some() {
        return Err(ContractError::InvalidField(
            "unwitnessed provider retry-after",
        ));
    }
    Ok(())
}

fn provider_execution_from_switchyard(
    value: &Value,
) -> Result<ProviderExecutionIdentityV1, ContractError> {
    Ok(ProviderExecutionIdentityV1 {
        provider_id: string(value, "provider")?.to_owned(),
        model_id: string(value, "model")?.to_owned(),
        app_server_session_identity: string(value, "app_server_session_identity")?.to_owned(),
        thread_id: string(value, "thread_id")?.to_owned(),
        turn_id: string(value, "turn_id")?.to_owned(),
        first_response_id: string(value, "first_response_id")?.to_owned(),
    })
}

fn validate_kind_lane(kind: &str, method: &str, lane: &str) -> Result<(), ContractError> {
    let valid = match kind {
        "CLIENT_REQUEST_ISSUED" => {
            lane == "CLIENT_REQUEST" && method.starts_with("client-request/")
        }
        "CLIENT_RESPONSE_RETAINED" => {
            lane == "CLIENT_RESPONSE" && method.starts_with("client-response/")
        }
        "PROVIDER_REQUEST_STARTED" => lane == "NOTIFICATION" && method == "providerRequest/started",
        "PROVIDER_EXECUTION_STEP" => lane == "NOTIFICATION" && method == "rawResponse/started",
        "PROVIDER_ADMISSION_REFUSED" => {
            lane == "NOTIFICATION" && method == "providerAdmission/refused"
        }
        "PROVIDER_RESPONSE_COMPLETED" => {
            lane == "NOTIFICATION" && method == "rawResponse/completed"
        }
        "WAITING_APPROVAL" => {
            lane == "SERVER_REQUEST"
                && matches!(
                    method,
                    "commandExecution/requestApproval" | "fileChange/requestApproval"
                )
        }
        "LOCAL_TURN_FACT" | "ACQUISITION_WATERMARK" => lane == "NOTIFICATION",
        "ADMISSION_DISCREPANCY" => matches!(
            lane,
            "LOSS"
                | "UNKNOWN"
                | "NOTIFICATION"
                | "SERVER_REQUEST"
                | "CLIENT_REQUEST"
                | "CLIENT_RESPONSE"
        ),
        _ => false,
    };
    if !valid {
        return Err(ContractError::InvalidField(
            "evidence kind/lane/method binding",
        ));
    }
    Ok(())
}

fn validate_safe_json(value: &Value) -> Result<(), ContractError> {
    const SAFE: i64 = 9_007_199_254_740_991;
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(number) => {
            let valid = number
                .as_i64()
                .is_some_and(|value| (-SAFE..=SAFE).contains(&value))
                || number.as_u64().is_some_and(|value| value <= SAFE as u64)
                || number
                    .as_f64()
                    .is_some_and(|value| value.is_finite() && value.abs() <= SAFE as f64);
            if valid {
                Ok(())
            } else {
                Err(ContractError::InvalidField("RFC8785 safe number"))
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_safe_json(value)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for value in values.values() {
                validate_safe_json(value)?;
            }
            Ok(())
        }
    }
}

/// Validate one complete provider-execution availability graph before journal admission.
///
/// This function is pure. It grants no authority and performs no store, adapter,
/// provider, lock, timer, or process operation.
pub fn validate_execution_availability_graph(
    requirement: &ForemanExecutionAvailabilityRequirementV1,
    policy: &ExecutionAvailabilityPolicyV1,
    dispatch: &ProviderDispatchOccurrenceV1,
    observation: &ExecutionAvailabilityObservationV1,
    disposition: &ProviderAdmissionDispositionV1,
    prior_deferrals: &[DeferredProviderDispatchV1],
    deferred: Option<&DeferredProviderDispatchV1>,
) -> Result<(), ContractError> {
    requirement.validate()?;
    policy.validate()?;
    dispatch.validate()?;
    observation.validate()?;
    disposition.validate()?;
    if requirement.policy_id != policy.policy_id
        || requirement.policy_digest != policy.policy_digest
        || dispatch.requirement_digest != requirement.requirement_digest
        || dispatch.policy_digest != policy.policy_digest
        || dispatch.packet_digest != requirement.packet_digest
        || dispatch.run_id != requirement.run_id
        || dispatch.adapter_id != requirement.adapter_id
        || dispatch.adapter_version != requirement.adapter_version
        || dispatch.adapter_protocol != requirement.adapter_protocol
        || dispatch.dispatch_ordinal > policy.maximum_dispatch_occurrences_per_attempt
    {
        return Err(ContractError::InvalidField(
            "requirement policy dispatch graph",
        ));
    }
    let selections = requirement
        .work_item_model_selections
        .get(&dispatch.work_item_id)
        .ok_or(ContractError::InvalidField("dispatch work-item selection"))?;
    if selections.get(usize::from(dispatch.selected_model_ordinal)) != Some(&dispatch.selection) {
        return Err(ContractError::InvalidField("selected model ordinal"));
    }
    if disposition.dispatch_digest != dispatch.dispatch_digest
        || disposition.requirement_digest != requirement.requirement_digest
        || disposition.policy_digest != policy.policy_digest
        || disposition.packet_digest != requirement.packet_digest
        || disposition.run_id != requirement.run_id
        || disposition.work_item_id != dispatch.work_item_id
        || disposition.work_attempt_id != dispatch.work_attempt_id
        || disposition.dispatch_occurrence_id != dispatch.dispatch_occurrence_id
        || disposition.provider_id != dispatch.selection.provider_id
        || disposition.model_id != dispatch.selection.model_id
        || disposition.adapter_process_occurrence_id != dispatch.adapter_process_occurrence_id
        || disposition.app_server_session_identity != dispatch.app_server_session_identity
    {
        return Err(ContractError::InvalidField("dispatch disposition graph"));
    }
    let (expected_source, expected_source_time) = switchyard_source_observation(disposition)?;
    if observation.provider_id != dispatch.selection.provider_id
        || observation.model_id != dispatch.selection.model_id
        || observation.model_class != dispatch.selection.model_class
        || observation.received_at != disposition.received_at
        || observation.observed_at != expected_source_time
        || observation.provider_retry_after != disposition.provider_retry_after
        || observation.source_identity != "switchyard:provider-admission"
        || observation.source_version != "v1"
        || !observation.is_current_at(disposition.received_at)
    {
        return Err(ContractError::InvalidField(
            "availability observation graph",
        ));
    }
    let expected_observation_state = match disposition.disposition {
        ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity => {
            ExecutionAvailabilityStateV1::ModelAtCapacity
        }
        ProviderAdmissionDispositionKindV1::NotAdmittedProviderUnavailable => {
            ExecutionAvailabilityStateV1::ProviderUnavailable
        }
        ProviderAdmissionDispositionKindV1::NotAdmittedRateLimited => {
            ExecutionAvailabilityStateV1::RateLimited
        }
        ProviderAdmissionDispositionKindV1::AuthenticationRefused => {
            ExecutionAvailabilityStateV1::AuthenticationRefused
        }
        ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => {
            if !matches!(
                observation.state,
                ExecutionAvailabilityStateV1::Unknown
                    | ExecutionAvailabilityStateV1::TransportError
                    | ExecutionAvailabilityStateV1::ProtocolError
            ) {
                return Err(ContractError::InvalidField(
                    "indeterminate observation state",
                ));
            }
            observation.state
        }
        ProviderAdmissionDispositionKindV1::ExecutionAdmitted => {
            ExecutionAvailabilityStateV1::Available
        }
        ProviderAdmissionDispositionKindV1::QuotaExhaustedFuelOwned => {
            return Err(ContractError::InvalidField("quota is FUEL-owned"));
        }
    };
    if observation.state != expected_observation_state {
        return Err(ContractError::InvalidField("disposition observation state"));
    }
    if observation.exact_evidence.as_ref() != expected_source.as_ref() {
        return Err(ContractError::InvalidField(
            "exact observation source binding",
        ));
    }

    if usize::from(dispatch.dispatch_ordinal) != prior_deferrals.len() + 1
        || prior_deferrals.len() >= usize::from(policy.maximum_dispatch_occurrences_per_attempt)
    {
        return Err(ContractError::InvalidField(
            "dispatch ordinal prior deferral continuity",
        ));
    }
    let mut cumulative_deferral_seconds = 0_u64;
    let mut prior_dispatch_ids = BTreeSet::new();
    let mut prior_refusal_time: Option<DateTime<Utc>> = None;
    let mut prior_model_ordinal: Option<u16> = None;
    for (index, prior) in prior_deferrals.iter().enumerate() {
        prior.validate()?;
        let prior_selection = selections
            .get(usize::from(prior.selected_model_ordinal))
            .ok_or(ContractError::InvalidField("prior deferred model ordinal"))?;
        let expected_remaining: Vec<u16> = if policy.allow_ordered_model_fallback {
            ((prior.selected_model_ordinal + 1)..(selections.len() as u16)).collect()
        } else {
            Vec::new()
        };
        if prior.requirement_digest != requirement.requirement_digest
            || prior.policy_digest != policy.policy_digest
            || prior.packet_digest != requirement.packet_digest
            || prior.run_id != requirement.run_id
            || prior.work_item_id != dispatch.work_item_id
            || prior.work_attempt_id != dispatch.work_attempt_id
            || prior.provider_id != prior_selection.provider_id
            || prior.model_id != prior_selection.model_id
            || prior.backoff_ordinal != index as u16
            || prior.remaining_model_ordinals != expected_remaining
            || prior.parked_resource_lock_policy != policy.parked_resource_lock_policy
            || !prior_dispatch_ids.insert(prior.last_dispatch_occurrence_id.clone())
            || prior_refusal_time.is_some_and(|time| prior.refusal_received_at < time)
            || prior_model_ordinal.is_some_and(|ordinal| {
                prior.selected_model_ordinal < ordinal
                    || prior.selected_model_ordinal > ordinal.saturating_add(1)
            })
            || prior.wake_at > dispatch.opened_at
        {
            return Err(ContractError::InvalidField("prior deferral graph"));
        }
        prior_refusal_time = Some(prior.refusal_received_at);
        prior_model_ordinal = Some(prior.selected_model_ordinal);
        cumulative_deferral_seconds = cumulative_deferral_seconds
            .checked_add(prior.backoff_seconds)
            .ok_or(ContractError::InvalidField("cumulative deferral overflow"))?;
    }
    if prior_model_ordinal.is_some_and(|ordinal| {
        dispatch.selected_model_ordinal < ordinal
            || dispatch.selected_model_ordinal > ordinal.saturating_add(1)
    }) {
        return Err(ContractError::InvalidField(
            "ordered model deferral continuity",
        ));
    }

    if disposition.disposition.permits_automatic_park() {
        let parked = deferred.ok_or(ContractError::InvalidField("deferred dispatch absence"))?;
        parked.validate()?;
        if parked.requirement_digest != requirement.requirement_digest
            || parked.policy_digest != policy.policy_digest
            || parked.disposition_digest != disposition.disposition_digest
            || parked.packet_digest != requirement.packet_digest
            || parked.run_id != requirement.run_id
            || parked.work_item_id != dispatch.work_item_id
            || parked.work_attempt_id != dispatch.work_attempt_id
            || parked.last_dispatch_occurrence_id != dispatch.dispatch_occurrence_id
            || parked.provider_id != dispatch.selection.provider_id
            || parked.model_id != dispatch.selection.model_id
            || parked.selected_model_ordinal != dispatch.selected_model_ordinal
            || parked.refusal_received_at != disposition.received_at
            || parked.backoff_ordinal + 1 != dispatch.dispatch_ordinal
            || parked.parked_resource_lock_policy != policy.parked_resource_lock_policy
            || !parked.provider_capacity_released
            || parked.semantic_retry
        {
            return Err(ContractError::InvalidField("deferred dispatch graph"));
        }
        let expected_remaining: Vec<u16> = if policy.allow_ordered_model_fallback {
            ((dispatch.selected_model_ordinal + 1)..(selections.len() as u16)).collect()
        } else {
            Vec::new()
        };
        if parked.remaining_model_ordinals != expected_remaining {
            return Err(ContractError::InvalidField("ordered fallback suffix"));
        }
        cumulative_deferral_seconds = cumulative_deferral_seconds
            .checked_add(parked.backoff_seconds)
            .ok_or(ContractError::InvalidField("cumulative deferral overflow"))?;
        if cumulative_deferral_seconds > policy.maximum_total_deferral_seconds {
            return Err(ContractError::InvalidField("maximum cumulative deferral"));
        }
        match parked.wake_basis {
            DeferredWakeBasisV1::ProviderRetryAfter => {
                let retry_after = disposition
                    .provider_retry_after
                    .ok_or(ContractError::InvalidField("provider retry-after absence"))?;
                let seconds = (retry_after - disposition.received_at).num_seconds();
                if parked.provider_retry_after != Some(retry_after)
                    || seconds <= 0
                    || parked.backoff_seconds != seconds as u64
                {
                    return Err(ContractError::InvalidField("provider retry-after wake"));
                }
            }
            DeferredWakeBasisV1::PolicyBackoff => {
                let expected = policy
                    .backoff_seconds
                    .get(usize::from(parked.backoff_ordinal))
                    .ok_or(ContractError::InvalidField("backoff ordinal"))?;
                if disposition.provider_retry_after.is_some()
                    || parked.provider_retry_after.is_some()
                    || parked.backoff_seconds != *expected
                {
                    return Err(ContractError::InvalidField("policy backoff wake"));
                }
            }
        }
    } else if deferred.is_some() {
        return Err(ContractError::InvalidField("unlawful automatic park"));
    }
    if matches!(
        disposition.mechanism_state,
        ProviderMechanismStateV1::PostAdmissionInterrupted
    ) && (disposition.provider_execution.is_none() || deferred.is_some())
    {
        return Err(ContractError::InvalidField(
            "post-admission resume-only law",
        ));
    }
    Ok(())
}

fn switchyard_source_observation(
    disposition: &ProviderAdmissionDispositionV1,
) -> Result<(Option<ExactAvailabilityEvidenceV1>, DateTime<Utc>), ContractError> {
    let raw = disposition.mapper_snapshot.validate()?;
    let snapshot: Value = serde_json::from_slice(&raw).map_err(json_error)?;
    let kind = match disposition.disposition {
        ProviderAdmissionDispositionKindV1::NotAdmittedModelAtCapacity => {
            "PROVIDER_ADMISSION_REFUSED"
        }
        ProviderAdmissionDispositionKindV1::ExecutionAdmitted => "PROVIDER_EXECUTION_STEP",
        ProviderAdmissionDispositionKindV1::AdmissionIndeterminate => "ADMISSION_DISCREPANCY",
        _ => return Ok((None, disposition.received_at)),
    };
    let record = snapshot["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record.get("kind").and_then(Value::as_str) == Some(kind))
        })
        .ok_or(ContractError::InvalidField("observation source evidence"))?;
    let observed_at = record["normalized"]
        .get("observed_at_ms")
        .and_then(Value::as_i64)
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or(disposition.received_at);
    let evidence = match record.get("raw") {
        Some(Value::Null)
            if matches!(
                disposition.disposition,
                ProviderAdmissionDispositionKindV1::AdmissionIndeterminate
            ) =>
        {
            None
        }
        Some(value) => Some(serde_json::from_value(value.clone()).map_err(json_error)?),
        None => return Err(ContractError::InvalidField("observation source raw")),
    };
    Ok((evidence, observed_at))
}
