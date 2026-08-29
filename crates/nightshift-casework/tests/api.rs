use std::{net::SocketAddr, path::Path};

use chrono::{TimeZone, Utc};
use nightshift_casework::{
    load_runs_at,
    server::{bind_loopback, Api},
};

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../qualification/nightshift-packet-v1/velvet-orrery"
);

fn api() -> (Api, String, Vec<u8>, Vec<u8>) {
    let runs = load_runs_at(
        &[Path::new(GOLDEN).to_path_buf()],
        Utc.with_ymd_and_hms(2026, 8, 31, 0, 0, 0).unwrap(),
    )
    .unwrap();
    let run = runs.values().next().unwrap();
    (
        Api::new(runs.clone()),
        run.projection.run_id.clone(),
        run.packet_bytes.clone(),
        run.receipt_bytes.clone(),
    )
}

#[test]
fn only_exact_five_get_routes_are_reachable() {
    let (api, run_id, packet, receipts) = api();
    assert_eq!(api.response("GET", "/healthz").status, 200);
    assert_eq!(api.response("GET", "/api/v1/runs").status, 200);
    assert_eq!(
        api.response("GET", &format!("/api/v1/runs/{run_id}"))
            .status,
        200
    );
    assert_eq!(
        api.response("GET", &format!("/api/v1/runs/{run_id}/raw/packet"))
            .body,
        packet
    );
    assert_eq!(
        api.response("GET", &format!("/api/v1/runs/{run_id}/raw/receipts"))
            .body,
        receipts
    );
    for path in [
        "/",
        "/api/v1/runs/../raw/packet",
        "/api/v1/runs/%2e%2e/raw/packet",
        "/api/v1/runs/not-a-digest",
        &format!("/api/v1/runs/{run_id}/raw/packet/extra"),
        &format!("/api/v1/runs/{run_id}/evidence/arbitrary"),
    ] {
        assert_eq!(api.response("GET", path).status, 404, "{path}");
    }
}

#[test]
fn every_write_method_is_405_and_cannot_change_source_bytes() {
    let (api, run_id, packet, receipts) = api();
    for method in ["POST", "PUT", "PATCH", "DELETE"] {
        assert_eq!(
            api.response(method, &format!("/api/v1/runs/{run_id}"))
                .status,
            405,
            "{method}"
        );
    }
    assert_eq!(
        api.response("GET", &format!("/api/v1/runs/{run_id}/raw/packet"))
            .body,
        packet
    );
    assert_eq!(
        api.response("GET", &format!("/api/v1/runs/{run_id}/raw/receipts"))
            .body,
        receipts
    );
}

#[test]
fn non_loopback_bind_is_refused() {
    let address: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let error = bind_loopback(address).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn responses_have_etags_and_raw_content_type() {
    let (api, run_id, _, _) = api();
    let projection = api.response("GET", &format!("/api/v1/runs/{run_id}"));
    assert_eq!(projection.content_type, "application/json");
    assert!(projection.etag.is_some());
    let raw = api.response("GET", &format!("/api/v1/runs/{run_id}/raw/packet"));
    assert_eq!(raw.content_type, "application/json");
    assert!(raw.etag.is_some());
}
