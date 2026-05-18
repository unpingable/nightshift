//! V1.2 integration acceptance: Night Shift consumes NQ's shipped V1
//! finding-export wire surface end-to-end.
//!
//! Acceptance bar (NQ FINDING_EXPORT V1 #11): "Night Shift can run
//! `nightshift watchbill` against this surface end-to-end — fetch,
//! capture, reconcile, emit packet — without reading any NQ internal
//! table directly."
//!
//! Two contracts proven here:
//!
//! 1. **Happy path** — given a real captured `nq findings export`
//!    JSONL line whose `admissibility.state == "observable"`, the
//!    pipeline runs capture → reconcile → packet without touching NQ
//!    internals. Subprocess boundary only.
//!
//! 2. **Refusal contract** — given a wire-shaped NQ finding whose
//!    `admissibility.state != "observable"`, Night Shift refuses it
//!    at parse time with a typed `NqInadmissible` error carrying the
//!    state and reason. The CLI's `--include-suppressed=false`
//!    default is not the gate; admissibility is.
//!
//! Fixtures:
//! - `nq-findings-observable.jsonl` — live capture (real evidence).
//! - `nq-findings-suppressed-derived.jsonl` — derived contract fixture
//!   (NOT live evidence — see `tests/fixtures/README.md`).

use std::path::PathBuf;

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::NightShiftError;
use nightshiftd::finding::FindingKey;
use nightshiftd::nq::{parse_nq_line, CliNqSource, NqSource};
use nightshiftd::pipeline::{capture_phase, reconcile_phase, CaptureOutcome, PipelineOptions};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::Store;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

/// Build a `CliNqSource` whose `findings export` invocation streams a
/// fixture JSONL file. Same call shape as production — Night Shift
/// shells out, parses JSONL stdout — but the upstream is a fixture
/// instead of a live `nq` binary. This is the cheapest way to drive
/// the full parse/translate/reconcile pipeline against captured wire
/// data.
fn fixture_backed_source(fixture: &str) -> CliNqSource {
    let path = fixtures_dir().join(fixture);
    let path_str = path
        .to_str()
        .expect("fixture path must be utf-8")
        .to_string();
    // `cat <abs-path>` ignores Night Shift's appended `findings export
    // --db ... --finding-key ...` args (they become positional args to
    // sh, unused). The trailing `--` separates them from the script.
    let script = format!("cat {path_str}");
    CliNqSource::new("/dev/null/placeholder.db").with_nq_argv(["/bin/sh", "-c", &script, "--"])
}

fn observable_target() -> FindingKey {
    // Matches the captured fixture: freelist_bloat detector on
    // labelwatch-host with a sqlite-path subject. `CliNqSource`
    // reconstructs NQ's canonical key from this and matches against
    // the JSONL row's `finding_key` field.
    FindingKey {
        source: "nq".into(),
        detector: "freelist_bloat".into(),
        subject: "labelwatch-host:/opt/driftwatch/deploy/data/facts_work.sqlite".into(),
    }
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: None,
        imported_basis_freshness_window_seconds: None,
    }
}

fn agenda() -> Agenda {
    Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap()
}

// -----------------------------------------------------------------------------
// 1. Happy-path acceptance
// -----------------------------------------------------------------------------

/// Capture + reconcile against a real captured V1 `nq findings export`
/// line. The fixture's admissibility is `observable`; the consumer
/// must accept it, build a snapshot, and produce a packet without
/// touching any NQ internal table.
#[test]
fn observable_finding_traverses_capture_reconcile_to_packet() {
    let nq = fixture_backed_source("nq-findings-observable.jsonl");
    let store = SqliteStore::open_in_memory().unwrap();
    let target = observable_target();

    let run_id = match capture_phase(&agenda(), &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(pkt) => {
            panic!("observable fixture must capture cleanly, got held packet: {pkt:?}")
        }
    };

    let packet = reconcile_phase(&run_id, &nq, &store, &opts()).unwrap();

    // Same fixture served at capture and reconcile → committed regime.
    assert!(
        packet.diagnosis.regime.starts_with("committed"),
        "expected committed regime for unchanged fixture, got: {}",
        packet.diagnosis.regime
    );
    assert!(
        packet.reconciliation_summary.ok_to_proceed,
        "committed regime must allow proceeding"
    );

    // Packet was persisted; the run is closed.
    let summary = store.get_run_summary(&run_id).unwrap().unwrap();
    assert!(
        summary.completed_at.is_some(),
        "reconciled run must be completed"
    );
    assert!(store.get_packet(&run_id).unwrap().is_some());
}

