//! Horizon processing phase.
//!
//! Runs after the reconciler's `adjudicate`. For each admissible
//! NQ-backed input, consults `HorizonPolicySource` for a horizon
//! declaration, consults the store for any prior tolerance grant,
//! and computes a `HorizonAction` via `horizon::action_for`. The
//! pipeline then applies the decisions to the store (write on
//! `Defer`; clear on the escalate variants).
//!
//! Kept separate from `reconciler.rs` so that module's `adjudicate`
//! remains pure. Acquisition of declarations + prior tolerance
//! lives here because it's the same failure-mode family as the NQ
//! acquisition (one local read per input, fixed by a single `now`
//! timestamp), but it touches two different sources that the pure
//! adjudicator has no business knowing about.
//!
//! # Invariant: horizon is producer-local
//!
//! Horizon is **declared by Night Shift**, not fetched from
//! Governor. `HorizonPolicySource` is an NS-internal surface
//! (agenda policy, operator declarations, tests). Governor is the
//! archivist — Phase B will forward NS's tolerance decisions to
//! Governor via `record_receipt` so the audit trail exists, but
//! Governor is never the lookup source at reconcile time. See
//! `horizon_policy.rs` module header for the full rationale.
//!
//! Phase A scope: only the four-way A5 distinction, fixture-only.
//! Phase B adds the real Governor RPC client for receipt emission,
//! `check_policy`, and `authorize_transition` — orthogonal to this
//! module's horizon decision logic.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::bundle::{CaptureInput, ReconciliationPhase, ReconciliationResult};
use crate::errors::Result;
use crate::finding::FindingKey;
use crate::governor_client::{EventKind, GovernorClient, RecordReceiptRequest};
use crate::horizon::{action_for, HorizonAction};
use crate::horizon_policy::HorizonPolicySource;
use crate::store::{Store, ToleranceRecord};

/// One horizon decision for one finding in one run, after acquiring
/// the horizon declaration and any prior tolerance grant.
///
/// `evidence_hash` is the reconciled evidence hash from the input —
/// carried through so the Governor receipt on `Defer` can cite the
/// evidence NS evaluated the horizon against. None for inputs the
/// reconciler produced without a current evidence hash (e.g. Stale
/// verdicts).
#[derive(Debug, Clone)]
pub struct HorizonOutcome {
    pub finding_key: FindingKey,
    pub input_id: String,
    pub action: HorizonAction,
    pub evidence_hash: Option<String>,
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
/// Pure over (input, result, declaration, prior, now) — the
/// acquisition of declaration + prior happens in the caller.
fn outcome_for_result(
    input: &CaptureInput,
    result: &ReconciliationResult,
    policy: &dyn HorizonPolicySource,
    store: &dyn Store,
    now: DateTime<Utc>,
) -> Result<Option<HorizonOutcome>> {
    if input.source != "nq" {
        return Ok(None);
    }
    let Some(finding_key) = FindingKey::from_nq_input_id(&input.input_id) else {
        return Ok(None);
    };
    let block = policy.horizon_for(&finding_key)?;
    let prior_record = store.load_tolerance(&finding_key)?;
    let prior = prior_record.as_ref().map(|r| r.to_prior_tolerance());
    let action = action_for(block.as_ref(), now, prior.as_ref());
    // Prefer the reconciled current evidence hash; fall back to
    // captured input hash so Defer outcomes always carry something
    // citable into the Governor receipt.
    let evidence_hash = result
        .current_evidence_hash
        .clone()
        .or_else(|| result.previous_evidence_hash.clone())
        .or_else(|| Some(input.evidence_hash.clone()));
    Ok(Some(HorizonOutcome {
        finding_key,
        input_id: input.input_id.clone(),
        action,
        evidence_hash,
    }))
}

/// Process the horizon phase for a reconciled run. Iterates the
/// adjudicated results, pulls each NQ-backed input's horizon
/// declaration and prior tolerance, and produces a
/// `HorizonOutcome` per input.
///
/// Non-NQ inputs and inputs whose finding_key cannot be parsed are
/// skipped (no outcome emitted). The `now` timestamp should be the
/// same one used in adjudicate — typically
/// `acquisition.acquired_at` — so the horizon evaluation is
/// deterministic relative to the reconcile's reference clock.
pub fn process_horizon(
    phase: &ReconciliationPhase,
    capture_inputs: &[CaptureInput],
    policy: &dyn HorizonPolicySource,
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
        if let Some(outcome) = outcome_for_result(input, result, policy, store, now)? {
            outcomes.push(outcome);
        }
    }
    Ok(outcomes)
}

