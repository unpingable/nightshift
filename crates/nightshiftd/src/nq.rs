//! NQ evidence source.
//!
//! Two implementations:
//! - `FixtureNqSource` — manifest-backed, for tests + dogfood without a
//!   live NQ.
//! - `CliNqSource` — shells out to `nq findings export` (canonical
//!   contract per NQ's `FINDING_EXPORT_GAP`) and translates to the
//!   internal `FindingSnapshot`.
//!
//! NQ findings are **evidence, not commands**. A snapshot is the
//! state at a specific generation; the Reconciler decides whether
//! and how it may be relied upon.
//!
//! See `GAP-nq-nightshift-contract.md`.

use std::path::PathBuf;
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::errors::{NightShiftError, Result};
use crate::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};

/// Trait for pulling the current snapshot of a finding by stable identity.
pub trait NqSource: Send + Sync {
    /// Fetch the current snapshot for a finding. Returns None if the
    /// finding is absent at the current generation.
    fn snapshot(&self, key: &FindingKey) -> Result<Option<FindingSnapshot>>;
}

/// Fixture-backed NQ source: reads a single JSON manifest file
/// containing a list of FindingSnapshots. Indexed by finding_key.
///
/// Manifest format:
/// ```json
/// { "findings": [ <FindingSnapshot>, ... ] }
/// ```
pub struct FixtureNqSource {
    pub manifest_path: PathBuf,
    findings: Vec<FindingSnapshot>,
}

#[derive(serde::Deserialize)]
struct Manifest {
    findings: Vec<FindingSnapshot>,
}

impl FixtureNqSource {
    pub fn load<P: Into<PathBuf>>(manifest_path: P) -> Result<Self> {
        let manifest_path = manifest_path.into();
        let raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
            NightShiftError::Store(format!("reading {}: {e}", manifest_path.display()))
        })?;
        let m: Manifest = serde_json::from_str(&raw)?;
        Ok(Self {
            manifest_path,
            findings: m.findings,
        })
    }
}

impl NqSource for FixtureNqSource {
    fn snapshot(&self, key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(self
            .findings
            .iter()
            .find(|s| s.finding_key == *key)
            .cloned())
    }
}