// -----------------------------------------------------------------------------
// 2. Refusal contract — parse level
// -----------------------------------------------------------------------------

/// Direct parse-level assertion: the suppressed-derived fixture
/// surfaces as `NqInadmissible` with the exact state and reason
/// fields preserved. This pins the typed-error shape so future
/// drift (e.g. someone replacing the variant with a generic
/// `InvalidAgenda`) breaks loudly.
#[test]
fn suppressed_finding_is_refused_at_parse_time_with_typed_error() {
    let path = fixtures_dir().join("nq-findings-suppressed-derived.jsonl");
    let raw = std::fs::read_to_string(&path).expect("fixture must exist");
    let line = raw.lines().next().expect("fixture must have one line");

    let err = parse_nq_line(line).expect_err("non-observable admissibility must refuse");

    match err {
        NightShiftError::NqInadmissible {
            finding_key,
            state,
            reason,
        } => {
            assert_eq!(state, "suppressed_by_ancestor", "state must be preserved");
            assert_eq!(
                reason, "testimony_dependency",
                "reason bucket must be preserved"
            );
            assert!(
                finding_key.contains("freelist_bloat"),
                "finding_key must identify the refused finding: {finding_key}"
            );
        }
        other => panic!("expected NqInadmissible, got: {other:?} — refusal contract drifted"),
    }
}

// -----------------------------------------------------------------------------
// 3. Refusal contract — end-to-end through CliNqSource
// -----------------------------------------------------------------------------

/// Refusal must propagate through the consumer's subprocess pipe, not
/// be silently dropped at the source layer. This is the full-pipe
/// twin of the parse-level test: same fixture, but driven through
/// `CliNqSource::snapshot` so the seam between source plumbing and
/// admissibility refusal is exercised end-to-end.
#[test]
fn suppressed_finding_refusal_surfaces_through_cli_source() {
    let nq = fixture_backed_source("nq-findings-suppressed-derived.jsonl");
    let target = observable_target(); // same finding_key, mutated admissibility only

    let err = nq
        .snapshot(&target)
        .expect_err("suppressed finding must surface as error, not Ok(None)");

    match err {
        NightShiftError::NqInadmissible { state, reason, .. } => {
            assert_eq!(state, "suppressed_by_ancestor");
            assert_eq!(reason, "testimony_dependency");
        }
        other => panic!("expected NqInadmissible to propagate through CliNqSource, got: {other:?}"),
    }
}

// -----------------------------------------------------------------------------
// 4. Slice B.1 — Imported Basis Freshness receipt on reconciliation result
// -----------------------------------------------------------------------------
//
// B.1 is observe-only: the pipeline calls `assess_freshness` between
// capture and packet, and the result lands on
// `bundle::ReconciliationResult.freshness`. The reconciler regime
// MUST NOT be steered by the freshness verdict at this slice —
// behavior is unchanged; the receipt is what changes.
//
// > "Make the clock seam visible before making it binding."

fn stale_producer_target() -> FindingKey {
    // Matches line 2 of `nq-findings-import-stale.jsonl`: the ingested
    // finding from `stale-corpus-extractor`. `CliNqSource` reconstructs
    // NQ's canonical key (`local/stale-corpus-extractor/...`) from this.
    FindingKey {
        source: "nq".into(),
        detector: "imported_corpus_health".into(),
        subject: "stale-corpus-extractor:claim:lemma-1/should_still_ingest".into(),
    }
}

fn fresh_import_target() -> FindingKey {
    // Matches line 1 of `nq-findings-import-clean.jsonl`.
    FindingKey {
        source: "nq".into(),
        detector: "imported_corpus_health".into(),
        subject: "synthetic-corpus-extractor:claim:lemma-42/missing_witness".into(),
    }
}

