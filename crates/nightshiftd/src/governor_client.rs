//! Governor client — the real authority-boundary RPC surface.
//!
//! Distinct from `horizon_policy.rs`: horizon is **producer-local**
//! (Night Shift declares), while this module is where Night Shift
//! forwards lifecycle events into Governor's receipt chain for
//! archival. Governor is the archivist; this module is the wire.
//!
//! # Phase B.1 scope
//!
//! Narrow trait with one method: `record_receipt`. This is the
//! horizon-audit path: when Night Shift defers under a horizon,
//! it emits a lifecycle event so Governor's receipt chain carries
//! the declaration. The other two NS adapter methods —
//! `check_policy` (action-gate eval) and `authorize_transition`
//! (authority-ladder promotion) — come in later commits when the
//! NS pipeline has real action-propose and promotion points to
//! wire them to. Today NS is capped at advise with no mutation,
//! so those calls have no natural firing sites.
//!
//! # Transport
//!
//! Two implementations ship:
//!
//! - `FixtureGovernorClient` — in-memory call recorder for tests.
//! - `JsonRpcGovernorClient` — speaks JSON-RPC 2.0 to the Governor
//!   daemon over a Unix socket with Content-Length-framed messages
//!   (per `agent_gov/src/governor/daemon.py:read_message` /
//!   `write_message`). One fresh connection per call — simple, no
//!   runtime, matches the low-volume deferral-event pattern. A
//!   persistent-connection variant can follow if call volume grows.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::errors::{NightShiftError, Result};
use crate::horizon::HorizonBlock;

/// Lifecycle event kind. Governor's frozen v1 closed enum per
/// `agent_gov/src/governor/nightshift_adapter.py:EventKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Authority-role receipt. An agenda moved up the authority
    /// ladder.
    #[serde(rename = "agenda.promoted")]
    AgendaPromoted,
    /// Authority-role receipt. An action was authorized. This is
    /// the horizon-bearing event Night Shift emits on deferral —
    /// NS authorized itself to continue observing under horizon.
    #[serde(rename = "action.authorized")]
    ActionAuthorized,
    /// Authority-role receipt. An action ran.
    #[serde(rename = "action.applied")]
    ActionApplied,
    /// Authority-role receipt. An action was denied.
    #[serde(rename = "action.denied")]
    ActionDenied,
    /// Measurement-role receipt. Post-action verification.
    #[serde(rename = "action.verified")]
    ActionVerified,
    /// Measurement-role receipt. An operator was paged.
    #[serde(rename = "escalation.paged")]
    EscalationPaged,
}

/// Request payload for `nightshift.record_receipt`.
///
/// Mirrors `agent_gov/src/governor/nightshift_adapter.py:RecordReceiptRequest`.
/// Governor validates every field; we model the same constraints here
/// so fixtures produce payloads the real daemon will accept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordReceiptRequest {
    pub event_kind: EventKind,
    pub run_id: String,
    pub agenda_id: String,
    /// NS-computed sha256:<64-hex>. Subject of the event — typically
    /// a hash of the finding identity.
    pub subject_hash: String,
    /// sha256:<64-hex>. Hash of the evidence backing the decision.
    pub evidence_hash: String,
    /// sha256:<64-hex>. Hash of the policy artifact that justified
    /// the decision.
    pub policy_hash: String,
    /// Authority level the event transitions FROM (for promotion
    /// events). Required for `agenda.promoted`; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_level: Option<String>,
    /// Authority level the event transitions TO (for promotion
    /// events). Required for `agenda.promoted`; None otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_level: Option<String>,
    /// Optional tolerability declaration. Present on events where
    /// NS is declaring a tolerance window (typically
    /// `action.authorized` for horizon-driven deferral).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<HorizonBlock>,
}

/// Response payload for `nightshift.record_receipt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordReceiptResponse {
    pub receipt_id: String,
    pub receipt_hash: String,
}

/// Trait for speaking to Governor. Phase B.1 exposes one method;
/// Phase B.2+ extends for `check_policy` and `authorize_transition`
/// without changing the existing surface.
pub trait GovernorClient: Send + Sync {
    fn record_receipt(&self, request: &RecordReceiptRequest) -> Result<RecordReceiptResponse>;
}

/// In-memory test fixture. Records every `record_receipt` call so
/// tests can assert on the emitted payload. Thread-safe for use
/// through `&dyn GovernorClient`.
pub struct FixtureGovernorClient {
    calls: Mutex<Vec<RecordReceiptRequest>>,
    /// Counter for generating synthetic receipt_ids. Monotonic per
    /// fixture instance so tests can predict values.
    counter: Mutex<u64>,
}

