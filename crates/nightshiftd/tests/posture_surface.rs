//! Operator-facing posture surface tests.
//!
//! Proves that, after a run persists through SqliteStore + run-ledger,
//! an operator can answer the constitutional questions from Nightshift
//! itself — without opening the SQLite file.

use std::path::PathBuf;

use nightshiftd::agenda::Agenda;
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::FixtureNqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
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
