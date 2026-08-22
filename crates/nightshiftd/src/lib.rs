//! Canonical Nightshift temporal observation and attention office.
//!
//! This crate schedules exact observation cycles, consumes qualified present
//! support and complete NQ diagnostics, computes non-authorizing posture and
//! attention, and submits immutable exact-work proposals to AG. It has no
//! standing, authorization, execution-custody, effect, or AG-continuation API.

pub mod ag_port;
pub mod authoring_context;
pub mod authoring_custody;
pub mod canonical_runtime;
pub mod canonical_store;
pub mod currentness;
pub mod decision_basis;
pub mod diagnostic_execution_v2;
pub mod diagnostic_posture;
pub mod errors;
pub mod external_observation;
pub mod nq_admission;
pub mod nq_disposition;
pub mod observation_resolver;

pub use errors::{NightShiftError, Result};
