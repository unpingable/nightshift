//! Horizon processing phase.
//!
//! Runs after the reconciler's `adjudicate`. For each admissible
//! NQ-backed input, consults `GovernorSource` for a gate receipt,
//! consults the store for any prior tolerance grant, and computes a
//! `HorizonAction` via `horizon::action_for`. The pipeline then
//! applies the decisions to the store (write on `Defer`; clear on
//! the escalate variants).
//!
//! Kept separate from `reconciler.rs` so that module's `adjudicate`
//! remains pure. Acquisition of receipts + prior tolerance lives
//! here because it's the same failure-mode family as the NQ
//! acquisition (one live read per input, fixed by a single `now`
//! timestamp), but it touches two different sources that the
//! pure adjudicator has no business knowing about.
//!
//! Phase A scope: only the four-way A5 distinction. Phase B will
//! extend this path with real Governor RPC (replacing
//! `FixtureGovernorSource`) and receipt-emission plumbing.

use chrono::{DateTime, Utc};

use crate::bundle::{CaptureInput, ReconciliationPhase, ReconciliationResult};
use crate::errors::Result;
use crate::finding::FindingKey;
use crate::governor::GovernorSource;
use crate::horizon::{action_for, HorizonAction};
use crate::store::{Store, ToleranceRecord};

/// One horizon decision for one finding in one run, after acquiring
/// the gate receipt and any prior tolerance grant.
#[derive(Debug, Clone)]
pub struct HorizonOutcome {
    pub finding_key: FindingKey,
    pub input_id: String,
    pub action: HorizonAction,
}

impl HorizonOutcome {
    /// True if the outcome requires the caller to persist a
    /// tolerance grant. Only `Defer` triggers a write.
    pub fn requires_tolerance_write(&self) -> bool {
        matches!(self.action, HorizonAction::Defer { .. })
    }

    /// True if the outcome requires the caller to clear any prior
    /// tolerance grant. Both escalate variants clear: expired
    /// tolerance is consumed; basis-invalidated tolerance is void.
    pub fn requires_tolerance_clear(&self) -> bool {
        matches!(
            self.action,
            HorizonAction::EscalateExpired { .. }
                | HorizonAction::EscalateBasisInvalidated { .. }
        )
    }
}

/// For a single reconciled result, compute the horizon outcome.
/// Pure over (input, result, receipt, prior, now) — the acquisition
/// of receipt + prior happens in the caller.
fn outcome_for_result(
    input: &CaptureInput,
    _result: &ReconciliationResult,
    governor: &dyn GovernorSource,
    store: &dyn Store,
    now: DateTime<Utc>,
) -> Result<Option<HorizonOutcome>> {
    if input.source != "nq" {
        return Ok(None);
    }
    let Some(finding_key) = FindingKey::from_nq_input_id(&input.input_id) else {
        return Ok(None);
    };
    let receipt = governor.fetch_gate_receipt(&finding_key)?;
    let block = receipt.as_ref().and_then(|r| r.horizon_block());
    let prior_record = store.load_tolerance(&finding_key)?;
    let prior = prior_record.as_ref().map(|r| r.to_prior_tolerance());
    let action = action_for(block.as_ref(), now, prior.as_ref());
    Ok(Some(HorizonOutcome {
        finding_key,
        input_id: input.input_id.clone(),
        action,
    }))
}

/// Process the horizon phase for a reconciled run. Iterates the
/// adjudicated results, pulls each NQ-backed input's gate receipt
/// and prior tolerance, and produces a `HorizonOutcome` per input.
///
/// Non-NQ inputs and inputs whose finding_key cannot be parsed are
/// skipped (no outcome emitted). The `now` timestamp should be the
/// same one used in adjudicate — typically
/// `acquisition.acquired_at` — so the horizon evaluation is
/// deterministic relative to the reconcile's reference clock.
pub fn process_horizon(
    phase: &ReconciliationPhase,
    capture_inputs: &[CaptureInput],
    governor: &dyn GovernorSource,
    store: &dyn Store,
    now: DateTime<Utc>,
) -> Result<Vec<HorizonOutcome>> {
    let input_by_id: std::collections::HashMap<&str, &CaptureInput> = capture_inputs
        .iter()
        .map(|i| (i.input_id.as_str(), i))
        .collect();
    let mut outcomes = Vec::new();
    for result in &phase.results {
        let Some(input) = input_by_id.get(result.input_id.as_str()) else {
            continue;
        };
        if let Some(outcome) = outcome_for_result(input, result, governor, store, now)? {
            outcomes.push(outcome);
        }
    }
    Ok(outcomes)
}

