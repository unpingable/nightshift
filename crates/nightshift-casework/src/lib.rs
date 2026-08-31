//! Deterministic read-only casework projections over exact Nightshift packet
//! and run-receipt bytes.
//!
//! This crate is an operator inspection tool. It creates no authorization,
//! aggregate result, retry, execution request, or canonical runtime state.

mod live_capacity;
mod live_loader;
mod live_model;
mod loader;
mod model;
pub mod server;
pub mod static_ui;

pub use live_loader::{load_live_run_at, LiveCaseworkError, LoadedLiveRun};
pub use live_model::*;
pub use loader::{load_run_at, load_runs_at, CaseworkError, LoadedRun};
pub use model::*;
