//! Governor adapter — gate receipts and horizon consumption.
//!
//! Phase A scope: only what the reconciler needs to exercise the
//! four-way horizon distinction (tolerated_active / expired_tolerance
//! / basis_invalidated / fresh_arrival). Per chatty's guardrail #1
//! on the 2026-04-23 scoping call:
//!
//! > Keep GovernorSource narrow — only what A actually needs for
//! > the four-way distinction; don't smuggle promotion-point policy
//! > calls into the trait yet.
//!
//! Phase B (separate commit) will add the full adapter —
//! `nightshift.check_policy`, `nightshift.authorize_transition`,
//! `nightshift.record_receipt` — speaking real JSON-RPC to the
//! Governor daemon. Phase B will introduce an `RpcGovernorSource`
//! alongside the `FixtureGovernorSource` here; the trait shape does
//! not have to grow for that transition.
//!
//! See:
//! - Governor's `GOV_GAP_TOLERABILITY_HORIZON_001` (shipped Commit A
//!   at agent_gov `7c09523`, Commit B at `9b0c2e5`).
//! - Night Shift memory `project_governor_rpc_surface.md`.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::{NightShiftError, Result};
use crate::finding::FindingKey;
use crate::horizon::{HorizonBlock, HorizonClass};

/// The nested horizon block carried on a gate receipt.
/// Governor spec: `{class, basis_id, basis_hash, expiry}`. Same
/// shape as the internal `HorizonBlock`; this DTO just marks the
/// boundary where wire/disk JSON becomes consumer logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireHorizon {
    pub class: HorizonClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<DateTime<Utc>>,
}

impl From<WireHorizon> for HorizonBlock {
    fn from(w: WireHorizon) -> Self {
        HorizonBlock {
            class: w.class,
            basis_id: w.basis_id,
            basis_hash: w.basis_hash,
            expiry: w.expiry,
        }
    }
}

/// A gate receipt as Night Shift consumes it.
///
/// Phase A models only the fields the reconciler reads. The full
/// receipt envelope (obligations, reason, downgrade_to, etc.) is
/// Phase B territory — modeled as optional raw JSON so Phase B can
/// attach structured types without rewriting the DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReceipt {
    pub receipt_id: String,
    /// The verdict string as returned by Governor's policy_engine.
    /// Phase A treats this as opaque; Phase B parses into the frozen
    /// `ns_verdict` enum (allow/deny/require_approval/downgrade).
    pub verdict: String,
    /// Stable finding identity the receipt pertains to. Included in
    /// the DTO so consumers can index receipts without a separate
    /// request/response correlation table.
    pub finding_key: FindingKey,
    /// Optional horizon block. Missing means the producer did not
    /// declare a horizon — consumer fail-closes to `now` per spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<WireHorizon>,
    /// Reserved for Phase B. Holds any extra envelope fields from
    /// the real Governor RPC shape (obligations, required_approvals,
    /// downgrade_to, reason, etc.) as untyped JSON so the DTO stays
    /// forward-compatible while Phase A ships fixture-only.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub extras: serde_json::Value,
}

impl GateReceipt {
    /// Convert the wire-side horizon block to the internal
    /// `HorizonBlock` the consumer logic operates on. `None` if the
    /// producer did not declare a horizon.
    pub fn horizon_block(&self) -> Option<HorizonBlock> {
        self.horizon.clone().map(Into::into)
    }
}

/// Trait for fetching gate receipts. Phase A interface — narrow by
/// design. Phase B keeps the same shape and adds methods for policy
/// checks, authorize_transition, record_receipt.
pub trait GovernorSource: Send + Sync {
    /// Fetch the current gate receipt for a finding, if any. `None`
    /// means the producer has not emitted a receipt for this finding
    /// (no horizon declared, consumer fail-closes to `now`).
    fn fetch_gate_receipt(&self, key: &FindingKey) -> Result<Option<GateReceipt>>;
}

/// Fixture-backed governor source: reads a single JSON manifest
/// file. Indexed by finding_key.
///
/// Manifest format:
/// ```json
/// { "receipts": [ <GateReceipt>, ... ] }
/// ```
pub struct FixtureGovernorSource {
    pub manifest_path: PathBuf,
    receipts: Vec<GateReceipt>,
}

#[derive(Deserialize)]
struct Manifest {
    receipts: Vec<GateReceipt>,
}

impl FixtureGovernorSource {
    pub fn load<P: Into<PathBuf>>(manifest_path: P) -> Result<Self> {
        let manifest_path = manifest_path.into();
        let raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
            NightShiftError::Store(format!("reading {}: {e}", manifest_path.display()))
        })?;
        let m: Manifest = serde_json::from_str(&raw)?;
        Ok(Self {
            manifest_path,
            receipts: m.receipts,
        })
    }

    /// In-memory constructor for tests that don't want to round-trip
    /// through a file.
    pub fn from_receipts(receipts: Vec<GateReceipt>) -> Self {
        Self {
            manifest_path: PathBuf::new(),
            receipts,
        }
    }
}

