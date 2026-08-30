use crate::model::{
    CapacityObservationV1, CapacityWindow, Confidence, ObservationDisposition, ObservationEvidence,
    SourceClass, WindowType, CAPACITY_OBSERVATION_SCHEMA_V1,
};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration as StdDuration;

const RAW_DOMAIN: &[u8] = b"nightshift.provider-capacity.raw-source/v1\0";

#[derive(Clone, Debug)]
pub struct CodexProbeOptions {
    pub account_profile_locator: String,
    pub observed_at: DateTime<Utc>,
    pub expires_after: Duration,
    pub timeout: StdDuration,
    pub maximum_response_bytes: usize,
}

/// Runs only the supported Codex App Server read method over a fresh bounded
/// stdio connection. It invokes no model turn and reads no configuration,
/// session, or credential file directly.
pub fn probe_codex_app_server(options: &CodexProbeOptions) -> CapacityObservationV1 {
    match collect_codex_response(options) {
        Ok(raw) => normalize_codex_response(&raw, options),
        Err(reason) => unknown_observation(&[], options, reason),
    }
}

fn collect_codex_response(options: &CodexProbeOptions) -> Result<Vec<u8>, String> {
    let mut child = Command::new("codex")
        .args(["app-server", "--listen", "stdio://"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "PROBE_PROCESS_UNAVAILABLE".to_string())?;

    let result = (|| {
        let mut input = child.stdin.take().ok_or("PROBE_STDIN_UNAVAILABLE")?;
        let output = child.stdout.take().ok_or("PROBE_STDOUT_UNAVAILABLE")?;
        let error = child.stderr.take().ok_or("PROBE_STDERR_UNAVAILABLE")?;
        let _error_drain = thread::spawn(move || {
            let _ = std::io::copy(&mut BufReader::new(error), &mut std::io::sink());
        });
        let requests = [
            serde_json::json!({"method":"initialize","id":1,"params":{"clientInfo":{
                "name":"nightshift_provider_capacity",
                "title":"Nightshift Provider Capacity Probe",
                "version":env!("CARGO_PKG_VERSION")
            },"capabilities":{"optOutNotificationMethods":["account/rateLimits/updated"]}}}),
            serde_json::json!({"method":"initialized","params":{}}),
            serde_json::json!({"method":"account/rateLimits/read","id":2}),
        ];
        for request in requests {
            serde_json::to_writer(&mut input, &request)
                .map_err(|_| "PROBE_REQUEST_ENCODE_FAILED")?;
            input
                .write_all(b"\n")
                .map_err(|_| "PROBE_REQUEST_WRITE_FAILED")?;
        }
        input.flush().map_err(|_| "PROBE_REQUEST_WRITE_FAILED")?;

        let (sender, receiver) = mpsc::channel();
        let maximum = options.maximum_response_bytes;
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            let mut total = 0usize;
            loop {
                let mut line = Vec::new();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) => {
                        let _ = sender.send(Err("PROBE_NO_OUTPUT".to_string()));
                        break;
                    }
                    Ok(_) => {
                        total = total.saturating_add(line.len());
                        if total > maximum {
                            let _ = sender.send(Err("PROBE_RESPONSE_OVERSIZED".to_string()));
                            break;
                        }
                        let target = serde_json::from_slice::<Value>(&line)
                            .ok()
                            .and_then(|value| value.get("id").and_then(Value::as_i64))
                            == Some(2);
                        if target {
                            let _ = sender.send(Ok(line));
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = sender.send(Err("PROBE_OUTPUT_READ_FAILED".to_string()));
                        break;
                    }
                }
            }
        });
        receiver
            .recv_timeout(options.timeout)
            .unwrap_or_else(|_| Err("PROBE_TIMEOUT".to_string()))
    })();

    let _ = child.kill();
    let _ = child.wait();
    result
}

