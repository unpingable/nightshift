//! Integration test for the AttentionState::WatchUntil write path
//! and its cross-run expiry lineage.
//!
//! The horizon unit tests in `src/horizon.rs` prove the four-way
//! decision is correct as pure logic. The `horizon_cross_run.rs`
//! suite proves `process_horizon` + `apply_horizon_outcomes` interact
//! correctly with the store. This file closes the loop the
//! observatory-family 2026-04-23 hand-off named: does the **packet**
//! — the operator-facing artifact — carry the right Attention state
//! across the run boundary?
//!
//! The canonical failure mode: run A defers at t0 under horizon=4h;
//! run B at t0+5h must produce EscalateExpired (with lineage) and
//! NOT ActOnVerdict(Missing) (fresh-arrival). The write obligation
//! that keeps those apart is the tolerance record stored on Defer
//! — and, as of the WatchUntil slice, the packet Attention record
//! that mirrors it.
//!
//! Clock control: wall-clock time with a brief sleep between runs.
//! We pick expiry = now + ~150ms, run A captures + reconciles
//! immediately (acquired_at < expiry → Defer), then sleep 300ms
//! so run B's acquired_at crosses expiry (→ EscalateExpired).

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::governor_client::FixtureGovernorClient;
use nightshiftd::horizon::{HorizonBlock, HorizonClass};
use nightshiftd::horizon_policy::{FixtureHorizonPolicySource, HorizonDeclaration};
use nightshiftd::nq::NqSource;
use nightshiftd::packet::AttentionState;
use nightshiftd::pipeline::{
    capture_phase, reconcile_phase_with_horizon, CaptureOutcome, PipelineOptions,
};
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

fn target_key() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn baseline_snapshot() -> FindingSnapshot {
    FindingSnapshot {
        finding_key: target_key(),
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

/// Always returns the baseline snapshot. Each run makes two NQ calls
/// (capture + reconcile/acquire); both must succeed.
struct StableNqSource {
    snapshot: Mutex<FindingSnapshot>,
}

impl StableNqSource {
    fn new() -> Self {
        Self {
            snapshot: Mutex::new(baseline_snapshot()),
        }
    }
}

impl NqSource for StableNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(Some(self.snapshot.lock().unwrap().clone()))
    }
}

fn agenda_and_target() -> (Agenda, FindingKey) {
    let agenda = Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();
    (agenda, target_key())
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: None,
    }
}

const BASIS_ID: &str = "maintenance-window-042";
const BASIS_HASH: &str = "sha256:basis-abc";

fn policy_with_expiry(
    key: &FindingKey,
    expiry: chrono::DateTime<Utc>,
) -> FixtureHorizonPolicySource {
    FixtureHorizonPolicySource::from_declarations(vec![HorizonDeclaration {
        finding_key: key.clone(),
        horizon: HorizonBlock {
            class: HorizonClass::Hours,
            basis_id: Some(BASIS_ID.into()),
            basis_hash: Some(BASIS_HASH.into()),
            expiry: Some(expiry),
        },
    }])
}

/// Defer on run A → packet Attention = WatchUntil, carrying the
/// deadline T in `re_alert_after` and basis B in
/// `tolerance_basis_{id,hash}`. This is the state transition that
/// was missing before the slice landed.
#[test]
fn run_a_packet_carries_watch_until_with_deadline_and_basis() {
    let nq = StableNqSource::new();
    let store = SqliteStore::open_in_memory().unwrap();
    let governor = FixtureGovernorClient::new();
    let (agenda, target) = agenda_and_target();

    // Future expiry — run A will defer.
    let expiry = Utc::now() + chrono::Duration::hours(4);
    let policy = policy_with_expiry(&target, expiry);

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture unexpectedly held the run"),
    };

    let packet = reconcile_phase_with_horizon(
        &run_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts(),
    )
    .unwrap();

    assert_eq!(
        packet.attention.attention_state,
        AttentionState::WatchUntil,
        "Defer on target must promote Attention to WatchUntil"
    );
    assert_eq!(
        packet.attention.re_alert_after,
        Some(expiry),
        "re_alert_after carries the horizon expiry T"
    );
    assert_eq!(
        packet.attention.tolerance_basis_id.as_deref(),
        Some(BASIS_ID),
        "tolerance_basis_id carries basis B identity"
    );
    assert_eq!(
        packet.attention.tolerance_basis_hash.as_deref(),
        Some(BASIS_HASH),
        "tolerance_basis_hash carries basis B content hash"
    );

    // Store-level side effects.
    let tol = store.load_tolerance(&target).unwrap().expect("Defer writes tolerance record");
    assert_eq!(tol.basis_id, BASIS_ID);
    assert_eq!(tol.basis_hash, BASIS_HASH);
    assert_eq!(tol.expires_at, expiry);
    assert_eq!(tol.granted_in_run_id, run_id);

    // Governor received exactly one record_receipt for the deferral.
    assert_eq!(
        governor.call_count(),
        1,
        "Defer emits exactly one record_receipt"
    );
}

