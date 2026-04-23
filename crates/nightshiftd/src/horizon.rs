//! Tolerability horizon — consumer-side decision logic.
//!
//! Governor's `GOV_GAP_TOLERABILITY_HORIZON_001` defines an optional
//! `horizon` block on gate receipts (and, forward-looking, on NQ
//! findings). The block names a tolerance window: how long a real
//! adverse condition is acceptable to leave alone under declared
//! basis. The rule that makes it load-bearing:
//!
//! > Horizon is orthogonal to verdict. A block with `horizon=hours`
//! > still blocks. Horizon declares consumer routing; verdict
//! > declares gate decision.
//!
//! This module is the **pure logic** Night Shift applies when
//! reading a horizon block. It does not parse receipts, does not
//! call Governor, does not write attention state. Those pieces ship
//! with the Governor adapter (filed 2026-04-16, activated by
//! horizon's arrival). This file is bounded prep.
//!
//! ## The four-way distinction
//!
//! Chatty's probe on 2026-04-23 named the expiry-lineage failure
//! mode: if run A defers a finding at t0 under `horizon=hours`
//! with `expiry=t0+4h`, and run B sees the same finding at t0+5h,
//! run B must be able to tell "previously tolerated, now expired"
//! apart from "brand new incident." Otherwise the collapse stack
//! (badness → urgency → intervention → escalation) fires exactly
//! where horizon was meant to prevent it.
//!
//! `action_for` takes an optional `PriorTolerance` so callers can
//! surface lineage when the expiry transition fires. The caller is
//! responsible for having written that prior record on the earlier
//! deferral — the consumer write obligation named as the Nightshift
//! delta in the 2026-04-23 dogfood pass.
//!
//! ## Consumer-side fail-closed
//!
//! Governor's spec: "undeclared horizon on an adverse finding →
//! consumer policy treats as 'now' unless policy explicitly permits
//! fail-open." Night Shift's default is fail-closed. A malformed
//! block (basis missing on non-`none`; expiry missing on a timed
//! class) is treated as malformed-therefore-fail-closed, not as a
//! tolerance declaration. Per spec: **missing ≠ tolerable**.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The frozen v1 horizon class enum (Governor spec).
///
/// Producer-side semantics are Governor's; this enum mirrors the
/// wire values. `rename_all = "snake_case"` keeps the on-wire and
/// on-disk forms identical so a single enum serves both the
/// GovernorSource wire DTO and Store persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizonClass {
    /// Producer declared "no horizon semantics apply here." Not
    /// tolerable, not urgent — just not a tolerance question.
    /// Consumer acts on verdict as usual.
    None,
    /// Adverse, no tolerance window. Consumer acts on verdict
    /// immediately.
    Now,
    /// Tolerable for a named expiry on the order of hours.
    Hours,
    /// Tolerable until the next business-hours window. Expiry on
    /// the block carries the wall-clock resolution.
    BusinessHours,
    /// Tolerable until an explicit scheduled time (e.g. a
    /// maintenance window).
    Scheduled,
    /// Adverse, do not intervene. Render for awareness, keep
    /// authority at observe regardless of verdict.
    ObserveOnly,
    /// Tolerable indefinitely under current policy. Render, hold
    /// at observe, no alert unless policy changes.
    Indefinite,
}

impl HorizonClass {
    /// Classes whose declaration requires `expiry`.
    pub fn requires_expiry(self) -> bool {
        matches!(self, Self::Hours | Self::BusinessHours | Self::Scheduled)
    }

