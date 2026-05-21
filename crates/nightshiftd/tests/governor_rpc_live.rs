//! Live integration test against a real Governor daemon.
//!
//! Env-gated: runs only if `NIGHTSHIFT_GOVERNOR_SOCKET` points at a
//! running daemon's Unix socket. If the var is unset, the test
//! logs a skip note and returns. This is the operator-run proof
//! that Phase B.2's JSON-RPC transport lines up with the daemon
//! Nightshift will talk to in production.
//!
//! # Running the test manually
//!
//! 1. Start a Governor daemon:
//!    ```sh
//!    cd ~/git/agent_gov
//!    governor serve                # uses $XDG_RUNTIME_DIR/governor-<hash>.sock
//!    # or: governor serve --socket-path /tmp/gov-test.sock
//!    ```
//! 2. Run this test pointed at the socket:
//!    ```sh
//!    NIGHTSHIFT_GOVERNOR_SOCKET=/tmp/gov-test.sock \
//!      cargo test -p nightshiftd --test governor_rpc_live
//!    ```
//!
//! The test calls `nightshift.record_receipt` with a valid
//! `sha256:<64 hex>` payload and asserts the response carries
//! `receipt_id` and `receipt_hash` strings. Positive-path only —
//! the daemon's known `standing.py` shadow at `daemon.py:243`
//! breaks error paths, so we do not probe them.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::errors::Result as NsResult;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::governor_client::{
    EventKind, GovernorClient, JsonRpcGovernorClient, RecordReceiptRequest,
};
use nightshiftd::horizon::{HorizonBlock, HorizonClass};
use nightshiftd::horizon_policy::{FixtureHorizonPolicySource, HorizonDeclaration};
use nightshiftd::nq::NqSource;
use nightshiftd::packet::AttentionState;
use nightshiftd::pipeline::{
    capture_phase, reconcile_phase_with_horizon, CaptureOutcome, PipelineOptions,
};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::ledger::RunLedgerEventKind;
use nightshiftd::store::Store;

const SOCKET_ENV: &str = "NIGHTSHIFT_GOVERNOR_SOCKET";

fn governor_socket() -> Option<PathBuf> {
    std::env::var(SOCKET_ENV).ok().map(PathBuf::from)
}