/// **The canonical cross-run test**: run A defers at t0 under
/// horizon=short-future; wall clock advances past expiry; run B at
/// t0+Δ correctly produces `expired_tolerance` — not
/// `fresh_arrival`. Shaped to the observatory-family 2026-04-23
/// hand-off spec:
///
/// > Build cross-run integration fixture: run A defers at t0 with
/// > horizon=hours, expiry=t0+4h; run B at t0+5h correctly produces
/// > expired_tolerance (not fresh_arrival).
///
/// We compress 4h / 5h to ~150ms / ~300ms so the test runs in
/// milliseconds, but the semantics are identical: run B's
/// `acquired_at` must cross the expiry the block declares, while
/// the block's basis is unchanged.
#[test]
fn run_b_produces_expired_tolerance_not_fresh_arrival() {
    let nq = StableNqSource::new();
    let store = SqliteStore::open_in_memory().unwrap();
    let governor = FixtureGovernorClient::new();
    let (agenda, target) = agenda_and_target();

    // Expiry far enough ahead that run A captures + reconciles before
    // it fires, but short enough that our sleep crosses it.
    let expiry = Utc::now() + chrono::Duration::milliseconds(150);
    let policy = policy_with_expiry(&target, expiry);

    // --- Run A: defer. ---
    let run_a_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("run A capture unexpectedly held"),
    };
    let packet_a = reconcile_phase_with_horizon(
        &run_a_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts(),
    )
    .unwrap();
    assert_eq!(
        packet_a.attention.attention_state,
        AttentionState::WatchUntil,
        "run A must defer under future-expiry horizon"
    );
    assert!(
        store.load_tolerance(&target).unwrap().is_some(),
        "run A must write a tolerance record the next run can find"
    );
    let receipts_after_a = governor.call_count();
    assert_eq!(receipts_after_a, 1, "run A Defer emits one receipt");

    // --- Clock crosses the expiry boundary. ---
    sleep(Duration::from_millis(300));
    assert!(
        Utc::now() > expiry,
        "sanity: wall clock must be past the horizon expiry before run B"
    );

    // --- Run B: same policy, same basis, past expiry → EscalateExpired. ---
    let run_b_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("run B capture unexpectedly held"),
    };
    let packet_b = reconcile_phase_with_horizon(
        &run_b_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts(),
    )
    .unwrap();

    // Packet state must reflect escalation, not deferral, and not
    // fresh arrival. WatchUntil fields must be absent.
    assert_eq!(
        packet_b.attention.attention_state,
        AttentionState::Unowned,
        "run B: EscalateExpired must clear WatchUntil from the packet Attention"
    );
    assert!(
        packet_b.attention.re_alert_after.is_none(),
        "run B: re_alert_after must not leak across an expired tolerance"
    );
    assert!(
        packet_b.attention.tolerance_basis_id.is_none(),
        "run B: tolerance_basis_id must not leak across an expired tolerance"
    );
    assert!(
        packet_b.attention.tolerance_basis_hash.is_none(),
        "run B: tolerance_basis_hash must not leak across an expired tolerance"
    );

    // Store-level proof that run B took the EscalateExpired path
    // (which clears) and not a Defer (which would rewrite) and not
    // ActOnVerdict(Missing) (which would leave the record
    // untouched). Tolerance record must now be absent.
    assert!(
        store.load_tolerance(&target).unwrap().is_none(),
        "run B: EscalateExpired must clear the consumed tolerance record"
    );

    // Governor receipt count must not advance on EscalateExpired —
    // Phase B.1 keeps escalation-record emission out of the
    // record_receipt path. If this count were 2, run B would have
    // taken the Defer path again. If it were 0, run A wouldn't have
    // written. 1 is the only healthy number.
    assert_eq!(
        governor.call_count(),
        1,
        "run B: EscalateExpired is silent in Phase B.1; only run A's Defer emitted"
    );
}