impl GovernorSource for FixtureGovernorSource {
    fn fetch_gate_receipt(&self, key: &FindingKey) -> Result<Option<GateReceipt>> {
        Ok(self
            .receipts
            .iter()
            .find(|r| r.finding_key == *key)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fk(detector: &str, subject: &str) -> FindingKey {
        FindingKey {
            source: "nq".into(),
            detector: detector.into(),
            subject: subject.into(),
        }
    }

    #[test]
    fn wire_horizon_converts_to_internal_block() {
        let w = WireHorizon {
            class: HorizonClass::Hours,
            basis_id: Some("basis-abc".into()),
            basis_hash: Some("hash-123".into()),
            expiry: Some("2026-04-23T20:00:00Z".parse().unwrap()),
        };
        let b: HorizonBlock = w.into();
        assert_eq!(b.class, HorizonClass::Hours);
        assert_eq!(b.basis_id.as_deref(), Some("basis-abc"));
        assert_eq!(b.basis_hash.as_deref(), Some("hash-123"));
        assert!(b.expiry.is_some());
    }

    #[test]
    fn gate_receipt_exposes_horizon_block_when_present() {
        let r = GateReceipt {
            receipt_id: "r_001".into(),
            verdict: "allow".into(),
            finding_key: fk("wal_bloat", "labelwatch-host:/var/lib/db"),
            horizon: Some(WireHorizon {
                class: HorizonClass::Hours,
                basis_id: Some("b".into()),
                basis_hash: Some("h".into()),
                expiry: Some("2026-04-23T20:00:00Z".parse().unwrap()),
            }),
            extras: serde_json::Value::Null,
        };
        let block = r.horizon_block().expect("horizon present");
        assert_eq!(block.class, HorizonClass::Hours);
    }

    #[test]
    fn gate_receipt_returns_none_horizon_when_absent() {
        let r = GateReceipt {
            receipt_id: "r_002".into(),
            verdict: "allow".into(),
            finding_key: fk("zfs_pool_degraded", "sushi-k:tank"),
            horizon: None,
            extras: serde_json::Value::Null,
        };
        assert!(r.horizon_block().is_none());
    }

    #[test]
    fn gate_receipt_round_trips_through_json() {
        let r = GateReceipt {
            receipt_id: "r_001".into(),
            verdict: "require_approval".into(),
            finding_key: fk("wal_bloat", "labelwatch-host:/var/lib/db"),
            horizon: Some(WireHorizon {
                class: HorizonClass::Scheduled,
                basis_id: Some("maintenance-window-042".into()),
                basis_hash: Some("sha256:deadbeef".into()),
                expiry: Some("2026-04-25T03:00:00Z".parse().unwrap()),
            }),
            extras: serde_json::json!({
                "obligations": ["require_human_approval"],
                "reason": "maintenance window declared"
            }),
        };
        let s = serde_json::to_string(&r).unwrap();
        let r2: GateReceipt = serde_json::from_str(&s).unwrap();
        assert_eq!(r2.receipt_id, "r_001");
        assert_eq!(r2.verdict, "require_approval");
        assert_eq!(
            r2.horizon.as_ref().unwrap().class,
            HorizonClass::Scheduled
        );
        assert_eq!(r2.extras["obligations"][0], "require_human_approval");
    }

    #[test]
    fn fixture_source_indexes_by_finding_key() {
        let key_a = fk("wal_bloat", "host-a:/db");
        let key_b = fk("wal_bloat", "host-b:/db");
        let key_miss = fk("wal_bloat", "host-c:/db");
        let src = FixtureGovernorSource::from_receipts(vec![
            GateReceipt {
                receipt_id: "r_a".into(),
                verdict: "allow".into(),
                finding_key: key_a.clone(),
                horizon: None,
                extras: serde_json::Value::Null,
            },
            GateReceipt {
                receipt_id: "r_b".into(),
                verdict: "allow".into(),
                finding_key: key_b.clone(),
                horizon: None,
                extras: serde_json::Value::Null,
            },
        ]);
        assert_eq!(
            src.fetch_gate_receipt(&key_a).unwrap().unwrap().receipt_id,
            "r_a"
        );
        assert_eq!(
            src.fetch_gate_receipt(&key_b).unwrap().unwrap().receipt_id,
            "r_b"
        );
        assert!(src.fetch_gate_receipt(&key_miss).unwrap().is_none());
    }

    #[test]
    fn fixture_source_loads_from_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("receipts.json");
        let manifest = r#"{
            "receipts": [
                {
                    "receipt_id": "r_001",
                    "verdict": "allow",
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
        let src = FixtureGovernorSource::load(&path).unwrap();
        let key = fk("wal_bloat", "labelwatch-host:/var/lib/db");
        let r = src.fetch_gate_receipt(&key).unwrap().unwrap();
        assert_eq!(r.receipt_id, "r_001");
        assert_eq!(
            r.horizon.as_ref().unwrap().class,
            HorizonClass::Hours
        );
    }
}