/// Compute a byte-stable evidence hash for a snapshot. Used to detect
/// `changed` status in the reconciler.
pub fn evidence_hash(snap: &FindingSnapshot) -> String {
    // Hash a deterministic projection: all fields except the
    // self-reported evidence_hash (which is *derived*, not input).
    let projection = serde_json::json!({
        "finding_key": snap.finding_key,
        "host": snap.host,
        "severity": snap.severity,
        "domain": snap.domain,
        "persistence_generations": snap.persistence_generations,
        "first_seen_at": snap.first_seen_at,
        "current_status": snap.current_status,
        "snapshot_generation": snap.snapshot_generation,
    });
    let serialized = serde_json::to_string(&projection).expect("json projection must serialize");
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

// ---------------------------------------------------------------------------
// CLI-backed NQ source.
//
// Executes `nq findings export --db <path> --finding-key <key>` and parses
// the canonical JSONL (schema "nq.finding_snapshot.v1", contract_version 1)
// per NQ's FINDING_EXPORT_GAP.
// ---------------------------------------------------------------------------

/// Expected NQ export schema string. Changes bump the version.
pub const NQ_EXPORT_SCHEMA: &str = "nq.finding_snapshot.v1";
pub const NQ_EXPORT_CONTRACT_VERSION: u32 = 1;

/// Shape of one NQ `FindingSnapshot` JSON object. Deserialize-only;
/// we only pull the subset Night Shift actually translates.
/// Unknown fields are ignored so NQ can add non-breaking detail.
#[derive(Debug, Clone, Deserialize)]
pub struct NqExportDto {
    pub schema: String,
    pub contract_version: u32,
    pub finding_key: String,
    pub identity: NqIdentityDto,
    pub lifecycle: NqLifecycleDto,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NqIdentityDto {
    pub scope: String,
    pub host: String,
    pub detector: String,
    pub subject: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NqLifecycleDto {
    pub first_seen_gen: i64,
    pub first_seen_at: String,
    pub last_seen_gen: i64,
    pub last_seen_at: String,
    pub consecutive_gens: i64,
    pub severity: String,
    pub condition_state: String,
}

/// Parse one NQ JSONL line into the DTO.
pub fn parse_nq_line(line: &str) -> Result<NqExportDto> {
    let dto: NqExportDto = serde_json::from_str(line)?;
    if dto.schema != NQ_EXPORT_SCHEMA {
        return Err(NightShiftError::InvalidAgenda(format!(
            "NQ export schema mismatch: expected {NQ_EXPORT_SCHEMA}, got {}",
            dto.schema
        )));
    }
    if dto.contract_version != NQ_EXPORT_CONTRACT_VERSION {
        return Err(NightShiftError::InvalidAgenda(format!(
            "NQ export contract_version mismatch: expected {NQ_EXPORT_CONTRACT_VERSION}, got {}",
            dto.contract_version
        )));
    }
    Ok(dto)
}

/// Translate an NQ export DTO into Night Shift's internal
/// `FindingSnapshot`. The mapping is explicit and coarse by design —
/// NQ carries more detail than Night Shift needs at the edge; richer
/// consumption can land in later slices.
pub fn translate_nq(dto: &NqExportDto) -> Result<FindingSnapshot> {
    let severity = match dto.lifecycle.severity.as_str() {
        "info" => Severity::Low,
        "warning" => Severity::Warning,
        "critical" => Severity::Critical,
        _ => Severity::Warning,
    };

    // condition_state is derived in NQ from consecutive_gens, absent_gens,
    // and visibility_state. v1 coarse set: open | clear | suppressed.
    let current_status = match dto.lifecycle.condition_state.as_str() {
        "open" => EvidenceState::Active,
        "clear" => EvidenceState::Recovered,
        // Suppressed findings are filtered out by NQ default; if one
        // reaches us we treat it as active — suppression is an NQ
        // operator decision we do not invert, but we do not silence
        // Night Shift's view either.
        "suppressed" => EvidenceState::Active,
        _ => EvidenceState::Active,
    };

    let first_seen_at = DateTime::parse_from_rfc3339(&dto.lifecycle.first_seen_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            NightShiftError::InvalidAgenda(format!(
                "NQ lifecycle.first_seen_at not RFC3339: {e}"
            ))
        })?;
    let last_seen_at = DateTime::parse_from_rfc3339(&dto.lifecycle.last_seen_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            NightShiftError::InvalidAgenda(format!(
                "NQ lifecycle.last_seen_at not RFC3339: {e}"
            ))
        })?;

    let persistence_generations = u32::try_from(dto.lifecycle.consecutive_gens.max(0))
        .unwrap_or(u32::MAX);
    let snapshot_generation = u64::try_from(dto.lifecycle.last_seen_gen.max(0))
        .unwrap_or(u64::MAX);

    // Night Shift FindingKey convention: source="nq",
    // detector=identity.detector, subject="<host>:<subject>" — keeps
    // host visible in the key string without inventing a new axis.
    let finding_key = FindingKey {
        source: "nq".into(),
        detector: dto.identity.detector.clone(),
        subject: format!("{}:{}", dto.identity.host, dto.identity.subject),
    };

    Ok(FindingSnapshot {
        finding_key,
        host: dto.identity.host.clone(),
        severity,
        domain: None,
        persistence_generations,
        first_seen_at,
        current_status,
        snapshot_generation,
        captured_at: last_seen_at,
        evidence_hash: String::new(),
    })
}

/// URL-encode bytes per NQ's `compute_finding_key` rules. Used to
/// reconstruct NQ's canonical finding_key from Night Shift's typed
/// identity for `--finding-key` exact-match lookups.
fn nq_enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// Reconstruct NQ's canonical finding_key for a given Night Shift
/// `FindingKey`. Night Shift's convention embeds `<host>:<subject>`
/// in `FindingKey.subject`; we split on the first `:` to recover
/// NQ's (host, subject) pair. v1 assumes scope = "local".
fn nq_canonical_key(key: &FindingKey) -> Result<String> {
    let (host, subject) = key.subject.split_once(':').ok_or_else(|| {
        NightShiftError::InvalidAgenda(format!(
            "FindingKey.subject must be '<host>:<subject>' for NQ CLI lookup, got: {}",
            key.subject
        ))
    })?;
    Ok(format!(
        "{}/{}/{}/{}",
        nq_enc("local"),
        nq_enc(host),
        nq_enc(&key.detector),
        nq_enc(subject)
    ))
}

/// NQ CLI-backed source.
///
/// On each `snapshot` call, shells out to `nq findings export` with
/// `--finding-key` set to the canonical NQ key reconstructed from the
/// target `FindingKey`. Output is JSONL; we expect zero or one line.
///
/// `nq_argv` is the invocation preamble: by default `["nq"]`, so the
/// final command is `nq findings export --db ... --format jsonl
/// --finding-key ...`. Tests can replace it with e.g.
/// `["/bin/sh", "-c", "<script>", "--"]` to inject controlled
/// failure modes without needing a real `nq` binary on disk.
///
/// The leading binary in `nq_argv` is resolved via:
/// 1. the explicit value set by `with_nq_bin` / `with_nq_argv`
/// 2. the `NIGHTSHIFT_NQ_BIN` env var (replaces argv[0] only)
/// 3. `nq` on PATH (default)
pub struct CliNqSource {
    pub db_path: PathBuf,
    nq_argv: Vec<std::ffi::OsString>,
}

impl CliNqSource {
    pub fn new<P: Into<PathBuf>>(db_path: P) -> Self {
        Self {
            db_path: db_path.into(),
            nq_argv: vec!["nq".into()],
        }
    }

    /// Override just the binary (argv[0]). Leaves any leading args
    /// previously set by `with_nq_argv` in place if `nq_argv.len() > 1`,
    /// otherwise replaces the single-element default.
    pub fn with_nq_bin<P: Into<PathBuf>>(mut self, nq_bin: P) -> Self {
        let bin: std::ffi::OsString = nq_bin.into().into_os_string();
        if self.nq_argv.is_empty() {
            self.nq_argv = vec![bin];
        } else {
            self.nq_argv[0] = bin;
        }
        self
    }

    /// Override the entire invocation preamble (`argv[0..]` before the
    /// `findings export ...` args). Mainly for tests.
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
        // Env override only applies when argv hasn't been customized
        // (i.e., still the default single "nq" entry).
        if argv.len() == 1 && argv[0] == "nq" {
            if let Ok(p) = std::env::var("NIGHTSHIFT_NQ_BIN") {
                argv[0] = p.into();
            }
        }
        argv
    }
}

