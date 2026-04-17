//! Coordination primitives — scope overlap classification and
//! the single shared `coordination_outcome` vocabulary used by both
//! preflight (risky classes) and reconciliation (all runs).
//!
//! See `GAP-parallel-ops.md`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::agenda::{Agenda, CriticalityClass, Scope};

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

/// Classification of why an agenda is (or isn't) a **risky class of
/// work** per `GAP-parallel-ops.md`. Risky classes require preflight
/// as a gate, not a background signal.
///
/// v1 recognizes a narrow set of risky conditions. More will be added
/// as the project grows; the structure here is so callers don't have
/// to reach into individual rules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RiskyClassification {
    pub protected_scope: bool,
}

impl RiskyClassification {
    pub fn is_risky(&self) -> bool {
        self.protected_scope
    }

    pub fn reasons(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.protected_scope {
            out.push("protected-class service in scope");
        }
        out
    }
}

/// Classify an agenda's risky-class status.
///
/// v1: `protected` criticality class → risky. Shared-infrastructure,
/// topology/config/publisher/source-change, and mode-transition
/// triggers come in later slices; their hooks already exist in the
/// schemas.
pub fn classify_risky(agenda: &Agenda) -> RiskyClassification {
    RiskyClassification {
        protected_scope: matches!(agenda.criticality.class, CriticalityClass::Protected),
    }
}

/// Preflight outcome for a single agenda given whether Continuity is
/// configured in this deployment.
///
/// v1 rule (per CLAUDE.md #18 and GAP-parallel-ops.md):
/// - non-risky agenda → `Clear`
/// - risky agenda + Continuity configured → `Clear` (v1 has no real
///   Continuity integration yet; this is the intended behavior once
///   a real query happens)
/// - risky agenda + Continuity **not** configured → `HoldForContext`
///   (the run cannot leave capture without coordination context)
pub fn preflight(agenda: &Agenda, continuity_configured: bool) -> CoordinationOutcome {
    let risk = classify_risky(agenda);
    if !risk.is_risky() {
        return CoordinationOutcome::Clear;
    }
    if continuity_configured {
        CoordinationOutcome::Clear
    } else {
        CoordinationOutcome::HoldForContext
    }
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
    use crate::agenda::Agenda;

    fn mk_agenda(class: CriticalityClass) -> Agenda {
        let yaml = format!(
            r#"
agenda_version: 0
agenda_id: test-{}
workflow_family: ops
owner: test
cadence: {{ kind: scheduled, expr: "0 3 * * *" }}
scope: {{ hosts: [h1], services: [], paths: [], repos: [] }}
artifact_target: packet
promotion_ceiling: advise
reconciler: {{ required: true }}
allowed_evidence_sources: [nq]
allowed_tool_classes: [discover, read, propose]
budget: {{ max_wall_seconds: 600 }}
governor_binding: {{ required_above: observe }}
criticality:
  class: {}
  re_alert_after: 4h
  silence_max_duration: 72h
  {protected_role}
diagnosis: {{ mode: self_check }}
"#,
            match class {
                CriticalityClass::Protected => "protected",
                _ => "standard",
            },
            match class {
                CriticalityClass::Standard => "standard",
                CriticalityClass::BusinessCritical => "business_critical",
                CriticalityClass::Safety => "safety",
                CriticalityClass::Protected => "protected",
            },
            protected_role = if matches!(class, CriticalityClass::Protected) {
                "protected_role: observation_critical"
            } else {
                ""
            }
        );
        serde_yaml::from_str(&yaml).expect("test agenda must parse")
    }

    #[test]
    fn classify_risky_flags_protected_class() {
        let a = mk_agenda(CriticalityClass::Protected);
        let r = classify_risky(&a);
        assert!(r.is_risky());
        assert!(!r.reasons().is_empty());
    }

    #[test]
    fn classify_risky_does_not_flag_standard_class() {
        let a = mk_agenda(CriticalityClass::Standard);
        let r = classify_risky(&a);
        assert!(!r.is_risky());
    }

    #[test]
    fn preflight_clears_non_risky_without_continuity() {
        let a = mk_agenda(CriticalityClass::Standard);
        assert_eq!(preflight(&a, false), CoordinationOutcome::Clear);
    }

    #[test]
    fn preflight_holds_risky_without_continuity() {
        // This is the load-bearing proof of commit D:
        // a protected-class agenda cannot clear preflight without a
        // coordination substrate. Hooked in != used; not hooked in
        // != fine.
        let a = mk_agenda(CriticalityClass::Protected);
        assert_eq!(preflight(&a, false), CoordinationOutcome::HoldForContext);
        assert!(!preflight(&a, false).may_proceed());
    }

    #[test]
    fn preflight_clears_risky_with_continuity_stub() {
        // v1 stub behavior: with continuity "configured" (no real
        // query yet) risky agendas clear. Real content-checking
        // arrives in a later slice.
        let a = mk_agenda(CriticalityClass::Protected);
        assert_eq!(preflight(&a, true), CoordinationOutcome::Clear);
    }

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