pub fn normalize_codex_response(
    raw_response: &[u8],
    options: &CodexProbeOptions,
) -> CapacityObservationV1 {
    let parsed: Value = match serde_json::from_slice(raw_response) {
        Ok(value) => value,
        Err(_) => return unknown_observation(raw_response, options, "MALFORMED_RESPONSE"),
    };
    if parsed.get("id").and_then(Value::as_i64) != Some(2) {
        return unknown_observation(raw_response, options, "RESPONSE_ID_MISMATCH");
    }
    if parsed.get("error").is_some() {
        return unknown_observation(raw_response, options, "PROVIDER_READ_REFUSED");
    }
    let Some(snapshot) = parsed.pointer("/result/rateLimits") else {
        return unknown_observation(raw_response, options, "LAYOUT_UNRECOGNIZED");
    };

    let mut windows = Vec::new();
    for (name, pointer) in [("primary", "/primary"), ("secondary", "/secondary")] {
        let Some(window) = snapshot.pointer(pointer) else {
            continue;
        };
        if window.is_null() {
            continue;
        }
        let Some(used) = window.get("usedPercent").and_then(Value::as_f64) else {
            return unknown_observation(raw_response, options, "LAYOUT_UNRECOGNIZED");
        };
        if !used.is_finite() || !(0.0..=100.0).contains(&used) {
            return unknown_observation(raw_response, options, "IMPOSSIBLE_PERCENTAGE");
        }
        let duration = match window.get("windowDurationMins") {
            Some(Value::Number(number)) => match number.as_u64() {
                Some(value) if value > 0 => Some(value),
                _ => return unknown_observation(raw_response, options, "IMPOSSIBLE_WINDOW"),
            },
            Some(Value::Null) | None => None,
            _ => return unknown_observation(raw_response, options, "LAYOUT_UNRECOGNIZED"),
        };
        let resets_at = match window.get("resetsAt") {
            Some(Value::Number(number)) => {
                match number
                    .as_i64()
                    .and_then(|value| DateTime::from_timestamp(value, 0))
                {
                    Some(value) => Some(value),
                    None => {
                        return unknown_observation(raw_response, options, "IMPOSSIBLE_RESET");
                    }
                }
            }
            Some(Value::Null) | None => None,
            _ => return unknown_observation(raw_response, options, "LAYOUT_UNRECOGNIZED"),
        };
        let window_type = match duration {
            Some(300) => WindowType::FiveHour,
            Some(10_080) => WindowType::Weekly,
            _ => WindowType::ProviderDefined,
        };
        windows.push(CapacityWindow {
            window_id: name.to_string(),
            window_type,
            remaining_fraction: Some((100.0 - used) / 100.0),
            remaining_units: None,
            resets_at,
        });
    }
    if windows.is_empty() {
        return unknown_observation(raw_response, options, "NO_CAPACITY_WINDOWS");
    }
    if windows.len() == 2
        && windows[0].window_type == windows[1].window_type
        && windows[0].resets_at == windows[1].resets_at
        && windows[0].remaining_fraction != windows[1].remaining_fraction
    {
        return unknown_observation(raw_response, options, "CONTRADICTORY_WINDOWS");
    }
    make_observation(
        raw_response,
        options,
        Confidence::High,
        ObservationDisposition::Usable,
        Vec::new(),
        windows,
    )
}

pub fn unknown_observation(
    raw_response: &[u8],
    options: &CodexProbeOptions,
    reason: impl Into<String>,
) -> CapacityObservationV1 {
    make_observation(
        raw_response,
        options,
        Confidence::Unknown,
        ObservationDisposition::Unknown,
        vec![reason.into()],
        Vec::new(),
    )
}

fn make_observation(
    raw: &[u8],
    options: &CodexProbeOptions,
    confidence: Confidence,
    disposition: ObservationDisposition,
    unknown_reasons: Vec<String>,
    windows: Vec<CapacityWindow>,
) -> CapacityObservationV1 {
    let mut value = CapacityObservationV1 {
        schema: CAPACITY_OBSERVATION_SCHEMA_V1.to_string(),
        provider_id: "openai-codex".to_string(),
        account_profile_locator: options.account_profile_locator.clone(),
        model_family: None,
        observed_at: options.observed_at,
        expires_at: options.observed_at + options.expires_after,
        source_class: if disposition == ObservationDisposition::Usable {
            SourceClass::Observed
        } else {
            SourceClass::Unknown
        },
        confidence,
        disposition,
        unknown_reasons,
        windows,
        evidence: ObservationEvidence {
            probe_id: "codex-app-server-rate-limits-read-v1".to_string(),
            protocol_method: "account/rateLimits/read".to_string(),
            protocol_version: "codex-cli-0.147.0".to_string(),
            raw_source_digest: raw_digest(raw),
        },
        observation_digest: String::new(),
    };
    value.observation_digest = value.compute_digest().expect("observation serializes");
    value
}

fn raw_digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(RAW_DOMAIN);
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}
