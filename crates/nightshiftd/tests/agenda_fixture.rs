//! Integration tests for loading the shipped fixture agenda + NQ
//! manifest and running the v1 Watchbill pipeline end-to-end.

use std::path::PathBuf;

use nightshiftd::agenda::{Agenda, AuthorityLevel, CriticalityClass, WorkflowFamily};
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::FixtureNqSource;
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};

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

    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };

    let opts = PipelineOptions {
        no_governor: true,
        run_id: "run_test_001".into(),
    };

    let packet = run_watchbill(&agenda, &target, &nq, &opts).expect("pipeline must succeed");

    assert_eq!(packet.agenda_id, "wal-bloat-review");
    assert_eq!(packet.run_id, "run_test_001");
    assert_eq!(
        packet.proposed_action.requested_authority_level,
        AuthorityLevel::Advise
    );
    // --no-governor must have lowered governor_present flag
    assert!(!packet.authority_result.governor_present);

    // Attention key is the stable finding_key (per GAP-attention-state.md)
    assert_eq!(packet.attention.attention_key, target);

    // Reconciliation should succeed: same finding in manifest ≡ captured.
    assert!(packet.reconciliation_summary.ok_to_proceed);

    // Nothing mutated.
    assert!(matches!(
        packet.proposed_action.kind,
        nightshiftd::packet::ProposedActionKind::Advisory
    ));
    assert!(packet.proposed_action.reversible);
}
