//! Reconciler — freshness and invalidation pass.
//!
//> The 3am agent must not act on 11pm vibes.
//!
//! The reconciler takes a captured `Bundle` and assigns each input a
//! reconciliation result: `committed`, `changed`, `stale`, or
//! `invalidated`, plus a reliance class and valid_for scope.
//!
//! Recheck is the gate, not metadata.

use chrono::Utc;

use crate::bundle::{
    Bundle, CaptureInput, InputStatus, InvalidationRule, ReconciliationPhase, ReconciliationResult,
    ReconciliationSummary, RelianceClass, RelianceScope, ValidFor,
};
use crate::errors::Result;
use crate::finding::{FindingKey, FindingSnapshot};
use crate::nq::{evidence_hash, NqSource};

/// Result of reconciling a single NQ input against current NQ state.
///
/// Returned as a standalone decision so it can be unit-tested without
/// a whole pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NqReconciliation {
    pub status: InputStatus,
    pub notes: Option<String>,
    pub previous_evidence_hash: Option<String>,
    pub current_evidence_hash: Option<String>,
}

/// Reconcile a single NQ-backed input given its capture record and
/// the current snapshot from the NQ source (or `None` if absent).
///
/// This is the core of "reconcile before propose." It does not touch
/// state; it returns a decision.
pub fn reconcile_nq_input(
    captured: &CaptureInput,
    current: Option<&FindingSnapshot>,
    now: chrono::DateTime<chrono::Utc>,
) -> NqReconciliation {
    // Freshness expiry takes precedence — stale captured evidence is
    // stale regardless of what the source now says.
    if let Some(expires_at) = captured.freshness.expires_at {
        if now >= expires_at {
            return NqReconciliation {
                status: InputStatus::Stale,
                notes: Some(format!(
                    "captured freshness expired at {} (now {})",
                    expires_at.to_rfc3339(),
                    now.to_rfc3339()
                )),
                previous_evidence_hash: Some(captured.evidence_hash.clone()),
                current_evidence_hash: current.map(evidence_hash),
            };
        }
    }

    match current {
        None => {
            // Finding absent at current generation — check invalidates_if.
            let absent_rule = captured
                .freshness
                .invalidates_if
                .iter()
                .any(|r| matches!(r, InvalidationRule::FindingAbsentForNGenerations { .. }));
            if absent_rule {
                NqReconciliation {
                    status: InputStatus::Invalidated,
                    notes: Some("finding absent from current NQ generation".into()),
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: None,
                }
            } else {
                // No absence rule declared — treat as stale rather than invalid.
                NqReconciliation {
                    status: InputStatus::Stale,
                    notes: Some(
                        "finding absent; no invalidation rule declared — downgrading to stale".into(),
                    ),
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: None,
                }
            }
        }
        Some(snap) => {
            let current_hash = evidence_hash(snap);
            if current_hash == captured.evidence_hash {
                NqReconciliation {
                    status: InputStatus::Committed,
                    notes: None,
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: Some(current_hash),
                }
            } else {
                NqReconciliation {
                    status: InputStatus::Changed,
                    notes: Some("evidence hash differs from capture".into()),
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: Some(current_hash),
                }
            }
        }
    }
}

/// Map an input status to its reliance class (v1 defaults).
///
/// Diagnosis-review and agenda policy may downgrade further; this is
/// the reconciler's default assignment.
pub fn reliance_class_for(status: InputStatus, source: &str) -> RelianceClass {
    match (status, source) {
        (InputStatus::Invalidated, _) => RelianceClass::None,
        (InputStatus::Stale, _) => RelianceClass::Historical,
        (InputStatus::Committed | InputStatus::Changed, "nq" | "governor") => {
            RelianceClass::Authoritative
        }
        (InputStatus::Committed | InputStatus::Changed, "continuity") => RelianceClass::Hint,
        (InputStatus::Committed | InputStatus::Changed, _) => RelianceClass::Hint,
        (InputStatus::Observed, _) => RelianceClass::None,
    }
}