    /// Classes that require `basis_id + basis_hash`. Per spec:
    /// "non-'none' horizon without basis_id + basis_hash is a
    /// schema violation."
    pub fn requires_basis(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// The horizon block as Night Shift consumes it. Field names
/// mirror Governor's spec for wire compatibility (when Night Shift
/// later forwards this to Governor via `record_receipt`), but the
/// **origin is producer-local**, not a Governor read path. See
/// `horizon_policy.rs` and the module header of `reconcile_horizon`
/// for the invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HorizonBlock {
    /// On-wire as `"kind"` (Governor's contract field name, per
    /// `agent_gov/src/governor/gate_receipt.py:HorizonBlock`).
    /// Rust field stays `class` for internal ergonomics.
    #[serde(rename = "kind")]
    pub class: HorizonClass,
    /// Required for `class != None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_id: Option<String>,
    /// Required for `class != None`. Content hash of the basis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_hash: Option<String>,
    /// Required for `class in {Hours, BusinessHours, Scheduled}`.
    /// Wall-clock expiry. `now >= expiry` triggers escalation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<DateTime<Utc>>,
}

/// Prior tolerance record carried forward from an earlier run.
///
/// The consumer is responsible for writing this when `action_for`
/// returns `Defer`. Without it, a later run facing expired
/// tolerance cannot distinguish "previously tolerated, now
/// expired" from "brand new incident" — the collapse stack fires.
///
/// `basis_hash` is recorded so later runs can detect
/// basis-invalidation: the live artifact's basis hash no longer
/// matches what we tolerated under, so the tolerance grant is
/// void even if expiry hasn't arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriorTolerance {
    pub basis_id: String,
    pub basis_hash: String,
    pub prior_class: HorizonClass,
    pub expired_at: DateTime<Utc>,
}

/// The decision the consumer should act on after reading a horizon
/// block. Orthogonal to verdict: the verdict still says
/// block/allow; this says act-now / defer / render-no-intervene /
/// render-holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HorizonAction {
    /// Proceed on the verdict as if no horizon applied. Carries
    /// the reason for diagnostic surfacing.
    ActOnVerdict { reason: ActReason },
    /// Timed class with future expiry. Consumer must defer AND
    /// write an attention-state record so the next run can detect
    /// the expiry transition with lineage. `basis_hash` is included
    /// so the consumer has the full tolerance-record payload to
    /// persist without re-reading the block.
    Defer {
        until: DateTime<Utc>,
        basis_id: String,
        basis_hash: String,
        class: HorizonClass,
    },
    /// Timed class with past expiry AND a prior tolerance record.
    /// Escalate, but surface lineage — operator sees "was
    /// tolerated, now expired," not "brand new."
    EscalateExpired { prior: PriorTolerance },
    /// Timed class where the current block's `basis_hash` diverges
    /// from the prior record's. The tolerance grant is void
    /// regardless of expiry — the justification has changed.
    /// Surface both sides so the operator sees what was tolerated
    /// and what the live artifact now claims.
    EscalateBasisInvalidated {
        prior: PriorTolerance,
        current_basis_hash: String,
    },
    /// `class=ObserveOnly`. Render for awareness; keep authority
    /// at observe even if verdict would authorize action.
    RenderNoIntervene { basis_id: String },
    /// `class=Indefinite`. Render; hold at observe; no alert
    /// unless policy changes.
    RenderHolding { basis_id: String },
}

/// Why `ActOnVerdict` fired. Kept distinct so packet diagnostics
/// can surface WHY horizon wasn't honored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActReason {
    /// Producer did not declare a horizon block. Fail-closed to
    /// `now`. Missing ≠ tolerable (spec).
    Missing,
    /// Producer declared `class=None`.
    NoneDeclared,
    /// Producer declared `class=Now`.
    NowDeclared,
    /// Block present but malformed: basis missing on a non-`none`
    /// class, or expiry missing on a timed class. Fail-closed.
    Malformed,
    /// Timed class with expiry past AND no prior tolerance
    /// record. Cannot surface lineage; escalate as fresh. This is
    /// the symptom the write obligation prevents.
    ExpiredWithoutPrior,
}

