//! Night Shift daemon library crate.
//!
//! v1 MVP: Watchbill, observe/advise only, NQ pull, capture → reconcile →
//! packet → record. No mutation. See `docs/DESIGN.md` for the full spec
//! and the v1 field budget.

pub mod agenda;
pub mod bundle;
pub mod coordination;
pub mod errors;
pub mod finding;
pub mod ledger;
pub mod liveness;
pub mod nq;
pub mod nq_peek;
pub mod packet;
pub mod pipeline;
pub mod posture;
pub mod reconciler;
pub mod store;

pub use errors::{NightShiftError, Result};
