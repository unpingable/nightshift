//! NQ liveness consumer.
//!
//! NQ publishes a `liveness.json` artifact and exposes it via
//! `nq liveness export` as a canonical `LivenessSnapshot` (schema
//! `nq.liveness_snapshot.v1`). Night Shift consumes this surface to
//! answer one question before trusting any captured finding evidence:
//!
//! > Is the witness still witnessing?
//!
//! If liveness gating fails, the run does not consult the finding
//! source at all. The captured premise of any finding produced by a
//! dead or stuck NQ is itself stale. Per the slice-5 contract
//! (`docs/GAP-nq-nightshift-contract.md`), the resulting packet is
//! Stale-shape: revalidate-only, no remediation.
//!
//! ## Wrinkle: clock skew on the upstream side
//!
//! NQ's `freshness.fresh` field is computed inside `nq liveness
//! export` by clamping `age.max(0)` before threshold comparison.
//! That means a future-dated artifact (clock skew, time travel,
//! corruption) returns `fresh: true` with a negative `age_seconds`.
//! Auditable — but blindly trusting `fresh` would assert freshness
//! on impossible data.
//!
//! Night Shift therefore **does not consume `freshness.fresh`
//! directly**. We pull `freshness.age_seconds`, apply our own
//! threshold, and treat negative ages as `LivenessVerdict::Skewed`
//! (epistemic hole, not freshness). This sidesteps the wrinkle by
//! contract — independent of any future fix on the NQ side.
//!
//! See `project_liveness_consumer_pending.md` and nq-claude's
//! 2026-04-20 dogfood note.

use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{NightShiftError, Result};

pub const LIVENESS_SCHEMA: &str = "nq.liveness_snapshot.v1";
pub const LIVENESS_CONTRACT_VERSION: u32 = 1;

/// Default liveness staleness threshold when the operator does not
/// override. Picked at ~1.5x typical NQ scan cadence (~60s) so a
/// single missed scan does not panic the gate.
pub const DEFAULT_STALENESS_THRESHOLD_SECONDS: u64 = 90;

/// The NQ liveness DTO (schema `nq.liveness_snapshot.v1`).
///
/// Mirrors the wire format produced by `nq liveness export`. All
/// fields are public so consumers can record them in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivenessSnapshot {
    pub schema: String,
    pub contract_version: u32,
    pub instance_id: String,
    pub witness: WitnessRecord,
    pub freshness: FreshnessRecord,
    pub source: SourceRecord,
    pub export: ExportRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessRecord {
    pub generation_id: u64,
    pub generated_at: DateTime<Utc>,
    pub schema_version: u32,
    pub status: String,
    pub findings_observed: u32,
    pub findings_suppressed: u32,
    pub detectors_run: u32,
    pub liveness_format_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessRecord {
    pub age_seconds: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_threshold_seconds: Option<u64>,
    /// NQ's own freshness verdict. Night Shift **does not consume
    /// this directly** — see module docs (clock-skew wrinkle). Kept
    /// in the DTO for round-trip fidelity and ledger recording only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fresh: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub artifact_path: String,
    pub artifact_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRecord {
    pub exported_at: DateTime<Utc>,
    pub source: String,
    pub contract_version: u32,
}

/// Night Shift's own freshness verdict on a liveness snapshot, with
/// the upstream `fresh: bool` flag explicitly *not* trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivenessVerdict {
    /// Age is non-negative and within the threshold. Source is
    /// witnessing recently enough to trust further evidence from it.
    Fresh,
    /// Age exceeds the threshold. The witness is silent or stuck.
    Stale {
        age_seconds: i64,
        threshold_seconds: u64,
    },
    /// Age is negative — the artifact is timestamped in the future
    /// relative to wall-clock now. This is an epistemic hole, not
    /// freshness; the consumer cannot decide whether the witness is
    /// alive without resolving the skew first.
    Skewed { age_seconds: i64 },
}

impl LivenessVerdict {
    pub fn is_fresh(&self) -> bool {
        matches!(self, LivenessVerdict::Fresh)
    }

    pub fn explain(&self) -> String {
        match self {
            LivenessVerdict::Fresh => "liveness fresh: NQ witness recently active".into(),
            LivenessVerdict::Stale {
                age_seconds,
                threshold_seconds,
            } => format!(
                "liveness stale: witness silent for {age_seconds}s (threshold {threshold_seconds}s)"
            ),
            LivenessVerdict::Skewed { age_seconds } => format!(
                "liveness skewed: artifact timestamp is in the future (age_seconds={age_seconds}); resolve clock skew before trusting NQ"
            ),
        }
    }
}

/// Compute Night Shift's own freshness verdict from a snapshot.
/// Ignores the upstream `fresh` field by design (see module docs).
pub fn verdict_for(snap: &LivenessSnapshot, threshold_seconds: u64) -> LivenessVerdict {
    let age = snap.freshness.age_seconds;
    if age < 0 {
        return LivenessVerdict::Skewed { age_seconds: age };
    }
    // age is non-negative; safe to cast for comparison.
    let age_u = age as u64;
    if age_u <= threshold_seconds {
        LivenessVerdict::Fresh
    } else {
        LivenessVerdict::Stale {
            age_seconds: age,
            threshold_seconds,
        }
    }
}

/// Parse a single LivenessSnapshot from the wire format.
///
/// Accepts either the pretty-`json` (object) or `jsonl` (single
/// object per line) shapes — both deserialize the same root object.
pub fn parse_snapshot(raw: &str) -> Result<LivenessSnapshot> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(NightShiftError::Store(
            "empty liveness payload".into(),
        ));
    }
    let snap: LivenessSnapshot = serde_json::from_str(trimmed)?;
    if snap.schema != LIVENESS_SCHEMA {
        return Err(NightShiftError::Store(format!(
            "unexpected liveness schema: got {}, expected {}",
            snap.schema, LIVENESS_SCHEMA
        )));
    }
    if snap.contract_version != LIVENESS_CONTRACT_VERSION {
        return Err(NightShiftError::Store(format!(
            "unexpected liveness contract_version: got {}, expected {}",
            snap.contract_version, LIVENESS_CONTRACT_VERSION
        )));
    }
    Ok(snap)
}

