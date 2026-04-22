//! Integration tests for the deferred-run split
//! (`GAP-deferred-run-split.md`).
//!
//! The reconciler unit tests prove adjudication is pure; the
//! `reconciler_pipeline` integration tests prove verdicts flow
//! through the convenience `run_watchbill` path. This file proves
//! the split itself:
//!
//! - `capture_phase` alone persists an open run without a packet.
//! - `reconcile_phase` against a previously captured run produces
//!   the correct verdict end-to-end, exercising the three-axis
//!   contract under a real wall-time gap between capture and
//!   reconcile (simulated by separating the two invocations).
//! - One-shot reconcile: a completed run refuses re-reconcile.
//! - `RunNotFound` on a nonexistent run_id.
//! - The bundle after reconcile persists the reconcile-time current
//!   snapshot — invariant #2 of the GAP doc.
//! - The ledger records `RunCurrentSnapshotAcquired` on the reconcile
//!   path.
//! - `watchbill run` is equivalent to `capture` + `reconcile` for
//!   the same-generation case.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::bundle::InputStatus;
use nightshiftd::errors::{NightShiftError, Result};
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::ledger::RunLedgerEventKind;
use nightshiftd::nq::NqSource;
use nightshiftd::pipeline::{
    capture_phase, reconcile_phase, run_watchbill, CaptureOutcome, PipelineOptions,
};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::Store;

fn fixtures_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn baseline_snapshot() -> FindingSnapshot {
    FindingSnapshot {
        finding_key: FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
        },
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 6,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 14, 32, 15).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: 39000,
        captured_at: Utc.with_ymd_and_hms(2026, 4, 17, 3, 0, 0).unwrap(),
        evidence_hash: String::new(),
    }
}

/// Scripted NQ source — returns head of `snapshots` on each call,
/// popping until one entry remains, then repeats. `None` simulates
/// "finding absent at current generation."
///
/// Post-split call pattern: capture_phase makes 1 call, reconcile_phase
/// makes 1 call via `acquire_current`, packet-build makes 0 calls
/// (reads persisted snapshot). So a full deferred run consumes 2
/// entries; a standalone capture_phase consumes 1.
struct ScriptedNqSource {
    snapshots: Mutex<Vec<Option<FindingSnapshot>>>,
}

impl ScriptedNqSource {
    fn new(script: Vec<Option<FindingSnapshot>>) -> Self {
        assert!(
            !script.is_empty(),
            "ScriptedNqSource needs at least one entry"
        );
        Self {
            snapshots: Mutex::new(script),
        }
    }

    fn remaining(&self) -> usize {
        self.snapshots.lock().unwrap().len()
    }
}

impl NqSource for ScriptedNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        let mut s = self.snapshots.lock().unwrap();
        if s.len() > 1 {
            Ok(s.remove(0))
        } else {
            Ok(s[0].clone())
        }
    }
}

fn agenda_and_target() -> (Agenda, FindingKey) {
    let agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };
    (agenda, target)
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: None,
    }
}

// ----- capture-only, no reconcile -----

