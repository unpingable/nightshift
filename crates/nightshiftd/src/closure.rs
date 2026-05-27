//! Closure-candidate predicate — Slice 4, Gate 1 (partial).
//!
//! This module emits a *review-gating verdict*, not closure
//! authority. There is no `close` verb in v1; NS does not mutate
//! incident state. The predicate's purpose is to make the refusal
//! cases explicit and testable now, so that when a close verb
//! eventually exists it can consult this surface and refuse closure
//! on the same grounds.
//!
//! Per `working/decisions/pre-positioned-doctrine-gates.md` Gate 1
//! and the framing in `working/roadmaps/nightshift_v1_runtime_ladder.md`:
//!
//! - **closure candidate ≠ closure authorization.** The predicate is
//!   enforceable refusal; it does not authorize anything.
//! - **Missing channel classification blocks eligibility rather than
//!   disappearing from the model.** When NS cannot distinguish
//!   proxy-channel from consequence-channel evidence on an
//!   `IncidentShape` finding, the verdict is
//!   `UnassessableMissingChannelClassification` — *not* eligible,
//!   not silently dropped from the predicate.
//!
//! `EligibleForClosureReview` is defined in the enum but unreachable
//! under v1 conditions. Reaching it requires NQ to expose a wire
//! shape that distinguishes proxy-channel from consequence-channel
//! findings; until then, every `IncidentShape` finding lacks that
//! channel classification and falls through to `Unassessable`. The
//! variant exists to keep the shape of the design space honest:
//! "we might eventually approve review" is named, not unspoken.
//!
//! No case emits a positive `eligible_for_closure` — there is no
//! such variant. "Review has a way of becoming approved in a trench
//! coat" (see SLICE_4 FEATURE-HISTORY field notes).

use serde::{Deserialize, Serialize};

use crate::bundle::InputStatus;
use crate::finding::EvidenceState;
use crate::packet::AttentionState;
use crate::posture_class::PostureClass;

/// Why a finding fails the closure-candidate predicate. These are
/// the v1 known blocker classes; each maps to existing
/// reconciliation state surfaced by an earlier slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotEligibleReason {
    /// The finding's evidence shape is silence (Slice C.1
    /// `PostureClass::SilenceShape`). Quiet on the observation
    /// surface does not equal recovered in substrate, so closure is
    /// refused.
    ProxyQuiet,
    /// Reconciliation produced `EvidenceState::Stale` or
    /// `InputStatus::Stale` — typically Slice B's
    /// `imported_producer_basis_stale` or the Slice 5 contract's
    /// Stale-shape path.
    StaleBasis,
    /// Reconciliation produced `InputStatus::Invalidated` per the
    /// Slice 5 contract — the bundle's basis is no longer
    /// admissible for any further inference.
    InvalidatedBasis,
    /// The NQ liveness gate did not clear; no findings were
    /// consulted on this run.
    LivenessGateFailed,
    /// Preflight coordination did not clear — protected-class
    /// service in scope, overlapping concurrent actor, etc. The run
    /// halted before reconcile.
    PreflightHeld,
    /// An operator ack or silence is in force per the Slice 3
    /// attention projection. Closing under active operator
    /// attention would launder operator intent.
    OperatorAttentionActive,
}

/// Closure-candidate verdict. Three variants; one of them
/// (`EligibleForClosureReview`) is intentionally unreachable in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClosureCandidate {
    /// The predicate refuses closure for a named reason.
    NotEligible { reason: NotEligibleReason },
    /// The finding is `IncidentShape` (or `Unknown`) and no blocker
    /// fired, but NQ's wire shape does not yet distinguish
    /// proxy-channel from consequence-channel findings. Closure
    /// cannot be assessed; refusal is conservative.
    ///
    /// Unblocks when NQ adds channel classification. See
    /// FEATURE-HISTORY § `SLICE_4_CLOSURE_CANDIDATE V1` for the
    /// deferred-trigger spec.
    UnassessableMissingChannelClassification,
    /// All known blockers are absent AND the finding's channel
    /// classification names a consequence-channel witness. **Not
    /// emitted in v1**: NQ has no channel-classification wire shape
    /// yet, so every IncidentShape finding falls through to
    /// `Unassessable`. The variant exists to keep the enum honest
    /// about the eventual approval path.
    EligibleForClosureReview,
}

/// Hold context for the closure predicate. `Reconciled` carries the
/// NQ `InputStatus` so the assessor can distinguish Stale /
/// Invalidated / committed cases. Held variants carry the gate that
/// halted the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Reconciled { nq_input_status: InputStatus },
    PreflightHeld,
    LivenessGateFailed,
}