/// Trait for reading the current liveness state from somewhere.
pub trait LivenessSource: Send + Sync {
    fn current(&self) -> Result<LivenessSnapshot>;
}

/// Fixture-backed liveness source: returns the same snapshot on
/// every call. For tests and operator dry-runs.
pub struct FixtureLivenessSource {
    snapshot: LivenessSnapshot,
}

impl FixtureLivenessSource {
    pub fn new(snapshot: LivenessSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn from_json(raw: &str) -> Result<Self> {
        Ok(Self::new(parse_snapshot(raw)?))
    }
}

impl LivenessSource for FixtureLivenessSource {
    fn current(&self) -> Result<LivenessSnapshot> {
        Ok(self.snapshot.clone())
    }
}

/// CLI-backed liveness source: shells out to
/// `nq liveness export --artifact <path> --format json`.
///
/// Does NOT pass `--stale-threshold-seconds` — Night Shift computes
/// its own verdict from `age_seconds` to sidestep the upstream
/// negative-age wrinkle.
///
/// Binary resolution mirrors `nq::CliNqSource`:
/// 1. explicit value via `with_nq_bin` / `with_nq_argv`
/// 2. `NIGHTSHIFT_NQ_BIN` env var
/// 3. `nq` on PATH
pub struct CliLivenessSource {
    pub artifact_path: PathBuf,
    nq_argv: Vec<std::ffi::OsString>,
}

impl CliLivenessSource {
    pub fn new<P: Into<PathBuf>>(artifact_path: P) -> Self {
        Self {
            artifact_path: artifact_path.into(),
            nq_argv: vec!["nq".into()],
        }
    }

    pub fn with_nq_bin<P: Into<PathBuf>>(mut self, nq_bin: P) -> Self {
        let bin: std::ffi::OsString = nq_bin.into().into_os_string();
        if self.nq_argv.is_empty() {
            self.nq_argv = vec![bin];
        } else {
            self.nq_argv[0] = bin;
        }
        self
    }