impl HorizonAction {
    pub fn explain(&self) -> String {
        match self {
            HorizonAction::ActOnVerdict { reason } => format!(
                "horizon: act on verdict ({})",
                match reason {
                    ActReason::Missing => "no block declared; fail-closed to now",
                    ActReason::NoneDeclared => "class=none declared",
                    ActReason::NowDeclared => "class=now declared",
                    ActReason::Malformed => "malformed block; fail-closed to now",
                    ActReason::ExpiredWithoutPrior =>
                        "timed horizon expired with no prior tolerance record",
                }
            ),
            HorizonAction::Defer {
                until,
                basis_id,
                basis_hash,
                class,
            } => format!(
                "horizon: defer under {class:?} until {until} (basis {basis_id} hash {basis_hash}); write tolerance record"
            ),
            HorizonAction::EscalateExpired { prior } => format!(
                "horizon: escalate — tolerated under basis {} ({:?}) until {}, now expired",
                prior.basis_id, prior.prior_class, prior.expired_at
            ),
            HorizonAction::EscalateBasisInvalidated {
                prior,
                current_basis_hash,
            } => format!(
                "horizon: escalate — tolerance invalidated; was under basis {} (hash {}), live artifact now declares hash {}",
                prior.basis_id, prior.basis_hash, current_basis_hash
            ),
            HorizonAction::RenderNoIntervene { basis_id } => format!(
                "horizon: observe_only — render, do not intervene (basis {basis_id})"
            ),
            HorizonAction::RenderHolding { basis_id } => format!(
                "horizon: indefinite — render, no alert unless policy changes (basis {basis_id})"
            ),
        }
    }
}