/// sha256 of a finding-key string, formatted as Governor expects
/// (`sha256:<64 hex>`).
fn subject_hash_for(key: &FindingKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_string().as_bytes());
    let digest = hasher.finalize();
    format!("sha256:{}", hex::encode(digest))
}

/// Zero-hash used when the reconciler produced a `Defer` outcome
/// without a current evidence hash (unexpected in practice, but
/// the Governor receipt payload must not be left empty — Governor
/// validates non-empty evidence_hash).
const ZERO_EVIDENCE_HASH: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// Apply horizon outcomes to the store and to Governor.
///
/// Store effects:
///   `Defer` → write a `ToleranceRecord` (upsert) so the next run
///   sees it as `PriorTolerance`. The two escalate variants →
///   clear the tolerance record so a later run doesn't re-consume
///   the stale grant. `ActOnVerdict` and the render-only variants
///   → no store change.
///
/// Governor effects (B.1 scope — record_receipt only):
///   `Defer` → emit an `action.authorized` event carrying the
///   horizon declaration so Governor's receipt chain archives the
///   tolerance grant. Other outcomes do not emit receipts in B.1;
///   B.2+ will add `escalation.paged` on the escalate variants and
///   `action.authorized` wiring at real action-propose points.
///
/// `run_id` and `agenda_id` identify the emitting run on both the
/// tolerance record and any receipts. `granted_at` is used as the
/// tolerance record's grant timestamp.
pub fn apply_horizon_outcomes(
    outcomes: &[HorizonOutcome],
    store: &dyn Store,
    governor: &dyn GovernorClient,
    run_id: &str,
    agenda_id: &str,
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

                // Forward the deferral to Governor as an authority
                // receipt. The horizon block rides along unchanged;
                // Governor is the archivist here, not the decider.
                let block = crate::horizon::HorizonBlock {
                    class: *class,
                    basis_id: Some(basis_id.clone()),
                    basis_hash: Some(basis_hash.clone()),
                    expiry: Some(*until),
                };
                let request = RecordReceiptRequest {
                    event_kind: EventKind::ActionAuthorized,
                    run_id: run_id.into(),
                    agenda_id: agenda_id.into(),
                    subject_hash: subject_hash_for(&outcome.finding_key),
                    evidence_hash: outcome
                        .evidence_hash
                        .clone()
                        .unwrap_or_else(|| ZERO_EVIDENCE_HASH.into()),
                    policy_hash: basis_hash.clone(),
                    from_level: None,
                    to_level: None,
                    horizon: Some(block),
                };
                governor.record_receipt(&request)?;
            }
            HorizonAction::EscalateExpired { .. }
            | HorizonAction::EscalateBasisInvalidated { .. } => {
                store.clear_tolerance(&outcome.finding_key)?;
            }
            HorizonAction::ActOnVerdict { .. }
            | HorizonAction::RenderNoIntervene { .. }
            | HorizonAction::RenderHolding { .. } => {
                // No-op: no prior record to create or clear on these
                // paths, and no Governor event to emit in B.1.
                // Phase B.2 can extend: escalation.paged on the
                // escalate variants, action.authorized from real
                // propose-time check_policy elsewhere.
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
    use crate::governor_client::FixtureGovernorClient;
    use crate::horizon::{HorizonBlock, HorizonClass};
    use crate::horizon_policy::{FixtureHorizonPolicySource, HorizonDeclaration};
    use crate::store::sqlite::SqliteStore;
    use chrono::TimeZone;

    /// Handy sentinel for tests that don't care about the specific
    /// hash value — the apply layer treats `evidence_hash` as opaque.
    const FAKE_EVIDENCE_HASH: &str =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111";

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

    fn declaration_for(
        key: &FindingKey,
        class: HorizonClass,
        basis_id: &str,
        basis_hash: &str,
        expiry: Option<DateTime<Utc>>,
    ) -> HorizonDeclaration {
        HorizonDeclaration {
            finding_key: key.clone(),
            horizon: HorizonBlock {
                class,
                basis_id: Some(basis_id.into()),
                basis_hash: Some(basis_hash.into()),
                expiry,
            },
        }
    }

    // --- outcome-of-one-finding tests ---

    #[test]
    fn no_declaration_produces_act_on_verdict_missing() {
        let store = SqliteStore::open_in_memory().unwrap();
        let key = fk("wal_bloat", "host:/db");
        let input = input_for(&key, "hash-1");
        let result = result_for(&input.input_id);
        let phase = phase_with(vec![result], Utc::now());
        let policy = FixtureHorizonPolicySource::from_declarations(vec![]);
        let outcomes =
            process_horizon(&phase, &[input], &policy, &store, Utc::now()).unwrap();
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
        let policy = FixtureHorizonPolicySource::from_declarations(vec![declaration_for(
            &key,
            HorizonClass::Hours,
            "basis-abc",
            "hash-abc",
            Some(expiry),
        )]);
        let outcomes = process_horizon(&phase, &[input], &policy, &store, now).unwrap();
        assert!(matches!(outcomes[0].action, HorizonAction::Defer { .. }));
    }

    #[test]
    fn non_nq_inputs_are_skipped() {
        let store = SqliteStore::open_in_memory().unwrap();
        let mut input = input_for(&fk("wal_bloat", "host:/db"), "hash-1");
        input.source = "continuity".into();
        let result = result_for(&input.input_id);
        let phase = phase_with(vec![result], Utc::now());
        let policy = FixtureHorizonPolicySource::from_declarations(vec![]);
        let outcomes =
            process_horizon(&phase, &[input], &policy, &store, Utc::now()).unwrap();
        assert!(outcomes.is_empty(), "non-nq inputs must not produce outcomes");
    }

    // --- apply-outcomes tests ---

    #[test]
    fn apply_defer_writes_tolerance_record() {
        let store = SqliteStore::open_in_memory().unwrap();
        let governor = FixtureGovernorClient::new();
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
            evidence_hash: Some(FAKE_EVIDENCE_HASH.into()),
        };
        apply_horizon_outcomes(&[outcome], &store, &governor, "run_a", "agenda_x", now).unwrap();
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
            evidence_hash: Some(FAKE_EVIDENCE_HASH.into()),
        };
        let governor = FixtureGovernorClient::new();
        apply_horizon_outcomes(
            &[outcome],
            &store,
            &governor,
            "run_b",
            "agenda_x",
            Utc.with_ymd_and_hms(2026, 4, 23, 17, 0, 0).unwrap(),
        )
        .unwrap();
        assert!(
            store.load_tolerance(&key).unwrap().is_none(),
            "EscalateExpired must clear prior tolerance"
        );
        assert_eq!(
            governor.call_count(),
            0,
            "EscalateExpired must not emit a Governor receipt in B.1"
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
            evidence_hash: Some(FAKE_EVIDENCE_HASH.into()),
        };
        let governor = FixtureGovernorClient::new();
        apply_horizon_outcomes(
            &[outcome],
            &store,
            &governor,
            "run_b",
            "agenda_x",
            Utc::now(),
        )
        .unwrap();
        assert!(
            store.load_tolerance(&key).unwrap().is_none(),
            "EscalateBasisInvalidated must clear prior tolerance"
        );
        assert_eq!(
            governor.call_count(),
            0,
            "EscalateBasisInvalidated must not emit a Governor receipt in B.1"
        );
    }

    #[test]
    fn apply_act_on_verdict_is_noop() {
        let store = SqliteStore::open_in_memory().unwrap();
        let governor = FixtureGovernorClient::new();
        let key = fk("wal_bloat", "host:/db");
        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::ActOnVerdict {
                reason: crate::horizon::ActReason::Missing,
            },
            evidence_hash: None,
        };
        apply_horizon_outcomes(
            &[outcome],
            &store,
            &governor,
            "run_x",
            "agenda_x",
            Utc::now(),
        )
        .unwrap();
        assert!(store.load_tolerance(&key).unwrap().is_none());
        assert_eq!(governor.call_count(), 0);
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
            evidence_hash: None,
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
            evidence_hash: None,
        };
        let invalidated = HorizonOutcome {
            finding_key: fk("d", "s"),
            input_id: "x".into(),
            action: HorizonAction::EscalateBasisInvalidated {
                prior,
                current_basis_hash: "new".into(),
            },
            evidence_hash: None,
        };
        assert!(expired.requires_tolerance_clear());
        assert!(invalidated.requires_tolerance_clear());
        assert!(!expired.requires_tolerance_write());
        assert!(!invalidated.requires_tolerance_write());
    }

    /// Phase B.1 core test: Defer emits a Governor `record_receipt`
    /// with `event_kind=action.authorized` and the horizon block
    /// attached. This is the audit trail.
    #[test]
    fn defer_emits_record_receipt_with_horizon() {
        use crate::governor_client::EventKind;
        let store = SqliteStore::open_in_memory().unwrap();
        let governor = FixtureGovernorClient::new();
        let key = fk("wal_bloat", "host:/db");
        let expiry = Utc.with_ymd_and_hms(2026, 4, 23, 16, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 4, 23, 12, 0, 0).unwrap();
        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host:/db".into(),
            action: HorizonAction::Defer {
                until: expiry,
                basis_id: "maintenance-window-042".into(),
                basis_hash: "sha256:basis-abc".into(),
                class: HorizonClass::Hours,
            },
            evidence_hash: Some(FAKE_EVIDENCE_HASH.into()),
        };
        apply_horizon_outcomes(
            &[outcome],
            &store,
            &governor,
            "run_a",
            "wal-bloat-review",
            now,
        )
        .unwrap();

        let calls = governor.recorded_calls();
        assert_eq!(calls.len(), 1, "Defer must emit exactly one receipt");
        let req = &calls[0];
        assert_eq!(req.event_kind, EventKind::ActionAuthorized);
        assert_eq!(req.run_id, "run_a");
        assert_eq!(req.agenda_id, "wal-bloat-review");
        assert_eq!(req.evidence_hash, FAKE_EVIDENCE_HASH);
        assert_eq!(req.policy_hash, "sha256:basis-abc");
        let block = req.horizon.as_ref().expect("horizon must be attached");
        assert_eq!(block.class, HorizonClass::Hours);
        assert_eq!(block.basis_id.as_deref(), Some("maintenance-window-042"));
        assert_eq!(block.basis_hash.as_deref(), Some("sha256:basis-abc"));
        assert_eq!(block.expiry, Some(expiry));
    }

    #[test]
    fn defer_subject_hash_is_sha256_of_finding_key_string() {
        let store = SqliteStore::open_in_memory().unwrap();
        let governor = FixtureGovernorClient::new();
        let key = fk("wal_bloat", "host-x:/db");
        let expiry = Utc::now() + chrono::Duration::hours(4);
        let outcome = HorizonOutcome {
            finding_key: key.clone(),
            input_id: "nq:finding:wal_bloat:host-x:/db".into(),
            action: HorizonAction::Defer {
                until: expiry,
                basis_id: "b".into(),
                basis_hash: "sha256:h".into(),
                class: HorizonClass::Hours,
            },
            evidence_hash: Some(FAKE_EVIDENCE_HASH.into()),
        };
        apply_horizon_outcomes(&[outcome], &store, &governor, "r", "a", Utc::now()).unwrap();
        let req = &governor.recorded_calls()[0];
        // Compute expected: sha256 of key.as_string()
        let mut hasher = Sha256::new();
        hasher.update(key.as_string().as_bytes());
        let expected = format!("sha256:{}", hex::encode(hasher.finalize()));
        assert_eq!(req.subject_hash, expected);
    }
}
