//! Durable, non-authorizing local agent-compute scheduling for one exact packet.
//!
//! This operator-tool crate is outside canonical `nightshiftd`. Scheduler state
//! is mechanism evidence and never a campaign result or target-effect authority.

pub mod contract;
pub mod scheduler;
pub mod store;

pub use contract::*;
pub use scheduler::{LiveRunProjectionV1, LiveWorkItemV1, Scheduler};
pub use store::{
    read_only_run_snapshot, CapacityAdmissionEvidenceV1, ForemanError, ForemanStore,
    ReadOnlyCapacityAdmissionV1, ReadOnlyCapacityRequirementV1, ReadOnlyEventRowV1,
    ReadOnlyRunSnapshotV1, ReadOnlyTerminalReceiptRowV1,
};

pub const FOREMAN_ADMISSION_SCHEMA_V1: &str = "nightshift.foreman-admission/v1";
pub const FOREMAN_CAPACITY_ADMISSION_SCHEMA_V1: &str = "nightshift.foreman-capacity-admission/v1";
pub const FOREMAN_CAPACITY_REQUIREMENT_SCHEMA_V1: &str =
    "nightshift.foreman-capacity-requirement/v1";
pub const FOREMAN_EXECUTION_PROFILE_SCHEMA_V2: &str = "nightshift.foreman-execution-profile/v2";
pub const WORKER_START_REQUEST_SCHEMA_V2: &str = "nightshift.worker-start-request/v2";
pub const WORKER_BRIEF_BASIS_SCHEMA_V2: &str = "nightshift.worker-brief-basis/v2";
pub const MAXIMUM_ADAPTER_TIMEOUT_SECONDS: u64 = 86_400;
pub const MAXIMUM_WORKER_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
pub const MAXIMUM_PREDECESSOR_RECEIPTS: usize = 1024;
pub const MAXIMUM_WORKER_BRIEF_BYTES: usize = 16 * 1024 * 1024;
pub const WORKER_ADAPTER_CAPABILITIES_SCHEMA_V1: &str = "nightshift.worker-adapter-capabilities/v1";
pub const WORKER_ATTEMPT_BINDING_SCHEMA_V1: &str = "nightshift.worker-attempt-binding/v1";
pub const WORKER_ADAPTER_EVENT_SCHEMA_V1: &str = "nightshift.worker-adapter-event/v1";
pub const WORKER_TERMINAL_RECEIPT_SCHEMA_V1: &str = "nightshift.worker-terminal-receipt/v1";
pub const WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1: &str =
    "nightshift.work-item-not-started-receipt/v1";
pub const LIVE_RUN_PROJECTION_SCHEMA_V1: &str = "nightshift.foreman-live-run/v1";