/// Decide the consumer action for a horizon block.
///
/// Pure function: `now` and `prior` are injected so tests can
/// exercise every branch without a wall-clock read or a store
/// call. This is the four-way distinction (fresh / live-tolerated
/// / expired-with-lineage / expired-without-lineage) plus the two
/// terminal observe-states (`ObserveOnly`, `Indefinite`), plus the
/// trivial passthroughs (`None`, `Now`, missing, malformed).
pub fn action_for(
    block: Option<&HorizonBlock>,
    now: DateTime<Utc>,
    prior: Option<&PriorTolerance>,
) -> HorizonAction {
    let Some(block) = block else {
        return HorizonAction::ActOnVerdict {
            reason: ActReason::Missing,
        };
    };

    if block.class.requires_basis() && (block.basis_id.is_none() || block.basis_hash.is_none()) {
        return HorizonAction::ActOnVerdict {
            reason: ActReason::Malformed,
        };
    }

    match block.class {
        HorizonClass::None => HorizonAction::ActOnVerdict {
            reason: ActReason::NoneDeclared,
        },
        HorizonClass::Now => HorizonAction::ActOnVerdict {
            reason: ActReason::NowDeclared,
        },
        HorizonClass::ObserveOnly => HorizonAction::RenderNoIntervene {
            basis_id: block.basis_id.clone().expect("basis checked above"),
        },
        HorizonClass::Indefinite => HorizonAction::RenderHolding {
            basis_id: block.basis_id.clone().expect("basis checked above"),
        },
        HorizonClass::Hours | HorizonClass::BusinessHours | HorizonClass::Scheduled => {
            let Some(expiry) = block.expiry else {
                return HorizonAction::ActOnVerdict {
                    reason: ActReason::Malformed,
                };
            };
            let block_basis_hash = block
                .basis_hash
                .as_deref()
                .expect("basis checked above");
            // Basis invalidation beats expiry: a changed basis voids
            // the grant regardless of whether expiry has arrived.
            if let Some(p) = prior {
                if p.basis_hash != block_basis_hash {
                    return HorizonAction::EscalateBasisInvalidated {
                        prior: p.clone(),
                        current_basis_hash: block_basis_hash.to_string(),
                    };
                }
            }
            if now < expiry {
                HorizonAction::Defer {
                    until: expiry,
                    basis_id: block.basis_id.clone().expect("basis checked above"),
                    basis_hash: block_basis_hash.to_string(),
                    class: block.class,
                }
            } else {
                match prior {
                    Some(p) => HorizonAction::EscalateExpired { prior: p.clone() },
                    None => HorizonAction::ActOnVerdict {
                        reason: ActReason::ExpiredWithoutPrior,
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<Utc> {
        s.parse().expect("valid RFC3339 timestamp")
    }

    fn basis_block(class: HorizonClass, expiry: Option<DateTime<Utc>>) -> HorizonBlock {
        HorizonBlock {
            class,
            basis_id: Some("basis-abc".into()),
            basis_hash: Some("hash-123".into()),
            expiry,
        }
    }

    #[test]
    fn missing_block_fails_closed_to_act() {
        let now = ts("2026-04-23T12:00:00Z");
        let action = action_for(None, now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::Missing
            }
        );
    }

    #[test]
    fn class_none_acts_on_verdict() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = HorizonBlock {
            class: HorizonClass::None,
            basis_id: None,
            basis_hash: None,
            expiry: None,
        };
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::NoneDeclared
            }
        );
    }

    #[test]
    fn class_now_acts_on_verdict_with_basis() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = basis_block(HorizonClass::Now, None);
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::NowDeclared
            }
        );
    }

    #[test]
    fn class_now_without_basis_is_malformed() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = HorizonBlock {
            class: HorizonClass::Now,
            basis_id: None,
            basis_hash: None,
            expiry: None,
        };
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::Malformed
            }
        );
    }

    #[test]
    fn timed_class_with_future_expiry_defers() {
        let now = ts("2026-04-23T12:00:00Z");
        let expiry = ts("2026-04-23T16:00:00Z");
        let block = basis_block(HorizonClass::Hours, Some(expiry));
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::Defer {
                until: expiry,
                basis_id: "basis-abc".into(),
                basis_hash: "hash-123".into(),
                class: HorizonClass::Hours,
            }
        );
    }

    #[test]
    fn timed_class_without_expiry_is_malformed() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = basis_block(HorizonClass::BusinessHours, None);
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::Malformed
            }
        );
    }

    #[test]
    fn scheduled_class_honors_expiry() {
        let now = ts("2026-04-23T12:00:00Z");
        let expiry = ts("2026-04-25T03:00:00Z");
        let block = basis_block(HorizonClass::Scheduled, Some(expiry));
        let action = action_for(Some(&block), now, None);
        match action {
            HorizonAction::Defer { class, .. } => {
                assert_eq!(class, HorizonClass::Scheduled);
            }
            other => panic!("expected Defer, got {other:?}"),
        }
    }

    #[test]
    fn expiry_boundary_at_equal_escalates_not_defers() {
        // now == expiry — treat as past (fail-closed). `now < expiry`
        // is the defer condition; equality escalates.
        let t = ts("2026-04-23T16:00:00Z");
        let block = basis_block(HorizonClass::Hours, Some(t));
        let action = action_for(Some(&block), t, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::ExpiredWithoutPrior
            }
        );
    }

    #[test]
    fn observe_only_renders_without_intervening() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = basis_block(HorizonClass::ObserveOnly, None);
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::RenderNoIntervene {
                basis_id: "basis-abc".into()
            }
        );
    }

    #[test]
    fn observe_only_without_basis_is_malformed() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = HorizonBlock {
            class: HorizonClass::ObserveOnly,
            basis_id: None,
            basis_hash: None,
            expiry: None,
        };
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::ActOnVerdict {
                reason: ActReason::Malformed
            }
        );
    }

    #[test]
    fn indefinite_renders_holding() {
        let now = ts("2026-04-23T12:00:00Z");
        let block = basis_block(HorizonClass::Indefinite, None);
        let action = action_for(Some(&block), now, None);
        assert_eq!(
            action,
            HorizonAction::RenderHolding {
                basis_id: "basis-abc".into()
            }
        );
    }

    /// **Load-bearing: the four-way distinction end-to-end.**
    ///
    /// Run A at t0 defers under horizon=hours with expiry=t0+4h.
    /// Consumer is responsible for writing a PriorTolerance record
    /// (simulated here). Run B at t0+5h sees the same block (now
    /// expired). With the prior record, B escalates with lineage.
    /// Without it, B escalates as fresh — the symptom the write
    /// obligation prevents.
    #[test]
    fn expiry_transition_preserves_lineage_across_runs() {
        let t0 = ts("2026-04-23T12:00:00Z");
        let expiry = ts("2026-04-23T16:00:00Z");
        let t5 = ts("2026-04-23T17:00:00Z");
        let block = basis_block(HorizonClass::Hours, Some(expiry));

        // Run A: defer. Consumer must now write a tolerance record.
        let action_a = action_for(Some(&block), t0, None);
        match action_a {
            HorizonAction::Defer {
                until,
                basis_id,
                basis_hash,
                class,
            } => {
                assert_eq!(until, expiry);
                assert_eq!(basis_id, "basis-abc");
                assert_eq!(basis_hash, "hash-123");
                assert_eq!(class, HorizonClass::Hours);
            }
            other => panic!("run A expected Defer, got {other:?}"),
        }

        // Simulate the consumer's tolerance-record write.
        let prior = PriorTolerance {
            basis_id: "basis-abc".into(),
            basis_hash: "hash-123".into(),
            prior_class: HorizonClass::Hours,
            expired_at: expiry,
        };

        // Run B with lineage: escalate, surface prior record.
        let action_b = action_for(Some(&block), t5, Some(&prior));
        assert_eq!(
            action_b,
            HorizonAction::EscalateExpired {
                prior: prior.clone()
            }
        );

        // Run B without lineage (consumer forgot to write): escalate
        // as fresh. The reason distinguishes this from a truly new
        // finding (ExpiredWithoutPrior vs Missing) so diagnostics can
        // still flag the write-obligation failure.
        let action_b_no_lineage = action_for(Some(&block), t5, None);
        assert_eq!(
            action_b_no_lineage,
            HorizonAction::ActOnVerdict {
                reason: ActReason::ExpiredWithoutPrior
            }
        );
    }

    #[test]
    fn explain_surfaces_reason_for_every_act_variant() {
        for reason in [
            ActReason::Missing,
            ActReason::NoneDeclared,
            ActReason::NowDeclared,
            ActReason::Malformed,
            ActReason::ExpiredWithoutPrior,
        ] {
            let action = HorizonAction::ActOnVerdict { reason };
            let s = action.explain();
            assert!(s.starts_with("horizon: act on verdict"), "got: {s}");
        }
    }

    #[test]
    fn explain_surfaces_lineage_for_escalate_expired() {
        let prior = PriorTolerance {
            basis_id: "basis-xyz".into(),
            basis_hash: "hash-xyz".into(),
            prior_class: HorizonClass::Scheduled,
            expired_at: ts("2026-04-23T16:00:00Z"),
        };
        let action = HorizonAction::EscalateExpired { prior };
        let s = action.explain();
        assert!(s.contains("basis-xyz"), "got: {s}");
        assert!(s.contains("Scheduled"), "got: {s}");
        assert!(s.contains("expired"), "got: {s}");
    }

    /// Fourth A5 case: basis divergence voids the tolerance grant
    /// regardless of whether expiry has arrived. Run A tolerated
    /// under basis hash X; run B sees same finding with a block
    /// declaring hash Y — the justification has changed, tolerance
    /// is invalid, escalate with both sides surfaced so the
    /// operator sees what changed.
    #[test]
    fn basis_invalidated_with_future_expiry_escalates_not_defers() {
        let t0 = ts("2026-04-23T12:00:00Z");
        let future_expiry = ts("2026-04-23T20:00:00Z");
        let block = HorizonBlock {
            class: HorizonClass::Hours,
            basis_id: Some("basis-new".into()),
            basis_hash: Some("hash-new".into()),
            expiry: Some(future_expiry),
        };
        let prior = PriorTolerance {
            basis_id: "basis-old".into(),
            basis_hash: "hash-old".into(),
            prior_class: HorizonClass::Hours,
            expired_at: future_expiry,
        };
        let action = action_for(Some(&block), t0, Some(&prior));
        match action {
            HorizonAction::EscalateBasisInvalidated {
                prior: p,
                current_basis_hash,
            } => {
                assert_eq!(p.basis_hash, "hash-old");
                assert_eq!(current_basis_hash, "hash-new");
            }
            other => panic!("expected EscalateBasisInvalidated, got {other:?}"),
        }
    }

    /// Basis invalidation beats expiry: if basis hash has diverged,
    /// we escalate as invalidated (not as expired) even when the
    /// expiry is also past. The operator needs to see that the
    /// tolerance was revoked by a basis change, not merely lapsed.
    #[test]
    fn basis_invalidated_beats_expiry_when_both_conditions_hold() {
        let past_expiry = ts("2026-04-23T10:00:00Z");
        let now = ts("2026-04-23T12:00:00Z");
        let block = HorizonBlock {
            class: HorizonClass::BusinessHours,
            basis_id: Some("basis-new".into()),
            basis_hash: Some("hash-new".into()),
            expiry: Some(past_expiry),
        };
        let prior = PriorTolerance {
            basis_id: "basis-old".into(),
            basis_hash: "hash-old".into(),
            prior_class: HorizonClass::BusinessHours,
            expired_at: past_expiry,
        };
        let action = action_for(Some(&block), now, Some(&prior));
        assert!(
            matches!(action, HorizonAction::EscalateBasisInvalidated { .. }),
            "expected basis-invalidated to win over expired, got {action:?}"
        );
    }

    /// Regression guard: matching basis hash preserves the normal
    /// Defer path on future expiry. Basis-invalidation check must
    /// not fire when hashes agree.
    #[test]
    fn matching_basis_hash_preserves_defer_on_future_expiry() {
        let t0 = ts("2026-04-23T12:00:00Z");
        let future_expiry = ts("2026-04-23T20:00:00Z");
        let block = basis_block(HorizonClass::Hours, Some(future_expiry));
        let prior = PriorTolerance {
            basis_id: "basis-abc".into(),
            basis_hash: "hash-123".into(),
            prior_class: HorizonClass::Hours,
            expired_at: future_expiry,
        };
        let action = action_for(Some(&block), t0, Some(&prior));
        match action {
            HorizonAction::Defer {
                until,
                basis_id,
                basis_hash,
                class,
            } => {
                assert_eq!(until, future_expiry);
                assert_eq!(basis_id, "basis-abc");
                assert_eq!(basis_hash, "hash-123");
                assert_eq!(class, HorizonClass::Hours);
            }
            other => panic!("expected Defer, got {other:?}"),
        }
    }

    #[test]
    fn explain_surfaces_both_sides_for_basis_invalidated() {
        let prior = PriorTolerance {
            basis_id: "basis-old".into(),
            basis_hash: "hash-old".into(),
            prior_class: HorizonClass::Hours,
            expired_at: ts("2026-04-23T16:00:00Z"),
        };
        let action = HorizonAction::EscalateBasisInvalidated {
            prior,
            current_basis_hash: "hash-new".into(),
        };
        let s = action.explain();
        assert!(s.contains("basis-old"), "got: {s}");
        assert!(s.contains("hash-old"), "got: {s}");
        assert!(s.contains("hash-new"), "got: {s}");
        assert!(s.contains("invalidated"), "got: {s}");
    }

    #[test]
    fn requires_basis_matches_spec() {
        assert!(!HorizonClass::None.requires_basis());
        assert!(HorizonClass::Now.requires_basis());
        assert!(HorizonClass::Hours.requires_basis());
        assert!(HorizonClass::BusinessHours.requires_basis());
        assert!(HorizonClass::Scheduled.requires_basis());
        assert!(HorizonClass::ObserveOnly.requires_basis());
        assert!(HorizonClass::Indefinite.requires_basis());
    }

    #[test]
    fn requires_expiry_matches_spec() {
        assert!(!HorizonClass::None.requires_expiry());
        assert!(!HorizonClass::Now.requires_expiry());
        assert!(HorizonClass::Hours.requires_expiry());
        assert!(HorizonClass::BusinessHours.requires_expiry());
        assert!(HorizonClass::Scheduled.requires_expiry());
        assert!(!HorizonClass::ObserveOnly.requires_expiry());
        assert!(!HorizonClass::Indefinite.requires_expiry());
    }
}
