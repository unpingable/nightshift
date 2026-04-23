//! Horizon policy source — Night Shift's **local** declarations of
//! tolerance windows for findings.
//!
//! # The load-bearing distinction
//!
//! **Horizon is producer-local policy. Governor is the archivist,
//! not the lookup source for per-finding horizon.**
//!
//! An earlier reading of `GOV_GAP_TOLERABILITY_HORIZON_001` had
//! Nightshift fetching horizon-bearing gate receipts from Governor
//! at reconcile time. That shape does not match the real Governor
//! RPC surface (preflight against `agent_gov` Commit B, 2026-04-23):
//!
//! - `nightshift.check_policy` returns verdict-only — no horizon on
//!   the response, and the receipt it emits has `horizon=None`.
//! - `nightshift.record_receipt` is the **write** path: Nightshift
//!   declares horizon and Governor forwards it onto the receipt for
//!   archival. Governor does not originate horizon from this call.
//! - `receipts.detail` / `receipts.horizon_expiring_soon` are
//!   inspection surfaces, not per-finding reconcile-time read paths.
//!
//! So horizon must be declared by Nightshift — derived from agenda
//! policy, operator input, or other NS-local sources — and then
//! (in Phase B) forwarded to Governor via `record_receipt` so the
//! tolerance decision shows up in the audit trail.
//!
//! This module exposes that **NS-local horizon source** as a trait.
//! `FixtureHorizonPolicySource` is the test/dogfood impl. A future
//! `AgendaHorizonPolicySource` will read from the agenda's declared
//! tolerance rules. Neither talks to Governor.
//!
//! The companion `GovernorClient` surface (Phase B) is where the
//! real three RPC methods live; these are orthogonal concerns and
//! must not be conflated.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::{NightShiftError, Result};
use crate::finding::FindingKey;
use crate::horizon::HorizonBlock;

/// One horizon declaration for one finding. The manifest shape for
/// fixture sources and (forward-looking) any producer that wants to
/// dump a batch of declarations as a single JSON artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonDeclaration {
    pub finding_key: FindingKey,
    pub horizon: HorizonBlock,
}

/// Trait for reading Night Shift's declared horizon for a finding.
///
/// Narrow by design — one method. The trait is NS-internal; it does
/// not front any Governor RPC. See module header for the invariant.
pub trait HorizonPolicySource: Send + Sync {
    /// Return the horizon declaration for a finding, if any. `None`
    /// means NS has no horizon declared for this finding —
    /// consumer fail-closes to `now` per spec (`missing ≠ tolerable`).
    fn horizon_for(&self, key: &FindingKey) -> Result<Option<HorizonBlock>>;
}

/// Fixture-backed horizon policy source: reads a single JSON
/// manifest file. Indexed by finding_key.
///
/// Manifest format:
/// ```json
/// { "declarations": [
///     { "finding_key": {...}, "horizon": {...} },
///     ...
/// ]}
/// ```
pub struct FixtureHorizonPolicySource {
    pub manifest_path: PathBuf,
    declarations: Vec<HorizonDeclaration>,
}

#[derive(Deserialize)]
struct Manifest {
    declarations: Vec<HorizonDeclaration>,
}

impl FixtureHorizonPolicySource {
    pub fn load<P: Into<PathBuf>>(manifest_path: P) -> Result<Self> {
        let manifest_path = manifest_path.into();
        let raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
            NightShiftError::Store(format!("reading {}: {e}", manifest_path.display()))
        })?;
        let m: Manifest = serde_json::from_str(&raw)?;
        Ok(Self {
            manifest_path,
            declarations: m.declarations,
        })
    }

    /// In-memory constructor for tests that don't want to round-trip
    /// through a file.
    pub fn from_declarations(declarations: Vec<HorizonDeclaration>) -> Self {
        Self {
            manifest_path: PathBuf::new(),
            declarations,
        }
    }
}

