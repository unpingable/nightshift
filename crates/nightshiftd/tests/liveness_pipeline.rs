//! Pipeline-level integration tests for the NQ liveness gate.
//!
//! Covers the gate's three operator-facing behaviors:
//!   1. Fresh witness → gate clears, run proceeds, ledger records
//!      `RunLivenessGateCleared`.
//!   2. Stale witness → run halts with a Stale-shape packet,
//!      revalidate-only proposal, ledger records
//!      `RunLivenessGateFailed`. NQ finding source is not consulted.
//!   3. Skewed witness (negative `age_seconds`) → run halts the same
//!      way regardless of the upstream `fresh: true` flag (per the
//!      clock-skew wrinkle nq-claude flagged 2026-04-20).

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::ledger::RunLedgerEventKind;
use nightshiftd::liveness::FixtureLivenessSource;
use nightshiftd::nq::NqSource;
use nightshiftd::pipeline::{run_watchbill_with_liveness, PipelineOptions};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::{RunFilter, Store};

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

/// NQ source that records every `snapshot()` call. Used to assert
/// that the liveness gate prevents NQ from being consulted when the
/// gate fails.
struct CountingNqSource {
    inner: FindingSnapshot,
    calls: Mutex<u32>,
}

impl CountingNqSource {
    fn new(snap: FindingSnapshot) -> Self {
        Self {
            inner: snap,
            calls: Mutex::new(0),
        }
    }

    fn call_count(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}

impl NqSource for CountingNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        *self.calls.lock().unwrap() += 1;
        Ok(Some(self.inner.clone()))
    }
}

fn fresh_dto() -> &'static str {
    r#"{
        "schema": "nq.liveness_snapshot.v1",
        "contract_version": 1,
        "instance_id": "labelwatch-host",
        "witness": {
            "generation_id": 43755,
            "generated_at": "2026-04-20T17:38:17.064301118Z",
            "schema_version": 29,
            "status": "ok",
            "findings_observed": 9,
            "findings_suppressed": 0,
            "detectors_run": 3,
            "liveness_format_version": 1
        },
        "freshness": {
            "age_seconds": 25,
            "stale_threshold_seconds": null,
            "fresh": null
        },
        "source": {
            "artifact_path": "/opt/notquery/liveness.json",
            "artifact_kind": "file"
        },
        "export": {
            "exported_at": "2026-04-20T17:38:42.546651838Z",
            "source": "nq",
            "contract_version": 1
        }
    }"#
}

fn stale_dto() -> &'static str {
    // age_seconds: 600 → comfortably stale at any reasonable threshold.
    fresh_dto().replace("\"age_seconds\": 25", "\"age_seconds\": 600").leak()
}

fn skewed_dto() -> &'static str {
    // Negative age_seconds + upstream fresh: true (the wrinkle).
    let s = fresh_dto()
        .replace("\"age_seconds\": 25", "\"age_seconds\": -116750929")
        .replace("\"fresh\": null", "\"fresh\": true");
    Box::leak(s.into_boxed_str())
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

fn opts(threshold: u64) -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: Some(threshold),
    }
}

#[test]
fn pipeline_proceeds_when_liveness_is_fresh() {
    let nq = CountingNqSource::new(baseline_snapshot());
    let liveness = FixtureLivenessSource::from_json(fresh_dto()).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet =
        run_watchbill_with_liveness(&agenda, &target, &nq, Some(&liveness), &store, &opts(60))
            .unwrap();

    // NQ was consulted (capture + reconcile + packet current_snapshot).
    assert!(
        nq.call_count() >= 2,
        "fresh liveness must allow NQ consultation; got {} calls",
        nq.call_count()
    );
    // Packet is a normal Committed shape, not a Stale gate failure.
    assert!(
        packet.diagnosis.regime.starts_with("committed"),
        "got regime {:?}",
        packet.diagnosis.regime
    );

    let runs = store
        .list_runs(RunFilter {
            target_finding_key: Some(target.as_string()),
            ..Default::default()
        })
        .unwrap();
    let events: Vec<_> = store
        .list_events(&runs[0].run_id)
        .unwrap()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert!(
        events
            .iter()
            .any(|k| matches!(k, RunLedgerEventKind::RunLivenessGateCleared)),
        "missing RunLivenessGateCleared; events were {events:?}"
    );
}

