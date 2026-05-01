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
