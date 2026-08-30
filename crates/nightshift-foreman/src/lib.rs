//! Durable, non-authorizing local agent-compute scheduling for one exact packet.
//!
//! This operator-tool crate is outside canonical `nightshiftd`. Scheduler state
//! is mechanism evidence and never a campaign result or target-effect authority.

pub mod contract;
pub mod scheduler;
pub mod store;

pub use contract::*;
pub use scheduler::{LiveRunProjectionV1, LiveWorkItemV1, Scheduler};
pub use store::{ForemanError, ForemanStore};

pub const FOREMAN_ADMISSION_SCHEMA_V1: &str = "nightshift.foreman-admission/v1";
pub const FOREMAN_EXECUTION_PROFILE_SCHEMA_V1: &str = "nightshift.foreman-execution-profile/v1";
pub const WORKER_START_REQUEST_SCHEMA_V1: &str = "nightshift.worker-start-request/v1";
pub const WORKER_ADAPTER_EVENT_SCHEMA_V1: &str = "nightshift.worker-adapter-event/v1";
pub const WORKER_TERMINAL_RECEIPT_SCHEMA_V1: &str = "nightshift.worker-terminal-receipt/v1";
pub const WORK_ITEM_NOT_STARTED_RECEIPT_SCHEMA_V1: &str =
    "nightshift.work-item-not-started-receipt/v1";
pub const LIVE_RUN_PROJECTION_SCHEMA_V1: &str = "nightshift.foreman-live-run/v1";
