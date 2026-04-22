//! Reconciler — freshness and invalidation pass.
//!
//> The 3am agent must not act on 11pm vibes.
//!
//! The reconciler takes a captured `Bundle` and assigns each input a
//! reconciliation result: `committed`, `changed`, `stale`, or
//! `invalidated`, plus a reliance class and valid_for scope.
//!
//! Recheck is the gate, not metadata.
//!
//! # Two-phase shape (GAP-deferred-run-split.md)
//!
//! Reconcile is internally two phases:
//!
//! 1. [`acquire_current`] — the single, explicit reconcile-time live
//!    acquisition. One call per NQ-backed input; results are bundled
//!    into a [`ReconciliationAcquisition`]. This is the only live
//!    dependency in the reconcile path.
//! 2. [`adjudicate`] — pure function over
//!    (bundle, acquisition, policy fingerprint). Produces the
//!    [`ReconciliationPhase`] with no further live NQ dependency. Given
//!    the same inputs, always produces the same output.
//!
//! [`reconcile_bundle`] composes both phases for callers (e.g.
//! `watchbill run`) that want the same-generation path.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

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
    /// Semantic-axis fields whose value differs between the captured
    /// snapshot and the current snapshot. Empty for `Committed`
    /// (including the churn-only path) and for `Stale` /
    /// `Invalidated`. Populated for `Changed`. See
    /// `GAP-nq-nightshift-contract.md` (three-axis split).
    pub semantic_changes: Vec<&'static str>,
}

/// The semantic-axis projection of a finding snapshot. Two snapshots
/// with the same projection have the same *meaning*, regardless of
/// churn (`snapshot_generation`, `last_seen_at`,
/// `persistence_generations`, `evidence_hash`).
///
/// Per the gap-doc three-axis split, only these fields participate in
/// the `Committed` vs `Changed` decision. `first_seen_at` is treated
/// as a lifecycle anchor (identity-adjacent) and is not included; if
/// it ever drifts under a stable `finding_key`, that is closer to
/// identity drift than to semantic change and the reconciler does
/// not currently model it.
fn semantic_diff(captured: &FindingSnapshot, current: &FindingSnapshot) -> Vec<&'static str> {
    let mut diff = Vec::new();
    if captured.severity != current.severity {
        diff.push("severity");
    }
    if captured.current_status != current.current_status {
        diff.push("evidence_state");
    }
    if captured.domain != current.domain {
        diff.push("domain");
    }
    diff
}