    pub fn with_nq_argv<I, S>(mut self, argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<std::ffi::OsString>,
    {
        self.nq_argv = argv.into_iter().map(Into::into).collect();
        if self.nq_argv.is_empty() {
            self.nq_argv = vec!["nq".into()];
        }
        self
    }

    fn resolved_argv(&self) -> Vec<std::ffi::OsString> {
        let mut argv = self.nq_argv.clone();
        if argv.len() == 1 && argv[0] == "nq" {
            if let Ok(p) = std::env::var("NIGHTSHIFT_NQ_BIN") {
                argv[0] = p.into();
            }
        }
        argv
    }
}

impl LivenessSource for CliLivenessSource {
    fn current(&self) -> Result<LivenessSnapshot> {
        let argv = self.resolved_argv();
        let (bin, leading) = argv.split_first().expect("resolved_argv guarantees non-empty");
        let output = Command::new(bin)
            .args(leading)
            .arg("liveness")
            .arg("export")
            .arg("--artifact")
            .arg(&self.artifact_path)
            .arg("--format")
            .arg("json")
            .output()
            .map_err(|e| {
                NightShiftError::Store(format!(
                    "invoking {}: {e}",
                    std::path::Path::new(bin).display()
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NightShiftError::Store(format!(
                "nq liveness export failed (exit {:?}): {}",
                output.status.code(),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8(output.stdout).map_err(|e| {
            NightShiftError::Store(format!("nq liveness export non-utf8 stdout: {e}"))
        })?;
        parse_snapshot(&stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_dto_json() -> &'static str {
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

    fn skewed_dto_json() -> &'static str {
        r#"{
            "schema": "nq.liveness_snapshot.v1",
            "contract_version": 1,
            "instance_id": "labelwatch-host",
            "witness": {
                "generation_id": 99999,
                "generated_at": "2030-01-01T00:00:00.000000000Z",
                "schema_version": 29,
                "status": "ok",
                "findings_observed": 0,
                "findings_suppressed": 0,
                "detectors_run": 3,
                "liveness_format_version": 1
            },
            "freshness": {
                "age_seconds": -116750929,
                "stale_threshold_seconds": 60,
                "fresh": true
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
    fn parse_accepts_canonical_dto() {
        let snap = parse_snapshot(fresh_dto_json()).expect("must parse");
        assert_eq!(snap.schema, "nq.liveness_snapshot.v1");
        assert_eq!(snap.contract_version, 1);
        assert_eq!(snap.instance_id, "labelwatch-host");
        assert_eq!(snap.witness.findings_observed, 9);
        assert_eq!(snap.freshness.age_seconds, 25);
    }

    #[test]
    fn parse_rejects_wrong_schema() {
        let bad = fresh_dto_json().replace("nq.liveness_snapshot.v1", "nq.liveness_snapshot.v2");
        let err = parse_snapshot(&bad).unwrap_err();
        assert!(format!("{err}").contains("liveness schema"), "got: {err}");
    }

    #[test]
    fn parse_rejects_wrong_contract_version() {
        let bad = fresh_dto_json().replace("\"contract_version\": 1", "\"contract_version\": 2");
        let err = parse_snapshot(&bad).unwrap_err();
        assert!(
            format!("{err}").contains("contract_version"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_rejects_empty_payload() {
        let err = parse_snapshot("   \n  ").unwrap_err();
        assert!(format!("{err}").contains("empty"), "got: {err}");
    }

    #[test]
    fn verdict_fresh_when_under_threshold() {
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        assert_eq!(verdict_for(&snap, 60), LivenessVerdict::Fresh);
        assert_eq!(verdict_for(&snap, 25), LivenessVerdict::Fresh);
    }

    #[test]
    fn verdict_stale_when_over_threshold() {
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        let v = verdict_for(&snap, 10);
        assert_eq!(
            v,
            LivenessVerdict::Stale {
                age_seconds: 25,
                threshold_seconds: 10
            }
        );
        assert!(!v.is_fresh());
    }

    /// **Wrinkle test (chatty/nq-claude flagged 2026-04-20)**.
    /// A future-dated artifact (clock skew) reports
    /// `freshness.fresh: true` and a negative `age_seconds` because
    /// the upstream clamps `age.max(0)`. Night Shift's verdict must
    /// be `Skewed`, not `Fresh`. Trusting the upstream flag would
    /// assert freshness on impossible data.
    #[test]
    fn verdict_skewed_on_negative_age_even_when_upstream_says_fresh() {
        let snap = parse_snapshot(skewed_dto_json()).unwrap();
        assert_eq!(snap.freshness.fresh, Some(true), "test precondition");
        assert!(snap.freshness.age_seconds < 0, "test precondition");

        let v = verdict_for(&snap, 60);
        assert!(matches!(v, LivenessVerdict::Skewed { .. }));
        assert!(!v.is_fresh());
        assert!(v.explain().contains("clock skew"));
    }

    #[test]
    fn fixture_source_returns_loaded_snapshot() {
        let src = FixtureLivenessSource::from_json(fresh_dto_json()).unwrap();
        let snap = src.current().unwrap();
        assert_eq!(snap.witness.findings_observed, 9);
    }

    /// CliLivenessSource with a synthetic `nq` script — proves the
    /// invocation shape and parsing without a real binary.
    #[test]
    fn cli_source_invokes_expected_argv_and_parses_stdout() {
        let dto = fresh_dto_json();
        let script = format!("printf '%s' '{}'", dto);
        let src = CliLivenessSource::new("/tmp/whatever-liveness.json").with_nq_argv(vec![
            "/bin/sh",
            "-c",
            &script,
            "--",
        ]);
        let snap = src.current().expect("synthetic nq script must succeed");
        assert_eq!(snap.witness.findings_observed, 9);
    }

    #[test]
    fn cli_source_propagates_upstream_failure() {
        let src = CliLivenessSource::new("/tmp/whatever").with_nq_argv(vec![
            "/bin/sh",
            "-c",
            "echo 'liveness artifact missing' >&2; exit 2",
            "--",
        ]);
        let err = src.current().unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("nq liveness export failed"), "got: {msg}");
        assert!(msg.contains("liveness artifact missing"), "got: {msg}");
    }
}
