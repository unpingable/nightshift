use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    os::fd::AsRawFd,
    os::unix::fs::{MetadataExt as _, OpenOptionsExt as _},
    path::{Path, PathBuf},
};

use chrono::{DateTime, SecondsFormat, Utc};
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
    AdapterEventV1, ExecutionProfileV2, ForemanAdmissionV1, HumanQuestionV1, LiveRunProjectionV1,
    NotStartedReceiptV1, ReceiptRepositoryV1, Scheduler, SchedulerStateV1, TerminalReceiptV1,
    WorkerStartRequestV2, WORKER_START_REQUEST_SCHEMA_V2, WORKER_TERMINAL_RECEIPT_SCHEMA_V1,
};

const INTERNAL_EVENT_SCHEMA: &str = "nightshift.foreman-journal-event/v1";
const BRIEF_DIGEST_DOMAIN: &[u8] = b"nightshift.worker-brief.digest/v1\0";
const RAW_DIGEST_DOMAIN: &[u8] = b"nightshift.foreman-retained-raw.digest/v1\0";

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InternalPayload {
    RunAdmitted,
    AttemptCreated {
        resource_lock_keys: Vec<String>,
        start_request: Box<WorkerStartRequestV2>,
    },
    DispatchRequested,
    ResumeRequested,
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
        let connection = self.connection()?;
        let (packet, _, profile, _) = load_contracts(&connection, run_id)?;
        worker_brief_bytes(&connection, &packet, &profile, run_id, work_item_id)
    }

    pub fn prepare_attempt(
        &self,
        run_id: &str,
        work_item_id: &str,
        recorded_at: DateTime<Utc>,
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
        let (packet, _, profile, _) = load_contracts(&transaction, run_id)?;
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

fn append_internal(
    transaction: &Transaction<'_>,
    event: &InternalEvent,
) -> Result<(), ForemanError> {
    let raw =
        serde_jcs::to_vec(event).map_err(|error| ForemanError::Serialization(error.to_string()))?;
    transaction.execute(
        "INSERT INTO events
         (event_id, run_id, work_item_id, attempt_id, kind, recorded_at, raw_bytes, raw_digest)
         VALUES (?1, ?2, ?3, ?4, 'internal', ?5, ?6, ?7)",
        params![
            event.event_id,
            event.run_id,
            event.work_item_id,
            event.attempt_id,
            event.recorded_at.to_rfc3339(),
            raw,
            raw_digest(&raw),
        ],
    )?;
    Ok(())
}

type RawContractRow = (Vec<u8>, Vec<u8>, Vec<u8>, u16);

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
            "SELECT packet_bytes, admission_bytes, profile_bytes, maximum_concurrent_workers
             FROM runs WHERE run_id = ?1",
            [run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let (packet_raw, admission_raw, profile_raw, maximum) =
        row.ok_or_else(|| ForemanError::UnknownRun(run_id.to_owned()))?;
    let packet = NightshiftPacketV1::from_slice(&packet_raw)
        .map_err(|error| ForemanError::Packet(error.to_string()))?;
    packet
        .validate_integrity()
        .map_err(|error| ForemanError::Packet(error.to_string()))?;
    let admission = ForemanAdmissionV1::from_slice(&admission_raw)?;
    admission.validate()?;
    let profile = ExecutionProfileV2::from_slice(&profile_raw)?;
    profile.validate()?;
    Ok((packet, admission, profile, maximum))
}

fn load_projection(
    connection: &Connection,
    run_id: &str,
) -> Result<LiveRunProjectionV1, ForemanError> {
    let (packet, admission, profile, maximum) = load_contracts(connection, run_id)?;
    let mut statement = connection.prepare(
        "SELECT sequence, work_item_id, attempt_id, kind, raw_bytes, raw_digest
         FROM events WHERE run_id = ?1 ORDER BY sequence ASC",
    )?;
    let rows = statement.query_map([run_id], |row| {
        Ok((
            row.get::<_, u64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Vec<u8>>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    let mut events = Vec::new();
    for row in rows {
        let (sequence, work_item_id, attempt_id, kind, raw, raw_digest) = row?;
        let replay_kind = if kind == "adapter_event" {
            ReplayKind::Adapter(Box::new(AdapterEventV1::from_slice(&raw)?))
        } else {
            let event: InternalEvent = serde_json::from_slice(&raw)
                .map_err(|error| ForemanError::Serialization(error.to_string()))?;
            match event.payload {
                InternalPayload::RunAdmitted => ReplayKind::RunAdmitted,
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
            sequence,
            work_item_id,
            attempt_id,
            kind: replay_kind,
            raw_digest,
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
    let mut predecessors = BTreeMap::new();
    for dependency in &item.dependencies {
        let raw: Vec<u8> = connection.query_row(
            "SELECT raw_bytes FROM terminal_receipts WHERE run_id = ?1 AND work_item_id = ?2",
            params![run_id, dependency],
            |row| row.get(0),
        )?;
        predecessors.insert(dependency.clone(), raw_digest(&raw));
    }
    let value = serde_json::json!({
        "schema": "nightshift.worker-brief-basis/v1",
        "packet_digest": packet.packet_digest,
        "work_item": item,
        "predecessor_receipt_raw_digests": predecessors,
        "global_constraints": packet.global_constraints,
        "execution": profile.work_items[work_item_id],
    });
    serde_jcs::to_vec(&value).map_err(|error| ForemanError::Serialization(error.to_string()))
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