/// Run the full deferred pipeline (capture + reconcile) against a
/// fixture and return the persisted bundle's freshness receipt for
/// the single NQ input.
///
/// `window_seconds = None` ⇒ default. Tests pass an explicit
/// window when they want to control producer-time freshness relative
/// to wall-clock `now` (the fixtures' `producer_extraction_time`
/// values are fixed in JSON; tests use the window to bracket
/// fresh vs stale verdicts deterministically).
fn freshness_receipt_for_fixture(
    fixture: &str,
    target: FindingKey,
    window_seconds: Option<u64>,
) -> (
    nightshiftd::bundle::Bundle,
    nightshiftd::packet::Packet,
    Option<nightshiftd::freshness::FreshnessReceipt>,
) {
    let nq = fixture_backed_source(fixture);
    let store = SqliteStore::open_in_memory().unwrap();
    let mut o = opts();
    o.imported_basis_freshness_window_seconds = window_seconds;

    let run_id = match capture_phase(&agenda(), &target, &nq, None, &store, &o).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(pkt) => {
            panic!("fixture must capture cleanly, got held packet: {pkt:?}")
        }
    };
    let packet = reconcile_phase(&run_id, &nq, &store, &o).unwrap();
    let bundle = store
        .get_bundle(&run_id)
        .unwrap()
        .expect("reconciled bundle persisted");
    let receipt = bundle
        .reconciliation
        .as_ref()
        .and_then(|r| r.results.first())
        .and_then(|r| r.freshness.clone());
    (bundle, packet, receipt)
}

/// A fresh ingested finding produces a `fresh` receipt with
/// `producer_extraction_time` as the basis kind, both clocks visible,
/// and `import_lag_seconds` recorded. The window passed here
/// (one year) is wide enough to absorb the gap between the fixture's
/// fixed `producer_extraction_time` and the test runner's wall clock.
#[test]
fn b1_fresh_imported_finding_receipt_records_both_clocks() {
    let one_year_seconds: u64 = 365 * 24 * 3600;
    let (_bundle, packet, receipt) = freshness_receipt_for_fixture(
        "nq-findings-import-clean.jsonl",
        fresh_import_target(),
        Some(one_year_seconds),
    );
    let r = receipt.expect("B.1 must populate freshness receipt for ingested findings");
    assert_eq!(r.assessment, "fresh");
    assert_eq!(r.reason, "none");
    assert_eq!(r.freshness_basis.kind, "producer_extraction_time");
    assert!(r.freshness_basis.timestamp.is_some());
    assert_eq!(r.custody_basis.kind, "finding_snapshot.captured_at");
    assert!(r.custody_basis.timestamp.is_some());
    assert!(r.import_lag_seconds.is_some());

    // Behavior invariant: the regime is still the same shape it
    // would be without the freshness receipt.
    assert!(
        packet.diagnosis.regime.starts_with("committed"),
        "B.1 does not steer reconciliation; regime should still be committed for unchanged fixture, got {:?}",
        packet.diagnosis.regime
    );
}

/// **The B.1 killshot.** An ingested finding whose producer extracted
/// months ago lands a freshness receipt of `stale` with reason
/// `imported_producer_basis_stale` under the default window. The
/// reconciler regime is unaffected — visibility, not steering.
#[test]
fn b1_stale_producer_basis_surfaces_on_receipt_without_steering_regime() {
    let (_bundle, packet, receipt) = freshness_receipt_for_fixture(
        "nq-findings-import-stale.jsonl",
        stale_producer_target(),
        None, // default window (1h) — producer is 4+ months old, way past
    );
    let r = receipt.expect("freshness receipt must populate for stale-producer ingest");
    assert_eq!(
        r.assessment, "stale",
        "B.1 must report stale on the receipt when producer basis exceeds window"
    );
    assert_eq!(r.reason, "imported_producer_basis_stale");
    assert_eq!(r.freshness_basis.kind, "producer_extraction_time");
    assert!(
        r.import_lag_seconds.unwrap_or(0) > 1_000_000,
        "import lag is recorded for audit (millions of seconds for this fixture)"
    );

    // The headline observe-only invariant: regime is NOT mutated by
    // the freshness verdict. The capture and reconcile pass see the
    // same fixture line, so the regime is the existing
    // `committed` shape — unchanged by B.1.
    assert!(
        packet.diagnosis.regime.starts_with("committed"),
        "B.1 visibility-only: regime must NOT be steered by freshness verdict; got {:?}",
        packet.diagnosis.regime
    );
    assert!(
        packet.reconciliation_summary.ok_to_proceed,
        "B.1 must not block the run on the freshness verdict alone"
    );
}
