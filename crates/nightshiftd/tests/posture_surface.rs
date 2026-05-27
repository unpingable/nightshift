//! Operator-facing posture surface tests.
//!
//! Proves that, after a run persists through SqliteStore + run-ledger,
//! an operator can answer the constitutional questions from Nightshift
//! itself — without opening the SQLite file.

use std::path::PathBuf;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::liveness::FixtureLivenessSource;
use nightshiftd::nq::{FixtureNqSource, NqSource};
use nightshiftd::pipeline::{
    run_watchbill, run_watchbill_with_liveness, PipelineOptions,
};
use nightshiftd::posture::{list_postures, load_posture, render_list_row, render_show, PostureFilter};
use nightshiftd::store::sqlite::SqliteStore;

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

fn protected_target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "publisher_stale".into(),
        subject: "observatory-host:nq-publisher".into(),
    }
}

fn ordinary_target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn opts(continuity_configured: bool) -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured,
        trigger: None,
        liveness_threshold_seconds: None,
        imported_basis_freshness_window_seconds: None,
    }
}

#[test]
fn held_run_is_queryable_with_reason_and_finding_key() {
    // Load-bearing proof for slice 2:
    // after a protected-class agenda holds at preflight, the operator
    // can recover — from the Store alone — the run_id, the
    // target finding_key, the hold reason, and the ordered event
    // timeline. No SQLite archaeology required.
    let agenda_path = fixtures_dir().join("nq-publisher-protected.yaml");
    let manifest = fixtures_dir().join("nq-manifest.json");
    let agenda = Agenda::from_yaml_file(&agenda_path).unwrap();
    let nq = FixtureNqSource::load(&manifest).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let target = protected_target();
    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts(false)).unwrap();
    assert!(!packet.reconciliation_summary.ok_to_proceed);

    // List — the run shows up and is labeled HELD.
    let filter = PostureFilter::default();
    let postures = list_postures(&store, &filter).unwrap();
    assert_eq!(postures.len(), 1);
    let p = &postures[0];
    assert!(p.is_held(), "posture must report held");
    assert_eq!(p.status_label(), "HELD");

    // Reason carries the risky-class explanation.
    let reason = p.hold_reason().expect("held run must have a reason");
    assert!(
        reason.contains("protected-class service in scope"),
        "reason did not mention protected-class: {reason}"
    );

    // Target finding_key is recovered from the run row.
    assert_eq!(
        p.summary.target_finding_key.as_deref(),
        Some(target.as_string().as_str())
    );

    // Rendering for a list surface mentions HELD and the hold cause.
    let row = render_list_row(p);
    assert!(row.contains("HELD"), "list row missing HELD: {row}");
    assert!(row.contains("hold:"), "list row missing hold line: {row}");

    // Rendering for the detail surface includes the event timeline.
    let show = render_show(p);
    assert!(show.contains("run_captured"));
    assert!(show.contains("run_preflight_hold"));
    assert!(show.contains("run_completed"));
    assert!(
        !show.contains("run_reconciled"),
        "a held run must not render run_reconciled in its timeline: {show}"
    );
}

#[test]
fn protected_class_hold_is_visible_without_manual_sql() {
    // Same proof, stated as an operator workflow: after running both an
    // ordinary agenda (which clears preflight and reconciles) and a
    // protected-class agenda (which holds), the operator can ask the
    // store for held runs and get only the held one.
    let ordinary_agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    let protected_agenda = Agenda::from_yaml_file(&fixtures_dir().join("nq-publisher-protected.yaml")).unwrap();
    let nq = FixtureNqSource::load(fixtures_dir().join("nq-manifest.json")).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    run_watchbill(&ordinary_agenda, &ordinary_target(), &nq, &store, &opts(false)).unwrap();
    run_watchbill(
        &protected_agenda,
        &protected_target(),
        &nq,
        &store,
        &opts(false),
    )
    .unwrap();

    // all runs
    let all = list_postures(&store, &PostureFilter::default()).unwrap();
    assert_eq!(all.len(), 2);

    // held only
    let held = list_postures(
        &store,
        &PostureFilter {
            held_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(held.len(), 1, "exactly one run was held");
    assert_eq!(
        held[0].summary.agenda_id, "nq-publisher-watch",
        "the held run is the protected-class agenda"
    );
    assert!(held[0]
        .hold_reason()
        .unwrap()
        .contains("protected-class service in scope"));

    // `load_posture` round-trips a known run_id.
    let one = load_posture(&store, &held[0].summary.run_id).unwrap().unwrap();
    assert_eq!(one.summary.run_id, held[0].summary.run_id);
    assert!(one.is_held());
}

#[test]
fn list_filters_by_finding_key() {
    let ordinary_agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    let protected_agenda = Agenda::from_yaml_file(&fixtures_dir().join("nq-publisher-protected.yaml")).unwrap();
    let nq = FixtureNqSource::load(fixtures_dir().join("nq-manifest.json")).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    run_watchbill(&ordinary_agenda, &ordinary_target(), &nq, &store, &opts(false)).unwrap();
    run_watchbill(
        &protected_agenda,
        &protected_target(),
        &nq,
        &store,
        &opts(false),
    )
    .unwrap();

    let filtered = list_postures(
        &store,
        &PostureFilter {
            target_finding_key: Some(protected_target().as_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].summary.target_finding_key.as_deref(),
        Some(protected_target().as_string().as_str())
    );
}

// ---------------------------------------------------------------------
// Slice 2 — operator visibility surfaces
// ---------------------------------------------------------------------

/// NQ source that returns a fresh wal_bloat snapshot. Used by the
/// happy-path tests below.
struct StaticNqSource(FindingSnapshot);

impl NqSource for StaticNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(Some(self.0.clone()))
    }
}

fn baseline_snapshot() -> FindingSnapshot {
    FindingSnapshot {
        finding_key: ordinary_target(),
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 6,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 14, 32, 15).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: 39000,
        captured_at: Utc.with_ymd_and_hms(2026, 4, 17, 3, 0, 0).unwrap(),
        evidence_hash: String::new(),
        origin: None,
        silence: None,
    }
}

