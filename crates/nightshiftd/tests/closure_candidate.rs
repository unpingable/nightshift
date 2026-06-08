//! Slice 4 acceptance — closure-candidate predicate exercised
//! end-to-end through the pipeline, not just in unit-test
//! isolation. Each test asserts that the persisted packet's
//! `closure_candidate` matches the refusal shape Gate 1 demands.
//!
//! Seven cases per `working/decisions/pre-positioned-doctrine-gates.md`
//! Gate 1 + the ChatGPT tuning that landed alongside this slice:
//!
//! 1. SilenceShape → not_eligible(proxy_quiet)
//! 2. InputStatus::Stale → not_eligible(stale_basis)
//! 3. Liveness/freshness failure → not_eligible(liveness_gate_failed)
//! 4. InputStatus::Invalidated → not_eligible(invalidated_basis)
//! 5. Active operator attention → not_eligible(operator_attention_active)
//! 6. IncidentShape without consequence-witness →
//!    unassessable_missing_consequence_witness
//! 7. **No case emits `EligibleForClosureReview`** — invariant
//!    sweep already lives in `src/closure.rs::tests`; this file
//!    confirms the pipeline-level path never produces it either.
//!
//! Closure-candidate is a *review-gating* surface. There is no
//! close verb; this slice ships refusal, not authorization.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{Duration, TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::attention::AttentionRow;
use nightshiftd::closure::{ClosureCandidate, NotEligibleReason};
use nightshiftd::errors::Result;
use nightshiftd::finding::{
    EvidenceState, FindingKey, FindingSilence, FindingSnapshot, Severity,
};
use nightshiftd::liveness::FixtureLivenessSource;
use nightshiftd::nq::NqSource;
use nightshiftd::pipeline::{
    run_watchbill, run_watchbill_with_liveness, PipelineOptions,
};
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

fn target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn protected_target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "publisher_stale".into(),
        subject: "observatory-host:nq-publisher".into(),
    }
}

fn baseline_snapshot() -> FindingSnapshot {
    FindingSnapshot {
        finding_key: target(),
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 6,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: 1,
        captured_at: Utc::now(),
        evidence_hash: String::new(),
        origin: None,
        silence: None,

        position: None,    }
}

fn silence_shape_snapshot() -> FindingSnapshot {
    let mut s = baseline_snapshot();
    s.silence = Some(FindingSilence {
        scope: "extraction".into(),
        basis: "age_threshold".into(),
        duration_s: 3600,
        expected: "none".into(),
    });
    s
}

struct ConstantNqSource(Mutex<FindingSnapshot>);

impl NqSource for ConstantNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(Some(self.0.lock().unwrap().clone()))
    }
}

/// A scripted NQ source that swaps the returned snapshot between
/// capture and reconcile. Used for the Invalidated test — capture
/// sees the finding, reconcile sees it absent → Slice 5 contract
/// routes that as Invalidated.
struct CaptureAbsentNqSource {
    captured: FindingSnapshot,
    calls: Mutex<u32>,
}

impl NqSource for CaptureAbsentNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        let mut n = self.calls.lock().unwrap();
        *n += 1;
        // First call (capture) sees the finding; subsequent calls
        // (reconcile current-snapshot) see it absent.
        if *n == 1 {
            Ok(Some(self.captured.clone()))
        } else {
            Ok(None)
        }
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

// ---------------------------------------------------------------------
// Test 1 — SilenceShape → proxy_quiet
// ---------------------------------------------------------------------

#[test]
fn silence_shape_finding_refuses_with_proxy_quiet() {
    let store = SqliteStore::open_in_memory().unwrap();
    let nq = ConstantNqSource(Mutex::new(silence_shape_snapshot()));
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::ProxyQuiet,
        }
    );
}

// ---------------------------------------------------------------------
// Test 2 — InputStatus::Stale (via imported-basis-stale) → stale_basis
// ---------------------------------------------------------------------

#[test]
fn imported_basis_stale_refuses_with_stale_basis() {
    use nightshiftd::finding::FindingOrigin;

    let store = SqliteStore::open_in_memory().unwrap();
    let mut snap = baseline_snapshot();
    // Mark as ingested from an import with an extraction time
    // outside the freshness window — Slice B routes this to
    // EvidenceState::Stale / InputStatus::Stale.
    snap.origin = Some(FindingOrigin {
        source: "import".into(),
        producer_id: "test-producer".into(),
        extraction_run_id: "run-2025-01-01".into(),
        producer_extraction_time: Some(
            Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
        ),
        producer_extraction_time_raw: Some("2025-01-01T00:00:00Z".into()),
        import_contract_version: 1,
    });
    let nq = ConstantNqSource(Mutex::new(snap));
    let opts = PipelineOptions {
        imported_basis_freshness_window_seconds: Some(60),
        ..opts()
    };
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::StaleBasis,
        },
        "stale-import basis must refuse closure; packet={:#?}",
        pkt.closure_candidate
    );
}

// ---------------------------------------------------------------------
// Test 3 — Liveness gate failure → liveness_gate_failed
// ---------------------------------------------------------------------

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

