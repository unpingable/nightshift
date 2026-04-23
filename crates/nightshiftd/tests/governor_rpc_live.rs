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

use nightshiftd::governor_client::{
    EventKind, GovernorClient, JsonRpcGovernorClient, RecordReceiptRequest,
};
use nightshiftd::horizon::{HorizonBlock, HorizonClass};

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