impl NqSource for CliNqSource {
    fn snapshot(&self, key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        if key.source != "nq" {
            return Ok(None);
        }
        let canonical = nq_canonical_key(key)?;
        let argv = self.resolved_argv();
        let (bin, leading) = argv.split_first().expect("resolved_argv guarantees non-empty");
        let output = Command::new(bin)
            .args(leading)
            .arg("findings")
            .arg("export")
            .arg("--db")
            .arg(&self.db_path)
            .arg("--format")
            .arg("jsonl")
            .arg("--finding-key")
            .arg(&canonical)
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
                "nq findings export failed (status={}): {stderr}",
                output.status
            )));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| NightShiftError::Store(format!("non-utf8 nq output: {e}")))?;

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let dto = parse_nq_line(line)?;
            // Double-check the returned row matches what we asked for
            // (defensive against schema drift or CLI filter regressions).
            if dto.finding_key != canonical {
                continue;
            }
            let mut snap = translate_nq(&dto)?;
            snap.evidence_hash = evidence_hash(&snap);
            return Ok(Some(snap));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn sample() -> FindingSnapshot {
        FindingSnapshot {
            finding_key: FindingKey {
                source: "nq".into(),
                detector: "wal_bloat".into(),
                subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
            },
            host: "labelwatch-host".into(),
            severity: crate::finding::Severity::Warning,
            domain: Some("delta_g".into()),
            persistence_generations: 4,
            first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 14, 32, 15).unwrap(),
            current_status: crate::finding::EvidenceState::Active,
            snapshot_generation: 39532,
            captured_at: Utc.with_ymd_and_hms(2026, 4, 16, 22, 0, 0).unwrap(),
            evidence_hash: String::new(),
        }
    }

    #[test]
    fn evidence_hash_is_stable() {
        let a = sample();
        let mut b = a.clone();
        b.captured_at = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
        // captured_at is NOT part of the hash projection — same hash
        assert_eq!(evidence_hash(&a), evidence_hash(&b));
    }

    #[test]
    fn evidence_hash_changes_with_state_transition() {
        let a = sample();
        let mut b = a.clone();
        b.current_status = crate::finding::EvidenceState::Resolving;
        assert_ne!(evidence_hash(&a), evidence_hash(&b));
    }

    #[test]
    fn evidence_hash_changes_with_generations() {
        let a = sample();
        let mut b = a.clone();
        b.persistence_generations += 1;
        assert_ne!(evidence_hash(&a), evidence_hash(&b));
    }

    // ---- NQ CLI parser + translator tests ----

    const SAMPLE_NQ_JSONL: &str = r#"{"schema":"nq.finding_snapshot.v1","contract_version":1,"finding_key":"local/labelwatch-host/wal_bloat/%2Fvar%2Flib%2Flabelwatch.sqlite","identity":{"scope":"local","host":"labelwatch-host","detector":"wal_bloat","subject":"/var/lib/labelwatch.sqlite","rule_hash":null},"lifecycle":{"first_seen_gen":39000,"first_seen_at":"2026-04-10T14:32:15Z","last_seen_gen":39532,"last_seen_at":"2026-04-17T03:00:00Z","consecutive_gens":6,"absent_gens":0,"severity":"warning","visibility_state":"visible","condition_state":"open","finding_class":"accumulation","stability":null,"peak_value":518.0,"message":"wal grew to 518MB"},"diagnosis":null,"regime":{"trajectory":null,"persistence":null,"recovery":null,"co_occurrence":null,"resolution":null},"observations":{"total_count":6,"recent":[]},"generation":{"generation_id":39532,"started_at":null,"completed_at":null,"status":null,"sources_expected":null,"sources_ok":null,"sources_failed":null},"export":{"exported_at":"2026-04-17T03:00:00Z","changed_since":null,"source":"nq","contract_version":1}}"#;

    #[test]
    fn parse_accepts_expected_schema_and_version() {
        let dto = parse_nq_line(SAMPLE_NQ_JSONL).expect("canonical sample must parse");
        assert_eq!(dto.schema, NQ_EXPORT_SCHEMA);
        assert_eq!(dto.contract_version, NQ_EXPORT_CONTRACT_VERSION);
        assert_eq!(dto.identity.detector, "wal_bloat");
        assert_eq!(dto.lifecycle.consecutive_gens, 6);
        assert_eq!(dto.lifecycle.severity, "warning");
        assert_eq!(dto.lifecycle.condition_state, "open");
    }

    #[test]
    fn parse_rejects_wrong_schema() {
        let wrong = r#"{"schema":"nq.finding_snapshot.v2","contract_version":1,"finding_key":"x","identity":{"scope":"local","host":"h","detector":"d","subject":"s"},"lifecycle":{"first_seen_gen":0,"first_seen_at":"2026-01-01T00:00:00Z","last_seen_gen":0,"last_seen_at":"2026-01-01T00:00:00Z","consecutive_gens":0,"severity":"info","condition_state":"open"}}"#;
        let err = parse_nq_line(wrong).unwrap_err();
        assert!(
            format!("{err}").contains("schema mismatch"),
            "error did not mention schema mismatch: {err}"
        );
    }

    #[test]
    fn parse_rejects_wrong_contract_version() {
        let wrong = r#"{"schema":"nq.finding_snapshot.v1","contract_version":999,"finding_key":"x","identity":{"scope":"local","host":"h","detector":"d","subject":"s"},"lifecycle":{"first_seen_gen":0,"first_seen_at":"2026-01-01T00:00:00Z","last_seen_gen":0,"last_seen_at":"2026-01-01T00:00:00Z","consecutive_gens":0,"severity":"info","condition_state":"open"}}"#;
        let err = parse_nq_line(wrong).unwrap_err();
        assert!(
            format!("{err}").contains("contract_version mismatch"),
            "error did not mention contract_version: {err}"
        );
    }

    #[test]
    fn translate_builds_finding_snapshot() {
        let dto = parse_nq_line(SAMPLE_NQ_JSONL).unwrap();
        let snap = translate_nq(&dto).expect("translate must succeed");

        assert_eq!(snap.finding_key.source, "nq");
        assert_eq!(snap.finding_key.detector, "wal_bloat");
        assert_eq!(
            snap.finding_key.subject,
            "labelwatch-host:/var/lib/labelwatch.sqlite"
        );
        assert_eq!(snap.host, "labelwatch-host");
        assert_eq!(snap.severity, Severity::Warning);
        assert_eq!(snap.persistence_generations, 6);
        assert_eq!(snap.snapshot_generation, 39532);
        assert_eq!(snap.current_status, EvidenceState::Active);
    }

    #[test]
    fn translate_maps_condition_state_to_evidence_state() {
        let dto_clear = {
            let mut d = parse_nq_line(SAMPLE_NQ_JSONL).unwrap();
            d.lifecycle.condition_state = "clear".into();
            d
        };
        let snap = translate_nq(&dto_clear).unwrap();
        assert_eq!(snap.current_status, EvidenceState::Recovered);

        let dto_suppressed = {
            let mut d = parse_nq_line(SAMPLE_NQ_JSONL).unwrap();
            d.lifecycle.condition_state = "suppressed".into();
            d
        };
        let snap = translate_nq(&dto_suppressed).unwrap();
        assert_eq!(snap.current_status, EvidenceState::Active);
    }

    #[test]
    fn translate_maps_severity_strings_to_enum() {
        let cases = [
            ("info", Severity::Low),
            ("warning", Severity::Warning),
            ("critical", Severity::Critical),
            ("unknown-value", Severity::Warning), // conservative fallback
        ];
        for (input, expected) in cases {
            let mut d = parse_nq_line(SAMPLE_NQ_JSONL).unwrap();
            d.lifecycle.severity = input.into();
            let snap = translate_nq(&d).unwrap();
            assert_eq!(snap.severity, expected, "severity string: {input}");
        }
    }

    #[test]
    fn nq_canonical_key_round_trips_for_simple_inputs() {
        let key = FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
        };
        let canonical = nq_canonical_key(&key).unwrap();
        assert_eq!(
            canonical,
            "local/labelwatch-host/wal_bloat/%2Fvar%2Flib%2Flabelwatch.sqlite"
        );
    }

    #[test]
    fn nq_canonical_key_percent_encodes_special_chars() {
        let key = FindingKey {
            source: "nq".into(),
            detector: "check_failed".into(),
            subject: "host one:#1".into(),
        };
        let canonical = nq_canonical_key(&key).unwrap();
        // space → %20, # → %23
        assert!(canonical.contains("%20"), "missing space encoding: {canonical}");
        assert!(canonical.contains("%23"), "missing # encoding: {canonical}");
    }

    #[test]
    fn nq_canonical_key_rejects_missing_host_colon() {
        let key = FindingKey {
            source: "nq".into(),
            detector: "d".into(),
            subject: "no-colon-here".into(),
        };
        assert!(nq_canonical_key(&key).is_err());
    }
}
