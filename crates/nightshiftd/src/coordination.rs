//! Coordination primitives — scope overlap classification and
//! the single shared `coordination_outcome` vocabulary used by both
//! preflight (risky classes) and reconciliation (all runs).
//!
//! See `GAP-parallel-ops.md`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agenda::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlapClass {
    Disjoint,
    SharedRead,
    SharedWrite,
    Contested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationOutcome {
    Clear,
    HoldForContext,
    Coordinate,
    BlockForResolution,
    OperatorOverride,
}

impl CoordinationOutcome {
    /// Whether this outcome allows the run to proceed past capture.
    pub fn may_proceed(self) -> bool {
        matches!(self, Self::Clear | Self::OperatorOverride)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrentActor {
    pub actor_id: String,
    #[serde(default)]
    pub session: Option<String>,
    pub touched_at: DateTime<Utc>,
    pub scope_overlap: OverlapClass,
    #[serde(default)]
    pub last_breadcrumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrentActivity {
    pub overlap_class: OverlapClass,
    pub coordination_outcome: CoordinationOutcome,
    #[serde(default)]
    pub actors: Vec<ConcurrentActor>,
}

/// Canonical scope key derived from `(hosts, services, paths, repos)`.
/// Two actors in the same scope must produce the same key.
pub fn scope_key(scope: &Scope) -> String {
    fn canonical(v: &[String]) -> Vec<String> {
        let mut out = v.to_vec();
        out.sort();
        out.dedup();
        out
    }
    let hosts = canonical(&scope.hosts).join(",");
    let services = canonical(&scope.services).join(",");
    let paths = canonical(&scope.paths).join(",");
    let repos = canonical(&scope.repos).join(",");
    format!("h=[{hosts}]|s=[{services}]|p=[{paths}]|r=[{repos}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_key_is_order_and_dup_insensitive() {
        let a = Scope {
            hosts: vec!["beta".into(), "alpha".into(), "alpha".into()],
            services: vec![],
            paths: vec![],
            repos: vec![],
        };
        let b = Scope {
            hosts: vec!["alpha".into(), "beta".into()],
            services: vec![],
            paths: vec![],
            repos: vec![],
        };
        assert_eq!(scope_key(&a), scope_key(&b));
    }

    #[test]
    fn scope_key_distinguishes_axes() {
        // same string value on different axes must not collide
        let a = Scope {
            hosts: vec!["x".into()],
            services: vec![],
            paths: vec![],
            repos: vec![],
        };
        let b = Scope {
            hosts: vec![],
            services: vec!["x".into()],
            paths: vec![],
            repos: vec![],
        };
        assert_ne!(scope_key(&a), scope_key(&b));
    }

    #[test]
    fn may_proceed_is_narrow() {
        assert!(CoordinationOutcome::Clear.may_proceed());
        assert!(CoordinationOutcome::OperatorOverride.may_proceed());
        assert!(!CoordinationOutcome::HoldForContext.may_proceed());
        assert!(!CoordinationOutcome::Coordinate.may_proceed());
        assert!(!CoordinationOutcome::BlockForResolution.may_proceed());
    }
}
