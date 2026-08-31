use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    os::fd::AsRawFd,
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
};

use chrono::{DateTime, SecondsFormat, Utc};
use nightshift_provider_capacity::{
    decide_capacity, AdmissionDisposition as CapacityAdmissionDisposition, CapacityDecisionV1,
    CapacityObservationV1, CapacityPolicyV1,
};
use nightshiftd::packet::NightshiftPacketV1;
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    contract::{ContractError, TeardownDeclarationV1},
    scheduler::{AcceptedOutcomeV1, ReplayEvent, ReplayKind},
    AdapterEventV1, CapacityCostClassV1, ExecutionProfileV2, ForemanAdmissionV1,
    ForemanCapacityAdmissionV1, ForemanCapacityRequirementV1, HumanQuestionV1, LiveRunProjectionV1,
    NotStartedReceiptV1, ReceiptRepositoryV1, Scheduler, SchedulerStateV1, TerminalReceiptV1,
    WorkerBriefV2, WorkerStartRequestV2, MAXIMUM_CAPACITY_HISTORY_BYTES,
    MAXIMUM_PREDECESSOR_RECEIPTS, MAXIMUM_WORKER_BRIEF_BYTES, WORKER_BRIEF_BASIS_SCHEMA_V2,
    WORKER_START_REQUEST_SCHEMA_V2, WORKER_TERMINAL_RECEIPT_SCHEMA_V1,
};

const INTERNAL_EVENT_SCHEMA: &str = "nightshift.foreman-journal-event/v1";
const BRIEF_DIGEST_DOMAIN: &[u8] = b"nightshift.worker-brief.digest/v2\0";
const RAW_DIGEST_DOMAIN: &[u8] = b"nightshift.foreman-retained-raw.digest/v1\0";
const MAXIMUM_CAPACITY_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error)]
pub enum ForemanError {
    #[error("SQLite store error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("contract refused: {0}")]
    Contract(#[from] ContractError),
    #[error("packet refused: {0}")]
    Packet(String),
    #[error("run already exists: {0}")]
    DuplicateRun(String),
    #[error("unknown run: {0}")]
    UnknownRun(String),
    #[error("unknown work item: {0}")]
    UnknownWorkItem(String),
    #[error("identity mismatch: {0}")]
    IdentityMismatch(&'static str),
    #[error("scheduler transition refused: {0}")]
    Transition(String),
    #[error("resource unavailable: {0}")]
    ResourceUnavailable(String),
    #[error("duplicate event: {0}")]
    DuplicateEvent(String),
    #[error("bounded input exceeded: {0}")]
    InputTooLarge(&'static str),
    #[error("closeout refused; nonterminal work items: {0}")]
    IncompleteCloseout(String),
    #[error("read-only store refused: {0}")]
    ReadOnlyStore(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
}

enum StoreAccess {
    ReadWrite,
    ReadOnly { descriptor: File },
}

pub struct ForemanStore {
    path: PathBuf,
    access: StoreAccess,
}

pub struct CapacityAdmissionEvidenceV1<'a> {
    pub admission_bytes: &'a [u8],
    pub observation_bytes: &'a [u8],
    pub policy_bytes: &'a [u8],
    pub decision_bytes: &'a [u8],
}

struct ValidatedCapacityAdmission {
    admission: ForemanCapacityAdmissionV1,
    observation: CapacityObservationV1,
    policy: CapacityPolicyV1,
    decision: CapacityDecisionV1,
    admission_bytes: Vec<u8>,
    observation_bytes: Vec<u8>,
    policy_bytes: Vec<u8>,
    decision_bytes: Vec<u8>,
}

/// One exact event row retained by the append-only foreman journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyEventRowV1 {
    pub sequence: u64,
    pub event_id: String,
    pub work_item_id: Option<String>,
    pub attempt_id: Option<String>,
    pub kind: String,
    pub recorded_at: String,
    pub raw_bytes: Vec<u8>,
    pub raw_digest: String,
}

/// One exact accepted terminal or not-started receipt row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyTerminalReceiptRowV1 {
    pub work_item_id: String,
    pub attempt_id: Option<String>,
    pub receipt_digest: String,
    pub raw_bytes: Vec<u8>,
    pub receipt_kind: String,
    pub state: String,
    pub result_classification: String,
}

/// A transaction-consistent read snapshot for read-only operator projections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyCapacityRequirementV1 {
    pub recorded_at: String,
    pub requirement: ForemanCapacityRequirementV1,
    pub requirement_bytes: Vec<u8>,
}

/// One exact capacity-admission event retained by the append-only journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyCapacityAdmissionV1 {
    pub work_item_id: String,
    pub attempt_id: String,
    pub recorded_at: String,
    pub capacity_admission: ForemanCapacityAdmissionV1,
    pub admission_bytes: Vec<u8>,
    pub observation_bytes: Vec<u8>,
    pub policy_bytes: Vec<u8>,
    pub decision_bytes: Vec<u8>,
}

///
/// Every byte vector is copied exactly from the existing SQLite BLOB. Creating
/// this value performs no schema initialization, journal-mode assignment, or
/// write transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyRunSnapshotV1 {
    pub run_id: String,
    pub packet_bytes: Vec<u8>,
    pub admission_bytes: Vec<u8>,
    pub profile_bytes: Vec<u8>,
    pub projection: LiveRunProjectionV1,
    pub events: Vec<ReadOnlyEventRowV1>,
    pub capacity_requirement: Option<ReadOnlyCapacityRequirementV1>,
    pub capacity_admissions: Vec<ReadOnlyCapacityAdmissionV1>,
    pub terminal_receipts: Vec<ReadOnlyTerminalReceiptRowV1>,
    pub final_snapshot_bytes: Option<Vec<u8>>,
}