/// Negative control: without run A's tolerance record, the SAME
/// past-expiry horizon block on run B produces `ActOnVerdict
/// (ExpiredWithoutPrior)`, not `EscalateExpired` — Attention stays
/// at Unowned, tolerance stays empty, governor is untouched. This
/// is the "fresh arrival that arrived stale" symptom that the
/// write-obligation from run A specifically prevents.
#[test]
fn without_run_a_write_run_b_sees_expired_without_prior() {
    let nq = StableNqSource::new();
    let store = SqliteStore::open_in_memory().unwrap();
    let governor = FixtureGovernorClient::new();
    let (agenda, target) = agenda_and_target();

    // Expiry already in the past. No prior tolerance record exists
    // because we skip run A entirely.
    let expiry = Utc::now() - chrono::Duration::hours(1);
    let policy = policy_with_expiry(&target, expiry);
    assert!(
        store.load_tolerance(&target).unwrap().is_none(),
        "precondition: no prior tolerance record"
    );

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture unexpectedly held"),
    };
    let packet = reconcile_phase_with_horizon(
        &run_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts(),
    )
    .unwrap();

    // ActOnVerdict(ExpiredWithoutPrior) should leave Attention at
    // Unowned and record nothing on the tolerance / governor surfaces.
    assert_eq!(
        packet.attention.attention_state,
        AttentionState::Unowned,
        "ExpiredWithoutPrior must not produce a WatchUntil"
    );
    assert!(packet.attention.re_alert_after.is_none());
    assert!(packet.attention.tolerance_basis_id.is_none());
    assert!(packet.attention.tolerance_basis_hash.is_none());
    assert!(
        store.load_tolerance(&target).unwrap().is_none(),
        "ActOnVerdict must not write a tolerance record"
    );
    assert_eq!(
        governor.call_count(),
        0,
        "ActOnVerdict does not emit a record_receipt"
    );
}

/// Dogfood-back smoke test. Renders the Defer-path packet as YAML
/// so an operator can eyeball the result directly: is the
/// `attention_state: watch_until` row readable? Does
/// `re_alert_after` carry a sensible expiry? Are the
/// `tolerance_basis_*` fields populated? Ignored by default —
/// run with `cargo test --test horizon_packet_state -- --ignored
/// --nocapture` to see the rendered packet.
#[test]
#[ignore]
fn render_defer_packet_for_operator_inspection() {
    let nq = StableNqSource::new();
    let store = SqliteStore::open_in_memory().unwrap();
    let governor = FixtureGovernorClient::new();
    let (agenda, target) = agenda_and_target();

    let expiry = Utc::now() + chrono::Duration::hours(4);
    let policy = policy_with_expiry(&target, expiry);

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("held"),
    };
    let packet = reconcile_phase_with_horizon(
        &run_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts(),
    )
    .unwrap();

    let rendered = serde_yaml::to_string(&packet).unwrap();
    println!("--- Defer-path packet (operator view) ---");
    println!("{rendered}");
    println!("--- end ---");
}

/// Horizon wiring off: if the caller does not supply a
/// HorizonPolicySource + GovernorClient, the pipeline must behave
/// exactly as it did pre-horizon — packet.attention is Unowned,
/// tolerance store is untouched. This protects existing callers
/// (main.rs, legacy tests) that haven't been horizon-upgraded yet.
#[test]
fn horizon_disabled_preserves_pre_horizon_behavior() {
    let nq = StableNqSource::new();
    let store = SqliteStore::open_in_memory().unwrap();
    let (agenda, target) = agenda_and_target();

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture unexpectedly held"),
    };
    let packet =
        reconcile_phase_with_horizon(&run_id, &nq, None, None, &store, &opts()).unwrap();

    assert_eq!(packet.attention.attention_state, AttentionState::Unowned);
    assert!(packet.attention.re_alert_after.is_none());
    assert!(packet.attention.tolerance_basis_id.is_none());
    assert!(packet.attention.tolerance_basis_hash.is_none());
    assert!(
        store.load_tolerance(&target).unwrap().is_none(),
        "horizon phase skipped when policy/governor absent"
    );
}