/// Assess closure candidacy for a packet's evidence + attention +
/// outcome shape. Pure function; no I/O.
///
/// Priority order of refusals (first match wins):
/// 1. `PreflightHeld` — run never reached reconcile
/// 2. `LivenessGateFailed` — same, halted earlier
/// 3. `InvalidatedBasis` — Slice 5 contract Invalidated
/// 4. `StaleBasis` — Slice 5 Stale, Slice B freshness, etc.
/// 5. `OperatorAttentionActive` — Slice 3 ack/silence in force
/// 6. `ProxyQuiet` — Slice C.1 SilenceShape
/// 7. Otherwise: `UnassessableMissingChannelClassification`
///
/// Note that `EligibleForClosureReview` is never returned by this
/// function in v1. To reach it, NQ would need to expose channel
/// classification AND the finding would need to be marked as a
/// consequence-channel witness. Neither condition is satisfiable
/// today.
pub fn assess(
    posture_class: PostureClass,
    attention_state: AttentionState,
    evidence_state: EvidenceState,
    outcome: RunOutcome,
) -> ClosureCandidate {
    use ClosureCandidate::*;
    use NotEligibleReason::*;

    // 1–2: held runs. Build-of-packet site decides which by passing
    // the right outcome.
    match outcome {
        RunOutcome::PreflightHeld => {
            return NotEligible { reason: PreflightHeld };
        }
        RunOutcome::LivenessGateFailed => {
            return NotEligible {
                reason: LivenessGateFailed,
            };
        }
        RunOutcome::Reconciled { nq_input_status } => {
            // 3: Invalidated wins over Stale (it is the stronger
            // refusal — basis is no longer admissible at all).
            if nq_input_status == InputStatus::Invalidated {
                return NotEligible { reason: InvalidatedBasis };
            }
            // 4: Stale, either from Slice 5 input status or from the
            // packet's evidence state (Slice B routes
            // imported_producer_basis_stale → EvidenceState::Stale).
            if nq_input_status == InputStatus::Stale
                || evidence_state == EvidenceState::Stale
            {
                return NotEligible { reason: StaleBasis };
            }
        }
    }

    // 5: operator attention. Slice 3 sets attention_state to
    // Acknowledged or Silenced when the projection applied. Note:
    // WatchUntil is a horizon-driven state (system-set), not
    // operator-set, and does NOT count as operator attention for
    // closure purposes — the watch is for the system to reassess,
    // not an explicit operator hold.
    match attention_state {
        AttentionState::Acknowledged
        | AttentionState::Silenced
        | AttentionState::Investigating
        | AttentionState::HandedOff => {
            return NotEligible {
                reason: OperatorAttentionActive,
            };
        }
        AttentionState::Unowned | AttentionState::WatchUntil => {}
    }

    // 6: silence-shape posture (Slice C.1).
    if posture_class == PostureClass::SilenceShape {
        return NotEligible { reason: ProxyQuiet };
    }

    // 7: default — IncidentShape or Unknown without channel
    // classification. Conservative refusal, not silent eligibility.
    UnassessableMissingChannelClassification
}

impl ClosureCandidate {
    /// Operator-facing label for `runs show` rendering. Stable
    /// strings, scannable.
    pub fn render_label(&self) -> String {
        match self {
            ClosureCandidate::NotEligible { reason } => {
                format!("not_eligible({})", reason_str(*reason))
            }
            ClosureCandidate::UnassessableMissingChannelClassification => {
                "unassessable (missing channel classification)".into()
            }
            ClosureCandidate::EligibleForClosureReview => {
                "eligible_for_closure_review".into()
            }
        }
    }
}