/// Apply horizon outcomes to the store.
///
/// `Defer` → write a `ToleranceRecord` (upsert) so the next run
/// sees it as `PriorTolerance`. All other outcomes that involve a
/// prior record (the two escalate variants) → clear the record so
/// a later run doesn't re-consume the stale grant. `ActOnVerdict`
/// and the render-only variants → no-op.
///
/// `run_id` is recorded on new tolerance records as the grant's
/// provenance. `granted_at` is recorded as-is (the caller should
/// pass wall-clock now or the same reconcile reference time).
pub fn apply_horizon_outcomes(
    outcomes: &[HorizonOutcome],
    store: &dyn Store,
    run_id: &str,
    granted_at: DateTime<Utc>,
) -> Result<()> {
    for outcome in outcomes {
        match &outcome.action {
            HorizonAction::Defer {
                until,
                basis_id,
                basis_hash,
                class,
            } => {
                let record = ToleranceRecord {
                    finding_key: outcome.finding_key.clone(),
                    basis_id: basis_id.clone(),
                    basis_hash: basis_hash.clone(),
                    prior_class: *class,
                    expires_at: *until,
                    granted_at,
                    granted_in_run_id: run_id.into(),
                };
                store.save_tolerance(&record)?;
            }
            HorizonAction::EscalateExpired { .. }
            | HorizonAction::EscalateBasisInvalidated { .. } => {
                store.clear_tolerance(&outcome.finding_key)?;
            }
            HorizonAction::ActOnVerdict { .. }
            | HorizonAction::RenderNoIntervene { .. }
            | HorizonAction::RenderHolding { .. } => {
                // No-op: no prior record to create or clear on these
                // paths. If a prior record exists and the current
                // receipt is Missing/None/Now/ObserveOnly/Indefinite,
                // it means the producer stopped declaring a timed
                // tolerance — Phase A leaves such records in place
                // (they'll expire naturally or be cleared on the
                // next basis-mismatched timed receipt). Phase B can
                // tighten this if needed.
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{
        Freshness, InputStatus, InvalidationRule, RelianceClass, RelianceScope,
        ReconciliationSummary, ValidFor,
    };
    use crate::finding::{EvidenceState, FindingSnapshot, Severity};
    use crate::governor::{FixtureGovernorSource, GateReceipt, WireHorizon};
    use crate::horizon::HorizonClass;
    use crate::store::sqlite::SqliteStore;
    use chrono::TimeZone;

    fn fk(detector: &str, subject: &str) -> FindingKey {
        FindingKey {
            source: "nq".into(),
            detector: detector.into(),
            subject: subject.into(),
        }
    }

    fn input_for(key: &FindingKey, hash: &str) -> CaptureInput {
        CaptureInput {
            input_id: format!("nq:finding:{}:{}", key.detector, key.subject),
            source: "nq".into(),
            kind: "nq_finding_snapshot".into(),
            status: InputStatus::Observed,
            evidence_hash: hash.into(),
            freshness: Freshness {
                captured_at: Utc.with_ymd_and_hms(2026, 4, 23, 11, 0, 0).unwrap(),
                expires_at: None,
                invalidates_if: vec![InvalidationRule::FindingAbsentForNGenerations { n: 1 }],
            },
            payload_ref: "ledger://...".into(),
            admissible_for: vec![],
            inadmissible_for: vec![],
            captured_finding_snapshot: Some(FindingSnapshot {
                finding_key: key.clone(),
                host: "labelwatch-host".into(),
                severity: Severity::Warning,
                domain: None,
                persistence_generations: 3,
                first_seen_at: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
                current_status: EvidenceState::Active,
                snapshot_generation: 100,
                captured_at: Utc.with_ymd_and_hms(2026, 4, 23, 11, 0, 0).unwrap(),
                evidence_hash: hash.into(),
            }),
        }
    }

    fn result_for(input_id: &str) -> ReconciliationResult {
        ReconciliationResult {
            input_id: input_id.into(),
            status: InputStatus::Committed,
            reliance_class: RelianceClass::Authoritative,
            scope: RelianceScope {
                run_id: "run_test".into(),
                valid_for: vec![ValidFor::Authorization, ValidFor::Diagnosis],
            },
            previous_evidence_hash: None,
            current_evidence_hash: None,
            notes: None,
            concurrent_activity: None,
            current_finding_snapshot: None,
        }
    }

    fn phase_with(results: Vec<ReconciliationResult>, at: DateTime<Utc>) -> ReconciliationPhase {
        ReconciliationPhase {
            reconciled_at: at,
            reconciled_by: "test".into(),
            results,
            summary: ReconciliationSummary {
                ok_to_proceed: true,
                ..Default::default()
            },
        }
    }

    fn receipt_for(
        key: &FindingKey,
        class: HorizonClass,
        basis_id: &str,
        basis_hash: &str,
        expiry: Option<DateTime<Utc>>,
    ) -> GateReceipt {
        GateReceipt {
            receipt_id: format!("r_{basis_id}"),
            verdict: "allow".into(),
            finding_key: key.clone(),
            horizon: Some(WireHorizon {
                class,
                basis_id: Some(basis_id.into()),
                basis_hash: Some(basis_hash.into()),
                expiry,
            }),
            extras: serde_json::Value::Null,
        }
    }

    // --- outcome-of-one-finding tests ---

    #[test]
    fn no_receipt_produces_act_on_verdict_missing() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let input = input_for(&key, "hash-1");
        let result = result_for(&input.input_id);
        let phase = phase_with(vec![result], Utc::now());
        let governor = FixtureGovernorSource::from_receipts(vec![]);
        let outcomes =
            process_horizon(&phase, &[input], &governor, &store, Utc::now()).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(matches!(
            outcomes[0].action,
            HorizonAction::ActOnVerdict { .. }
        ));
    }

    #[test]
    fn hours_horizon_future_expiry_defers() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let input = input_for(&key, "hash-1");
        let result = result_for(&input.input_id);
        let now = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
        let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
        let phase = phase_with(vec![result], now);
        let governor = FixtureGovernorSource::from_receipts(vec![receipt_for(
            &key,
            HorizonClass::Hours,
            "basis-abc",
            "hash-abc",
            Some(expiry),
        )]);
        let outcomes = process_horizon(&phase, &[input], &governor, &store, now).unwrap();
        assert!(matches!(outcomes[0].action, HorizonAction::Defer { .. }));
    }

    #[test]
    fn non_nq_inputs_are_skipped() {
        let store = SqliteStore::open_in_memory().unwrap();
        let mut input = input_for(&fk("wal_bloat", "host:/db"), "hash-1");
        input.source = "continuity".into();
        let result = result_for(&input.input_id);
        let phase = phase_with(vec![result], Utc::now());
        let governor = FixtureGovernorSource::from_receipts(vec![]);
        let outcomes =
            process_horizon(&phase, &[input], &governor, &store, Utc::now()).unwrap();
        assert!(outcomes.is_empty(), "non-nq inputs must not produce outcomes");
    }

    // --- apply-outcomes tests ---

    #[test]
    fn apply_defer_writes_tolerance_record() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::Defer {
                until: expiry,
                basis_id: "basis-abc".into(),
                basis_hash: "hash-abc".into(),
                class: HorizonClass::Hours,
            },
        };
        apply_horizon_outcomes(&[outcome], &store, "run_a", now).unwrap();
        let record = store.load_tolerance(&key).unwrap().unwrap();
        assert_eq!(record.basis_id, "basis-abc");
        assert_eq!(record.basis_hash, "hash-abc");
        assert_eq!(record.expires_at, expiry);
        assert_eq!(record.granted_in_run_id, "run_a");
    }

    #[test]
    fn apply_escalate_expired_clears_tolerance() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        // Pre-existing tolerance record
        let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
        store
            .save_tolerance(&ToleranceRecord {
                finding_key: key.clone(),
                basis_id: "basis-abc".into(),
                basis_hash: "hash-abc".into(),
                prior_class: HorizonClass::Hours,
                expires_at: expiry,
                granted_at: Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap(),
                granted_in_run_id: "run_a".into(),
            })
            .unwrap();
        assert!(store.load_tolerance(&key).unwrap().is_some());

        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::EscalateExpired {
                prior: crate::horizon::PriorTolerance {
                    basis_id: "basis-abc".into(),
                    basis_hash: "hash-abc".into(),
                    prior_class: HorizonClass::Hours,
                    expired_at: expiry,
                },
            },
        };
        apply_horizon_outcomes(
            &[outcome],
            &store,
            "run_b",
            Utc.with_ymd_and_hms(2026, 4, 23, 17, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(
            store.load_tolerance(&key).unwrap().is_none(),
            "EscalateExpired must clear prior tolerance"
        );
    }

    #[test]
    fn apply_escalate_basis_invalidated_clears_tolerance() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
        store
            .save_tolerance(&ToleranceRecord {
                finding_key: key.clone(),
                basis_id: "basis-old".into(),
                basis_hash: "hash-old".into(),
                prior_class: HorizonClass::Hours,
                expires_at: expiry,
                granted_at: Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap(),
                granted_in_run_id: "run_a".into(),
            })
            .unwrap();

        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::EscalateBasisInvalidated {
                prior: crate::horizon::PriorTolerance {
                    basis_id: "basis-old".into(),
                    basis_hash: "hash-old".into(),
                    prior_class: HorizonClass::Hours,
                    expired_at: expiry,
                },
                current_basis_hash: "hash-new".into(),
            },
        };
        apply_horizon_outcomes(&[outcome], &store, "run_b", Utc::now()).unwrap();
        assert!(
            store.load_tolerance(&key).unwrap().is_none(),
            "EscalateBasisInvalidated must clear prior tolerance"
        );
    }

    #[test]
    fn apply_act_on_verdict_is_noop() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::ActOnVerdict {
                reason: crate::horizon::ActReason::Missing,
            },
        };
        apply_horizon_outcomes(&[outcome], &store, "run_x", Utc::now()).unwrap();
        assert!(store.load_tolerance(&key).unwrap().is_none());
    }

    #[test]
    fn requires_tolerance_write_matches_defer_only() {
        let defer = HorizonOutcome {
            finding_key: fk("d", "s"),
            input_id: "x".into(),
            action: HorizonAction::Defer {
                until: Utc::now(),
                basis_id: "b".into(),
                basis_hash: "h".into(),
                class: HorizonClass::Hours,
            },
        };
        assert!(defer.requires_tolerance_write());
        assert!(!defer.requires_tolerance_clear());
    }

    #[test]
    fn requires_tolerance_clear_matches_both_escalate_variants() {
        let prior = crate::horizon::PriorTolerance {
            basis_id: "b".into(),
            basis_hash: "h".into(),
            prior_class: HorizonClass::Hours,
            expired_at: Utc::now(),
        };
        let expired = HorizonOutcome {
            finding_key: fk("d", "s"),
            input_id: "x".into(),
            action: HorizonAction::EscalateExpired {
                prior: prior.clone(),
            },
        };
        let invalidated = HorizonOutcome {
            finding_key: fk("d", "s"),
            input_id: "x".into(),
            action: HorizonAction::EscalateBasisInvalidated {
                prior,
                current_basis_hash: "new".into(),
            },
        };
        assert!(expired.requires_tolerance_clear());
        assert!(invalidated.requires_tolerance_clear());
        assert!(!expired.requires_tolerance_write());
        assert!(!invalidated.requires_tolerance_write());
    }
}
