//! Integration tests for loading the shipped fixture agenda + NQ
//! manifest and running the v1 Watchbill pipeline end-to-end.

use std::path::PathBuf;

use nightshiftd::agenda::{Agenda, AuthorityLevel, CriticalityClass, WorkflowFamily};
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::FixtureNqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::{RunFilter, Store};

fn fixtures_dir() -> PathBuf {
    // Walk up to the repo root from the crate manifest dir.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

#[test]
fn fixture_agenda_parses_and_validates() {
    let path = fixtures_dir().join("wal-bloat-review.yaml");
    let agenda = Agenda::from_yaml_file(&path).expect("agenda must parse");
    assert_eq!(agenda.agenda_id, "wal-bloat-review");
    assert!(matches!(agenda.workflow_family, WorkflowFamily::Ops));
    assert!(matches!(agenda.promotion_ceiling, AuthorityLevel::Advise));
    assert!(matches!(
        agenda.criticality.class,
        CriticalityClass::Standard
    ));
}

#[test]
fn fixture_manifest_provides_target_finding() {
    let manifest = fixtures_dir().join("nq-manifest.json");
    let nq = FixtureNqSource::load(&manifest).expect("manifest must load");
    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };
    let snap = nightshiftd::nq::NqSource::snapshot(&nq, &target)
        .expect("snapshot call must succeed")
        .expect("fixture finding must exist");
    assert_eq!(snap.persistence_generations, 6);
}

#[test]
fn v1_pipeline_produces_advise_packet_without_governor() {
    let agenda_path = fixtures_dir().join("wal-bloat-review.yaml");
    let manifest = fixtures_dir().join("nq-manifest.json");
    let agenda = Agenda::from_yaml_file(&agenda_path).unwrap();
    let nq = FixtureNqSource::load(&manifest).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };

    let opts = PipelineOptions {
        no_governor: true,
        trigger: None,
    };

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts).expect("pipeline must succeed");

    assert_eq!(packet.agenda_id, "wal-bloat-review");
    assert_eq!(
        packet.proposed_action.requested_authority_level,
        AuthorityLevel::Advise
    );
    assert!(!packet.authority_result.governor_present);
    assert_eq!(packet.attention.attention_key, target);
    assert!(packet.reconciliation_summary.ok_to_proceed);
    assert!(matches!(
        packet.proposed_action.kind,
        nightshiftd::packet::ProposedActionKind::Advisory
    ));
    assert!(packet.proposed_action.reversible);
}

#[test]
fn same_finding_across_two_runs_persists_and_is_queryable() {
    let agenda_path = fixtures_dir().join("wal-bloat-review.yaml");
    let manifest = fixtures_dir().join("nq-manifest.json");
    let agenda = Agenda::from_yaml_file(&agenda_path).unwrap();
    let nq = FixtureNqSource::load(&manifest).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };

    let opts = PipelineOptions {
        no_governor: true,
        trigger: None,
    };

    let p1 = run_watchbill(&agenda, &target, &nq, &store, &opts).unwrap();
    let p2 = run_watchbill(&agenda, &target, &nq, &store, &opts).unwrap();
    assert_ne!(p1.run_id, p2.run_id, "each run must have a distinct run_id");

    // Both runs are queryable by the same stable finding_key.
    let runs_for_finding = store
        .list_runs(RunFilter {
            target_finding_key: Some(target.as_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(runs_for_finding.len(), 2);
    assert_eq!(
        runs_for_finding[0].target_finding_key,
        runs_for_finding[1].target_finding_key,
        "same finding_key persists across runs (per GAP-attention-state.md)"
    );

    // Each run produced the ledger events we expect.
    for r in &runs_for_finding {
        let events = store.list_events(&r.run_id).unwrap();
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert!(
            kinds
                .iter()
                .any(|k| matches!(k, nightshiftd::ledger::RunLedgerEventKind::RunCaptured)),
            "missing RunCaptured event for {}",
            r.run_id
        );
        assert!(
            kinds
                .iter()
                .any(|k| matches!(k, nightshiftd::ledger::RunLedgerEventKind::RunReconciled)),
            "missing RunReconciled event for {}",
            r.run_id
        );
        assert!(
            kinds
                .iter()
                .any(|k| matches!(k, nightshiftd::ledger::RunLedgerEventKind::RunCompleted)),
            "missing RunCompleted event for {}",
            r.run_id
        );
    }

    // Bundles and packets round-trip out of the store.
    for r in &runs_for_finding {
        let b = store.get_bundle(&r.run_id).unwrap().unwrap();
        assert_eq!(b.run_id, r.run_id);
        let p = store.get_packet(&r.run_id).unwrap().unwrap();
        assert_eq!(p.run_id, r.run_id);
    }
}
