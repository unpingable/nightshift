use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const CASEWORK_LIVE_RUN_SCHEMA_V1: &str = "nightshift.casework-live-run/v1";
pub const CASEWORK_LIVE_RUN_INDEX_SCHEMA_V1: &str = "nightshift.casework-live-run-index/v1";
pub const CASEWORK_LIVE_RUN_DIGEST_DOMAIN_V1: &[u8] = b"nightshift.casework-live-run.digest/v1\0";
pub const CASEWORK_LIVE_NAVIGATION_DOMAIN_V1: &[u8] =
    b"nightshift.casework-live-run.navigation/v1\0";
pub const FOREMAN_JOURNAL_FRAMING_V1: &[u8] = b"NIGHTSHIFT-FOREMAN-JOURNAL-FRAMING-V1\0";
pub const FOREMAN_ACCEPTED_RECEIPTS_FRAMING_V1: &[u8] =
    b"NIGHTSHIFT-FOREMAN-ACCEPTED-RECEIPTS-FRAMING-V1\0";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseworkLiveRunV1 {
    pub schema: String,
    pub projection_digest: String,
    pub navigation_id: String,
    pub run_id: String,
    pub evaluated_at: String,
    pub packet: LivePacketV1,
    pub admission: LiveAdmissionV1,
    pub execution_profile: LiveExecutionProfileV1,
    pub foreman: LiveForemanV1,
    pub work_items: Vec<LiveWorkItemV1>,
    pub resource_claims: Vec<LiveResourceClaimV1>,
    pub events: Vec<LiveEventV1>,
    pub raw_sources: LiveRawSourcesV1,
    pub sealed_case_run_id: Option<String>,
    pub provider_capacity: LiveProviderCapacityV1,
    pub authority_effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LivePacketV1 {
    pub packet_id: String,
    pub packet_digest: String,
    pub exact_bytes_sha256: String,
    pub integrity: String,
    pub created_at: String,
    pub current_until: String,
    pub currentness: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveAdmissionV1 {
    pub admission_digest: String,
    pub exact_bytes_sha256: String,
    pub admitted_at: String,
    pub expires_at: String,
    pub currentness: String,
    pub maximum_concurrent_workers: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveExecutionProfileV1 {
    pub profile_digest: String,
    pub exact_bytes_sha256: String,
    pub budget_policy_ref: String,
    pub capacity_binding_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveForemanV1 {
    pub source_schema: String,
    pub lifecycle: String,
    pub scheduler_state_counts: BTreeMap<String, usize>,
    pub terminal_receipt_count: usize,
    pub not_started_receipt_count: usize,
    pub closed_final_receipts_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveWorkItemV1 {
    pub work_item_id: String,
    pub track: String,
    pub campaign_codename: String,
    pub campaign_slug: String,
    pub dependencies: Vec<String>,
    pub entry_predicates: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub scheduler_state: String,
    pub scheduler_state_recognized: bool,
    pub dependency_terminality: BTreeMap<String, bool>,
    pub resource_lock_keys: Vec<String>,
    pub active_attempt_id: Option<String>,
    pub adapter_id: String,
    pub adapter_version: String,
    pub provider_model_class: String,
    pub provider_identity: Option<String>,
    pub model_identity: Option<String>,
    pub session_identity: Option<String>,
    pub thread_identity: Option<String>,
    pub turn_identity: Option<String>,
    pub queue_identity: Option<String>,
    pub last_event_sequence: Option<u64>,
    pub last_event_digest: Option<String>,
    pub human_questions: Vec<LiveQuestionV1>,
    pub accepted_receipt_kind: Option<String>,
    pub accepted_outcome: Option<LiveAcceptedOutcomeV1>,
    pub accepted_outcome_absent_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveQuestionV1 {
    pub question_id: String,
    pub question: String,
    pub exhausted_evidence: String,
    pub safe_default: String,
    pub consequences: String,
    pub resume_point: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveAcceptedOutcomeV1 {
    pub state: String,
    pub result_classification: String,
    pub receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveResourceClaimV1 {
    pub resource_lock_key: String,
    pub work_item_id: String,
    pub attempt_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveEventV1 {
    pub sequence: u64,
    pub event_id: String,
    pub work_item_id: Option<String>,
    pub attempt_id: Option<String>,
    pub kind: String,
    pub recorded_at: String,
    pub retained_raw_digest: String,
    pub exact_bytes_sha256: String,
    pub raw_length: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveRawSourcesV1 {
    pub packet_sha256: String,
    pub admission_sha256: String,
    pub profile_sha256: String,
    pub journal_framing_sha256: String,
    pub accepted_receipts_framing_sha256: String,
    pub final_snapshot_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveProviderCapacityV1 {
    pub status: String,
    pub observation_digest: Option<String>,
    pub policy_digest: Option<String>,
    pub decision_digest: Option<String>,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseworkLiveRunIndexV1 {
    pub schema: String,
    pub runs: Vec<CaseworkLiveRunIndexEntryV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseworkLiveRunIndexEntryV1 {
    pub navigation_id: String,
    pub run_id: String,
    pub projection_digest: String,
    pub packet_id: String,
    pub packet_digest: String,
    pub lifecycle: String,
    pub sealed_case_run_id: Option<String>,
    pub scheduler_state_counts: BTreeMap<String, usize>,
}
