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
        continuity_configured: false,
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
fn protected_class_agenda_without_continuity_holds_at_preflight() {
    // Load-bearing proof for commit D:
    //
    //   A protected-class agenda (observation-critical or
    //   control-plane-critical service in scope) is a risky class of
    //   work per GAP-parallel-ops.md. Without Continuity configured,
    //   preflight *must* hold the run before reconcile. This is the
    //   structural guarantee of CLAUDE.md invariant #18: coordination
    //   safety is not optional for risky classes.
    //
    // The packet must document the hold. No reconciliation happens.
    // No proposed action requests anything above `observe`.
    let agenda_path = fixtures_dir().join("nq-publisher-protected.yaml");
    let manifest = fixtures_dir().join("nq-manifest.json");
    let agenda = Agenda::from_yaml_file(&agenda_path).unwrap();
    let nq = FixtureNqSource::load(&manifest).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let target = FindingKey {
        source: "nq".into(),
        detector: "publisher_stale".into(),
        subject: "observatory-host:nq-publisher".into(),
    };

    let opts = PipelineOptions {
        no_governor: true,
        continuity_configured: false, // <— the condition that triggers hold
        trigger: None,
    };

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts).expect("hold path returns a packet");

    // Reconciliation did NOT succeed — the run was held.
    assert!(!packet.reconciliation_summary.ok_to_proceed);
    assert!(!packet.reconciliation_summary.blocked.is_empty());

    // Nothing was proposed above observe; nothing mutated.
    assert_eq!(
        packet.proposed_action.requested_authority_level,
        AuthorityLevel::Observe
    );
    assert!(matches!(
        packet.proposed_action.kind,
        nightshiftd::packet::ProposedActionKind::Advisory
    ));
    assert!(packet.proposed_action.reversible);
    assert!(packet.attention.silence_reason.is_some());

    // Ledger shows captured → preflight_hold → completed, in order,
    // with NO reconciled event.
    let runs = store.list_runs(RunFilter::default()).unwrap();
    assert_eq!(runs.len(), 1);
    let events = store.list_events(&runs[0].run_id).unwrap();
    let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();

    use nightshiftd::ledger::RunLedgerEventKind::*;
    assert!(kinds.iter().any(|k| matches!(k, RunCaptured)));
    assert!(
        kinds.iter().any(|k| matches!(k, RunPreflightHold)),
        "RunPreflightHold event missing; kinds were {:?}",
        kinds
    );
    assert!(
        !kinds.iter().any(|k| matches!(k, RunReconciled)),
        "a held run must not emit RunReconciled; kinds were {:?}",
        kinds
    );
    assert!(kinds.iter().any(|k| matches!(k, RunCompleted)));
}

#[test]
fn protected_class_agenda_with_continuity_clears_preflight() {
    // Mirror of the above: once Continuity is "configured" (v1 stub),
    // the risky agenda clears preflight and reconciliation proceeds.
    // Proves the hold is caused by the coordination substrate state,
    // not by the class itself.
    let agenda_path = fixtures_dir().join("nq-publisher-protected.yaml");
    let manifest = fixtures_dir().join("nq-manifest.json");
    let agenda = Agenda::from_yaml_file(&agenda_path).unwrap();
    let nq = FixtureNqSource::load(&manifest).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();

    let target = FindingKey {
        source: "nq".into(),
        detector: "publisher_stale".into(),
        subject: "observatory-host:nq-publisher".into(),
    };

    let opts = PipelineOptions {
        no_governor: true,
        continuity_configured: true,
        trigger: None,
    };

    let packet = run_watchbill(&agenda, &target, &nq, &store, &opts).unwrap();
    assert!(packet.reconciliation_summary.ok_to_proceed);

    let runs = store.list_runs(RunFilter::default()).unwrap();
    let events = store.list_events(&runs[0].run_id).unwrap();
    let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
    use nightshiftd::ledger::RunLedgerEventKind::*;
    assert!(kinds.iter().any(|k| matches!(k, RunPreflightCleared)));
    assert!(kinds.iter().any(|k| matches!(k, RunReconciled)));
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
        continuity_configured: false,
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