#[test]
fn liveness_gate_failure_refuses_with_liveness_gate_failed() {
    let stale_dto: &str = Box::leak(
        fresh_liveness_dto()
            .replace("\"age_seconds\": 25", "\"age_seconds\": 600")
            .into_boxed_str(),
    );
    let store = SqliteStore::open_in_memory().unwrap();
    let nq = ConstantNqSource(Mutex::new(baseline_snapshot()));
    let liveness = FixtureLivenessSource::from_json(stale_dto).unwrap();

    let opts = PipelineOptions {
        liveness_threshold_seconds: Some(60),
        ..opts()
    };
    let pkt = run_watchbill_with_liveness(
        &agenda(),
        &target(),
        &nq,
        Some(&liveness),
        &store,
        &opts,
    )
    .unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::LivenessGateFailed,
        }
    );
}

// ---------------------------------------------------------------------
// Test 4 — InputStatus::Invalidated → invalidated_basis
// ---------------------------------------------------------------------

#[test]
fn finding_invalidated_at_reconcile_refuses_with_invalidated_basis() {
    let store = SqliteStore::open_in_memory().unwrap();
    let nq = CaptureAbsentNqSource {
        captured: baseline_snapshot(),
        calls: Mutex::new(0),
    };
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::InvalidatedBasis,
        },
        "absent-at-reconcile must route to Invalidated; got {:#?}",
        pkt.closure_candidate
    );
}

// ---------------------------------------------------------------------
// Test 5 — Active operator attention → operator_attention_active
// ---------------------------------------------------------------------

#[test]
fn active_ack_refuses_with_operator_attention_active() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = ConstantNqSource(Mutex::new(baseline_snapshot()));

    store
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(Utc::now() + Duration::hours(4)),
            None,
            None,
        ))
        .unwrap();
    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::OperatorAttentionActive,
        }
    );
}

#[test]
fn active_silence_refuses_with_operator_attention_active_over_proxy_quiet() {
    // A silence-shape finding under active operator silence must
    // refuse on operator-attention grounds, not on proxy-quiet.
    // Operator intent wins. (Also exercised in closure unit tests;
    // here we prove the pipeline path agrees.)
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = ConstantNqSource(Mutex::new(silence_shape_snapshot()));

    store
        .save_attention(&AttentionRow::silence(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Utc::now() + Duration::hours(2),
            "rolling restart".into(),
        ))
        .unwrap();
    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::OperatorAttentionActive,
        }
    );
}

// ---------------------------------------------------------------------
// Test 6 — IncidentShape default → unassessable
// ---------------------------------------------------------------------

#[test]
fn incident_shape_with_no_blockers_yields_unassessable() {
    // Critical conservatism check: a healthy-looking, all-blockers-
    // absent IncidentShape finding does NOT round up to
    // eligible_for_closure_review. It rounds to Unassessable,
    // because NQ has no channel classification yet.
    let store = SqliteStore::open_in_memory().unwrap();
    let nq = ConstantNqSource(Mutex::new(baseline_snapshot()));
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::UnassessableMissingConsequenceWitness,
        "IncidentShape with no blockers must be Unassessable, not Eligible"
    );
}

// ---------------------------------------------------------------------
// Test 7 — Preflight held → preflight_held
// ---------------------------------------------------------------------

#[test]
fn preflight_held_run_refuses_with_preflight_held() {
    use nightshiftd::nq::FixtureNqSource;

    let agenda =
        Agenda::from_yaml_file(&fixtures_dir().join("nq-publisher-protected.yaml"))
            .unwrap();
    let nq = FixtureNqSource::load(fixtures_dir().join("nq-manifest.json")).unwrap();
    let store = SqliteStore::open_in_memory().unwrap();
    let pkt = run_watchbill(&agenda, &protected_target(), &nq, &store, &opts()).unwrap();

    assert_eq!(
        pkt.closure_candidate,
        ClosureCandidate::NotEligible {
            reason: NotEligibleReason::PreflightHeld,
        }
    );
}

// ---------------------------------------------------------------------
// Pipeline-level invariant: no run produces EligibleForClosureReview.
//
// The unit-test combinatorial sweep in `src/closure.rs::tests` covers
// all input states. This test confirms the wire path agrees: across
// every shipped scenario, the persisted packet's closure_candidate
// is never `EligibleForClosureReview`.
// ---------------------------------------------------------------------

#[test]
fn no_pipeline_path_emits_eligible_for_closure_review() {
    // Exercise a broad mix and assert none emit Eligible.
    let store = SqliteStore::open_in_memory().unwrap();

    // Baseline reconciled
    let nq = ConstantNqSource(Mutex::new(baseline_snapshot()));
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts()).unwrap();
    assert_ne!(pkt.closure_candidate, ClosureCandidate::EligibleForClosureReview);

    // SilenceShape
    let store = SqliteStore::open_in_memory().unwrap();
    let nq = ConstantNqSource(Mutex::new(silence_shape_snapshot()));
    let pkt = run_watchbill(&agenda(), &target(), &nq, &store, &opts()).unwrap();
    assert_ne!(pkt.closure_candidate, ClosureCandidate::EligibleForClosureReview);

    // Operator ack
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    store
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(Utc::now() + Duration::hours(4)),
            None,
            None,
        ))
        .unwrap();
    let nq = ConstantNqSource(Mutex::new(baseline_snapshot()));
    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    assert_ne!(pkt.closure_candidate, ClosureCandidate::EligibleForClosureReview);
}