fn sha256_of(suffix: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(suffix.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

#[test]
fn live_record_receipt_with_horizon_round_trips() {
    let Some(socket_path) = governor_socket() else {
        eprintln!(
            "skip: {SOCKET_ENV} not set — live test runs only when pointed at a real \
             Governor daemon. Start one with `governor serve` and re-run."
        );
        return;
    };

    let client = JsonRpcGovernorClient::new(&socket_path);

    // Well-formed request. Governor's `HorizonBlock.__post_init__`
    // enforces `sha256:<64 hex>` on basis_hash and the top-level
    // hashes, so every hash here is computed from real bytes.
    let request = RecordReceiptRequest {
        event_kind: EventKind::ActionAuthorized,
        run_id: "run_live_test".into(),
        agenda_id: "wal-bloat-review".into(),
        subject_hash: sha256_of("subject:labelwatch-host:/var/lib/labelwatch.sqlite"),
        evidence_hash: sha256_of("evidence:baseline"),
        policy_hash: sha256_of("policy:horizon-basis-abc"),
        from_level: None,
        to_level: None,
        horizon: Some(HorizonBlock {
            class: HorizonClass::Hours,
            basis_id: Some("policy:defer".into()),
            basis_hash: Some(sha256_of("basis:defer-001")),
            expiry: Some("2026-04-24T03:00:00Z".parse().unwrap()),
        }),
    };

    let response = client.record_receipt(&request).unwrap_or_else(|e| {
        panic!(
            "live record_receipt against {} failed: {e}",
            socket_path.display()
        )
    });
    assert!(
        !response.receipt_id.is_empty(),
        "live daemon must return a non-empty receipt_id, got {:?}",
        response.receipt_id
    );
    assert!(
        !response.receipt_hash.is_empty(),
        "live daemon must return a non-empty receipt_hash, got {:?}",
        response.receipt_hash
    );
    eprintln!(
        "live record_receipt OK → receipt_id={} receipt_hash={}",
        response.receipt_id, response.receipt_hash
    );
}

/// Always returns one fixed snapshot. Capture and reconcile phases
/// each call `snapshot` once; both must succeed and return the same
/// bytes so the reconciler reaches Committed and the horizon phase
/// runs through Defer.
struct StableNqSource {
    snapshot: Mutex<FindingSnapshot>,
}

impl StableNqSource {
    fn new(snapshot: FindingSnapshot) -> Self {
        Self {
            snapshot: Mutex::new(snapshot),
        }
    }
}

impl NqSource for StableNqSource {
    fn snapshot(&self, _key: &FindingKey) -> NsResult<Option<FindingSnapshot>> {
        Ok(Some(self.snapshot.lock().unwrap().clone()))
    }
}

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

/// End-to-end dogfood: drive `capture_phase` + `reconcile_phase_with_horizon`
/// against the **live Governor daemon**. Mirrors the canonical
/// `run_a_packet_carries_watch_until_with_deadline_and_basis` test in
/// `horizon_packet_state.rs`, but swaps the in-memory
/// `FixtureGovernorClient` for the real `JsonRpcGovernorClient`.
///
/// Acceptance: pipeline returns a packet whose Attention is
/// `WatchUntil` (proving the horizon phase ran), the live daemon
/// returned a `receipt_id`, and that `receipt_id` is now visible
/// on the packet's `receipt_references.governor_receipts` — the
/// pipe-through debt closed in commit 61a5789. Plus the run
/// ledger carries a `RunHorizonOutcome` event with the same
/// receipt_id so `runs show` exposes it in the timeline.
#[test]
fn live_horizon_defer_pipeline_round_trips() {
    let Some(socket_path) = governor_socket() else {
        eprintln!(
            "skip: {SOCKET_ENV} not set — live pipeline test runs only when pointed at a real \
             Governor daemon. Start one with `governor serve --socket <path>` and re-run."
        );
        return;
    };

    let target = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };
    let snapshot = FindingSnapshot {
        finding_key: target.clone(),
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
    };
    let nq = StableNqSource::new(snapshot);
    let store = SqliteStore::open_in_memory().unwrap();
    let governor = JsonRpcGovernorClient::new(&socket_path);
    let agenda =
        Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap();

    // Future-expiry horizon → Defer.
    let expiry = Utc::now() + chrono::Duration::hours(4);
    let basis_id = "maintenance-window-042";
    // Live daemon's `HorizonBlock.__post_init__` enforces
    // `sha256:<64 hex>`. The in-memory FixtureGovernorClient does not
    // validate, so canonical fixture tests pass with synthetic
    // strings — that fixture/real drift is part of the dogfood
    // findings.
    let basis_hash = sha256_of("basis:maintenance-window-042");
    let policy = FixtureHorizonPolicySource::from_declarations(vec![HorizonDeclaration {
        finding_key: target.clone(),
        horizon: HorizonBlock {
            class: HorizonClass::Hours,
            basis_id: Some(basis_id.into()),
            basis_hash: Some(basis_hash.clone()),
            expiry: Some(expiry),
        },
    }]);
    let opts = PipelineOptions {
        no_governor: false,
        ..Default::default()
    };

    let run_id = match capture_phase(&agenda, &target, &nq, None, &store, &opts).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("capture unexpectedly held the run"),
    };
    let packet = reconcile_phase_with_horizon(
        &run_id,
        &nq,
        Some(&policy),
        Some(&governor),
        &store,
        &opts,
    )
    .unwrap_or_else(|e| {
        panic!(
            "reconcile_phase_with_horizon against live daemon at {} failed: {e}",
            socket_path.display()
        )
    });

    // Defer must promote Attention to WatchUntil with the deadline +
    // basis, exactly as the in-memory canonical test asserts. If this
    // diverges from the FixtureGovernorClient path, the live wire is
    // not behaving like the in-memory contract.
    assert_eq!(
        packet.attention.attention_state,
        AttentionState::WatchUntil,
        "live Defer must promote Attention to WatchUntil"
    );
    assert_eq!(packet.attention.re_alert_after, Some(expiry));
    assert_eq!(packet.attention.tolerance_basis_id.as_deref(), Some(basis_id));
    assert_eq!(
        packet.attention.tolerance_basis_hash.as_deref(),
        Some(basis_hash.as_str())
    );

    // Tolerance record must be persisted so the next run sees it.
    let tol = store
        .load_tolerance(&target)
        .unwrap()
        .expect("Defer must write a tolerance record");
    assert_eq!(tol.basis_id, basis_id);
    assert_eq!(tol.basis_hash, basis_hash.as_str());
    assert_eq!(tol.expires_at, expiry);
    assert_eq!(tol.granted_in_run_id, run_id);

    // Pipe-through debt closed (commit 61a5789): the live daemon's
    // receipt_id must now reach the packet. Exactly one Defer fired
    // on the target finding, so exactly one receipt_id appears.
    assert_eq!(
        packet.receipt_references.governor_receipts.len(),
        1,
        "live Defer must surface the daemon's receipt_id on the packet"
    );
    let live_receipt_id = &packet.receipt_references.governor_receipts[0];
    assert!(
        !live_receipt_id.is_empty(),
        "live daemon receipt_id must be non-empty"
    );

    // The same receipt_id must appear in the run-ledger event so
    // `runs show` exposes it in the timeline (not only on the
    // packet). Verdict observable on both surfaces.
    let events = store.list_events(&run_id).unwrap();
    let horizon_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.kind, RunLedgerEventKind::RunHorizonOutcome))
        .collect();
    assert_eq!(
        horizon_events.len(),
        1,
        "live Defer must emit exactly one RunHorizonOutcome event"
    );
    assert_eq!(
        horizon_events[0]
            .payload
            .get("receipt_id")
            .and_then(|v| v.as_str()),
        Some(live_receipt_id.as_str()),
        "ledger receipt_id must equal packet receipt_id"
    );

    eprintln!(
        "live horizon-defer pipeline OK → run_id={run_id} attention_state=WatchUntil \
         receipt_id={live_receipt_id}"
    );
}