/// Reconcile a single NQ-backed input given its capture record and
/// the current snapshot from the NQ source (or `None` if absent).
///
/// Applies the three-axis split from `GAP-nq-nightshift-contract.md`:
/// identity drift → `Invalidated`, churn-only changes → `Committed`,
/// semantic-axis differences → `Changed`. `evidence_hash` mismatch is
/// only a cheap inequality check; on mismatch, the captured semantic
/// fields are read from `captured.captured_finding_snapshot` and
/// compared explicitly.
///
/// If the captured input lacks `captured_finding_snapshot` (legacy
/// bundle or non-NQ caller), the reconciler degrades to the previous
/// hash-only behavior and notes the degradation. This preserves
/// backwards-compatible behavior for older persisted bundles while
/// the new path becomes the default for fresh runs.
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
                semantic_changes: Vec::new(),
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
                    semantic_changes: Vec::new(),
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
                    semantic_changes: Vec::new(),
                }
            }
        }
        Some(snap) => {
            let current_hash = evidence_hash(snap);
            // Cheap path: byte-identical evidence ⇒ trivially Committed,
            // no semantic-axis read needed.
            if current_hash == captured.evidence_hash {
                return NqReconciliation {
                    status: InputStatus::Committed,
                    notes: None,
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: Some(current_hash),
                    semantic_changes: Vec::new(),
                };
            }
            // Identity drift under a stable input_id ⇒ Invalidated.
            // The reconciler keys on the input_id's encoded
            // finding_key; if NQ now returns a snapshot with a
            // different finding_key, the captured premise no longer
            // refers to the same finding.
            if let Some(captured_snap) = captured.captured_finding_snapshot.as_ref() {
                if captured_snap.finding_key != snap.finding_key {
                    return NqReconciliation {
                        status: InputStatus::Invalidated,
                        notes: Some(format!(
                            "identity drift: captured finding_key {} → current {}",
                            captured_snap.finding_key.as_string(),
                            snap.finding_key.as_string()
                        )),
                        previous_evidence_hash: Some(captured.evidence_hash.clone()),
                        current_evidence_hash: Some(current_hash),
                        semantic_changes: Vec::new(),
                    };
                }
                let diff = semantic_diff(captured_snap, snap);
                if diff.is_empty() {
                    NqReconciliation {
                        status: InputStatus::Committed,
                        notes: Some(format!(
                            "churn-only change (snapshot_generation {} → {}); semantic axis unchanged",
                            captured_snap.snapshot_generation, snap.snapshot_generation
                        )),
                        previous_evidence_hash: Some(captured.evidence_hash.clone()),
                        current_evidence_hash: Some(current_hash),
                        semantic_changes: Vec::new(),
                    }
                } else {
                    NqReconciliation {
                        status: InputStatus::Changed,
                        notes: Some(format!(
                            "semantic-axis change: {}",
                            diff.join(", ")
                        )),
                        previous_evidence_hash: Some(captured.evidence_hash.clone()),
                        current_evidence_hash: Some(current_hash),
                        semantic_changes: diff,
                    }
                }
            } else {
                // Legacy / non-NQ-snapshot path: no captured semantic
                // axis to compare against. Hash mismatch alone is not
                // sufficient evidence of semantic change, but it is
                // not nothing either. Surface it as `Changed` with an
                // explicit note that the verdict was hash-only.
                NqReconciliation {
                    status: InputStatus::Changed,
                    notes: Some(
                        "evidence hash differs; captured semantic snapshot unavailable, verdict is hash-only"
                            .into(),
                    ),
                    previous_evidence_hash: Some(captured.evidence_hash.clone()),
                    current_evidence_hash: Some(current_hash),
                    semantic_changes: Vec::new(),
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

/// The explicit reconcile-time live acquisition result.
///
/// One entry per NQ-backed captured input. A `None` value means the
/// finding was absent from the current generation — a load-bearing
/// distinction for the adjudicator (absence + absence-rule →
/// Invalidated; absence without rule → Stale).
///
/// Non-NQ inputs do not appear here in v1. `acquired_at` is a single
/// wall-clock read taken once, passed to `adjudicate` as the `now`
/// reference for freshness-TTL checks. Fixing the reference clock at
/// acquisition time is what makes `adjudicate` deterministic.
#[derive(Debug, Clone)]
pub struct ReconciliationAcquisition {
    pub acquired_at: DateTime<Utc>,
    pub current_snapshots: HashMap<String, Option<FindingSnapshot>>,
}

impl ReconciliationAcquisition {
    /// Count of inputs that returned a present snapshot. Used for
    /// ledger payloads and operator-facing summaries.
    pub fn present_count(&self) -> usize {
        self.current_snapshots
            .values()
            .filter(|v| v.is_some())
            .count()
    }

    /// Count of inputs that returned absent. Load-bearing for absence
    /// detection: the adjudicator needs to know this was *observed
    /// absent*, not *not asked*.
    pub fn absent_count(&self) -> usize {
        self.current_snapshots
            .values()
            .filter(|v| v.is_none())
            .count()
    }
}

/// Policy fingerprint — the versioned declarative material that the
/// adjudicator may legitimately vary over between capture and
/// reconcile. Per `GAP-deferred-run-split.md`: any variation is
/// allowed only if surfaced explicitly and versioned in the output.
///
/// v1 stubs this. Real policy versioning arrives with the Governor
/// adapter; the shape is reserved so the pure adjudication signature
/// doesn't change when policy lands.
#[derive(Debug, Clone, Default)]
pub struct PolicyFingerprint {
    pub version: Option<String>,
}

/// Perform the reconcile-time live acquisition pass.
///
/// One live NQ call per NQ-backed input. Returns a structure that
/// fully specifies the observed state; [`adjudicate`] is then pure
/// over it. Intended to be called exactly once per run's reconcile
/// phase; the pipeline persists the resulting current snapshots into
/// the bundle so replay / debugging needs no further live calls.
pub fn acquire_current(
    bundle: &Bundle,
    nq: &dyn NqSource,
) -> Result<ReconciliationAcquisition> {
    let mut current_snapshots = HashMap::new();
    for input in &bundle.capture.inputs {
        if input.source != "nq" {
            continue;
        }
        let key = parse_nq_input_id(&input.input_id)?;
        let snap = nq.snapshot(&key)?;
        current_snapshots.insert(input.input_id.clone(), snap);
    }
    Ok(ReconciliationAcquisition {
        acquired_at: Utc::now(),
        current_snapshots,
    })
}

/// Pure adjudication pass.
///
/// Deterministic over `(bundle, acquisition, policy_fp)`: the same
/// three inputs always produce the same `ReconciliationPhase`. No
/// live dependency, no wall-clock read beyond the acquisition
/// timestamp already frozen in `acquisition.acquired_at`.
///
/// v1: only NQ-backed inputs are adjudicated against acquired state.
/// Other input kinds keep their captured status and are assigned
/// default reliance classes.
pub fn adjudicate(
    bundle: &Bundle,
    acquisition: &ReconciliationAcquisition,
    _policy_fp: &PolicyFingerprint,
) -> ReconciliationPhase {
    let now = acquisition.acquired_at;
    let mut results = Vec::with_capacity(bundle.capture.inputs.len());

    for input in &bundle.capture.inputs {
        let (status, notes, prev_hash, curr_hash, current_snapshot) = if input.source == "nq" {
            let current = acquisition
                .current_snapshots
                .get(&input.input_id)
                .cloned()
                .flatten();
            let r = reconcile_nq_input(input, current.as_ref(), now);
            (
                r.status,
                r.notes,
                r.previous_evidence_hash,
                r.current_evidence_hash,
                current,
            )
        } else {
            // Non-NQ inputs: keep observed → committed without a live check in v1.
            (
                InputStatus::Committed,
                None,
                Some(input.evidence_hash.clone()),
                Some(input.evidence_hash.clone()),
                None,
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
            current_finding_snapshot: current_snapshot,
        });
    }

    let summary = build_summary(&results);

    ReconciliationPhase {
        reconciled_at: now,
        reconciled_by: "scheduler".into(),
        results,
        summary,
    }
}

/// Convenience: acquire current state and adjudicate in a single
/// call. Preserves the pre-split contract for same-generation callers
/// like `watchbill run`; the deferred path uses
/// [`acquire_current`] + [`adjudicate`] explicitly.
pub fn reconcile_bundle(bundle: &Bundle, nq: &dyn NqSource) -> Result<ReconciliationPhase> {
    let acquisition = acquire_current(bundle, nq)?;
    Ok(adjudicate(bundle, &acquisition, &PolicyFingerprint::default()))
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
        captured_input_with_snap(hash, expires_at, None)
    }

    fn captured_input_with_snap(
        hash: &str,
        expires_at: Option<chrono::DateTime<Utc>>,
        snap: Option<FindingSnapshot>,
    ) -> CaptureInput {
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
            captured_finding_snapshot: snap,
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

    /// **Load-bearing test for the three-axis split**
    /// (`GAP-nq-nightshift-contract.md`).
    ///
    /// Captured snapshot and current snapshot are identical on the
    /// semantic axis (severity, evidence_state, domain) but differ on
    /// the churn axis (`snapshot_generation`). `evidence_hash` flips
    /// because the generation participates in the integrity hash.
    /// The reconciler must report `Committed`, not `Changed`. If this
    /// test ever flips to `Changed`, every long-lived NQ finding will
    /// be classified as semantically changed every minute in steady
    /// state, and the reconciler is operationally useless.
    #[test]
    fn committed_when_only_churn() {
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let current_snap = snap(1); // only snapshot_generation differs
        let current_hash = evidence_hash(&current_snap);
        assert_ne!(
            captured_hash, current_hash,
            "test precondition: hash must differ when generation moves"
        );

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(
            r.status,
            InputStatus::Committed,
            "churn-only change must reconcile as Committed; got {:?} (notes={:?})",
            r.status,
            r.notes
        );
        assert!(r.semantic_changes.is_empty());
        assert_ne!(r.previous_evidence_hash, r.current_evidence_hash);
        assert!(
            r.notes.as_deref().unwrap_or("").contains("churn-only"),
            "verdict notes should mark the cheap-path explanation; got {:?}",
            r.notes
        );
    }

    #[test]
    fn changed_when_severity_promotes() {
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let mut current_snap = snap(0);
        current_snap.severity = Severity::Critical; // semantic-axis move

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Changed);
        assert_eq!(r.semantic_changes, vec!["severity"]);
    }

    #[test]
    fn changed_when_evidence_state_flips() {
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let mut current_snap = snap(0);
        current_snap.current_status = EvidenceState::Resolving;

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Changed);
        assert_eq!(r.semantic_changes, vec!["evidence_state"]);
    }

    #[test]
    fn changed_when_domain_shifts() {
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let mut current_snap = snap(0);
        current_snap.domain = Some("delta_q".into());

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Changed);
        assert_eq!(r.semantic_changes, vec!["domain"]);
    }

    #[test]
    fn changed_carries_multi_field_diff() {
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let mut current_snap = snap(0);
        current_snap.severity = Severity::Critical;
        current_snap.current_status = EvidenceState::Worsening;

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Changed);
        assert_eq!(r.semantic_changes, vec!["severity", "evidence_state"]);
    }

    #[test]
    fn invalidated_when_identity_drifts() {
        // Captured snapshot has one finding_key; NQ now returns a
        // snapshot under a different finding_key for the same
        // input_id. That is identity drift, not semantic change.
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let mut current_snap = snap(1);
        current_snap.finding_key = FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "labelwatch-host:/var/lib/somewhere-else.db".into(),
        };

        let c = captured_input_with_snap(&captured_hash, None, Some(captured_snap));
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Invalidated);
        assert!(r.notes.as_deref().unwrap_or("").contains("identity drift"));
    }

    #[test]
    fn legacy_bundle_without_captured_snapshot_falls_back_to_hash_only_changed() {
        // CaptureInput with no captured_finding_snapshot (legacy
        // shape). Hash mismatch must still produce a verdict; without
        // the semantic axis to read, the reconciler degrades to
        // hash-only Changed and notes the degradation. This protects
        // older persisted bundles from silently misclassifying.
        let captured_snap = snap(0);
        let captured_hash = evidence_hash(&captured_snap);
        let current_snap = snap(1);

        let c = captured_input(&captured_hash, None); // no snapshot
        let r = reconcile_nq_input(&c, Some(&current_snap), Utc::now());

        assert_eq!(r.status, InputStatus::Changed);
        assert!(r.semantic_changes.is_empty());
        assert!(
            r.notes.as_deref().unwrap_or("").contains("hash-only"),
            "verdict notes must surface the hash-only degradation; got {:?}",
            r.notes
        );
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

    // ----- Pure `adjudicate` tests (deferred-run split) -----
    //
    // These exercise adjudication directly, without any `NqSource`.
    // They are the operational proof of
    // `GAP-deferred-run-split.md` invariant #3: once reconcile-time
    // acquisition is persisted, adjudication is deterministic with no
    // further live NQ dependency.

    use crate::bundle::{Bundle, CapturePhase};

    fn bundle_with_single_input(input: CaptureInput) -> Bundle {
        Bundle {
            bundle_version: 0,
            agenda_id: "test-agenda".into(),
            run_id: "run_test".into(),
            capture: CapturePhase {
                captured_at: input.freshness.captured_at,
                captured_by: "test".into(),
                capture_reason: "unit test".into(),
                inputs: vec![input],
            },
            reconciliation: None,
        }
    }

    fn acquisition_of(
        input_id: &str,
        snap: Option<FindingSnapshot>,
        at: DateTime<Utc>,
    ) -> ReconciliationAcquisition {
        let mut m = HashMap::new();
        m.insert(input_id.into(), snap);
        ReconciliationAcquisition {
            acquired_at: at,
            current_snapshots: m,
        }
    }

    #[test]
    fn adjudicate_is_pure_no_live_source_needed() {
        // Classic churn-only bundle. `adjudicate` has no NqSource
        // argument — the "current" snapshot is supplied via the
        // acquisition. This test exists to lock the signature as
        // pure: if someone later re-threads live state into
        // adjudicate, this test fails to compile.
        let captured = snap(0);
        let hash = evidence_hash(&captured);
        let current = snap(1); // churn only

        let input = captured_input_with_snap(&hash, None, Some(captured));
        let bundle = bundle_with_single_input(input.clone());
        let acq = acquisition_of(
            &input.input_id,
            Some(current),
            Utc.with_ymd_and_hms(2026, 4, 22, 3, 0, 0).unwrap(),
        );

        let phase = adjudicate(&bundle, &acq, &PolicyFingerprint::default());
        assert_eq!(phase.results.len(), 1);
        assert_eq!(phase.results[0].status, InputStatus::Committed);
        assert!(phase.results[0].current_finding_snapshot.is_some());
    }

    #[test]
    fn adjudicate_persists_current_snapshot_into_result() {
        // The reconcile-time acquisition must be captured in the
        // ReconciliationResult so the bundle is self-contained for
        // replay. Covers invariant #2 of GAP-deferred-run-split.md:
        // reconcile-time live acquisition is persisted as part of
        // the run record.
        let captured = snap(0);
        let hash = evidence_hash(&captured);
        let mut current = snap(0);
        current.severity = Severity::Critical; // semantic change

        let input = captured_input_with_snap(&hash, None, Some(captured));
        let bundle = bundle_with_single_input(input.clone());
        let acq = acquisition_of(&input.input_id, Some(current.clone()), Utc::now());

        let phase = adjudicate(&bundle, &acq, &PolicyFingerprint::default());
        let r = &phase.results[0];
        assert_eq!(r.status, InputStatus::Changed);
        let persisted = r
            .current_finding_snapshot
            .as_ref()
            .expect("adjudicate must persist the acquired snapshot");
        assert_eq!(persisted.severity, Severity::Critical);
        assert_eq!(persisted.finding_key, current.finding_key);
    }

    #[test]
    fn adjudicate_is_deterministic_over_fixed_inputs() {
        // Load-bearing test for invariant #3 of
        // GAP-deferred-run-split.md: once acquisition is persisted,
        // adjudication is deterministic. Running adjudicate twice
        // over the same bundle + acquisition + policy must produce
        // byte-for-byte identical phases (modulo collection ordering
        // of independent vecs, which we assert on the load-bearing
        // fields explicitly).
        let captured = snap(0);
        let hash = evidence_hash(&captured);
        let current = snap(1); // churn only

        let input = captured_input_with_snap(&hash, None, Some(captured));
        let bundle = bundle_with_single_input(input.clone());
        let acq_at = Utc.with_ymd_and_hms(2026, 4, 22, 3, 0, 0).unwrap();
        let acq = acquisition_of(&input.input_id, Some(current), acq_at);
        let policy = PolicyFingerprint::default();

        let phase_a = adjudicate(&bundle, &acq, &policy);
        let phase_b = adjudicate(&bundle, &acq, &policy);

        assert_eq!(phase_a.reconciled_at, phase_b.reconciled_at);
        assert_eq!(phase_a.reconciled_at, acq_at);
        assert_eq!(phase_a.results.len(), phase_b.results.len());
        for (ra, rb) in phase_a.results.iter().zip(phase_b.results.iter()) {
            assert_eq!(ra.input_id, rb.input_id);
            assert_eq!(ra.status, rb.status);
            assert_eq!(ra.previous_evidence_hash, rb.previous_evidence_hash);
            assert_eq!(ra.current_evidence_hash, rb.current_evidence_hash);
            assert_eq!(ra.notes, rb.notes);
            assert_eq!(
                ra.current_finding_snapshot.as_ref().map(|s| &s.finding_key),
                rb.current_finding_snapshot.as_ref().map(|s| &s.finding_key),
            );
        }
    }

    #[test]
    fn adjudicate_absent_snapshot_with_absence_rule_is_invalidated() {
        // Acquisition observed the finding absent at current
        // generation. Absence rule present → Invalidated. This is
        // the absence path of the three-axis contract, proven
        // without any NqSource.
        let input = captured_input("sha256:abc", None);
        let bundle = bundle_with_single_input(input.clone());
        let acq = acquisition_of(&input.input_id, None, Utc::now());

        let phase = adjudicate(&bundle, &acq, &PolicyFingerprint::default());
        assert_eq!(phase.results[0].status, InputStatus::Invalidated);
        assert!(phase.results[0].current_finding_snapshot.is_none());
        assert!(!phase.summary.ok_to_proceed);
    }

    #[test]
    fn adjudicate_respects_freshness_ttl_without_reading_current() {
        // TTL-expired: adjudicate returns Stale regardless of what
        // the acquisition observed. This is the first branch in
        // reconcile_nq_input; covered here to lock the behavior at
        // the adjudication-surface level, not just the per-input
        // primitive.
        let captured = snap(0);
        let hash = evidence_hash(&captured);
        let expired = Utc::now() - Duration::hours(1);
        let input = captured_input_with_snap(&hash, Some(expired), Some(captured.clone()));
        let bundle = bundle_with_single_input(input.clone());
        // Acquisition observed the finding still present + identical.
        let acq = acquisition_of(&input.input_id, Some(captured), Utc::now());

        let phase = adjudicate(&bundle, &acq, &PolicyFingerprint::default());
        assert_eq!(phase.results[0].status, InputStatus::Stale);
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