#[test]
fn pipeline_halts_with_stale_packet_when_liveness_is_stale() {
    let nq = CountingNqSource::new(baseline_snapshot());
    let liveness = FixtureLivenessSource::from_json(stale_dto()).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet =
        run_watchbill_with_liveness(&agenda, &target, &nq, Some(&liveness), &store, &opts(60))
            .unwrap();

    // NQ MUST NOT be consulted — that is the load-bearing property
    // of the gate. If this fails, the gate let a dead-witness run
    // proceed to capture and the contract is broken.
    assert_eq!(
        nq.call_count(),
        0,
        "stale liveness must prevent NQ consultation; gate let through {} calls",
        nq.call_count()
    );

    assert!(
        packet.diagnosis.regime.starts_with("stale"),
        "expected a stale regime; got {:?}",
        packet.diagnosis.regime
    );
    assert!(
        packet
            .proposed_action
            .steps
            .iter()
            .any(|s| s.contains("revalidate")),
        "stale packet must propose revalidation; got steps {:?}",
        packet.proposed_action.steps
    );
    assert!(
        packet
            .proposed_action
            .steps
            .iter()
            .all(|s| !s.contains("restart")),
        "stale packet must not propose remediation; got {:?}",
        packet.proposed_action.steps
    );
    assert!(!packet.reconciliation_summary.ok_to_proceed);

    let runs = store
        .list_runs(RunFilter {
            target_finding_key: Some(target.as_string()),
            ..Default::default()
        })
        .unwrap();
    let events: Vec<_> = store
        .list_events(&runs[0].run_id)
        .unwrap()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert!(
        events
            .iter()
            .any(|k| matches!(k, RunLedgerEventKind::RunLivenessGateFailed)),
        "missing RunLivenessGateFailed; events were {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|k| matches!(k, RunLedgerEventKind::RunCaptured)),
        "no RunCaptured event should fire when the gate halts capture; events were {events:?}"
    );
    // No bundle persisted — nothing was captured.
    assert!(store.get_bundle(&runs[0].run_id).unwrap().is_none());
}

/// **Wrinkle test at pipeline level**.
///
/// The DTO carries `freshness.fresh: true` (per the upstream
/// `age.max(0)` clamp wrinkle) but `age_seconds` is negative. The
/// gate must NOT trust the upstream verdict — it must compute its
/// own and halt the run as Stale-shape.
#[test]
fn pipeline_halts_when_liveness_is_skewed_even_if_upstream_says_fresh() {
    let nq = CountingNqSource::new(baseline_snapshot());
    let liveness = FixtureLivenessSource::from_json(skewed_dto()).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet =
        run_watchbill_with_liveness(&agenda, &target, &nq, Some(&liveness), &store, &opts(60))
            .unwrap();

    assert_eq!(
        nq.call_count(),
        0,
        "skewed liveness must prevent NQ consultation regardless of upstream fresh flag"
    );
    assert!(packet.diagnosis.regime.starts_with("stale"));
    let evidence_text = packet.diagnosis.evidence.join("\n");
    assert!(
        evidence_text.contains("clock skew"),
        "skewed verdict must be explained in packet evidence; got {evidence_text:?}"
    );
}

/// Liveness is *optional*. When no source is supplied to the
/// pipeline, behavior is identical to runs without a gate. This
/// preserves the CLAUDE.md invariant that missing intelligence
/// dependencies do not raise OR lower authority by their absence.
#[test]
fn pipeline_runs_without_gate_when_no_liveness_source_supplied() {
    let nq = CountingNqSource::new(baseline_snapshot());
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let packet =
        run_watchbill_with_liveness(&agenda, &target, &nq, None, &store, &opts(60)).unwrap();

    assert!(packet.diagnosis.regime.starts_with("committed"));
    assert!(nq.call_count() >= 2);

    let runs = store
        .list_runs(RunFilter {
            target_finding_key: Some(target.as_string()),
            ..Default::default()
        })
        .unwrap();
    let events: Vec<_> = store
        .list_events(&runs[0].run_id)
        .unwrap()
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert!(
        !events
            .iter()
            .any(|k| matches!(k, RunLedgerEventKind::RunLivenessGateCleared)),
        "no liveness events should fire when no source is supplied"
    );
    assert!(!events
        .iter()
        .any(|k| matches!(k, RunLedgerEventKind::RunLivenessGateFailed)));
}