impl HorizonPolicySource for FixtureHorizonPolicySource {
    fn horizon_for(&self, key: &FindingKey) -> Result<Option<HorizonBlock>> {
        Ok(self
            .declarations
            .iter()
            .find(|d| d.finding_key == *key)
            .map(|d| d.horizon.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::horizon::HorizonClass;

    fn fk(detector: &str, subject: &str) -> FindingKey {
        FindingKey {
            source: "nq".into(),
            detector: detector.into(),
            subject: subject.into(),
        }
    }

    #[test]
    fn fixture_source_indexes_by_finding_key() {
        let key_a = fk("wal_bloat", "host-a:/db");
        let key_b = fk("wal_bloat", "host-b:/db");
        let key_miss = fk("wal_bloat", "host-c:/db");
        let src = FixtureHorizonPolicySource::from_declarations(vec![
            HorizonDeclaration {
                finding_key: key_a.clone(),
                horizon: HorizonBlock {
                    class: HorizonClass::Hours,
                    basis_id: Some("basis-a".into()),
                    basis_hash: Some("hash-a".into()),
                    expiry: Some("2026-04-23T20:00:00Z".parse().unwrap()),
                },
            },
            HorizonDeclaration {
                finding_key: key_b.clone(),
                horizon: HorizonBlock {
                    class: HorizonClass::ObserveOnly,
                    basis_id: Some("basis-b".into()),
                    basis_hash: Some("hash-b".into()),
                    expiry: None,
                },
            },
        ]);
        assert_eq!(
            src.horizon_for(&key_a).unwrap().unwrap().class,
            HorizonClass::Hours
        );
        assert_eq!(
            src.horizon_for(&key_b).unwrap().unwrap().class,
            HorizonClass::ObserveOnly
        );
        assert!(src.horizon_for(&key_miss).unwrap().is_none());
    }

    #[test]
    fn horizon_declaration_round_trips_through_json() {
        let d = HorizonDeclaration {
            finding_key: fk("wal_bloat", "labelwatch-host:/var/lib/db"),
            horizon: HorizonBlock {
                class: HorizonClass::Scheduled,
                basis_id: Some("maintenance-window-042".into()),
                basis_hash: Some("sha256:deadbeef".into()),
                expiry: Some("2026-04-25T03:00:00Z".parse().unwrap()),
            },
        };
        let s = serde_json::to_string(&d).unwrap();
        let d2: HorizonDeclaration = serde_json::from_str(&s).unwrap();
        assert_eq!(d2.finding_key, d.finding_key);
        assert_eq!(d2.horizon.class, HorizonClass::Scheduled);
        assert_eq!(d2.horizon.basis_id.as_deref(), Some("maintenance-window-042"));
    }

    #[test]
    fn fixture_source_loads_from_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("horizons.json");
        let manifest = r#"{
            "declarations": [
                {
                    "finding_key": {
                        "source": "nq",
                        "detector": "wal_bloat",
                        "subject": "labelwatch-host:/var/lib/db"
                    },
                    "horizon": {
                        "class": "hours",
                        "basis_id": "basis-abc",
                        "basis_hash": "hash-123",
                        "expiry": "2026-04-23T20:00:00Z"
                    }
                }
            ]
        }"#;
        std::fs::write(&path, manifest).unwrap();
        let src = FixtureHorizonPolicySource::load(&path).unwrap();
        let key = fk("wal_bloat", "labelwatch-host:/var/lib/db");
        let block = src.horizon_for(&key).unwrap().unwrap();
        assert_eq!(block.class, HorizonClass::Hours);
        assert_eq!(block.basis_id.as_deref(), Some("basis-abc"));
    }

    #[test]
    fn none_class_declaration_round_trips_without_expiry() {
        let d = HorizonDeclaration {
            finding_key: fk("x", "y"),
            horizon: HorizonBlock {
                class: HorizonClass::None,
                basis_id: None,
                basis_hash: None,
                expiry: None,
            },
        };
        let s = serde_json::to_string(&d).unwrap();
        let d2: HorizonDeclaration = serde_json::from_str(&s).unwrap();
        assert_eq!(d2.horizon.class, HorizonClass::None);
        assert!(d2.horizon.expiry.is_none());
    }
}