impl Default for FixtureGovernorClient {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureGovernorClient {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            counter: Mutex::new(0),
        }
    }

    /// Snapshot of every `record_receipt` call made so far, in
    /// insertion order.
    pub fn recorded_calls(&self) -> Vec<RecordReceiptRequest> {
        self.calls
            .lock()
            .expect("fixture call log never poisoned")
            .clone()
    }

    /// Convenience: total call count.
    pub fn call_count(&self) -> usize {
        self.calls
            .lock()
            .expect("fixture call log never poisoned")
            .len()
    }
}

impl GovernorClient for FixtureGovernorClient {
    fn record_receipt(&self, request: &RecordReceiptRequest) -> Result<RecordReceiptResponse> {
        let mut counter = self
            .counter
            .lock()
            .map_err(|e| NightShiftError::Store(format!("fixture counter poisoned: {e}")))?;
        *counter += 1;
        let n = *counter;
        drop(counter);
        self.calls
            .lock()
            .map_err(|e| NightShiftError::Store(format!("fixture call log poisoned: {e}")))?
            .push(request.clone());
        Ok(RecordReceiptResponse {
            receipt_id: format!("fixture_receipt_{n:04}"),
            receipt_hash: format!("sha256:{:064x}", n),
        })
    }
}

/// Default per-call socket timeout. Governor's adapter handlers
/// finish in sub-millisecond time; 5s is plenty and leaves room
/// for a stopped-the-world daemon to surface as a timeout rather
/// than hang Nightshift indefinitely.
pub const DEFAULT_RPC_TIMEOUT_SECONDS: u64 = 5;

/// Real JSON-RPC 2.0 client speaking to the Governor daemon over a
/// Unix socket. One fresh `UnixStream` per call — no persistent
/// connection, no async runtime.
///
/// Wire framing is `Content-Length: <N>\r\n\r\n<body>` (LSP-style,
/// matching `agent_gov/src/governor/daemon.py:read_message`).
/// Responses are JSON-RPC 2.0 `{"jsonrpc", "id", "result" | "error"}`.
pub struct JsonRpcGovernorClient {
    socket_path: PathBuf,
    next_id: AtomicU64,
    timeout: Duration,
}

impl JsonRpcGovernorClient {
    /// Create a client bound to a Unix socket path. The path is
    /// resolved lazily on each call — `UnixStream::connect` is what
    /// fails if the daemon isn't running, not construction.
    pub fn new<P: Into<PathBuf>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.into(),
            next_id: AtomicU64::new(1),
            timeout: Duration::from_secs(DEFAULT_RPC_TIMEOUT_SECONDS),
        }
    }

    /// Override the default per-call timeout. Applies to both
    /// connect and read/write.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    fn allocate_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Perform one JSON-RPC call. Opens a connection, writes the
    /// request, reads the response, closes. On a JSON-RPC error
    /// response, returns `Err` carrying the error code + message.
    fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.allocate_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let stream = UnixStream::connect(&self.socket_path).map_err(|e| {
            NightShiftError::Store(format!(
                "connect to governor socket {}: {e}",
                self.socket_path.display()
            ))
        })?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| NightShiftError::Store(format!("set read timeout: {e}")))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|e| NightShiftError::Store(format!("set write timeout: {e}")))?;

        write_frame(&stream, &request)?;
        let response = read_frame(&stream)?;

        // Validate response shape.
        if response.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(NightShiftError::Store(format!(
                "governor response missing jsonrpc 2.0: {response}"
            )));
        }
        if let Some(err) = response.get("error") {
            let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
            let message = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("<no message>");
            return Err(NightShiftError::Store(format!(
                "governor rpc {method} failed (code {code}): {message}"
            )));
        }
        response.get("result").cloned().ok_or_else(|| {
            NightShiftError::Store(format!(
                "governor response has neither result nor error: {response}"
            ))
        })
    }
}

impl GovernorClient for JsonRpcGovernorClient {
    fn record_receipt(&self, request: &RecordReceiptRequest) -> Result<RecordReceiptResponse> {
        let params = serde_json::to_value(request)?;
        let result = self.call("nightshift.record_receipt", params)?;
        let resp: RecordReceiptResponse = serde_json::from_value(result).map_err(|e| {
            NightShiftError::Store(format!("parse record_receipt response: {e}"))
        })?;
        Ok(resp)
    }
}

/// Write a Content-Length-framed JSON-RPC message to a writable
/// stream. Matches `agent_gov/src/governor/daemon.py:write_message`.
fn write_frame<W: Write>(mut writer: W, msg: &serde_json::Value) -> Result<()> {
    let body = serde_json::to_vec(msg)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer
        .write_all(header.as_bytes())
        .map_err(|e| NightShiftError::Store(format!("write rpc header: {e}")))?;
    writer
        .write_all(&body)
        .map_err(|e| NightShiftError::Store(format!("write rpc body: {e}")))?;
    writer
        .flush()
        .map_err(|e| NightShiftError::Store(format!("flush rpc: {e}")))?;
    Ok(())
}