/// Default `valid_for` scope for a given reliance class.
pub fn valid_for_default(cls: RelianceClass) -> Vec<ValidFor> {
    match cls {
        RelianceClass::Authoritative => vec![
            ValidFor::Authorization,
            ValidFor::Proposal,
            ValidFor::Diagnosis,
            ValidFor::PacketContext,
        ],
        RelianceClass::AuthoritativeForCoordination => vec![
            ValidFor::CoordinationGating,
            ValidFor::Diagnosis,
            ValidFor::PacketContext,
        ],
        RelianceClass::Hint => vec![ValidFor::Diagnosis, ValidFor::PacketContext],
        RelianceClass::Historical => vec![ValidFor::PacketContext],
        RelianceClass::None => vec![],
    }
}

/// Reconcile a full bundle against current state, producing a
/// `ReconciliationPhase` to attach to it.
///
/// v1: only NQ-backed inputs are actively reconciled against a live
/// source. Other input kinds keep their captured status and are
/// assigned default reliance classes.
pub fn reconcile_bundle(bundle: &Bundle, nq: &dyn NqSource) -> Result<ReconciliationPhase> {
    let now = Utc::now();
    let mut results = Vec::with_capacity(bundle.capture.inputs.len());

    for input in &bundle.capture.inputs {
        let (status, notes, prev_hash, curr_hash) = if input.source == "nq" {
            // Extract the finding_key from the input_id ("nq:finding:<source>:<detector>:<subject>")
            // or from the payload. For v1 fixture source, the input_id carries it directly.
            let key = parse_nq_input_id(&input.input_id)?;
            let current = nq.snapshot(&key)?;
            let r = reconcile_nq_input(input, current.as_ref(), now);
            (r.status, r.notes, r.previous_evidence_hash, r.current_evidence_hash)
        } else {
            // Non-NQ inputs: keep observed → committed without a live check in v1.
            (
                InputStatus::Committed,
                None,
                Some(input.evidence_hash.clone()),
                Some(input.evidence_hash.clone()),
            )
        };

        let reliance_class = reliance_class_for(status, &input.source);
        let valid_for = valid_for_default(reliance_class);

        results.push(ReconciliationResult {
            input_id: input.input_id.clone(),
            status,
            reliance_class,
            scope: RelianceScope {
                run_id: bundle.run_id.clone(),
                valid_for,
            },
            previous_evidence_hash: prev_hash,
            current_evidence_hash: curr_hash,
            notes,
            concurrent_activity: None,
        });
    }

    let summary = build_summary(&results);

    Ok(ReconciliationPhase {
        reconciled_at: now,
        reconciled_by: "scheduler".into(),
        results,
        summary,
    })
}

fn build_summary(results: &[ReconciliationResult]) -> ReconciliationSummary {
    let mut s = ReconciliationSummary {
        ok_to_proceed: true,
        ..Default::default()
    };
    let mut any_invalidated = false;
    for r in results {
        let id = r.input_id.clone();
        if r.scope.valid_for.contains(&ValidFor::Authorization) {
            s.admissible_for_authorization.push(id.clone());
        }
        if r.scope.valid_for.contains(&ValidFor::Proposal) {
            s.admissible_for_proposal.push(id.clone());
        }
        if r.scope.valid_for.contains(&ValidFor::Diagnosis) {
            s.admissible_for_diagnosis.push(id.clone());
        }
        if r.scope.valid_for.contains(&ValidFor::CoordinationGating) {
            s.coordination_gating.push(id.clone());
        }
        if matches!(r.reliance_class, RelianceClass::Hint) {
            s.hints_only.push(id.clone());
        }
        if matches!(r.reliance_class, RelianceClass::Historical) {
            s.downgraded.push(id.clone());
        }
        if matches!(r.status, InputStatus::Invalidated) {
            s.blocked.push(id.clone());
            any_invalidated = true;
        }
    }
    // v1 stance: if an authoritative input invalidated, ok_to_proceed = false.
    // Downgraded/stale inputs do not block; the packet notes the gap.
    if any_invalidated {
        s.ok_to_proceed = false;
    }
    s
}

