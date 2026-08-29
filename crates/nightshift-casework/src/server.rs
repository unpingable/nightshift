use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::{LoadedRun, RunIndexEntryV1, RunIndexV1, RunSummaryV1};

const MAX_REQUEST_LINE_BYTES: u64 = 8 * 1024;
const MAX_HEADER_LINES: usize = 100;

#[derive(Clone, Debug)]
pub struct Api {
    runs: BTreeMap<String, LoadedRun>,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub etag: Option<String>,
    pub body: Vec<u8>,
}

impl Api {
    pub fn new(runs: BTreeMap<String, LoadedRun>) -> Self {
        Self { runs }
    }

    pub fn response(&self, method: &str, path: &str) -> Response {
        if method != "GET" {
            return Response::text(405, "Method Not Allowed", b"method not allowed\n".to_vec());
        }
        if path == "/healthz" {
            return Response::json(
                200,
                "OK",
                br#"{"status":"ok"}"#.to_vec(),
                Some(quoted_etag("healthz")),
            );
        }
        if path == "/api/v1/runs" {
            let index = RunIndexV1 {
                schema: "nightshift.casework-run-index/v1".to_owned(),
                runs: self.runs.values().map(index_entry).collect(),
            };
            return json_response(&index, None);
        }
        let Some((run_id, suffix)) = parse_run_path(path) else {
            return Response::text(404, "Not Found", b"not found\n".to_vec());
        };
        let Some(run) = self.runs.get(run_id) else {
            return Response::text(404, "Not Found", b"not found\n".to_vec());
        };
        match suffix {
            "" => json_response(
                &run.projection,
                Some(quoted_etag(&run.projection.projection_digest)),
            ),
            "/raw/packet" => Response::json(
                200,
                "OK",
                run.packet_bytes.clone(),
                Some(quoted_etag(&run.projection.packet.source_bytes_digest)),
            ),
            "/raw/receipts" => Response::json(
                200,
                "OK",
                run.receipt_bytes.clone(),
                Some(quoted_etag(&run.projection.receipts.source_bytes_digest)),
            ),
            _ => Response::text(404, "Not Found", b"not found\n".to_vec()),
        }
    }
}

impl Response {
    fn json(status: u16, reason: &'static str, body: Vec<u8>, etag: Option<String>) -> Self {
        Self {
            status,
            reason,
            content_type: "application/json",
            etag,
            body,
        }
    }

    fn text(status: u16, reason: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            etag: None,
            body,
        }
    }
}

pub fn bind_loopback(address: SocketAddr) -> std::io::Result<TcpListener> {
    if !address.ip().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "nightshift-casework refuses non-loopback binds",
        ));
    }
    TcpListener::bind(address)
}

pub fn serve(listener: TcpListener, api: Api) -> std::io::Result<()> {
    let local = listener.local_addr()?;
    if !local.ip().is_loopback() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "listener is not bound to loopback",
        ));
    }
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_stream(&mut stream, &api) {
                    eprintln!("nightshift-casework request error: {error}");
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn handle_stream(stream: &mut TcpStream, api: &Api) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader
        .by_ref()
        .take(MAX_REQUEST_LINE_BYTES)
        .read_line(&mut request_line)?;
    let mut parts = request_line.trim_end_matches(['\r', '\n']).split(' ');
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    let version = parts.next().unwrap_or("");
    let request_valid = !method.is_empty()
        && path.starts_with('/')
        && (version == "HTTP/1.0" || version == "HTTP/1.1")
        && parts.next().is_none();

    let mut headers_complete = false;
    for _ in 0..MAX_HEADER_LINES {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if line == "\r\n" || line == "\n" {
            headers_complete = true;
            break;
        }
    }
    let response = if request_valid && headers_complete {
        api.response(method, path)
    } else {
        Response::text(400, "Bad Request", b"bad request\n".to_vec())
    };
    write_response(stream, &response)
}

fn write_response(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n",
        response.status,
        response.reason,
        response.content_type,
        response.body.len()
    )?;
    if let Some(etag) = &response.etag {
        write!(stream, "ETag: {etag}\r\n")?;
    }
    write!(
        stream,
        "Allow: GET\r\nCache-Control: private, max-age=0, must-revalidate\r\nContent-Security-Policy: default-src 'self'; connect-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\nCross-Origin-Opener-Policy: same-origin\r\nCross-Origin-Resource-Policy: same-origin\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n\r\n"
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn parse_run_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/api/v1/runs/")?;
    let (run_id, suffix) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    if run_id.len() != 64
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return None;
    }
    Some((run_id, suffix))
}

fn index_entry(run: &LoadedRun) -> RunIndexEntryV1 {
    let projection = &run.projection;
    RunIndexEntryV1 {
        run_id: projection.run_id.clone(),
        projection_digest: projection.projection_digest.clone(),
        packet_id: projection.packet.packet_id.clone(),
        packet_digest: projection.packet.packet_digest.clone(),
        receipt_updated_at: projection.receipts.updated_at.clone(),
        summary: RunSummaryV1 {
            work_item_count: projection.summary.work_item_count,
            state_counts: projection.summary.state_counts.clone(),
            human_question_count: projection.summary.human_question_count,
            packet_custody_discrepancy_count: projection.summary.packet_custody_discrepancy_count,
        },
        packet_integrity: projection.packet.integrity.clone(),
        packet_currentness_at_receipt_snapshot: projection
            .packet
            .currentness_at_receipt_snapshot
            .clone(),
        packet_currentness_now: projection.packet.currentness_now.clone(),
    }
}

fn json_response(value: &impl Serialize, etag: Option<String>) -> Response {
    match serde_json::to_vec(value) {
        Ok(body) => {
            let etag = etag.or_else(|| Some(quoted_etag(&format!("{:x}", Sha256::digest(&body)))));
            Response::json(200, "OK", body, etag)
        }
        Err(_) => Response::text(
            500,
            "Internal Server Error",
            b"serialization failed\n".to_vec(),
        ),
    }
}

fn quoted_etag(identity: &str) -> String {
    format!(
        "\"{}\"",
        identity.strip_prefix("sha256:").unwrap_or(identity)
    )
}