pub fn read_only_run_snapshot(
    path: impl AsRef<Path>,
    run_id: &str,
) -> Result<ReadOnlyRunSnapshotV1, ForemanError> {
    ForemanStore::open_read_only(path)?.read_only_run_snapshot(run_id)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InternalPayload {
    RunAdmitted,
    CapacityRequirementAdmitted {
        requirement: Box<ForemanCapacityRequirementV1>,
        requirement_bytes: Vec<u8>,
    },
    AttemptCreated {
        resource_lock_keys: Vec<String>,
        start_request: Box<WorkerStartRequestV2>,
    },
    DispatchRequested,
    ResumeRequested,
    CapacityAdmissionAccepted {
        capacity_admission: Box<ForemanCapacityAdmissionV1>,
        admission_bytes: Vec<u8>,
        observation_bytes: Vec<u8>,
        policy_bytes: Vec<u8>,
        decision_bytes: Vec<u8>,
    },
    TerminalAccepted {
        outcome: AcceptedOutcomeV1,
    },
    TerminalRefused {
        reason: String,
    },
    NotStartedAccepted {
        outcome: AcceptedOutcomeV1,
    },
    ResourcesReleased,
    RunClosed {
        final_receipts_digest: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InternalEvent {
    schema: String,
    event_id: String,
    run_id: String,
    work_item_id: Option<String>,
    attempt_id: Option<String>,
    recorded_at: DateTime<Utc>,
    payload: InternalPayload,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FinalReceiptDocument {
    schema: String,
    packet_digest: String,
    updated_at: String,
    work_items: Vec<FinalWorkItem>,
    human_questions: Vec<FinalQuestion>,
    repository_custody: Vec<FinalCustody>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FinalWorkItem {
    id: String,
    state: String,
    result_classification: String,
    repositories: Vec<ReceiptRepositoryV1>,
    tests: Vec<String>,
    evidence: Vec<String>,
    live_or_production_mutations: Vec<String>,
    remaining_trigger: String,
    next_lawful_action: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FinalQuestion {
    work_item: String,
    question: String,
    evidence_exhausted: String,
    safe_default: String,
    consequences: String,
    resume_point: String,
}

#[derive(Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct FinalCustody {
    repository: String,
    branch_head: String,
    push_custody: String,
    dirty: String,
    live_runtime: String,
    secrets: String,
    teardown: String,
}

impl ForemanStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ForemanError> {
        let store = Self {
            path: path.as_ref().to_owned(),
            access: StoreAccess::ReadWrite,
        };
        let connection = store.connection()?;
        initialize(&connection)?;
        Ok(store)
    }

    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, ForemanError> {
        let supplied = path.as_ref();
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(supplied)
            .map_err(|error| {
                ForemanError::ReadOnlyStore(format!(
                    "existing no-follow database required at {}: {error}",
                    supplied.display()
                ))
            })?;
        if !descriptor
            .metadata()
            .map_err(|error| ForemanError::ReadOnlyStore(error.to_string()))?
            .is_file()
        {
            return Err(ForemanError::ReadOnlyStore(format!(
                "existing regular database required at {}",
                supplied.display()
            )));
        }
        let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", descriptor.as_raw_fd()));
        if fs::read_link(&descriptor_path).is_err() {
            return Err(ForemanError::ReadOnlyStore(
                "descriptor-relative SQLite access is unavailable".to_owned(),
            ));
        }
        let store = Self {
            path: supplied.to_owned(),
            access: StoreAccess::ReadOnly { descriptor },
        };
        let connection = store.connection()?;
        require_existing_schema(&connection)?;
        Ok(store)
    }
    pub fn admit(
        &self,
        packet_bytes: &[u8],
        admission_bytes: &[u8],
        profile_bytes: &[u8],
        evaluated_at: DateTime<Utc>,
    ) -> Result<String, ForemanError> {
        self.admit_internal(
            packet_bytes,
            admission_bytes,
            profile_bytes,
            evaluated_at,
            None,
        )
    }

    pub fn admit_with_capacity_requirement(
        &self,
        packet_bytes: &[u8],
        admission_bytes: &[u8],
        profile_bytes: &[u8],
        capacity_requirement_bytes: &[u8],
        evaluated_at: DateTime<Utc>,
    ) -> Result<String, ForemanError> {
        if capacity_requirement_bytes.is_empty()
            || capacity_requirement_bytes.len() > MAXIMUM_CAPACITY_RECORD_BYTES
        {
            return Err(ForemanError::InputTooLarge("capacity requirement"));
        }
        let requirement = ForemanCapacityRequirementV1::from_slice(capacity_requirement_bytes)?;
        requirement.validate()?;
        if serde_jcs::to_vec(&requirement)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?
            != capacity_requirement_bytes
        {
            return Err(ForemanError::Transition(
                "capacity requirement bytes are not exact canonical owner bytes".to_owned(),
            ));
        }
        self.admit_internal(
            packet_bytes,
            admission_bytes,
            profile_bytes,
            evaluated_at,
            Some((requirement, capacity_requirement_bytes.to_vec())),
        )
    }

    fn admit_internal(
        &self,
        packet_bytes: &[u8],
        admission_bytes: &[u8],
        profile_bytes: &[u8],
        evaluated_at: DateTime<Utc>,
        capacity_requirement: Option<(ForemanCapacityRequirementV1, Vec<u8>)>,
    ) -> Result<String, ForemanError> {
        let packet = NightshiftPacketV1::from_slice(packet_bytes)
            .map_err(|error| ForemanError::Packet(error.to_string()))?;
        packet
            .validate_at(evaluated_at)
            .map_err(|error| ForemanError::Packet(error.to_string()))?;
        let admission = ForemanAdmissionV1::from_slice(admission_bytes)?;
        admission.validate_at(evaluated_at)?;
        let profile = ExecutionProfileV2::from_slice(profile_bytes)?;
        profile.validate()?;
        validate_bindings(&packet, &admission, &profile)?;
        if let Some((requirement, _)) = &capacity_requirement {
            validate_capacity_requirement(requirement, &packet, &admission, &profile)?;
        }

        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id = ?1)",
            [&admission.run_id],
            |row| row.get(0),
        )?;
        if exists {
            return Err(ForemanError::DuplicateRun(admission.run_id));
        }
        transaction.execute(
            "INSERT INTO runs
             (run_id, packet_digest, admission_digest, profile_digest, packet_bytes,
              admission_bytes, profile_bytes, admitted_at, expires_at, maximum_concurrent_workers)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                admission.run_id,
                packet.packet_digest,
                admission.admission_digest,
                profile.profile_digest,
                packet_bytes,
                admission_bytes,
                profile_bytes,
                admission.admitted_at.to_rfc3339(),
                admission.expires_at.to_rfc3339(),
                admission.maximum_concurrent_workers,
            ],
        )?;
        for item in &packet.work_items {
            transaction.execute(
                "INSERT INTO work_items (run_id, work_item_id, packet_ordinal, dependencies_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    admission.run_id,
                    item.id,
                    packet
                        .work_items
                        .iter()
                        .position(|candidate| candidate.id == item.id)
                        .unwrap_or_default(),
                    serde_jcs::to_vec(&item.dependencies)
                        .map_err(|error| ForemanError::Serialization(error.to_string()))?,
                ],
            )?;
        }
        let event = InternalEvent {
            schema: INTERNAL_EVENT_SCHEMA.to_owned(),
            event_id: format!("run-admitted-{}", Uuid::new_v4()),
            run_id: admission.run_id.clone(),
            work_item_id: None,
            attempt_id: None,
            recorded_at: evaluated_at,
            payload: InternalPayload::RunAdmitted,
        };
        append_internal(&transaction, &event)?;
        if let Some((requirement, requirement_bytes)) = capacity_requirement {
            let requirement_event = InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("capacity-required-{}", Uuid::new_v4()),
                run_id: admission.run_id.clone(),
                work_item_id: None,
                attempt_id: None,
                recorded_at: evaluated_at,
                payload: InternalPayload::CapacityRequirementAdmitted {
                    requirement: Box::new(requirement),
                    requirement_bytes,
                },
            };
            append_internal_bounded(
                &transaction,
                &requirement_event,
                profile.maximum_event_bytes,
                packet.work_items.len().saturating_add(1),
            )?;
        }
        transaction.commit()?;
        Ok(admission.run_id)
    }

    pub fn projection(&self, run_id: &str) -> Result<LiveRunProjectionV1, ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let projection = load_projection(&transaction, run_id)?;
        transaction.commit()?;
        Ok(projection)
    }

    pub fn worker_brief(&self, run_id: &str, work_item_id: &str) -> Result<Vec<u8>, ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let (packet, _, profile, _) = load_contracts(&transaction, run_id)?;
        let brief = worker_brief_bytes(&transaction, &packet, &profile, run_id, work_item_id)?;
        transaction.commit()?;
        Ok(brief)
    }
    pub fn prepare_attempt(
        &self,
        run_id: &str,
        work_item_id: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<WorkerStartRequestV2, ForemanError> {
        self.prepare_attempt_internal(run_id, work_item_id, recorded_at, None)
    }

    /// Atomically retain one exact FUEL decision and create the attempt it admitted.
    pub fn prepare_attempt_with_capacity(
        &self,
        run_id: &str,
        work_item_id: &str,
        evidence: CapacityAdmissionEvidenceV1<'_>,
        recorded_at: DateTime<Utc>,
    ) -> Result<WorkerStartRequestV2, ForemanError> {
        let capacity = validate_capacity_bundle(
            evidence.admission_bytes,
            evidence.observation_bytes,
            evidence.policy_bytes,
            evidence.decision_bytes,
        )?;
        self.prepare_attempt_internal(run_id, work_item_id, recorded_at, Some(capacity))
    }

    fn prepare_attempt_internal(
        &self,
        run_id: &str,
        work_item_id: &str,
        recorded_at: DateTime<Utc>,
        capacity: Option<ValidatedCapacityAdmission>,
    ) -> Result<WorkerStartRequestV2, ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let projection = load_projection(&transaction, run_id)?;
        let item = projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == work_item_id)
            .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
        if !matches!(
            item.scheduler_state,
            SchedulerStateV1::ReadyEntryEvaluation | SchedulerStateV1::WaitingResource
        ) {
            return Err(ForemanError::Transition(format!(
                "{work_item_id} is {:?}, not entry-evaluation eligible",
                item.scheduler_state
            )));
        }
        if prior_attempt_exists(&transaction, run_id, work_item_id)? {
            return Err(ForemanError::Transition(
                "V1 admits no automatic or implicit second attempt".to_owned(),
            ));
        }
        let active = projection
            .work_items
            .iter()
            .filter(|item| {
                item.active_attempt_id.is_some() && !item.scheduler_state.is_explicit_terminal()
            })
            .count();
        if active >= usize::from(projection.maximum_concurrent_workers) {
            return Err(ForemanError::ResourceUnavailable(
                "maximum concurrent workers reached".to_owned(),
            ));
        }
        for lock in &item.resource_lock_keys {
            let holder: Option<String> = transaction
                .query_row(
                    "SELECT work_item_id FROM resource_claims
                     WHERE run_id = ?1 AND resource_lock_key = ?2",
                    params![run_id, lock],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(holder) = holder {
                return Err(ForemanError::ResourceUnavailable(format!(
                    "{lock} held by {holder}"
                )));
            }
        }
        let (packet, admission, profile, _) = load_contracts(&transaction, run_id)?;
        let capacity_requirement = load_capacity_requirement(&transaction, run_id)?;
        match (capacity_requirement.as_ref(), capacity.as_ref()) {
            (Some(_), None) => {
                return Err(ForemanError::Transition(
                    "capacity-required run refuses legacy attempt preparation".to_owned(),
                ))
            }
            (None, Some(_)) => {
                return Err(ForemanError::Transition(
                    "legacy run has no immutable capacity requirement".to_owned(),
                ))
            }
            _ => {}
        }
        let attempt_id = format!("attempt-{}", Uuid::new_v4());
        let execution = profile
            .work_items
            .get(work_item_id)
            .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
        let adapter = &profile.adapters[&execution.adapter_id];
        let worker_brief_digest =
            worker_brief_digest(&transaction, &packet, &profile, run_id, work_item_id)?;
        let mut request = WorkerStartRequestV2 {
            schema: WORKER_START_REQUEST_SCHEMA_V2.to_owned(),
            adapter_id: adapter.adapter_id.clone(),
            adapter_version: adapter.adapter_version.clone(),
            request_digest: placeholder_digest(),
            adapter_protocol: adapter.protocol.clone(),
            packet_digest: packet.packet_digest.clone(),
            run_id: run_id.to_owned(),
            work_item_id: work_item_id.to_owned(),
            attempt_id: attempt_id.clone(),
            worker_brief_digest,
            workspace_identity: execution.workspace_identity.clone(),
            provider_model_class: execution.provider_model_class.clone(),
            timeout_seconds: profile.adapter_timeout_seconds,
            maximum_output_bytes: profile.maximum_event_bytes,
            recursive_worker_swarms_forbidden: true,
            approval_policy: "SURFACE_ONLY_NO_RESPONSE".to_owned(),
            expected_receipt_schema: WORKER_TERMINAL_RECEIPT_SCHEMA_V1.to_owned(),
        };
        request.seal()?;
        let brief = worker_brief_bytes(&transaction, &packet, &profile, run_id, work_item_id)?;
        WorkerBriefV2::from_slice_for_start(&brief, &request)?;
        if let Some(capacity) = capacity {
            validate_capacity_bindings(
                &capacity,
                &packet,
                &admission,
                &profile,
                capacity_requirement.as_ref().ok_or_else(|| {
                    ForemanError::Transition("capacity requirement missing".to_owned())
                })?,
                work_item_id,
                recorded_at,
            )?;
            let capacity_event = InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("capacity-admitted-{}", Uuid::new_v4()),
                run_id: run_id.to_owned(),
                work_item_id: Some(work_item_id.to_owned()),
                attempt_id: Some(attempt_id.clone()),
                recorded_at,
                payload: InternalPayload::CapacityAdmissionAccepted {
                    capacity_admission: Box::new(capacity.admission),
                    admission_bytes: capacity.admission_bytes,
                    observation_bytes: capacity.observation_bytes,
                    policy_bytes: capacity.policy_bytes,
                    decision_bytes: capacity.decision_bytes,
                },
            };
            append_internal_bounded(
                &transaction,
                &capacity_event,
                profile.maximum_event_bytes,
                packet.work_items.len().saturating_add(1),
            )?;
        }
        for lock in &execution.resource_lock_keys {
            transaction.execute(
                "INSERT INTO resource_claims
                 (run_id, resource_lock_key, work_item_id, attempt_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![run_id, lock, work_item_id, attempt_id],
            )?;
        }
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("attempt-created-{}", Uuid::new_v4()),
                run_id: run_id.to_owned(),
                work_item_id: Some(work_item_id.to_owned()),
                attempt_id: Some(attempt_id),
                recorded_at,
                payload: InternalPayload::AttemptCreated {
                    resource_lock_keys: execution.resource_lock_keys.clone(),
                    start_request: Box::new(request.clone()),
                },
            },
        )?;
        transaction.commit()?;
        Ok(request)
    }

    pub fn record_dispatch_requested(
        &self,
        run_id: &str,
        work_item_id: &str,
        attempt_id: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<(), ForemanError> {
        self.record_attempt_transition(
            run_id,
            work_item_id,
            attempt_id,
            recorded_at,
            InternalPayload::DispatchRequested,
            "dispatch-requested",
        )
    }

    pub fn record_resume_requested(
        &self,
        run_id: &str,
        work_item_id: &str,
        attempt_id: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<(), ForemanError> {
        self.record_attempt_transition(
            run_id,
            work_item_id,
            attempt_id,
            recorded_at,
            InternalPayload::ResumeRequested,
            "resume-requested",
        )
    }

    fn record_attempt_transition(
        &self,
        run_id: &str,
        work_item_id: &str,
        attempt_id: &str,
        recorded_at: DateTime<Utc>,
        payload: InternalPayload,
        prefix: &str,
    ) -> Result<(), ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        exact_active_attempt(&transaction, run_id, work_item_id, attempt_id)?;
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("{prefix}-{}", Uuid::new_v4()),
                run_id: run_id.to_owned(),
                work_item_id: Some(work_item_id.to_owned()),
                attempt_id: Some(attempt_id.to_owned()),
                recorded_at,
                payload,
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn accept_adapter_event(&self, raw: &[u8]) -> Result<(), ForemanError> {
        let event = AdapterEventV1::from_slice(raw)?;
        event.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (_, admission, profile, _) = load_contracts(&transaction, &event.run_id)?;
        if raw.len() as u64 > profile.maximum_event_bytes {
            return Err(ForemanError::InputTooLarge("adapter event"));
        }
        if event.packet_digest != admission.packet_digest {
            return Err(ForemanError::IdentityMismatch("packet_digest"));
        }
        exact_active_attempt(
            &transaction,
            &event.run_id,
            &event.work_item_id,
            &event.attempt_id,
        )?;
        let expected_adapter = &profile.work_items[&event.work_item_id].adapter_id;
        if &event.adapter_id != expected_adapter {
            return Err(ForemanError::IdentityMismatch("adapter_id"));
        }
        let adapter = &profile.adapters[expected_adapter];
        if event.adapter_version != adapter.adapter_version {
            return Err(ForemanError::IdentityMismatch("adapter_version"));
        }
        let projection = load_projection(&transaction, &event.run_id)?;
        let item = projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == event.work_item_id)
            .ok_or_else(|| ForemanError::UnknownWorkItem(event.work_item_id.clone()))?;
        validate_incremental_identity(
            item.provider_identity.as_deref(),
            event.provider_identity.as_deref(),
            "provider_identity",
        )?;
        validate_incremental_identity(
            item.model_identity.as_deref(),
            event.model_identity.as_deref(),
            "model_identity",
        )?;
        validate_incremental_identity(
            item.session_identity.as_deref(),
            event.session_identity.as_deref(),
            "session_identity",
        )?;
        validate_incremental_identity(
            item.thread_identity.as_deref(),
            event.thread_identity.as_deref(),
            "thread_identity",
        )?;
        validate_incremental_identity(
            item.turn_identity.as_deref(),
            event.turn_identity.as_deref(),
            "turn_identity",
        )?;
        validate_incremental_identity(
            item.queue_identity.as_deref(),
            event.queue_identity.as_deref(),
            "queue_identity",
        )?;
        let duplicate: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM events WHERE run_id = ?1 AND event_id = ?2)",
            params![event.run_id, event.event_id],
            |row| row.get(0),
        )?;
        if duplicate {
            return Err(ForemanError::DuplicateEvent(event.event_id));
        }
        transaction.execute(
            "INSERT INTO events
             (event_id, run_id, work_item_id, attempt_id, kind, recorded_at, raw_bytes, raw_digest)
             VALUES (?1, ?2, ?3, ?4, 'adapter_event', ?5, ?6, ?7)",
            params![
                event.event_id,
                event.run_id,
                event.work_item_id,
                event.attempt_id,
                event.occurred_at.to_rfc3339(),
                raw,
                raw_digest(raw),
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn accept_terminal_receipt(&self, raw: &[u8]) -> Result<(), ForemanError> {
        let receipt = TerminalReceiptV1::from_slice(raw)?;
        receipt.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (_, admission, profile, _) = load_contracts(&transaction, &receipt.run_id)?;
        if raw.len() as u64 > profile.maximum_receipt_bytes {
            return Err(ForemanError::InputTooLarge("terminal receipt"));
        }
        if receipt.packet_digest != admission.packet_digest {
            return Err(ForemanError::IdentityMismatch("packet_digest"));
        }
        exact_active_attempt(
            &transaction,
            &receipt.run_id,
            &receipt.work_item_id,
            &receipt.attempt_id,
        )?;
        let expected_adapter = &profile.work_items[&receipt.work_item_id].adapter_id;
        if expected_adapter != &receipt.adapter_id {
            return Err(ForemanError::IdentityMismatch("adapter_id"));
        }
        if profile.adapters[expected_adapter].adapter_version != receipt.adapter_version {
            return Err(ForemanError::IdentityMismatch("adapter_version"));
        }
        let projection = load_projection(&transaction, &receipt.run_id)?;
        let item = projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == receipt.work_item_id)
            .ok_or_else(|| ForemanError::UnknownWorkItem(receipt.work_item_id.clone()))?;
        validate_receipt_identity(
            item.provider_identity.as_deref(),
            Some(receipt.provider_identity.as_str()),
            "provider_identity",
        )?;
        validate_receipt_identity(
            item.model_identity.as_deref(),
            Some(receipt.model_identity.as_str()),
            "model_identity",
        )?;
        validate_receipt_identity(
            item.session_identity.as_deref(),
            receipt.session_identity.as_deref(),
            "session_identity",
        )?;
        validate_receipt_identity(
            item.thread_identity.as_deref(),
            receipt.thread_identity.as_deref(),
            "thread_identity",
        )?;
        validate_receipt_identity(
            item.turn_identity.as_deref(),
            receipt.turn_identity.as_deref(),
            "turn_identity",
        )?;
        validate_receipt_identity(
            item.queue_identity.as_deref(),
            receipt.queue_identity.as_deref(),
            "queue_identity",
        )?;
        transaction.execute(
            "INSERT INTO terminal_receipts
             (run_id, work_item_id, attempt_id, receipt_digest, raw_bytes, receipt_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, 'terminal')",
            params![
                receipt.run_id,
                receipt.work_item_id,
                receipt.attempt_id,
                receipt.receipt_digest,
                raw,
            ],
        )?;
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("terminal-accepted-{}", Uuid::new_v4()),
                run_id: receipt.run_id.clone(),
                work_item_id: Some(receipt.work_item_id.clone()),
                attempt_id: Some(receipt.attempt_id.clone()),
                recorded_at: receipt.ended_at,
                payload: InternalPayload::TerminalAccepted {
                    outcome: AcceptedOutcomeV1 {
                        state: receipt.state,
                        result_classification: receipt.result_classification,
                        receipt_digest: receipt.receipt_digest,
                    },
                },
            },
        )?;
        release_resources(
            &transaction,
            &receipt.run_id,
            &receipt.work_item_id,
            &receipt.attempt_id,
            receipt.ended_at,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn record_terminal_refusal(
        &self,
        run_id: &str,
        work_item_id: &str,
        attempt_id: &str,
        reason: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<(), ForemanError> {
        if reason.trim().is_empty() || reason.len() > 4096 {
            return Err(ForemanError::Transition(
                "invalid refusal reason".to_owned(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        exact_active_attempt(&transaction, run_id, work_item_id, attempt_id)?;
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("terminal-refused-{}", Uuid::new_v4()),
                run_id: run_id.to_owned(),
                work_item_id: Some(work_item_id.to_owned()),
                attempt_id: Some(attempt_id.to_owned()),
                recorded_at,
                payload: InternalPayload::TerminalRefused {
                    reason: reason.to_owned(),
                },
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn accept_not_started(&self, raw: &[u8]) -> Result<(), ForemanError> {
        let receipt = NotStartedReceiptV1::from_slice(raw)?;
        receipt.validate()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let projection = load_projection(&transaction, &receipt.run_id)?;
        let item = projection
            .work_items
            .iter()
            .find(|item| item.work_item_id == receipt.work_item_id)
            .ok_or_else(|| ForemanError::UnknownWorkItem(receipt.work_item_id.clone()))?;
        if !matches!(
            item.scheduler_state,
            SchedulerStateV1::ReadyEntryEvaluation | SchedulerStateV1::WaitingResource
        ) || item.active_attempt_id.is_some()
        {
            return Err(ForemanError::Transition(
                "not-started receipt requires an eligible item with no attempt".to_owned(),
            ));
        }
        let (_, admission, profile, _) = load_contracts(&transaction, &receipt.run_id)?;
        if raw.len() as u64 > profile.maximum_receipt_bytes {
            return Err(ForemanError::InputTooLarge("not-started receipt"));
        }
        if receipt.packet_digest != admission.packet_digest {
            return Err(ForemanError::IdentityMismatch("packet_digest"));
        }
        transaction.execute(
            "INSERT INTO terminal_receipts
             (run_id, work_item_id, attempt_id, receipt_digest, raw_bytes, receipt_kind)
             VALUES (?1, ?2, NULL, ?3, ?4, 'not_started')",
            params![
                receipt.run_id,
                receipt.work_item_id,
                receipt.receipt_digest,
                raw,
            ],
        )?;
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("not-started-accepted-{}", Uuid::new_v4()),
                run_id: receipt.run_id.clone(),
                work_item_id: Some(receipt.work_item_id.clone()),
                attempt_id: None,
                recorded_at: receipt.recorded_at,
                payload: InternalPayload::NotStartedAccepted {
                    outcome: AcceptedOutcomeV1 {
                        state: receipt.state,
                        result_classification: receipt.result_classification,
                        receipt_digest: receipt.receipt_digest,
                    },
                },
            },
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn close(&self, run_id: &str, updated_at: DateTime<Utc>) -> Result<Vec<u8>, ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(bytes) = transaction
            .query_row(
                "SELECT raw_bytes FROM final_snapshots WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(bytes);
        }
        let projection = load_projection(&transaction, run_id)?;
        let incomplete: Vec<_> = projection
            .work_items
            .iter()
            .filter(|item| !item.scheduler_state.is_explicit_terminal())
            .map(|item| item.work_item_id.clone())
            .collect();
        if !incomplete.is_empty() {
            return Err(ForemanError::IncompleteCloseout(incomplete.join(", ")));
        }
        let latest_evidence = latest_terminal_evidence_at(&transaction, run_id)?;
        if updated_at < latest_evidence {
            return Err(ForemanError::Transition(format!(
                "closeout snapshot time {updated_at} precedes retained terminal evidence {latest_evidence}"
            )));
        }
        let (packet, _, _, _) = load_contracts(&transaction, run_id)?;
        let document = build_final_document(&transaction, &packet, run_id, updated_at)?;
        let raw = serde_jcs::to_vec(&document)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        let digest = raw_digest(&raw);
        transaction.execute(
            "INSERT INTO final_snapshots (run_id, updated_at, raw_digest, raw_bytes)
             VALUES (?1, ?2, ?3, ?4)",
            params![run_id, updated_at.to_rfc3339(), digest, raw],
        )?;
        append_internal(
            &transaction,
            &InternalEvent {
                schema: INTERNAL_EVENT_SCHEMA.to_owned(),
                event_id: format!("run-closed-{}", Uuid::new_v4()),
                run_id: run_id.to_owned(),
                work_item_id: None,
                attempt_id: None,
                recorded_at: updated_at,
                payload: InternalPayload::RunClosed {
                    final_receipts_digest: digest,
                },
            },
        )?;
        transaction.commit()?;
        Ok(raw)
    }

    pub fn export_final(&self, run_id: &str) -> Result<Vec<u8>, ForemanError> {
        self.connection()?
            .query_row(
                "SELECT raw_bytes FROM final_snapshots WHERE run_id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| ForemanError::Transition("run has no final snapshot".to_owned()))
    }

    pub fn raw_terminal_receipt(
        &self,
        run_id: &str,
        work_item_id: &str,
    ) -> Result<Vec<u8>, ForemanError> {
        self.connection()?
            .query_row(
                "SELECT raw_bytes FROM terminal_receipts
                 WHERE run_id = ?1 AND work_item_id = ?2",
                params![run_id, work_item_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| ForemanError::Transition("work item has no accepted receipt".to_owned()))
    }

    pub fn export_events(&self, run_id: &str) -> Result<Vec<Value>, ForemanError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        ensure_run(&transaction, run_id)?;
        let events = {
            let mut statement = transaction
                .prepare("SELECT raw_bytes FROM events WHERE run_id = ?1 ORDER BY sequence ASC")?;
            let rows = statement.query_map([run_id], |row| row.get::<_, Vec<u8>>(0))?;
            rows.map(|row| {
                let bytes = row?;
                serde_json::from_slice(&bytes)
                    .map_err(|error| ForemanError::Serialization(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?
        };
        transaction.commit()?;
        Ok(events)
    }

    pub fn read_only_run_snapshot(
        &self,
        run_id: &str,
    ) -> Result<ReadOnlyRunSnapshotV1, ForemanError> {
        if !matches!(self.access, StoreAccess::ReadOnly { .. }) {
            return Err(ForemanError::ReadOnlyStore(
                "operator snapshot requires an explicitly read-only store".to_owned(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let (packet_bytes, admission_bytes, profile_bytes): (Vec<u8>, Vec<u8>, Vec<u8>) =
            transaction
                .query_row(
                    "SELECT packet_bytes, admission_bytes, profile_bytes
                     FROM runs WHERE run_id = ?1",
                    [run_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?
                .ok_or_else(|| ForemanError::UnknownRun(run_id.to_owned()))?;
        let projection = load_projection(&transaction, run_id)?;
        let packet = NightshiftPacketV1::from_slice(&packet_bytes)
            .map_err(|error| ForemanError::Packet(error.to_string()))?;
        packet
            .validate_integrity()
            .map_err(|error| ForemanError::Packet(error.to_string()))?;
        let admission = ForemanAdmissionV1::from_slice(&admission_bytes)?;
        admission.validate()?;
        let profile = ExecutionProfileV2::from_slice(&profile_bytes)?;
        profile.validate()?;
        validate_capacity_history_size(
            &transaction,
            run_id,
            profile.maximum_event_bytes,
            packet.work_items.len().saturating_add(1),
        )?;
        let events = {
            let mut statement = transaction.prepare(
                "SELECT sequence, event_id, work_item_id, attempt_id, kind, recorded_at,
                        raw_bytes, raw_digest
                 FROM events WHERE run_id = ?1 ORDER BY sequence ASC",
            )?;
            let rows = statement.query_map([run_id], |row| {
                Ok(ReadOnlyEventRowV1 {
                    sequence: row.get(0)?,
                    event_id: row.get(1)?,
                    work_item_id: row.get(2)?,
                    attempt_id: row.get(3)?,
                    kind: row.get(4)?,
                    recorded_at: row.get(5)?,
                    raw_bytes: row.get(6)?,
                    raw_digest: row.get(7)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let capacity_history =
            validate_capacity_history(&transaction, &events, &packet, &admission, &profile)?;
        let capacity_requirement = capacity_history.requirement;
        let capacity_admissions = capacity_history.admissions;
        let run_closed = validate_read_only_run_closed_binding(&events)?;
        let terminal_receipts = {
            let mut statement = transaction.prepare(
                "SELECT work_item_id, attempt_id, receipt_digest, raw_bytes, receipt_kind
                 FROM terminal_receipts WHERE run_id = ?1 ORDER BY work_item_id ASC",
            )?;
            let rows = statement.query_map([run_id], |row| {
                Ok(ReadOnlyTerminalReceiptRowV1 {
                    work_item_id: row.get(0)?,
                    attempt_id: row.get(1)?,
                    receipt_digest: row.get(2)?,
                    raw_bytes: row.get(3)?,
                    receipt_kind: row.get(4)?,
                    state: String::new(),
                    result_classification: String::new(),
                })
            })?;
            let rows = rows.collect::<Result<Vec<_>, _>>()?;
            let mut validated = Vec::with_capacity(rows.len());
            for row in rows {
                validated.push(validate_read_only_receipt_row(
                    run_id,
                    row,
                    &projection.packet_digest,
                    &profile,
                )?);
            }
            validated
        };
        validate_read_only_projection_receipts(&projection, &terminal_receipts)?;
        let final_snapshot_row = transaction
            .query_row(
                "SELECT updated_at, raw_digest, raw_bytes
                 FROM final_snapshots WHERE run_id = ?1",
                [run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()?;
        let final_snapshot_bytes = match (
            final_snapshot_row,
            run_closed,
            projection.closed_final_receipts_digest.as_deref(),
        ) {
            (None, None, None) => None,
            (
                Some((updated_at, retained_digest, bytes)),
                Some((run_closed_sequence, run_closed_at, run_closed_digest)),
                Some(replayed_digest),
            ) => {
                if retained_digest != raw_digest(&bytes)
                    || retained_digest != replayed_digest
                    || retained_digest != run_closed_digest
                    || events.last().map(|event| event.sequence) != Some(run_closed_sequence)
                {
                    return Err(ForemanError::ReadOnlyStore(
                        "final snapshot digest and RunClosed replay disagree".to_owned(),
                    ));
                }
                let updated_at = DateTime::parse_from_rfc3339(&updated_at)
                    .map_err(|error| ForemanError::Serialization(error.to_string()))?
                    .with_timezone(&Utc);
                if updated_at != run_closed_at
                    || updated_at < latest_terminal_evidence_at(&transaction, run_id)?
                {
                    return Err(ForemanError::ReadOnlyStore(
                        "final snapshot and RunClosed time custody disagree".to_owned(),
                    ));
                }
                let rebuilt = build_final_document(&transaction, &packet, run_id, updated_at)?;
                let rebuilt = serde_jcs::to_vec(&rebuilt)
                    .map_err(|error| ForemanError::Serialization(error.to_string()))?;
                if rebuilt != bytes {
                    return Err(ForemanError::ReadOnlyStore(
                        "final snapshot bytes do not reproduce from exact accepted receipts"
                            .to_owned(),
                    ));
                }
                Some(bytes)
            }
            _ => {
                return Err(ForemanError::ReadOnlyStore(
                    "final snapshot presence and RunClosed replay disagree".to_owned(),
                ));
            }
        };
        transaction.commit()?;
        Ok(ReadOnlyRunSnapshotV1 {
            run_id: run_id.to_owned(),
            packet_bytes,
            admission_bytes,
            profile_bytes,
            projection,
            events,
            capacity_requirement,
            capacity_admissions,
            terminal_receipts,
            final_snapshot_bytes,
        })
    }

    pub fn journal_mode(&self) -> Result<String, ForemanError> {
        Ok(self
            .connection()?
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?)
    }

    fn connection(&self) -> Result<Connection, ForemanError> {
        match &self.access {
            StoreAccess::ReadWrite => {
                let connection = Connection::open(&self.path)?;
                connection.pragma_update(None, "journal_mode", "WAL")?;
                connection.pragma_update(None, "foreign_keys", "ON")?;
                connection.busy_timeout(std::time::Duration::from_secs(5))?;
                Ok(connection)
            }
            StoreAccess::ReadOnly { descriptor } => {
                let descriptor_path =
                    PathBuf::from(format!("/proc/self/fd/{}", descriptor.as_raw_fd()));
                let resolved = fs::read_link(&descriptor_path).map_err(|error| {
                    ForemanError::ReadOnlyStore(format!(
                        "cannot resolve retained database descriptor: {error}"
                    ))
                })?;
                let resolved = resolved.to_str().ok_or_else(|| {
                    ForemanError::ReadOnlyStore(
                        "resolved database path must be valid UTF-8".to_owned(),
                    )
                })?;
                if resolved.ends_with(" (deleted)") {
                    return Err(ForemanError::ReadOnlyStore(
                        "retained database has been unlinked".to_owned(),
                    ));
                }
                let wal_path = PathBuf::from(format!("{resolved}-wal"));
                let shm_path = PathBuf::from(format!("{resolved}-shm"));
                let wal = open_read_only_sidecar(&wal_path)?;
                let shm = open_read_only_sidecar(&shm_path)?;
                let immutable = match (&wal, &shm) {
                    (None, None) => true,
                    (Some(_), Some(_)) => false,
                    _ => {
                        return Err(ForemanError::ReadOnlyStore(
                            "WAL and SHM sidecars must both be absent or both be regular files"
                                .to_owned(),
                        ));
                    }
                };
                let suffix = if immutable {
                    "?mode=ro&immutable=1"
                } else {
                    "?mode=ro"
                };
                let uri = format!("file:/proc/self/fd/{}{suffix}", descriptor.as_raw_fd());
                let connection = Connection::open_with_flags(
                    uri,
                    OpenFlags::SQLITE_OPEN_READ_ONLY
                        | OpenFlags::SQLITE_OPEN_URI
                        | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )
                .map_err(ForemanError::Sql)?;
                connection.pragma_update(None, "query_only", "ON")?;
                let query_only: u8 =
                    connection.query_row("PRAGMA query_only", [], |row| row.get(0))?;
                if query_only != 1 {
                    return Err(ForemanError::ReadOnlyStore(
                        "SQLite query_only was not enabled".to_owned(),
                    ));
                }
                if let (Some(wal), Some(shm)) = (&wal, &shm) {
                    require_same_file(wal, &wal_path, "WAL")?;
                    require_same_file(shm, &shm_path, "SHM")?;
                } else if wal_path.exists() || shm_path.exists() {
                    return Err(ForemanError::ReadOnlyStore(
                        "sidecar state changed during query-only connection open".to_owned(),
                    ));
                }
                Ok(connection)
            }
        }
    }
}

fn validate_read_only_event_row(
    row: &ReadOnlyEventRowV1,
    expected_run_id: &str,
    expected_packet_digest: &str,
    profile: &ExecutionProfileV2,
) -> Result<(), ForemanError> {
    if row.sequence == 0 || row.raw_digest != raw_digest(&row.raw_bytes) {
        return Err(ForemanError::ReadOnlyStore(
            "journal sequence or retained-raw digest mismatch".to_owned(),
        ));
    }
    if row.kind == "adapter_event" {
        let event = AdapterEventV1::from_slice(&row.raw_bytes)?;
        event.validate()?;
        if event.event_id != row.event_id
            || event.run_id != expected_run_id
            || event.packet_digest != expected_packet_digest
            || row.work_item_id.as_deref() != Some(event.work_item_id.as_str())
            || row.attempt_id.as_deref() != Some(event.attempt_id.as_str())
            || event.occurred_at.to_rfc3339() != row.recorded_at
            || profile
                .work_items
                .get(&event.work_item_id)
                .map(|work| work.adapter_id.as_str())
                != Some(event.adapter_id.as_str())
            || profile
                .adapters
                .get(&event.adapter_id)
                .map(|adapter| adapter.adapter_version.as_str())
                != Some(event.adapter_version.as_str())
        {
            return Err(ForemanError::ReadOnlyStore(
                "adapter event row identity mismatch".to_owned(),
            ));
        }
    } else if matches!(
        row.kind.as_str(),
        "internal" | "capacity_requirement" | "capacity_admission"
    ) {
        let event: InternalEvent = serde_json::from_slice(&row.raw_bytes)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        let kind_matches_payload = matches!(
            (row.kind.as_str(), &event.payload),
            ("internal", InternalPayload::RunAdmitted)
                | ("internal", InternalPayload::AttemptCreated { .. })
                | ("internal", InternalPayload::DispatchRequested)
                | ("internal", InternalPayload::ResumeRequested)
                | ("internal", InternalPayload::TerminalAccepted { .. })
                | ("internal", InternalPayload::TerminalRefused { .. })
                | ("internal", InternalPayload::NotStartedAccepted { .. })
                | ("internal", InternalPayload::ResourcesReleased)
                | ("internal", InternalPayload::RunClosed { .. })
                | (
                    "capacity_requirement",
                    InternalPayload::CapacityRequirementAdmitted { .. }
                )
                | (
                    "capacity_admission",
                    InternalPayload::CapacityAdmissionAccepted { .. }
                )
        );
        if !kind_matches_payload
            || event.schema != INTERNAL_EVENT_SCHEMA
            || event.event_id != row.event_id
            || event.run_id != expected_run_id
            || event.work_item_id != row.work_item_id
            || event.attempt_id != row.attempt_id
            || event.recorded_at.to_rfc3339() != row.recorded_at
        {
            return Err(ForemanError::ReadOnlyStore(
                "internal event row identity mismatch".to_owned(),
            ));
        }
        if serde_jcs::to_vec(&event)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?
            != row.raw_bytes
        {
            return Err(ForemanError::ReadOnlyStore(
                "internal event exact bytes are not canonical".to_owned(),
            ));
        }
        if let InternalPayload::AttemptCreated { start_request, .. } = &event.payload {
            start_request.validate()?;
            if start_request.run_id != expected_run_id
                || start_request.packet_digest != expected_packet_digest
                || event.work_item_id.as_deref() != Some(start_request.work_item_id.as_str())
                || event.attempt_id.as_deref() != Some(start_request.attempt_id.as_str())
            {
                return Err(ForemanError::ReadOnlyStore(
                    "attempt-created start request mismatch".to_owned(),
                ));
            }
        }
    } else {
        return Err(ForemanError::ReadOnlyStore(
            "unknown journal row kind".to_owned(),
        ));
    }
    Ok(())
}

fn read_only_capacity_admissions(
    events: &[ReadOnlyEventRowV1],
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
) -> Result<
    (
        Option<ReadOnlyCapacityRequirementV1>,
        Vec<ReadOnlyCapacityAdmissionV1>,
    ),
    ForemanError,
> {
    let mut requirement = None;
    let mut attempts = BTreeSet::new();
    let mut capacity_by_attempt = BTreeMap::new();
    for row in events {
        if row.kind == "adapter_event" {
            continue;
        }
        let event: InternalEvent = serde_json::from_slice(&row.raw_bytes)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        match event.payload {
            InternalPayload::CapacityRequirementAdmitted {
                requirement: candidate,
                requirement_bytes,
            } => {
                if requirement.is_some()
                    || event.work_item_id.is_some()
                    || event.attempt_id.is_some()
                    || serde_jcs::to_vec(&*candidate)
                        .map_err(|error| ForemanError::Serialization(error.to_string()))?
                        != requirement_bytes
                {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity requirement event custody is invalid".to_owned(),
                    ));
                }
                candidate.validate()?;
                validate_capacity_requirement(&candidate, packet, admission, profile)?;
                requirement = Some(ReadOnlyCapacityRequirementV1 {
                    recorded_at: event.recorded_at.to_rfc3339(),
                    requirement: *candidate,
                    requirement_bytes,
                });
            }
            InternalPayload::CapacityAdmissionAccepted {
                capacity_admission,
                admission_bytes,
                observation_bytes,
                policy_bytes,
                decision_bytes,
            } => {
                let work_item_id = event.work_item_id.ok_or_else(|| {
                    ForemanError::ReadOnlyStore(
                        "capacity admission lacks work item identity".to_owned(),
                    )
                })?;
                let attempt_id = event.attempt_id.ok_or_else(|| {
                    ForemanError::ReadOnlyStore(
                        "capacity admission lacks attempt identity".to_owned(),
                    )
                })?;
                let bundle = validate_capacity_bundle(
                    &admission_bytes,
                    &observation_bytes,
                    &policy_bytes,
                    &decision_bytes,
                )?;
                if bundle.admission != *capacity_admission
                    || capacity_by_attempt.contains_key(&attempt_id)
                {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity admission event exact bytes or identity disagree".to_owned(),
                    ));
                }
                let exact_requirement = requirement.as_ref().ok_or_else(|| {
                    ForemanError::ReadOnlyStore(
                        "capacity admission precedes immutable requirement".to_owned(),
                    )
                })?;
                validate_capacity_bindings(
                    &bundle,
                    packet,
                    admission,
                    profile,
                    &exact_requirement.requirement,
                    &work_item_id,
                    event.recorded_at,
                )?;
                capacity_by_attempt.insert(
                    attempt_id.clone(),
                    ReadOnlyCapacityAdmissionV1 {
                        work_item_id,
                        attempt_id,
                        recorded_at: event.recorded_at.to_rfc3339(),
                        capacity_admission: *capacity_admission,
                        admission_bytes,
                        observation_bytes,
                        policy_bytes,
                        decision_bytes,
                    },
                );
            }
            InternalPayload::AttemptCreated { .. } => {
                let attempt_id = event.attempt_id.ok_or_else(|| {
                    ForemanError::ReadOnlyStore("attempt-created lacks identity".to_owned())
                })?;
                attempts.insert(attempt_id);
            }
            _ => {}
        }
    }
    if requirement.is_some()
        && (attempts != capacity_by_attempt.keys().cloned().collect::<BTreeSet<_>>())
    {
        return Err(ForemanError::ReadOnlyStore(
            "capacity-required attempt graph is incomplete".to_owned(),
        ));
    }
    if requirement.is_none() && !capacity_by_attempt.is_empty() {
        return Err(ForemanError::ReadOnlyStore(
            "legacy run carries capacity admission evidence".to_owned(),
        ));
    }
    Ok((requirement, capacity_by_attempt.into_values().collect()))
}

struct ValidatedCapacityHistory {
    requirement: Option<ReadOnlyCapacityRequirementV1>,
    admissions: Vec<ReadOnlyCapacityAdmissionV1>,
}

fn validate_capacity_history(
    connection: &Connection,
    events: &[ReadOnlyEventRowV1],
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
) -> Result<ValidatedCapacityHistory, ForemanError> {
    validate_capacity_history_size(
        connection,
        &admission.run_id,
        profile.maximum_event_bytes,
        packet.work_items.len().saturating_add(1),
    )?;
    for row in events {
        validate_read_only_event_row(row, &admission.run_id, &packet.packet_digest, profile)?;
    }
    let (requirement, admissions) =
        read_only_capacity_admissions(events, packet, admission, profile)?;
    let parsed = events
        .iter()
        .map(|row| {
            if row.kind != "adapter_event" {
                serde_json::from_slice::<InternalEvent>(&row.raw_bytes)
                    .map(Some)
                    .map_err(|error| ForemanError::Serialization(error.to_string()))
            } else {
                Ok(None)
            }
        })
        .collect::<Result<Vec<_>, ForemanError>>()?;

    let run_positions = parsed
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            event.as_ref().and_then(|event| {
                matches!(event.payload, InternalPayload::RunAdmitted).then_some(index)
            })
        })
        .collect::<Vec<_>>();
    if run_positions.len() != 1 {
        return Err(ForemanError::ReadOnlyStore(
            "journal must contain one exact run-admitted event".to_owned(),
        ));
    }
    if requirement.is_some() {
        let index = run_positions[0];
        let next = parsed
            .get(index + 1)
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                ForemanError::ReadOnlyStore(
                    "capacity-required run lacks adjacent requirement event".to_owned(),
                )
            })?;
        if !matches!(
            next.payload,
            InternalPayload::CapacityRequirementAdmitted { .. }
        ) || events[index + 1].sequence != events[index].sequence + 1
            || next.recorded_at != parsed[index].as_ref().unwrap().recorded_at
        {
            return Err(ForemanError::ReadOnlyStore(
                "capacity requirement is not adjacent to exact run admission".to_owned(),
            ));
        }
    }

    for (index, event) in parsed.iter().enumerate() {
        let Some(event) = event else { continue };
        match &event.payload {
            InternalPayload::CapacityRequirementAdmitted { .. } => {
                if requirement.is_none() || index == 0 || run_positions[0] + 1 != index {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity requirement placement is invalid".to_owned(),
                    ));
                }
            }
            InternalPayload::CapacityAdmissionAccepted {
                capacity_admission, ..
            } => {
                let next = parsed
                    .get(index + 1)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| {
                        ForemanError::ReadOnlyStore(
                            "capacity admission lacks adjacent attempt creation".to_owned(),
                        )
                    })?;
                let InternalPayload::AttemptCreated { start_request, .. } = &next.payload else {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity admission is not followed by attempt creation".to_owned(),
                    ));
                };
                if events[index + 1].sequence != events[index].sequence + 1
                    || next.run_id != event.run_id
                    || next.work_item_id != event.work_item_id
                    || next.attempt_id != event.attempt_id
                    || next.recorded_at != event.recorded_at
                    || capacity_admission.packet_digest != start_request.packet_digest
                    || capacity_admission.run_id != start_request.run_id
                    || capacity_admission.work_item_id != start_request.work_item_id
                    || capacity_admission.adapter_id != start_request.adapter_id
                    || capacity_admission.profile_model_class != start_request.provider_model_class
                    || event.attempt_id.as_deref() != Some(start_request.attempt_id.as_str())
                {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity admission and attempt creation identities disagree".to_owned(),
                    ));
                }
            }
            InternalPayload::AttemptCreated { .. } if requirement.is_some() => {
                let previous = index
                    .checked_sub(1)
                    .and_then(|previous| parsed[previous].as_ref());
                if !previous.is_some_and(|previous| {
                    matches!(
                        previous.payload,
                        InternalPayload::CapacityAdmissionAccepted { .. }
                    )
                }) {
                    return Err(ForemanError::ReadOnlyStore(
                        "capacity-required attempt lacks adjacent admission".to_owned(),
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(ValidatedCapacityHistory {
        requirement,
        admissions,
    })
}

fn validate_read_only_receipt_row(
    expected_run_id: &str,
    mut row: ReadOnlyTerminalReceiptRowV1,
    expected_packet_digest: &str,
    profile: &ExecutionProfileV2,
) -> Result<ReadOnlyTerminalReceiptRowV1, ForemanError> {
    let (state, result_classification) = if row.receipt_kind == "terminal" {
        let receipt = TerminalReceiptV1::from_slice(&row.raw_bytes)?;
        receipt.validate()?;
        if receipt.run_id != expected_run_id
            || receipt.packet_digest != expected_packet_digest
            || receipt.work_item_id != row.work_item_id
            || row.attempt_id.as_deref() != Some(receipt.attempt_id.as_str())
            || receipt.receipt_digest != row.receipt_digest
            || profile
                .work_items
                .get(&receipt.work_item_id)
                .map(|work| work.adapter_id.as_str())
                != Some(receipt.adapter_id.as_str())
            || profile
                .adapters
                .get(&receipt.adapter_id)
                .map(|adapter| adapter.adapter_version.as_str())
                != Some(receipt.adapter_version.as_str())
        {
            return Err(ForemanError::ReadOnlyStore(
                "terminal receipt row identity mismatch".to_owned(),
            ));
        }
        (receipt.state, receipt.result_classification)
    } else if row.receipt_kind == "not_started" {
        let receipt = NotStartedReceiptV1::from_slice(&row.raw_bytes)?;
        receipt.validate()?;
        if receipt.run_id != expected_run_id
            || receipt.packet_digest != expected_packet_digest
            || receipt.work_item_id != row.work_item_id
            || row.attempt_id.is_some()
            || receipt.receipt_digest != row.receipt_digest
        {
            return Err(ForemanError::ReadOnlyStore(
                "not-started receipt row identity mismatch".to_owned(),
            ));
        }
        (receipt.state, receipt.result_classification)
    } else {
        return Err(ForemanError::ReadOnlyStore(
            "unknown accepted receipt kind".to_owned(),
        ));
    };
    row.state = state;
    row.result_classification = result_classification;
    Ok(row)
}

fn validate_read_only_run_closed_binding(
    events: &[ReadOnlyEventRowV1],
) -> Result<Option<(u64, DateTime<Utc>, String)>, ForemanError> {
    let mut binding = None;
    for row in events {
        if row.kind != "internal" {
            continue;
        }
        let event: InternalEvent = serde_json::from_slice(&row.raw_bytes)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        if let InternalPayload::RunClosed {
            final_receipts_digest,
        } = event.payload
        {
            if binding
                .replace((row.sequence, event.recorded_at, final_receipts_digest))
                .is_some()
            {
                return Err(ForemanError::ReadOnlyStore(
                    "multiple RunClosed events are not an exact terminal history".to_owned(),
                ));
            }
        }
    }
    Ok(binding)
}

fn validate_read_only_projection_receipts(
    projection: &LiveRunProjectionV1,
    receipts: &[ReadOnlyTerminalReceiptRowV1],
) -> Result<(), ForemanError> {
    let by_item: BTreeMap<_, _> = receipts
        .iter()
        .map(|receipt| (receipt.work_item_id.as_str(), receipt))
        .collect();
    if by_item.len() != receipts.len() {
        return Err(ForemanError::ReadOnlyStore(
            "duplicate accepted receipt work item".to_owned(),
        ));
    }
    if by_item.keys().any(|work_item_id| {
        !projection
            .work_items
            .iter()
            .any(|item| item.work_item_id == *work_item_id)
    }) {
        return Err(ForemanError::ReadOnlyStore(
            "accepted receipt references an unknown projected work item".to_owned(),
        ));
    }
    for item in &projection.work_items {
        match (
            by_item.get(item.work_item_id.as_str()),
            &item.accepted_terminal_outcome,
        ) {
            (None, None) if !item.scheduler_state.is_explicit_terminal() => {}
            (Some(receipt), Some(outcome))
                if receipt.receipt_digest == outcome.receipt_digest
                    && receipt.state == outcome.state
                    && receipt.result_classification == outcome.result_classification
                    && ((receipt.receipt_kind == "terminal"
                        && matches!(
                            item.scheduler_state,
                            SchedulerStateV1::TerminalReceiptAccepted
                        ))
                        || (receipt.receipt_kind == "not_started"
                            && matches!(item.scheduler_state, SchedulerStateV1::NotStarted))) => {}
            _ => {
                return Err(ForemanError::ReadOnlyStore(
                    "accepted receipt and replay projection disagree".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn open_read_only_sidecar(path: &Path) -> Result<Option<File>, ForemanError> {
    match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(file) => {
            if !file
                .metadata()
                .map_err(|error| ForemanError::ReadOnlyStore(error.to_string()))?
                .is_file()
            {
                return Err(ForemanError::ReadOnlyStore(format!(
                    "sidecar is not a regular file: {}",
                    path.display()
                )));
            }
            Ok(Some(file))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ForemanError::ReadOnlyStore(format!(
            "cannot retain sidecar {}: {error}",
            path.display()
        ))),
    }
}

fn require_same_file(file: &File, path: &Path, kind: &str) -> Result<(), ForemanError> {
    let retained = file
        .metadata()
        .map_err(|error| ForemanError::ReadOnlyStore(error.to_string()))?;
    let current = fs::symlink_metadata(path).map_err(|error| {
        ForemanError::ReadOnlyStore(format!("{kind} sidecar changed during open: {error}"))
    })?;
    if !current.is_file() || retained.dev() != current.dev() || retained.ino() != current.ino() {
        return Err(ForemanError::ReadOnlyStore(format!(
            "{kind} sidecar identity changed during query-only connection open"
        )));
    }
    Ok(())
}

fn require_existing_schema(connection: &Connection) -> Result<(), ForemanError> {
    const REQUIRED_TABLES: [&str; 6] = [
        "runs",
        "work_items",
        "events",
        "resource_claims",
        "terminal_receipts",
        "final_snapshots",
    ];
    for table in REQUIRED_TABLES {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1
             )",
            [table],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(ForemanError::ReadOnlyStore(format!(
                "database is missing required table {table}"
            )));
        }
    }
    Ok(())
}

fn initialize(connection: &Connection) -> Result<(), ForemanError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS runs (
            run_id TEXT PRIMARY KEY,
            packet_digest TEXT NOT NULL,
            admission_digest TEXT NOT NULL,
            profile_digest TEXT NOT NULL,
            packet_bytes BLOB NOT NULL,
            admission_bytes BLOB NOT NULL,
            profile_bytes BLOB NOT NULL,
            admitted_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            maximum_concurrent_workers INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS work_items (
            run_id TEXT NOT NULL REFERENCES runs(run_id),
            work_item_id TEXT NOT NULL,
            packet_ordinal INTEGER NOT NULL,
            dependencies_json BLOB NOT NULL,
            PRIMARY KEY (run_id, work_item_id)
        );
        CREATE TABLE IF NOT EXISTS events (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL,
            run_id TEXT NOT NULL REFERENCES runs(run_id),
            work_item_id TEXT,
            attempt_id TEXT,
            kind TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            raw_bytes BLOB NOT NULL,
            raw_digest TEXT NOT NULL,
            UNIQUE (run_id, event_id)
        );
        CREATE TABLE IF NOT EXISTS resource_claims (
            run_id TEXT NOT NULL REFERENCES runs(run_id),
            resource_lock_key TEXT NOT NULL,
            work_item_id TEXT NOT NULL,
            attempt_id TEXT NOT NULL,
            PRIMARY KEY (run_id, resource_lock_key)
        );
        CREATE TABLE IF NOT EXISTS terminal_receipts (
            run_id TEXT NOT NULL REFERENCES runs(run_id),
            work_item_id TEXT NOT NULL,
            attempt_id TEXT,
            receipt_digest TEXT NOT NULL,
            raw_bytes BLOB NOT NULL,
            receipt_kind TEXT NOT NULL CHECK (receipt_kind IN ('terminal', 'not_started')),
            PRIMARY KEY (run_id, work_item_id)
        );
        CREATE TABLE IF NOT EXISTS final_snapshots (
            run_id TEXT PRIMARY KEY REFERENCES runs(run_id),
            updated_at TEXT NOT NULL,
            raw_digest TEXT NOT NULL,
            raw_bytes BLOB NOT NULL
        );
        CREATE TRIGGER IF NOT EXISTS runs_no_update BEFORE UPDATE ON runs
            BEGIN SELECT RAISE(ABORT, 'runs are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS runs_no_delete BEFORE DELETE ON runs
            BEGIN SELECT RAISE(ABORT, 'runs are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS work_items_no_update BEFORE UPDATE ON work_items
            BEGIN SELECT RAISE(ABORT, 'work items are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS work_items_no_delete BEFORE DELETE ON work_items
            BEGIN SELECT RAISE(ABORT, 'work items are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS events_no_update BEFORE UPDATE ON events
            BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS events_no_delete BEFORE DELETE ON events
            BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS terminal_receipts_no_update BEFORE UPDATE ON terminal_receipts
            BEGIN SELECT RAISE(ABORT, 'terminal receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS terminal_receipts_no_delete BEFORE DELETE ON terminal_receipts
            BEGIN SELECT RAISE(ABORT, 'terminal receipts are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS final_snapshots_no_update BEFORE UPDATE ON final_snapshots
            BEGIN SELECT RAISE(ABORT, 'final snapshots are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS final_snapshots_no_delete BEFORE DELETE ON final_snapshots
            BEGIN SELECT RAISE(ABORT, 'final snapshots are append-only'); END;",
    )?;
    Ok(())
}

fn validate_bindings(
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
) -> Result<(), ForemanError> {
    if admission.packet_digest != packet.packet_digest
        || profile.packet_digest != packet.packet_digest
    {
        return Err(ForemanError::IdentityMismatch("packet_digest"));
    }
    if profile.admission_digest != admission.admission_digest {
        return Err(ForemanError::IdentityMismatch("admission_digest"));
    }
    if admission.maximum_concurrent_workers
        > packet.worker_budget.maximum_concurrent_mutating_workers
    {
        return Err(ForemanError::Transition(
            "admission exceeds packet worker budget".to_owned(),
        ));
    }
    let packet_items: BTreeSet<_> = packet.work_items.iter().map(|item| &item.id).collect();
    let profile_items: BTreeSet<_> = profile.work_items.keys().collect();
    if packet_items != profile_items {
        return Err(ForemanError::IdentityMismatch("profile work items"));
    }
    let allowed_adapters: BTreeSet<_> = admission.allowed_adapter_ids.iter().collect();
    let allowed_classes: BTreeSet<_> = admission.allowed_provider_model_classes.iter().collect();
    for execution in profile.work_items.values() {
        if !allowed_adapters.contains(&execution.adapter_id) {
            return Err(ForemanError::Transition(
                "profile adapter not admitted".to_owned(),
            ));
        }
        if !allowed_classes.contains(&execution.provider_model_class) {
            return Err(ForemanError::Transition(
                "provider/model class not admitted".to_owned(),
            ));
        }
    }
    Ok(())
}

fn load_capacity_requirement(
    connection: &Connection,
    run_id: &str,
) -> Result<Option<ForemanCapacityRequirementV1>, ForemanError> {
    let mut statement = connection.prepare(
        "SELECT raw_bytes FROM events WHERE run_id = ?1 AND kind IN ('internal', 'capacity_requirement') ORDER BY sequence",
    )?;
    let rows = statement.query_map([run_id], |row| row.get::<_, Vec<u8>>(0))?;
    let mut found = None;
    for row in rows {
        let raw = row?;
        let event: InternalEvent = serde_json::from_slice(&raw)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        if let InternalPayload::CapacityRequirementAdmitted {
            requirement,
            requirement_bytes,
        } = event.payload
        {
            if found.is_some()
                || serde_jcs::to_vec(&*requirement)
                    .map_err(|error| ForemanError::Serialization(error.to_string()))?
                    != requirement_bytes
            {
                return Err(ForemanError::ReadOnlyStore(
                    "capacity requirement history is not singular exact custody".to_owned(),
                ));
            }
            requirement.validate()?;
            found = Some(*requirement);
        }
    }
    Ok(found)
}

fn validate_capacity_requirement(
    requirement: &ForemanCapacityRequirementV1,
    packet: &NightshiftPacketV1,
    admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
) -> Result<(), ForemanError> {
    let packet_classes = packet
        .work_items
        .iter()
        .map(|work| work.model_routing.class.as_str())
        .collect::<BTreeSet<_>>();
    let requirement_classes = requirement
        .model_cost_classes
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if requirement.packet_digest != packet.packet_digest
        || requirement.admission_digest != admission.admission_digest
        || requirement.profile_digest != profile.profile_digest
        || requirement.run_id != admission.run_id
        || requirement.policy_id != profile.budget_policy_ref
        || packet_classes != requirement_classes
        || packet.work_items.iter().any(|work| {
            profile
                .work_items
                .get(&work.id)
                .is_none_or(|execution| execution.provider_model_class != work.model_routing.class)
        })
    {
        return Err(ForemanError::IdentityMismatch(
            "capacity requirement binding",
        ));
    }
    Ok(())
}

fn validate_capacity_bundle(
    admission_bytes: &[u8],
    observation_bytes: &[u8],
    policy_bytes: &[u8],
    decision_bytes: &[u8],
) -> Result<ValidatedCapacityAdmission, ForemanError> {
    for (name, bytes) in [
        ("capacity admission", admission_bytes),
        ("capacity observation", observation_bytes),
        ("capacity policy", policy_bytes),
        ("capacity decision", decision_bytes),
    ] {
        if bytes.is_empty() || bytes.len() > MAXIMUM_CAPACITY_RECORD_BYTES {
            return Err(ForemanError::InputTooLarge(name));
        }
    }
    let admission = ForemanCapacityAdmissionV1::from_slice(admission_bytes)?;
    admission.validate()?;
    let observation: CapacityObservationV1 = serde_json::from_slice(observation_bytes)
        .map_err(|error| ForemanError::Serialization(error.to_string()))?;
    let policy: CapacityPolicyV1 = serde_json::from_slice(policy_bytes)
        .map_err(|error| ForemanError::Serialization(error.to_string()))?;
    let decision: CapacityDecisionV1 = serde_json::from_slice(decision_bytes)
        .map_err(|error| ForemanError::Serialization(error.to_string()))?;
    observation
        .validate()
        .map_err(|error| ForemanError::Transition(error.to_string()))?;
    policy
        .validate()
        .map_err(|error| ForemanError::Transition(error.to_string()))?;
    decision
        .validate()
        .map_err(|error| ForemanError::Transition(error.to_string()))?;
    let reproduced_decision = decide_capacity(&observation, &policy, decision.decision_at)
        .map_err(|error| ForemanError::Transition(error.to_string()))?;
    if reproduced_decision != decision {
        return Err(ForemanError::Transition(
            "capacity decision is not the exact deterministic FUEL outcome".to_owned(),
        ));
    }
    for (name, expected, value) in [
        (
            "capacity admission",
            admission_bytes,
            serde_jcs::to_vec(&admission)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        ),
        (
            "capacity observation",
            observation_bytes,
            serde_jcs::to_vec(&observation)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        ),
        (
            "capacity policy",
            policy_bytes,
            serde_jcs::to_vec(&policy)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        ),
        (
            "capacity decision",
            decision_bytes,
            serde_jcs::to_vec(&decision)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        ),
    ] {
        if expected != value {
            return Err(ForemanError::Transition(format!(
                "{name} bytes are not exact canonical owner bytes"
            )));
        }
    }
    Ok(ValidatedCapacityAdmission {
        admission,
        observation,
        policy,
        decision,
        admission_bytes: admission_bytes.to_vec(),
        observation_bytes: observation_bytes.to_vec(),
        policy_bytes: policy_bytes.to_vec(),
        decision_bytes: decision_bytes.to_vec(),
    })
}

fn validate_capacity_bindings(
    capacity: &ValidatedCapacityAdmission,
    packet: &NightshiftPacketV1,
    foreman_admission: &ForemanAdmissionV1,
    profile: &ExecutionProfileV2,
    requirement: &ForemanCapacityRequirementV1,
    work_item_id: &str,
    recorded_at: DateTime<Utc>,
) -> Result<(), ForemanError> {
    let binding = &capacity.admission;
    let work = packet
        .work_items
        .iter()
        .find(|work| work.id == work_item_id)
        .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
    let execution = profile
        .work_items
        .get(work_item_id)
        .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
    if binding.capacity_requirement_digest != requirement.capacity_requirement_digest
        || binding.provider_id != requirement.provider_id
        || binding.policy_id != requirement.policy_id
        || requirement
            .model_cost_classes
            .get(&binding.packet_model_class)
            != Some(&binding.cost_class)
    {
        return Err(ForemanError::IdentityMismatch(
            "capacity requirement identity",
        ));
    }
    if binding.packet_digest != packet.packet_digest
        || binding.admission_digest != foreman_admission.admission_digest
        || binding.profile_digest != profile.profile_digest
        || binding.run_id != foreman_admission.run_id
        || binding.work_item_id != work_item_id
        || binding.adapter_id != execution.adapter_id
        || binding.packet_model_class != work.model_routing.class
        || binding.profile_model_class != execution.provider_model_class
    {
        return Err(ForemanError::IdentityMismatch("capacity admission binding"));
    }
    if capacity
        .observation
        .model_family
        .as_deref()
        .is_some_and(|model_family| model_family != binding.packet_model_class)
    {
        return Err(ForemanError::IdentityMismatch(
            "capacity observation model family",
        ));
    }
    if binding.provider_id != capacity.observation.provider_id
        || binding.provider_id != capacity.decision.provider_id
        || binding.policy_id != capacity.policy.policy_id
        || binding.policy_id != profile.budget_policy_ref
        || binding.observation_digest != capacity.observation.observation_digest
        || binding.policy_digest != capacity.policy.policy_digest
        || binding.decision_digest != capacity.decision.decision_digest
        || capacity.decision.observation_digest != capacity.observation.observation_digest
        || capacity.decision.policy_digest != capacity.policy.policy_digest
    {
        return Err(ForemanError::IdentityMismatch("capacity owner identity"));
    }
    if binding.evaluated_at != recorded_at
        || capacity.decision.decision_at != recorded_at
        || recorded_at < capacity.observation.observed_at
        || recorded_at >= capacity.observation.expires_at
    {
        return Err(ForemanError::Transition(
            "capacity decision is not current at exact attempt admission".to_owned(),
        ));
    }
    match capacity.decision.admission {
        CapacityAdmissionDisposition::NoNewWork => Err(ForemanError::Transition(
            "capacity decision admits no new work".to_owned(),
        )),
        CapacityAdmissionDisposition::CheapBoundedOnly
            if binding.cost_class != CapacityCostClassV1::Cheap =>
        {
            Err(ForemanError::Transition(
                "capacity decision admits only closed cheap model classes".to_owned(),
            ))
        }
        CapacityAdmissionDisposition::OrdinaryBounded
            if binding.cost_class == CapacityCostClassV1::Expensive
                && !capacity.decision.allow_new_expensive_work =>
        {
            Err(ForemanError::Transition(
                "capacity decision does not admit new expensive work".to_owned(),
            ))
        }
        _ => Ok(()),
    }
}

fn append_internal(
    transaction: &Transaction<'_>,
    event: &InternalEvent,
) -> Result<(), ForemanError> {
    let raw =
        serde_jcs::to_vec(event).map_err(|error| ForemanError::Serialization(error.to_string()))?;
    append_internal_raw(transaction, event, "internal", &raw)
}

fn append_internal_bounded(
    transaction: &Transaction<'_>,
    event: &InternalEvent,
    maximum_event_bytes: u64,
    maximum_capacity_rows: usize,
) -> Result<(), ForemanError> {
    let raw =
        serde_jcs::to_vec(event).map_err(|error| ForemanError::Serialization(error.to_string()))?;
    if raw.len() > maximum_event_bytes as usize {
        return Err(ForemanError::InputTooLarge("capacity journal event"));
    }
    let kind = match event.payload {
        InternalPayload::CapacityRequirementAdmitted { .. } => "capacity_requirement",
        InternalPayload::CapacityAdmissionAccepted { .. } => "capacity_admission",
        _ => {
            return Err(ForemanError::Transition(
                "bounded capacity append received non-capacity event".to_owned(),
            ))
        }
    };
    let (retained, retained_count) = validate_capacity_history_size(
        transaction,
        &event.run_id,
        maximum_event_bytes,
        maximum_capacity_rows,
    )?;
    if retained_count
        .checked_add(1)
        .is_none_or(|count| count > maximum_capacity_rows)
        || retained
            .checked_add(raw.len() as u64)
            .is_none_or(|total| total > MAXIMUM_CAPACITY_HISTORY_BYTES)
    {
        return Err(ForemanError::InputTooLarge("capacity journal history"));
    }
    append_internal_raw(transaction, event, kind, &raw)
}

fn append_internal_raw(
    transaction: &Transaction<'_>,
    event: &InternalEvent,
    kind: &str,
    raw: &[u8],
) -> Result<(), ForemanError> {
    transaction.execute(
        "INSERT INTO events
         (event_id, run_id, work_item_id, attempt_id, kind, recorded_at, raw_bytes, raw_digest)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event.event_id,
            event.run_id,
            event.work_item_id,
            event.attempt_id,
            kind,
            event.recorded_at.to_rfc3339(),
            raw,
            raw_digest(raw),
        ],
    )?;
    Ok(())
}

type RawContractRow = (
    String,
    String,
    String,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    String,
    String,
    u16,
);

fn load_contracts(
    connection: &Connection,
    run_id: &str,
) -> Result<
    (
        NightshiftPacketV1,
        ForemanAdmissionV1,
        ExecutionProfileV2,
        u16,
    ),
    ForemanError,
> {
    let row: Option<RawContractRow> = connection
        .query_row(
            "SELECT packet_digest, admission_digest, profile_digest,
                    packet_bytes, admission_bytes, profile_bytes,
                    admitted_at, expires_at, maximum_concurrent_workers
             FROM runs WHERE run_id = ?1",
            [run_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()?;
    let (
        packet_digest,
        admission_digest,
        profile_digest,
        packet_raw,
        admission_raw,
        profile_raw,
        admitted_at,
        expires_at,
        maximum,
    ) = row.ok_or_else(|| ForemanError::UnknownRun(run_id.to_owned()))?;
    let packet = NightshiftPacketV1::from_slice(&packet_raw)
        .map_err(|error| ForemanError::Packet(error.to_string()))?;
    packet
        .validate_integrity()
        .map_err(|error| ForemanError::Packet(error.to_string()))?;
    let admission = ForemanAdmissionV1::from_slice(&admission_raw)?;
    admission.validate()?;
    let profile = ExecutionProfileV2::from_slice(&profile_raw)?;
    profile.validate()?;
    validate_bindings(&packet, &admission, &profile)?;
    if admission.run_id != run_id
        || packet_digest != packet.packet_digest
        || admission_digest != admission.admission_digest
        || profile_digest != profile.profile_digest
        || admitted_at != admission.admitted_at.to_rfc3339()
        || expires_at != admission.expires_at.to_rfc3339()
        || maximum != admission.maximum_concurrent_workers
    {
        return Err(ForemanError::ReadOnlyStore(
            "run row and exact contract bytes disagree".to_owned(),
        ));
    }
    Ok((packet, admission, profile, maximum))
}

fn validate_capacity_history_size(
    connection: &Connection,
    run_id: &str,
    maximum_event_bytes: u64,
    maximum_capacity_rows: usize,
) -> Result<(u64, usize), ForemanError> {
    let mut statement = connection.prepare(
        "SELECT sequence, length(raw_bytes) FROM events
         WHERE run_id = ?1 AND kind IN ('capacity_requirement', 'capacity_admission')
         ORDER BY sequence ASC",
    )?;
    let rows = statement.query_map([run_id], |row| {
        Ok((row.get::<_, u64>(0)?, row.get::<_, u64>(1)?))
    })?;
    let mut total = 0_u64;
    let mut count = 0_usize;
    for row in rows {
        count = count
            .checked_add(1)
            .ok_or(ForemanError::InputTooLarge("capacity journal history"))?;
        if count > maximum_capacity_rows {
            return Err(ForemanError::InputTooLarge("capacity journal history"));
        }
        let (_sequence, length) = row?;
        if length == 0 || length > maximum_event_bytes {
            return Err(ForemanError::InputTooLarge("capacity journal event"));
        }
        total = total
            .checked_add(length)
            .ok_or(ForemanError::InputTooLarge("capacity journal history"))?;
        if total > MAXIMUM_CAPACITY_HISTORY_BYTES {
            return Err(ForemanError::InputTooLarge("capacity journal history"));
        }
    }
    Ok((total, count))
}

fn load_projection(
    connection: &Connection,
    run_id: &str,
) -> Result<LiveRunProjectionV1, ForemanError> {
    let (packet, admission, profile, maximum) = load_contracts(connection, run_id)?;
    validate_capacity_history_size(
        connection,
        run_id,
        profile.maximum_event_bytes,
        packet.work_items.len().saturating_add(1),
    )?;
    let mut statement = connection.prepare(
        "SELECT sequence, event_id, work_item_id, attempt_id, kind, recorded_at,
                raw_bytes, raw_digest
         FROM events WHERE run_id = ?1 ORDER BY sequence ASC",
    )?;
    let rows = statement.query_map([run_id], |row| {
        Ok(ReadOnlyEventRowV1 {
            sequence: row.get(0)?,
            event_id: row.get(1)?,
            work_item_id: row.get(2)?,
            attempt_id: row.get(3)?,
            kind: row.get(4)?,
            recorded_at: row.get(5)?,
            raw_bytes: row.get(6)?,
            raw_digest: row.get(7)?,
        })
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    validate_capacity_history(connection, &rows, &packet, &admission, &profile)?;

    let mut events = Vec::with_capacity(rows.len());
    for row in rows {
        let replay_kind = if row.kind == "adapter_event" {
            ReplayKind::Adapter(Box::new(AdapterEventV1::from_slice(&row.raw_bytes)?))
        } else {
            let event: InternalEvent = serde_json::from_slice(&row.raw_bytes)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?;
            match event.payload {
                InternalPayload::RunAdmitted => ReplayKind::RunAdmitted,
                InternalPayload::CapacityRequirementAdmitted { .. }
                | InternalPayload::CapacityAdmissionAccepted { .. } => ReplayKind::CapacityEvidence,
                InternalPayload::AttemptCreated {
                    resource_lock_keys, ..
                } => ReplayKind::AttemptCreated { resource_lock_keys },
                InternalPayload::DispatchRequested => ReplayKind::DispatchRequested,
                InternalPayload::ResumeRequested => ReplayKind::ResumeRequested,
                InternalPayload::TerminalAccepted { outcome } => {
                    ReplayKind::TerminalAccepted(outcome)
                }
                InternalPayload::TerminalRefused { .. } => ReplayKind::TerminalRefused,
                InternalPayload::NotStartedAccepted { outcome } => {
                    ReplayKind::NotStartedAccepted(outcome)
                }
                InternalPayload::ResourcesReleased => ReplayKind::ResourcesReleased,
                InternalPayload::RunClosed {
                    final_receipts_digest,
                } => ReplayKind::RunClosed {
                    final_receipts_digest,
                },
            }
        };
        events.push(ReplayEvent {
            sequence: row.sequence,
            work_item_id: row.work_item_id,
            attempt_id: row.attempt_id,
            kind: replay_kind,
            raw_digest: row.raw_digest,
        });
    }
    Ok(Scheduler::replay(
        &packet,
        run_id,
        &admission.admission_digest,
        &profile,
        maximum,
        &events,
    ))
}

fn prior_attempt_exists(
    connection: &Connection,
    run_id: &str,
    work_item_id: &str,
) -> Result<bool, ForemanError> {
    let mut statement = connection.prepare(
        "SELECT raw_bytes FROM events
         WHERE run_id = ?1 AND work_item_id = ?2 AND kind = 'internal'",
    )?;
    let rows = statement.query_map(params![run_id, work_item_id], |row| {
        row.get::<_, Vec<u8>>(0)
    })?;
    for row in rows {
        let event: InternalEvent = serde_json::from_slice(&row?)
            .map_err(|error| ForemanError::Serialization(error.to_string()))?;
        if matches!(event.payload, InternalPayload::AttemptCreated { .. }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn exact_active_attempt(
    connection: &Connection,
    run_id: &str,
    work_item_id: &str,
    attempt_id: &str,
) -> Result<(), ForemanError> {
    let projection = load_projection(connection, run_id)?;
    let item = projection
        .work_items
        .iter()
        .find(|item| item.work_item_id == work_item_id)
        .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
    if item.scheduler_state.is_explicit_terminal() {
        return Err(ForemanError::Transition(
            "terminal attempt cannot be resumed or reused".to_owned(),
        ));
    }
    if item.active_attempt_id.as_deref() != Some(attempt_id) {
        return Err(ForemanError::IdentityMismatch("attempt_id"));
    }
    Ok(())
}

fn validate_incremental_identity(
    frozen: Option<&str>,
    observed: Option<&str>,
    field: &'static str,
) -> Result<(), ForemanError> {
    if frozen.is_some() && observed.is_some() && frozen != observed {
        return Err(ForemanError::IdentityMismatch(field));
    }
    Ok(())
}

fn validate_receipt_identity(
    frozen: Option<&str>,
    receipt: Option<&str>,
    field: &'static str,
) -> Result<(), ForemanError> {
    if frozen.is_some() && frozen != receipt {
        return Err(ForemanError::IdentityMismatch(field));
    }
    Ok(())
}

fn latest_terminal_evidence_at(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<DateTime<Utc>, ForemanError> {
    let mut statement = transaction
        .prepare("SELECT receipt_kind, raw_bytes FROM terminal_receipts WHERE run_id = ?1")?;
    let rows = statement.query_map([run_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    let mut latest = None;
    for row in rows {
        let (kind, raw) = row?;
        let observed_at = if kind == "terminal" {
            TerminalReceiptV1::from_slice(&raw)?.ended_at
        } else {
            NotStartedReceiptV1::from_slice(&raw)?.recorded_at
        };
        latest = Some(latest.map_or(observed_at, |current: DateTime<Utc>| {
            current.max(observed_at)
        }));
    }
    latest.ok_or_else(|| ForemanError::IncompleteCloseout("no terminal evidence".to_owned()))
}

fn release_resources(
    transaction: &Transaction<'_>,
    run_id: &str,
    work_item_id: &str,
    attempt_id: &str,
    recorded_at: DateTime<Utc>,
) -> Result<(), ForemanError> {
    transaction.execute(
        "DELETE FROM resource_claims
         WHERE run_id = ?1 AND work_item_id = ?2 AND attempt_id = ?3",
        params![run_id, work_item_id, attempt_id],
    )?;
    append_internal(
        transaction,
        &InternalEvent {
            schema: INTERNAL_EVENT_SCHEMA.to_owned(),
            event_id: format!("resources-released-{}", Uuid::new_v4()),
            run_id: run_id.to_owned(),
            work_item_id: Some(work_item_id.to_owned()),
            attempt_id: Some(attempt_id.to_owned()),
            recorded_at,
            payload: InternalPayload::ResourcesReleased,
        },
    )
}

fn worker_brief_digest(
    connection: &Connection,
    packet: &NightshiftPacketV1,
    profile: &ExecutionProfileV2,
    run_id: &str,
    work_item_id: &str,
) -> Result<String, ForemanError> {
    let canonical = worker_brief_bytes(connection, packet, profile, run_id, work_item_id)?;
    Ok(domain_digest(BRIEF_DIGEST_DOMAIN, &canonical))
}

fn worker_brief_bytes(
    connection: &Connection,
    packet: &NightshiftPacketV1,
    profile: &ExecutionProfileV2,
    run_id: &str,
    work_item_id: &str,
) -> Result<Vec<u8>, ForemanError> {
    let item = packet
        .work_items
        .iter()
        .find(|item| item.id == work_item_id)
        .ok_or_else(|| ForemanError::UnknownWorkItem(work_item_id.to_owned()))?;
    if item.dependencies.len() > MAXIMUM_PREDECESSOR_RECEIPTS {
        return Err(ForemanError::InputTooLarge("predecessor receipt count"));
    }
    let packet_len: i64 = connection.query_row(
        "SELECT length(packet_bytes) FROM runs WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )?;
    let mut retained_lengths = Vec::new();
    let mut preflight_predecessors = BTreeMap::new();
    for dependency in &item.dependencies {
        let (receipt_kind, raw_len): (String, i64) = connection.query_row(
            "SELECT receipt_kind, length(raw_bytes) FROM terminal_receipts \
             WHERE run_id = ?1 AND work_item_id = ?2",
            params![run_id, dependency],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        retained_lengths.push(raw_len);
        preflight_predecessors.insert(
            dependency.clone(),
            serde_json::json!({
                "receipt_kind": receipt_kind,
                "retained_raw_digest": format!("sha256:{}", "0".repeat(64)),
                "encoding": "hex",
                "bytes_hex": "",
            }),
        );
    }
    let preflight_value = serde_json::json!({
        "schema": WORKER_BRIEF_BASIS_SCHEMA_V2,
        "packet_digest": packet.packet_digest,
        "packet_source": {
            "retained_raw_digest": format!("sha256:{}", "0".repeat(64)),
            "encoding": "hex",
            "bytes_hex": "",
        },
        "work_item": {
            "contract": "nightshift.orientation-packet/v1#work-item",
            "canonical_json": serde_jcs::to_string(item)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
        "predecessor_receipts": preflight_predecessors,
        "global_constraints": {
            "contract": "nightshift.orientation-packet/v1#global-constraints",
            "canonical_json": serde_jcs::to_string(&packet.global_constraints)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
        "execution": {
            "contract": "nightshift.foreman-execution-profile/v2#work-item",
            "canonical_json": serde_jcs::to_string(&profile.work_items[work_item_id])
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
    });
    let baseline = serde_jcs::to_vec(&preflight_value)
        .map_err(|error| ForemanError::Serialization(error.to_string()))?
        .len();
    let raw_total = retained_lengths
        .into_iter()
        .try_fold(packet_len, |total, length| {
            total
                .checked_add(length)
                .ok_or(ForemanError::InputTooLarge("worker brief"))
        })?;
    let expanded = usize::try_from(raw_total)
        .ok()
        .and_then(|length| length.checked_mul(2))
        .ok_or(ForemanError::InputTooLarge("worker brief"))?;
    if baseline
        .checked_add(expanded)
        .is_none_or(|size| size > MAXIMUM_WORKER_BRIEF_BYTES)
    {
        return Err(ForemanError::InputTooLarge("worker brief"));
    }
    let packet_raw: Vec<u8> = connection.query_row(
        "SELECT packet_bytes FROM runs WHERE run_id = ?1",
        [run_id],
        |row| row.get(0),
    )?;
    let mut predecessors = BTreeMap::new();
    for dependency in &item.dependencies {
        let (receipt_kind, raw): (String, Vec<u8>) = connection.query_row(
            "SELECT receipt_kind, raw_bytes FROM terminal_receipts \
             WHERE run_id = ?1 AND work_item_id = ?2",
            params![run_id, dependency],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        predecessors.insert(
            dependency.clone(),
            serde_json::json!({
                "receipt_kind": receipt_kind,
                "retained_raw_digest": raw_digest(&raw),
                "encoding": "hex",
                "bytes_hex": hex::encode(&raw),
            }),
        );
    }
    let value = serde_json::json!({
        "schema": WORKER_BRIEF_BASIS_SCHEMA_V2,
        "packet_digest": packet.packet_digest,
        "packet_source": {
            "retained_raw_digest": raw_digest(&packet_raw),
            "encoding": "hex",
            "bytes_hex": hex::encode(&packet_raw),
        },
        "work_item": {
            "contract": "nightshift.orientation-packet/v1#work-item",
            "canonical_json": serde_jcs::to_string(item)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
        "predecessor_receipts": predecessors,
        "global_constraints": {
            "contract": "nightshift.orientation-packet/v1#global-constraints",
            "canonical_json": serde_jcs::to_string(&packet.global_constraints)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
        "execution": {
            "contract": "nightshift.foreman-execution-profile/v2#work-item",
            "canonical_json": serde_jcs::to_string(&profile.work_items[work_item_id])
                .map_err(|error| ForemanError::Serialization(error.to_string()))?,
        },
    });
    let bytes = serde_jcs::to_vec(&value)
        .map_err(|error| ForemanError::Serialization(error.to_string()))?;
    if bytes.len() > MAXIMUM_WORKER_BRIEF_BYTES {
        return Err(ForemanError::InputTooLarge("worker brief"));
    }
    Ok(bytes)
}

fn build_final_document(
    connection: &Connection,
    packet: &NightshiftPacketV1,
    run_id: &str,
    updated_at: DateTime<Utc>,
) -> Result<FinalReceiptDocument, ForemanError> {
    let mut work_items = Vec::new();
    let mut questions = Vec::new();
    let mut custody = Vec::new();
    for item in &packet.work_items {
        let (kind, raw): (String, Vec<u8>) = connection.query_row(
            "SELECT receipt_kind, raw_bytes FROM terminal_receipts
             WHERE run_id = ?1 AND work_item_id = ?2",
            params![run_id, item.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if kind == "terminal" {
            let receipt = TerminalReceiptV1::from_slice(&raw)?;
            for question in &receipt.human_questions {
                questions.push(final_question(&item.id, question));
            }
            for repository in &receipt.repositories {
                custody.push(FinalCustody {
                    repository: repository.repository.clone(),
                    branch_head: format!("{}@{}", repository.branch, repository.head),
                    push_custody: repository.push_status.clone(),
                    dirty: "declared by exact worker receipt; no inference".to_owned(),
                    live_runtime: receipt.teardown.live_runtime.clone(),
                    secrets: receipt.teardown.secrets.clone(),
                    teardown: receipt.teardown.teardown.clone(),
                });
            }
            work_items.push(FinalWorkItem {
                id: item.id.clone(),
                state: receipt.state,
                result_classification: receipt.result_classification,
                repositories: receipt.repositories,
                tests: receipt.tests,
                evidence: receipt.evidence,
                live_or_production_mutations: receipt.live_or_production_mutations,
                remaining_trigger: receipt.remaining_trigger,
                next_lawful_action: receipt.next_lawful_action,
            });
        } else {
            let receipt = NotStartedReceiptV1::from_slice(&raw)?;
            for question in &receipt.human_questions {
                questions.push(final_question(&item.id, question));
            }
            work_items.push(FinalWorkItem {
                id: item.id.clone(),
                state: receipt.state,
                result_classification: receipt.result_classification,
                repositories: Vec::new(),
                tests: Vec::new(),
                evidence: receipt.evidence,
                live_or_production_mutations: Vec::new(),
                remaining_trigger: receipt.remaining_trigger,
                next_lawful_action: receipt.next_lawful_action,
            });
        }
    }
    Ok(FinalReceiptDocument {
        schema: "nightshift.run-receipts/v1".to_owned(),
        packet_digest: packet.packet_digest.clone(),
        updated_at: updated_at.to_rfc3339_opts(SecondsFormat::Secs, true),
        work_items,
        human_questions: questions,
        repository_custody: custody,
    })
}

fn final_question(work_item_id: &str, question: &HumanQuestionV1) -> FinalQuestion {
    FinalQuestion {
        work_item: work_item_id.to_owned(),
        question: question.question.clone(),
        evidence_exhausted: question.exhausted_evidence.clone(),
        safe_default: question.safe_default.clone(),
        consequences: question.consequences.clone(),
        resume_point: question.resume_point.clone(),
    }
}

fn ensure_run(connection: &Connection, run_id: &str) -> Result<(), ForemanError> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id = ?1)",
        [run_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(ForemanError::UnknownRun(run_id.to_owned()));
    }
    Ok(())
}

fn placeholder_digest() -> String {
    format!("sha256:{}", "0".repeat(64))
}

fn raw_digest(bytes: &[u8]) -> String {
    domain_digest(RAW_DIGEST_DOMAIN, bytes)
}

fn domain_digest(domain: &[u8], bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update(bytes);
    format!("sha256:{:x}", digest.finalize())
}

#[allow(dead_code)]
fn _teardown_contract_is_retained(_: TeardownDeclarationV1) {}

#[cfg(test)]
mod read_only_tests {
    use super::*;

    #[test]
    fn read_only_connection_enforces_query_only() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("query-only.sqlite");
        let writer = Connection::open(&database).unwrap();
        initialize(&writer).unwrap();
        drop(writer);

        let store = ForemanStore::open_read_only(&database).unwrap();
        let connection = store.connection().unwrap();
        let enabled: u8 = connection
            .query_row("PRAGMA query_only", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
        assert!(connection
            .execute("CREATE TABLE forbidden_read_side_effect (id INTEGER)", [])
            .is_err());
    }
}