fn parse_nq_input_id(id: &str) -> Result<FindingKey> {
    // Format: "nq:finding:<source>:<detector>:<subject>"
    // For v1 we accept "nq:finding:<detector>:<subject>" and default source=nq.
    let parts: Vec<&str> = id.splitn(4, ':').collect();
    match parts.as_slice() {
        ["nq", "finding", detector, subject] => Ok(FindingKey {
            source: "nq".into(),
            detector: (*detector).into(),
            subject: (*subject).into(),
        }),
        _ => Err(crate::NightShiftError::InvalidAgenda(format!(
            "cannot parse NQ input_id: {id}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{Freshness, InvalidationRule};
    use crate::finding::{EvidenceState, Severity};
    use chrono::{Duration, TimeZone, Utc};

    fn captured_input(hash: &str, expires_at: Option<chrono::DateTime<Utc>>) -> CaptureInput {
        CaptureInput {
            input_id: "nq:finding:wal_bloat:labelwatch-host:/var/lib/db".into(),
            source: "nq".into(),
            kind: "nq_finding_snapshot".into(),
            status: InputStatus::Observed,
            evidence_hash: hash.into(),
            freshness: Freshness {
                captured_at: Utc.with_ymd_and_hms(2026, 4, 16, 22, 0, 0).unwrap(),
                expires_at,
                invalidates_if: vec![
                    InvalidationRule::FindingAbsentForNGenerations { n: 1 },
                    InvalidationRule::HostUnreachable,
                ],
            },
            payload_ref: "ledger://...".into(),
            admissible_for: vec![],
            inadmissible_for: vec![],
        }
    }

    fn snap(hash_seed: u32) -> FindingSnapshot {
        FindingSnapshot {
            finding_key: FindingKey {
                source: "nq".into(),
                detector: "wal_bloat".into(),
                subject: "labelwatch-host:/var/lib/db".into(),
            },
            host: "labelwatch-host".into(),
            severity: Severity::Warning,
            domain: Some("delta_g".into()),
            persistence_generations: 4,
            first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 14, 32, 15).unwrap(),
            current_status: EvidenceState::Active,
            snapshot_generation: 39000 + u64::from(hash_seed),
            captured_at: Utc.with_ymd_and_hms(2026, 4, 17, 3, 0, 0).unwrap(),
            evidence_hash: String::new(),
        }
    }

    #[test]
    fn committed_when_hash_matches() {
        let s = snap(0);
        let hash = evidence_hash(&s);
        let c = captured_input(&hash, None);
        let r = reconcile_nq_input(&c, Some(&s), Utc::now());
        assert_eq!(r.status, InputStatus::Committed);
    }

    #[test]
    fn changed_when_hash_differs() {
        let s = snap(0);
        let hash = evidence_hash(&s);
        let s2 = snap(1); // different snapshot_generation → different hash
        let c = captured_input(&hash, None);
        let r = reconcile_nq_input(&c, Some(&s2), Utc::now());
        assert_eq!(r.status, InputStatus::Changed);
        assert_ne!(r.previous_evidence_hash, r.current_evidence_hash);
    }

    #[test]
    fn stale_when_expired() {
        let s = snap(0);
        let hash = evidence_hash(&s);
        let expired = Utc::now() - Duration::hours(1);
        let c = captured_input(&hash, Some(expired));
        let r = reconcile_nq_input(&c, Some(&s), Utc::now());
        assert_eq!(r.status, InputStatus::Stale);
    }

    #[test]
    fn invalidated_when_finding_absent_with_absence_rule() {
        let c = captured_input("sha256:abc", None);
        let r = reconcile_nq_input(&c, None, Utc::now());
        assert_eq!(r.status, InputStatus::Invalidated);
    }

    #[test]
    fn stale_when_finding_absent_without_absence_rule() {
        let mut c = captured_input("sha256:abc", None);
        c.freshness.invalidates_if.clear();
        let r = reconcile_nq_input(&c, None, Utc::now());
        assert_eq!(r.status, InputStatus::Stale);
    }

    #[test]
    fn reliance_class_reflects_status() {
        assert_eq!(
            reliance_class_for(InputStatus::Committed, "nq"),
            RelianceClass::Authoritative
        );
        assert_eq!(
            reliance_class_for(InputStatus::Changed, "nq"),
            RelianceClass::Authoritative
        );
        assert_eq!(
            reliance_class_for(InputStatus::Stale, "nq"),
            RelianceClass::Historical
        );
        assert_eq!(
            reliance_class_for(InputStatus::Invalidated, "nq"),
            RelianceClass::None
        );
        assert_eq!(
            reliance_class_for(InputStatus::Committed, "continuity"),
            RelianceClass::Hint
        );
    }
}
