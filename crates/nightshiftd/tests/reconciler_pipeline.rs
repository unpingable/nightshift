//! Pipeline-level integration tests for slice 5 — the reconciler
//! verdict surfaces at the packet boundary.
//!
//! The reconciler unit tests in `src/reconciler.rs` prove the
//! three-axis split is correct as a function. These tests prove the
//! pipeline carries the verdict through capture → reconcile → packet
//! correctly, with the right operator-facing content per
//! `GAP-nq-nightshift-contract.md` and the slice-5 design rulings.
//!
//! Each test wires a `ScriptedNqSource` so capture and reconcile see
//! distinct snapshot states. The pipeline invokes `nq.snapshot()`
//! three times per run (capture, reconciler internal, packet
//! current-snapshot fetch), so the script supplies one or two values
//! and the source repeats the last value indefinitely.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::bundle::InputStatus;
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::nq::NqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
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

/// A scripted NQ source: returns the head of `snapshots` on each
/// call, popping until a single entry remains, then repeats that
/// entry. `None` entries simulate "finding absent at this generation."
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
    }
}

/// **Load-bearing pipeline test for slice 5**.
///
/// Capture sees snapshot A; reconcile + packet see snapshot A' that
/// differs only on the churn axis (`snapshot_generation` advanced).
/// The pipeline must emit a packet with the churn-only Committed
/// regime, not the byte-identical cheap-path regime and not Changed.
#[test]
fn pipeline_renders_committed_for_churn_only_change() {
    let captured = baseline_snapshot();
    let mut churned = captured.clone();
    churned.snapshot_generation = captured.snapshot_generation + 15;
    churned.persistence_generations = captured.persistence_generations + 15;

    let nq = ScriptedNqSource::new(vec![Some(captured), Some(churned)]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts()).unwrap();

    let regime = &packet.diagnosis.regime;
    assert!(
        regime.starts_with("committed"),
        "expected a committed regime; got {regime:?}"
    );
    assert!(
        regime.contains("churn-only"),
        "churn-only path must be explicit in the regime; got {regime:?}"
    );
    assert!(
        packet
            .reconciliation_summary
            .admissible_for_authorization
            .iter()
            .any(|i| i.starts_with("nq:finding:")),
        "committed input must remain admissible for authorization"
    );
    // Steps stay normal-advisory — no revalidation language.
    assert!(packet
        .proposed_action
        .steps
        .iter()
        .all(|s| !s.contains("revalidate")));
}

#[test]
fn pipeline_renders_changed_when_severity_promotes() {
    let captured = baseline_snapshot();
    let mut current = captured.clone();
    current.snapshot_generation += 1;
    current.severity = Severity::Critical;

    let nq = ScriptedNqSource::new(vec![Some(captured), Some(current)]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts()).unwrap();

    assert!(packet.diagnosis.regime.starts_with("changed"));
    assert!(
        packet
            .proposed_action
            .risk_notes
            .iter()
            .any(|n| n.contains("severity")),
        "risk notes should surface which semantic field moved"
    );
}

// NB: there is no pipeline-level test for `Stale` in this slice.
// The pipeline today hard-codes `FindingAbsentForNGenerations { n: 1 }`
// on every capture, so an absent current finding always reconciles
// as `Invalidated`, never `Stale`. The other Stale path
// (`expires_at` on `Freshness`) is not surfaced by the agenda shape
// yet — that's a slice-6 concern when agendas can declare freshness
// windows. The reconciler's Stale logic is unit-tested in
// `src/reconciler.rs::tests::stale_when_expired` and
// `stale_when_finding_absent_without_absence_rule`. The pipeline's
// verdict-aware proposed_action for Stale is exercised by the
// `urgency_bumps_on_stale_evidence` unit test in `pipeline.rs`.

#[test]
fn pipeline_renders_invalidated_when_finding_disappears() {
    // Capture sees the baseline; by reconcile time, NQ no longer
    // has the finding. Pipeline must still produce a packet so the
    // disappearance is visible (per slice-5 ruling: silent
    // disappearance is the failure mode that turns into folklore).
    let captured = baseline_snapshot();
    let nq = ScriptedNqSource::new(vec![Some(captured), None]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts()).unwrap();

    let regime = &packet.diagnosis.regime;
    assert!(
        regime.starts_with("invalidated"),
        "expected invalidated regime; got {regime:?}"
    );
    assert!(
        packet
            .proposed_action
            .steps
            .iter()
            .any(|s| s.contains("no remediation proposed")),
        "invalidated proposal must explicitly refuse remediation; got {:?}",
        packet.proposed_action.steps
    );
    assert!(
        !packet.reconciliation_summary.ok_to_proceed,
        "invalidated input must mark ok_to_proceed=false"
    );

    // The run completed and the packet persisted — disappearance is
    // recorded, not swallowed.
    let stored = store.get_packet(&packet.run_id).unwrap().unwrap();
    assert_eq!(stored.packet_id, packet.packet_id);
}

#[test]
fn pipeline_renders_committed_cheap_path_when_evidence_is_byte_identical() {
    // Same snapshot returned every call → cheap path. Regime must
    // not say "churn-only"; it must say "byte-for-byte".
    let captured = baseline_snapshot();
    let nq = ScriptedNqSource::new(vec![Some(captured)]);
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts()).unwrap();

    assert!(packet.diagnosis.regime.contains("byte-for-byte"));
    // Sanity: this is just the existing committed-cheap path used by
    // the agenda_fixture suite. If this fails, the cheap path broke.
    assert_eq!(
        packet
            .reconciliation_summary
            .admissible_for_authorization
            .len(),
        1
    );
    let _: InputStatus = InputStatus::Committed; // import sanity
}
