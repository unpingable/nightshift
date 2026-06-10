//! Night Shift daemon library crate.
//!
//! v1 MVP: Watchbill, observe/advise only, NQ pull, capture → reconcile →
//! packet → record. No mutation. See `docs/architecture/DESIGN.md` for the full spec
//! and the v1 field budget.

pub mod agenda;
pub mod attention;
pub mod bundle;
pub mod closure;
pub mod coordination;
pub mod drill;
pub mod errors;
pub mod finding;
pub mod freshness;
pub mod governor_client;
pub mod horizon;
pub mod horizon_policy;
pub mod ledger;
pub mod liveness;
pub mod liveness_peek;
pub mod mvp_a;
pub mod nq;
pub mod nq_peek;
pub mod packet;
pub mod pipeline;
pub mod proposed_action;
pub mod posture;
pub mod posture_class;
pub mod reconcile_horizon;
pub mod reconciler;
pub mod scheduled;
pub mod store;
pub mod wal_bloat_stager;

pub use errors::{NightShiftError, Result};