/// `capture_phase` alone persists an open run, with a bundle but no
/// packet. Load-bearing proof of invariant #1: capture is a distinct
/// phase whose output is a captured run.
#[test]
fn capture_phase_leaves_run_open_without_packet() {
    let nq = ScriptedNqSource::new(vec![Some(baseline_snapshot())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let out = capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap();
    let run_id = match out {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("captured baseline should not hold"),
    };

    // Exactly one NQ call used by capture_phase.
    assert_eq!(nq.remaining(), 1, "capture_phase consumed one script entry");

    let summary = store
        .get_run_summary(&run_id)
        .unwrap()
        .expect("run must exist");
    assert!(
        summary.completed_at.is_none(),
        "captured-only run must remain open (completed_at IS NULL)"
    );

    let bundle = store
        .get_bundle(&run_id)
        .unwrap()
        .expect("captured bundle must persist");
    assert!(
        bundle.reconciliation.is_none(),
        "captured-only bundle must have no reconciliation phase yet"
    );

    // No packet yet — reconcile_phase produces the packet.
    assert!(store.get_packet(&run_id).unwrap().is_none());
}

// ----- capture → reconcile round-trip, per verdict -----

/// Capture sees snapshot A; reconcile sees semantically-changed A'.
/// The two-invocation flow exercises the *deferral* (not just the
/// mechanics): capture freezes baseline, reconcile's explicit
/// acquisition reads the moved state. Without the split this branch
/// is only unit-testable.
#[test]
fn deferred_capture_reconcile_renders_changed_when_severity_promotes() {
    let captured = baseline_snapshot();
    let mut current = captured.clone();
    current.snapshot_generation += 1;
    current.severity = Severity::Critical;

    let nq = ScriptedNqSource::new(vec![Some(captured), Some(current)]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold on baseline"),
    };

    let packet = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    assert!(packet.diagnosis.regime.starts_with("changed"));
    assert!(packet
        .proposed_action
        .risk_notes
        .iter()
        .any(|n| n.contains("severity")));
    assert!(packet.reconciliation_summary.ok_to_proceed);
}

#[test]
fn deferred_capture_reconcile_renders_invalidated_when_finding_disappears() {
    let captured = baseline_snapshot();
    let nq = ScriptedNqSource::new(vec![Some(captured), None]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold on baseline"),
    };

    let packet = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    assert!(packet.diagnosis.regime.starts_with("invalidated"));
    assert!(!packet.reconciliation_summary.ok_to_proceed);
    assert!(packet
        .proposed_action
        .steps
        .iter()
        .any(|s| s.contains("no remediation proposed")));
}

#[test]
fn deferred_capture_reconcile_renders_committed_when_unchanged() {
    let captured = baseline_snapshot();
    // Single-entry script: same snapshot at capture and reconcile.
    let nq = ScriptedNqSource::new(vec![Some(captured)]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold"),
    };

    let packet = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();
    assert!(packet.diagnosis.regime.starts_with("committed"));
}

// ----- persistence invariants -----

/// Invariant #2 of GAP-deferred-run-split.md: reconcile-time live
/// acquisition must be persisted as part of the run record. After
/// `reconcile_phase` completes, the bundle must carry the acquired
/// current snapshot, so replay / debugging needs no further live
/// NQ calls.
#[test]
fn reconcile_persists_current_snapshot_into_bundle() {
    let captured = baseline_snapshot();
    let mut current = captured.clone();
    current.snapshot_generation += 7;

    let nq = ScriptedNqSource::new(vec![Some(captured.clone()), Some(current.clone())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold"),
    };
    let _ = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    let bundle = store.get_bundle(&run_id).unwrap().unwrap();
    let recon = bundle
        .reconciliation
        .as_ref()
        .expect("reconciled bundle must have reconciliation phase");
    let result = &recon.results[0];
    let persisted = result
        .current_finding_snapshot
        .as_ref()
        .expect("reconcile must persist the acquired current snapshot");
    assert_eq!(persisted.snapshot_generation, current.snapshot_generation);
    // The captured (baseline) is still in capture.inputs, intact.
    let captured_in_bundle = bundle
        .capture
        .inputs
        .iter()
        .find_map(|i| i.captured_finding_snapshot.as_ref())
        .unwrap();
    assert_eq!(
        captured_in_bundle.snapshot_generation,
        captured.snapshot_generation,
        "captured baseline must be untouched by reconcile"
    );
}

/// The reconcile-time live acquisition is named in the ledger. An
/// operator reading the run posture can tell the single live read
/// happened and when.
#[test]
fn reconcile_emits_current_snapshot_acquired_event() {
    let nq = ScriptedNqSource::new(vec![Some(baseline_snapshot())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold"),
    };
    let _ = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    let events = store.list_events(&run_id).unwrap();
    let acquired = events
        .iter()
        .find(|e| matches!(e.kind, RunLedgerEventKind::RunCurrentSnapshotAcquired))
        .expect("ledger must record the reconcile-time acquisition step");
    assert_eq!(
        acquired.payload["inputs"].as_u64().unwrap(),
        1,
        "v1 bundle has one NQ input; acquisition payload should say so"
    );
    assert_eq!(acquired.payload["present"].as_u64().unwrap(), 1);

    // The acquisition event must precede the RunReconciled event.
    let positions: Vec<usize> = events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e.kind {
            RunLedgerEventKind::RunCurrentSnapshotAcquired => Some(i),
            RunLedgerEventKind::RunReconciled => Some(i),
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 2, "both events must be present");
    assert!(
        positions[0] < positions[1],
        "RunCurrentSnapshotAcquired must precede RunReconciled"
    );
}

// ----- one-shot enforcement -----

#[test]
fn reconcile_on_completed_run_errors_with_run_already_completed() {
    let nq = ScriptedNqSource::new(vec![Some(baseline_snapshot())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold"),
    };
    let _ = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    // Second reconcile on the same run is refused.
    let err = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap_err();
    match err {
        NightShiftError::RunAlreadyCompleted(r) => assert_eq!(r, run_id),
        other => panic!("expected RunAlreadyCompleted, got {other:?}"),
    }
}

#[test]
fn reconcile_on_missing_run_errors_with_run_not_found() {
    let nq = ScriptedNqSource::new(vec![Some(baseline_snapshot())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let err = reconcile_phase("run_does_not_exist", &nq, &store, &opts()).unwrap_err();
    match err {
        NightShiftError::RunNotFound(r) => assert_eq!(r, "run_does_not_exist"),
        other => panic!("expected RunNotFound, got {other:?}"),
    }
}

// ----- same-generation equivalence -----

/// `watchbill run` (via `run_watchbill`) is a thin convenience over
/// `capture` + `reconcile`. For the same-generation case, the verdict
/// must match. Catches regressions where the deferred path and the
/// convenience path diverge in behavior.
#[test]
fn run_watchbill_verdict_matches_capture_then_reconcile() {
    let captured = baseline_snapshot();
    let mut current = captured.clone();
    current.snapshot_generation += 1;
    current.severity = Severity::Critical;

    // Two separate stores so the two code paths are independent.
    let store_a = SqliteStore::open_in_memory().unwrap();
    let store_b = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    // Convenience path.
    let nq_conv =
        ScriptedNqSource::new(vec![Some(captured.clone()), Some(current.clone())]);
    let packet_conv = run_watchbill(&agenda, &target, &nq_conv, &store_a, &opts()).unwrap();

    // Split path.
    let nq_split = ScriptedNqSource::new(vec![Some(captured), Some(current)]);
    let run_id = match capture_phase(&agenda, &target, &nq_split, None, &store_b, &opts()).unwrap()
    {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture should not hold"),
    };
    let packet_split = reconcile_phase(&run_id, &nq_split, &store_b, &opts()).unwrap();

    // Verdict-level equivalence — ignore packet_id, run_id,
    // produced_at which are legitimately different between runs.
    assert_eq!(packet_conv.diagnosis.regime, packet_split.diagnosis.regime);
    assert_eq!(
        packet_conv.proposed_action.kind,
        packet_split.proposed_action.kind
    );
    assert_eq!(
        packet_conv.reconciliation_summary.ok_to_proceed,
        packet_split.reconciliation_summary.ok_to_proceed
    );
    assert_eq!(
        packet_conv.attention.evidence_state,
        packet_split.attention.evidence_state
    );
    // Sanity: both are `Changed` per the scripted state move.
    let nq_result = &packet_conv.reconciliation_summary;
    assert!(!nq_result.admissible_for_authorization.is_empty());
    // Canary that the `Changed` path ran.
    assert!(packet_conv.diagnosis.regime.starts_with("changed"));
    let _: InputStatus = InputStatus::Changed;
}