/// Read a Content-Length-framed JSON-RPC message. Matches
/// `agent_gov/src/governor/daemon.py:read_message`.
fn read_frame<R: Read>(reader: R) -> Result<serde_json::Value> {
    let mut buf = BufReader::new(reader);
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = buf
            .read_line(&mut line)
            .map_err(|e| NightShiftError::Store(format!("read rpc header line: {e}")))?;
        if n == 0 {
            return Err(NightShiftError::Store(
                "governor closed connection before response".into(),
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("Content-Length") {
                content_length = Some(value.trim().parse().map_err(|e| {
                    NightShiftError::Store(format!(
                        "parse Content-Length {value:?}: {e}"
                    ))
                })?);
            }
        }
    }
    let len = content_length.ok_or_else(|| {
        NightShiftError::Store("governor response missing Content-Length header".into())
    })?;
    let mut body = vec![0u8; len];
    buf.read_exact(&mut body)
        .map_err(|e| NightShiftError::Store(format!("read rpc body: {e}")))?;
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::HorizonClass;

    fn basic_request(event_kind: EventKind) -> RecordReceiptRequest {
        RecordReceiptRequest {
            event_kind,
            run_id: "run_abc".into(),
            agenda_id: "wal-bloat-review".into(),
            subject_hash: format!("sha256:{}", "a".repeat(64)),
            evidence_hash: format!("sha256:{}", "b".repeat(64)),
            policy_hash: format!("sha256:{}", "c".repeat(64)),
            from_level: None,
            to_level: None,
            horizon: None,
        }
    }

    #[test]
    fn event_kind_serializes_as_dotted_wire_value() {
        let s = serde_json::to_string(&EventKind::ActionAuthorized).unwrap();
        assert_eq!(s, "\"action.authorized\"");
        let s = serde_json::to_string(&EventKind::AgendaPromoted).unwrap();
        assert_eq!(s, "\"agenda.promoted\"");
        let s = serde_json::to_string(&EventKind::EscalationPaged).unwrap();
        assert_eq!(s, "\"escalation.paged\"");
    }

    #[test]
    fn request_round_trips_through_json() {
        let r = RecordReceiptRequest {
            horizon: Some(HorizonBlock {
                class: HorizonClass::Hours,
                basis_id: Some("policy:defer".into()),
                basis_hash: Some(format!("sha256:{}", "d".repeat(64))),
                expiry: Some("2026-04-24T03:00:00Z".parse().unwrap()),
            }),
            ..basic_request(EventKind::ActionAuthorized)
        };
        let s = serde_json::to_string(&r).unwrap();
        // Governor expects horizon class on the wire as "kind", not "class".
        assert!(s.contains("\"kind\":\"hours\""), "wire must use kind: {s}");
        assert!(s.contains("\"event_kind\":\"action.authorized\""));
        let r2: RecordReceiptRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(r2.event_kind, EventKind::ActionAuthorized);
        assert_eq!(r2.horizon.unwrap().class, HorizonClass::Hours);
    }

    #[test]
    fn from_to_level_omitted_when_none() {
        let r = basic_request(EventKind::ActionAuthorized);
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("from_level"));
        assert!(!s.contains("to_level"));
    }

    #[test]
    fn horizon_omitted_when_none() {
        let r = basic_request(EventKind::ActionAuthorized);
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("horizon"));
    }

    #[test]
    fn fixture_client_records_every_call_in_order() {
        let client = FixtureGovernorClient::new();
        let r1 = basic_request(EventKind::ActionAuthorized);
        let r2 = basic_request(EventKind::EscalationPaged);
        let resp1 = client.record_receipt(&r1).unwrap();
        let resp2 = client.record_receipt(&r2).unwrap();
        assert_ne!(resp1.receipt_id, resp2.receipt_id);
        assert_eq!(client.call_count(), 2);
        let calls = client.recorded_calls();
        assert_eq!(calls[0].event_kind, EventKind::ActionAuthorized);
        assert_eq!(calls[1].event_kind, EventKind::EscalationPaged);
    }

    #[test]
    fn fixture_client_receipt_ids_are_monotonic() {
        let client = FixtureGovernorClient::new();
        let r = basic_request(EventKind::ActionAuthorized);
        let a = client.record_receipt(&r).unwrap().receipt_id;
        let b = client.record_receipt(&r).unwrap().receipt_id;
        assert_eq!(a, "fixture_receipt_0001");
        assert_eq!(b, "fixture_receipt_0002");
    }

    // ---- JsonRpcGovernorClient: mock-server round-trip tests ----

    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;
    use std::thread;

    /// Spawn a Unix-socket server on a unique path that accepts one
    /// connection, reads one Content-Length-framed JSON-RPC request,
    /// applies `handler` to produce a response frame, writes it,
    /// then returns. Sends the captured request down `tx` for the
    /// test to inspect.
    fn spawn_mock_server(
        socket_path: PathBuf,
        tx: mpsc::Sender<serde_json::Value>,
        handler: impl FnOnce(&serde_json::Value) -> serde_json::Value + Send + 'static,
    ) -> thread::JoinHandle<()> {
        let listener = UnixListener::bind(&socket_path).expect("bind mock unix socket");
        thread::spawn(move || {
            let (stream, _addr) = listener.accept().expect("accept");
            let request = read_frame(&stream).expect("read request frame");
            tx.send(request.clone()).expect("send captured request");
            let response = handler(&request);
            write_frame(&stream, &response).expect("write response frame");
        })
    }

    fn well_formed_horizon_request() -> RecordReceiptRequest {
        RecordReceiptRequest {
            horizon: Some(HorizonBlock {
                class: HorizonClass::Hours,
                basis_id: Some("policy:defer".into()),
                basis_hash: Some(format!("sha256:{}", "d".repeat(64))),
                expiry: Some("2026-04-24T03:00:00Z".parse().unwrap()),
            }),
            ..basic_request(EventKind::ActionAuthorized)
        }
    }

    #[test]
    fn json_rpc_client_round_trips_record_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("governor.sock");
        let (tx, rx) = mpsc::channel();
        let server = spawn_mock_server(socket_path.clone(), tx, |req| {
            // Echo a success response with a matching id.
            let id = req.get("id").cloned().unwrap_or(serde_json::json!(1));
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "receipt_id": "r_mock_001",
                    "receipt_hash": "sha256:mock-hash",
                }
            })
        });

        let client = JsonRpcGovernorClient::new(&socket_path);
        let request = well_formed_horizon_request();
        let resp = client.record_receipt(&request).expect("rpc must succeed");
        assert_eq!(resp.receipt_id, "r_mock_001");
        assert_eq!(resp.receipt_hash, "sha256:mock-hash");

        server.join().expect("server thread panicked");
        let captured = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(captured["jsonrpc"], "2.0");
        assert_eq!(captured["method"], "nightshift.record_receipt");
        assert_eq!(captured["params"]["event_kind"], "action.authorized");
        // Horizon class serializes as "kind" on the wire (Governor contract).
        assert_eq!(captured["params"]["horizon"]["kind"], "hours");
        assert_eq!(
            captured["params"]["horizon"]["basis_id"],
            "policy:defer"
        );
    }

    #[test]
    fn json_rpc_client_surfaces_governor_error_response() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("governor.sock");
        let (tx, _rx) = mpsc::channel();
        let server = spawn_mock_server(socket_path.clone(), tx, |req| {
            let id = req.get("id").cloned().unwrap_or(serde_json::json!(1));
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": "Invalid params: basis_hash must match 'sha256:<64 hex chars>'",
                }
            })
        });

        let client = JsonRpcGovernorClient::new(&socket_path);
        let request = well_formed_horizon_request();
        let err = client
            .record_receipt(&request)
            .expect_err("error response must surface as Err");
        let msg = format!("{err}");
        assert!(
            msg.contains("nightshift.record_receipt failed"),
            "error msg must name the method: {msg}"
        );
        assert!(
            msg.contains("-32602") && msg.contains("Invalid params"),
            "error msg must carry code + message: {msg}"
        );
        server.join().expect("server thread panicked");
    }

    #[test]
    fn json_rpc_client_fails_when_socket_missing() {
        let client = JsonRpcGovernorClient::new("/nonexistent/path/governor.sock");
        let request = well_formed_horizon_request();
        let err = client
            .record_receipt(&request)
            .expect_err("missing socket must fail");
        let msg = format!("{err}");
        assert!(
            msg.contains("connect to governor socket"),
            "error msg must name the connect step: {msg}"
        );
    }

    #[test]
    fn json_rpc_client_allocates_monotonic_ids() {
        // A single client allocates ids monotonically across calls.
        // Verified by calling the `allocate_id` method directly
        // (avoids the awkwardness of re-targeting a client at a
        // second socket after the first listener closes).
        let client = JsonRpcGovernorClient::new("/does/not/matter");
        assert_eq!(client.allocate_id(), 1);
        assert_eq!(client.allocate_id(), 2);
        assert_eq!(client.allocate_id(), 3);
    }
}
