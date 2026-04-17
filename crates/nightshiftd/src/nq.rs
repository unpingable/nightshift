//! NQ evidence source.
//!
//! v1 MVP uses a fixture-backed source for testing and for dogfood
//! runs that don't require a live NQ daemon. The real client goes in
//! a later slice. See `GAP-nq-nightshift-contract.md`.
//!
//! NQ findings are **evidence, not commands**. A snapshot is the
//! state at a specific generation; the Reconciler decides whether
//! and how it may be relied upon.

use std::path::PathBuf;

use sha2::{Digest, Sha256};

use crate::errors::{NightShiftError, Result};
use crate::finding::{FindingKey, FindingSnapshot};

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
}