fn reason_str(r: NotEligibleReason) -> &'static str {
    match r {
        NotEligibleReason::ProxyQuiet => "proxy_quiet",
        NotEligibleReason::StaleBasis => "stale_basis",
        NotEligibleReason::InvalidatedBasis => "invalidated_basis",
        NotEligibleReason::LivenessGateFailed => "liveness_gate_failed",
        NotEligibleReason::PreflightHeld => "preflight_held",
        NotEligibleReason::OperatorAttentionActive => "operator_attention_active",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reconciled(status: InputStatus) -> RunOutcome {
        RunOutcome::Reconciled {
            nq_input_status: status,
        }
    }

    #[test]
    fn silence_shape_refuses_with_proxy_quiet() {
        let v = assess(
            PostureClass::SilenceShape,
            AttentionState::Unowned,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::ProxyQuiet,
            }
        );
    }

    #[test]
    fn stale_input_refuses_with_stale_basis() {
        let v = assess(
            PostureClass::IncidentShape,
            AttentionState::Unowned,
            EvidenceState::Stale,
            reconciled(InputStatus::Stale),
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::StaleBasis,
            }
        );
    }

    #[test]
    fn invalidated_refuses_with_invalidated_basis_over_stale() {
        // Invalidated must win even if evidence_state is also Stale —
        // Invalidated is the stronger refusal.
        let v = assess(
            PostureClass::IncidentShape,
            AttentionState::Unowned,
            EvidenceState::Stale,
            reconciled(InputStatus::Invalidated),
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::InvalidatedBasis,
            }
        );
    }

    #[test]
    fn liveness_held_refuses_with_liveness_gate_failed() {
        let v = assess(
            PostureClass::Unknown,
            AttentionState::Unowned,
            EvidenceState::Stale,
            RunOutcome::LivenessGateFailed,
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::LivenessGateFailed,
            }
        );
    }

    #[test]
    fn preflight_held_refuses_with_preflight_held() {
        let v = assess(
            PostureClass::Unknown,
            AttentionState::Unowned,
            EvidenceState::Active,
            RunOutcome::PreflightHeld,
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::PreflightHeld,
            }
        );
    }

    #[test]
    fn operator_ack_refuses_with_operator_attention_active() {
        let v = assess(
            PostureClass::IncidentShape,
            AttentionState::Acknowledged,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::OperatorAttentionActive,
            }
        );
    }

    #[test]
    fn operator_silence_refuses_with_operator_attention_active() {
        let v = assess(
            PostureClass::SilenceShape, // would otherwise be ProxyQuiet
            AttentionState::Silenced,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        // Operator attention is checked before posture — operator
        // intent is a stronger signal than the finding's shape.
        assert_eq!(
            v,
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::OperatorAttentionActive,
            }
        );
    }

    #[test]
    fn watch_until_does_not_count_as_operator_attention() {
        // WatchUntil is horizon-driven, not operator-driven. It does
        // not refuse closure on operator-attention grounds. The
        // finding falls through to the Incident-shape default
        // (Unassessable) when nothing else blocks.
        let v = assess(
            PostureClass::IncidentShape,
            AttentionState::WatchUntil,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        assert_eq!(v, ClosureCandidate::UnassessableMissingChannelClassification);
    }

    #[test]
    fn incident_shape_committed_falls_through_to_unassessable() {
        // No blockers active + IncidentShape → unassessable.
        // Critical: NOT EligibleForClosureReview.
        let v = assess(
            PostureClass::IncidentShape,
            AttentionState::Unowned,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        assert_eq!(v, ClosureCandidate::UnassessableMissingChannelClassification);
        assert_ne!(v, ClosureCandidate::EligibleForClosureReview);
    }

    #[test]
    fn unknown_posture_committed_also_falls_through_to_unassessable() {
        let v = assess(
            PostureClass::Unknown,
            AttentionState::Unowned,
            EvidenceState::Active,
            reconciled(InputStatus::Committed),
        );
        assert_eq!(v, ClosureCandidate::UnassessableMissingChannelClassification);
    }

    /// Load-bearing invariant: NO combination of v1 inputs can
    /// produce `EligibleForClosureReview`. The variant is reserved
    /// for a future state where NQ exposes channel classification.
    /// If this test ever fails, the closure semantics moved without
    /// updating the FEATURE-HISTORY § SLICE_4 contract.
    #[test]
    fn no_case_emits_eligible_for_closure_review() {
        let postures = [
            PostureClass::IncidentShape,
            PostureClass::SilenceShape,
            PostureClass::Unknown,
        ];
        let attentions = [
            AttentionState::Unowned,
            AttentionState::Acknowledged,
            AttentionState::Investigating,
            AttentionState::HandedOff,
            AttentionState::WatchUntil,
            AttentionState::Silenced,
        ];
        let evidence = [
            EvidenceState::Active,
            EvidenceState::Worsening,
            EvidenceState::Resolving,
            EvidenceState::Recovered,
            EvidenceState::Stale,
        ];
        let outcomes = [
            RunOutcome::Reconciled { nq_input_status: InputStatus::Observed },
            RunOutcome::Reconciled { nq_input_status: InputStatus::Committed },
            RunOutcome::Reconciled { nq_input_status: InputStatus::Changed },
            RunOutcome::Reconciled { nq_input_status: InputStatus::Stale },
            RunOutcome::Reconciled { nq_input_status: InputStatus::Invalidated },
            RunOutcome::PreflightHeld,
            RunOutcome::LivenessGateFailed,
        ];
        for &p in &postures {
            for &a in &attentions {
                for &e in &evidence {
                    for &o in &outcomes {
                        let v = assess(p, a, e, o);
                        assert_ne!(
                            v,
                            ClosureCandidate::EligibleForClosureReview,
                            "EligibleForClosureReview leaked: posture={p:?} \
                             attention={a:?} evidence={e:?} outcome={o:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn render_labels_are_stable() {
        assert_eq!(
            ClosureCandidate::NotEligible {
                reason: NotEligibleReason::ProxyQuiet,
            }
            .render_label(),
            "not_eligible(proxy_quiet)"
        );
        assert_eq!(
            ClosureCandidate::UnassessableMissingChannelClassification.render_label(),
            "unassessable (missing channel classification)"
        );
    }
}