/// Canonical fresh-witness liveness DTO mirroring
/// `tests/liveness_pipeline.rs`. Kept inline so this file is
/// independently runnable.
fn fresh_liveness_dto() -> &'static str {
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

fn stale_liveness_dto() -> &'static str {
    fresh_liveness_dto()
        .replace("\"age_seconds\": 25", "\"age_seconds\": 600")
        .leak()
}

#[test]
fn protected_class_hold_names_preflight_gate() {
    // Slice 2 hold-cause taxonomy: a preflight-held run reports
    // `hold_gate() == Some("preflight")` and `render_show` renders
    // the gate label.
    let agenda = Agenda::from_yaml_file(&fixtures_dir().join("nq-publisher-protected.yaml")).unwrap();
    let nq = FixtureNqSource::load(fixtures_dir().join("nq-manifest.json")).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let _ = run_watchbill(&agenda, &protected_target(), &nq, &store, &opts(false)).unwrap();

    let postures = list_postures(&store, &PostureFilter::default()).unwrap();
    let p = &postures[0];
    assert!(p.is_held());
    assert_eq!(p.hold_gate(), Some("preflight"));
    let show = render_show(p);
    assert!(show.contains("hold gate:  preflight"), "missing preflight gate label: {show}");
    assert!(
        show.contains("protected-class service in scope"),
        "preflight reason not preserved: {show}"
    );
}

#[test]
fn liveness_failed_run_is_held_with_liveness_gate() {
    // Slice 2 — the operator surface must label liveness-failed
    // runs as HELD (not "ok") and identify the gate by name.
    let agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    let nq = StaticNqSource(baseline_snapshot());
    let liveness = FixtureLivenessSource::from_json(stale_liveness_dto()).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let opts = PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: Some(60),
        imported_basis_freshness_window_seconds: None,
    };
    let _ = run_watchbill_with_liveness(
        &agenda,
        &ordinary_target(),
        &nq,
        Some(&liveness),
        &store,
        &opts,
    )
    .unwrap();

    let postures = list_postures(&store, &PostureFilter::default()).unwrap();
    let p = &postures[0];
    assert!(p.is_held(), "liveness-failed run must report held");
    assert_eq!(p.status_label(), "HELD", "liveness fail must surface as HELD, not ok");
    assert_eq!(p.hold_gate(), Some("liveness"));
    let reason = p.hold_reason().expect("liveness fail must have a reason");
    assert!(
        reason.starts_with("liveness_gate:") || reason.contains("liveness"),
        "reason must name the liveness gate: {reason}"
    );
    let show = render_show(p);
    assert!(
        show.contains("hold gate:  liveness"),
        "render_show must label gate: {show}"
    );
}

#[test]
fn render_show_surfaces_attention_block_for_reconciled_run() {
    // Slice 2 — a normal reconciled packet renders the attention
    // block (state + posture + proposed) so the operator can scan
    // urgency / next-action without parsing YAML.
    let agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    let nq = FixtureNqSource::load(fixtures_dir().join("nq-manifest.json")).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let _ = run_watchbill(&agenda, &ordinary_target(), &nq, &store, &opts(false)).unwrap();

    let postures = list_postures(&store, &PostureFilter::default()).unwrap();
    let show = render_show(&postures[0]);

    assert!(show.contains("attention:"), "render_show must surface attention state: {show}");
    assert!(show.contains("posture:"), "render_show must surface posture class: {show}");
    assert!(show.contains("proposed:"), "render_show must surface proposed action kind: {show}");
    // Optional fields are absent on this happy-path packet, so they
    // must NOT appear (avoid empty noise lines).
    assert!(
        !show.contains("watch basis:"),
        "render_show must not render absent optional fields: {show}"
    );
    assert!(
        !show.contains("next check:"),
        "render_show must not render absent next-check: {show}"
    );
    let load_again = load_posture(&store, &postures[0].summary.run_id).unwrap().unwrap();
    assert_eq!(load_again.hold_gate(), None, "a reconciled run has no hold gate");
}
