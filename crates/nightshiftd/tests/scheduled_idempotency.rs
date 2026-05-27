//! Slice 1 close-out: scheduled-trigger idempotency.
//!
//! Acceptance: re-running with `--trigger scheduled` against the same
//! `(agenda, finding)` and the same NQ `snapshot_generation` does
//! not silently double-count. The second invocation must either find
//! the existing run and report it, or open a new run with an
//! explicit reason. Manual and Event triggers always run.
//!
//! These tests cover the library-level `check_scheduled_idempotency`
//! seam directly. The CLI gate in `main.rs::run_watchbill_cmd` calls
//! it only when `--trigger scheduled` is in effect; that conditional
//! is exercised by inspection — the function is called *only* under
//! Scheduled, never under Manual or Event, and any caller that
//! invokes the pipeline without first calling this check will open
//! a fresh run (the existing default behavior, also tested below).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::nq::NqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
use nightshiftd::scheduled::{check_scheduled_idempotency, ScheduledOutcome};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::{RunFilter, RunTrigger, Store};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn agenda() -> Agenda {
    Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap()
}

fn snapshot_at_generation(generation: u64) -> FindingSnapshot {
    FindingSnapshot {
        finding_key: target(),
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 3,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: generation,
        captured_at: Utc::now(),
        evidence_hash: String::new(),
        origin: None,
        silence: None,
    }
}

/// A trivial NQ source that always returns the same snapshot. Useful
/// for testing the idempotency function because it cleanly separates
/// "what generation does NQ report" from "what generation was
/// previously persisted."
struct ConstantNqSource(FindingSnapshot);

impl NqSource for ConstantNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(Some(self.0.clone()))
    }
}

/// Empty NQ source — the finding is not present at the current
/// generation. Idempotency check must fall through to Proceed (the
/// pipeline will then emit the canonical absent-target error).
struct AbsentNqSource;

impl NqSource for AbsentNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(None)
    }
}

fn opts_scheduled() -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: Some(RunTrigger::Scheduled),
        liveness_threshold_seconds: None,
        imported_basis_freshness_window_seconds: None,
    }
}

#[test]
fn scheduled_skip_when_same_generation_already_reconciled() {
    let snap = snapshot_at_generation(42);
    let nq = ConstantNqSource(snap.clone());
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();

    // First scheduled run — completes a reconciliation at generation 42.
    let _packet = run_watchbill(&ag, &target(), &nq, &store, &opts_scheduled()).unwrap();

    // Second invocation: same generation, same finding. The
    // idempotency check must skip with a reference to the prior run.
    let outcome = check_scheduled_idempotency(&ag, &target(), &nq, &store).unwrap();
    let report = match outcome {
        ScheduledOutcome::Skipped(r) => r,
        ScheduledOutcome::Proceed => panic!("expected Skipped, got Proceed"),
    };
    assert_eq!(report.snapshot_generation, 42);
    assert!(report.prior_completed_at.is_some());
    assert!(report.prior_run_id.starts_with("run_"));
    let msg = report.message();
    assert!(msg.contains("scheduled-skip"));
    assert!(msg.contains("snapshot_generation=42"));
    assert!(msg.contains(&report.prior_run_id));
}

#[test]
fn scheduled_runs_when_generation_advances() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();

    // First run at generation 100.
    let nq_v1 = ConstantNqSource(snapshot_at_generation(100));
    let _packet = run_watchbill(&ag, &target(), &nq_v1, &store, &opts_scheduled()).unwrap();

    // Now NQ advances to generation 101 — the idempotency check
    // must let the next invocation proceed.
    let nq_v2 = ConstantNqSource(snapshot_at_generation(101));
    let outcome = check_scheduled_idempotency(&ag, &target(), &nq_v2, &store).unwrap();
    assert!(
        matches!(outcome, ScheduledOutcome::Proceed),
        "different generation must Proceed; got {outcome:?}"
    );

    // And actually running the pipeline a second time opens a new
    // run row, distinct from the first.
    let _packet2 = run_watchbill(&ag, &target(), &nq_v2, &store, &opts_scheduled()).unwrap();
    let runs = store
        .list_runs(RunFilter {
            target_finding_key: Some(target().as_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(runs.len(), 2, "second-generation invocation opens a new run");
    assert_ne!(runs[0].run_id, runs[1].run_id);
}

#[test]
fn idempotency_with_no_prior_runs_proceeds() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = ConstantNqSource(snapshot_at_generation(1));
    let outcome = check_scheduled_idempotency(&ag, &target(), &nq, &store).unwrap();
    assert!(matches!(outcome, ScheduledOutcome::Proceed));
}

#[test]
fn idempotency_proceeds_when_finding_absent_from_nq() {
    // If NQ doesn't have the target, the idempotency check has no
    // generation to compare against — let the pipeline handle the
    // not-present case.
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let outcome = check_scheduled_idempotency(&ag, &target(), &AbsentNqSource, &store).unwrap();
    assert!(matches!(outcome, ScheduledOutcome::Proceed));
}

#[test]
fn trigger_kind_is_persisted_on_run_row() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = ConstantNqSource(snapshot_at_generation(7));
    let _ = run_watchbill(&ag, &target(), &nq, &store, &opts_scheduled()).unwrap();
    let runs = store.list_runs(RunFilter::default()).unwrap();
    assert_eq!(runs.len(), 1);
    assert!(matches!(runs[0].trigger, RunTrigger::Scheduled));
}

/// Idempotency keys on (agenda, finding) — a second agenda
/// targeting the same finding-key does NOT see the first agenda's
/// run as a same-generation prior. Reason: agendas declare workflow
/// and scope; conflating them would let one agenda silence another.
/// See CLAUDE.md invariant 14 — attention/ack is operator intent
/// within agenda scope.
#[test]
fn idempotency_scopes_to_agenda_not_just_finding() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag_a = agenda();
    let nq = ConstantNqSource(snapshot_at_generation(50));

    // Run against agenda A.
    let _ = run_watchbill(&ag_a, &target(), &nq, &store, &opts_scheduled()).unwrap();

    // A different agenda with the same finding-target.
    let mut ag_b = agenda();
    ag_b.agenda_id = "wal-bloat-postmortem".into();
    // Store the agenda explicitly so the idempotency check's
    // list_runs filter has a row to find (the pipeline saves the
    // agenda on capture, but here we are checking *before* any
    // run for B).
    let _ = store.create_agenda(&ag_b).unwrap();

    let outcome = check_scheduled_idempotency(&ag_b, &target(), &nq, &store).unwrap();
    assert!(
        matches!(outcome, ScheduledOutcome::Proceed),
        "a different agenda must not be silenced by another agenda's prior run"
    );
}
