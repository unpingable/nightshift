//! Evidence findings from NQ (or equivalent evidence adapters).
//!
//! `FindingKey` is the stable identity that survives regeneration,
//! snapshot refresh, and status transitions. Attention state, run
//! ledger references, and reconciliation all key on it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FindingKey {
    pub source: String,
    pub detector: String,
    pub subject: String,
}

impl FindingKey {
    pub fn as_string(&self) -> String {
        format!("{}:{}:{}", self.source, self.detector, self.subject)
    }

    /// Parse from the `input_id` format used in `CaptureInput` for
    /// NQ-backed inputs: `"nq:finding:<detector>:<subject>"`. Returns
    /// `None` for non-NQ or malformed ids. Source defaults to `"nq"`.
    pub fn from_nq_input_id(id: &str) -> Option<Self> {
        let parts: Vec<&str> = id.splitn(4, ':').collect();
        match parts.as_slice() {
            ["nq", "finding", detector, subject] => Some(FindingKey {
                source: "nq".into(),
                detector: (*detector).into(),
                subject: (*subject).into(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_key_is_stable_under_clone() {
        let k = FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "labelwatch-host:/var/lib/db".into(),
        };
        assert_eq!(k.as_string(), k.clone().as_string());
    }

    #[test]
    fn finding_key_round_trips_through_json() {
        let k = FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "labelwatch-host:/var/lib/db".into(),
        };
        let s = serde_json::to_string(&k).unwrap();
        let k2: FindingKey = serde_json::from_str(&s).unwrap();
        assert_eq!(k, k2);
        assert_eq!(k.as_string(), k2.as_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Active,
    Worsening,
    Resolving,
    Recovered,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Warning,
    Critical,
}

/// A snapshot of a single finding at a single generation.
/// Evidence, not a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSnapshot {
    pub finding_key: FindingKey,
    pub host: String,
    pub severity: Severity,
    pub domain: Option<String>,
    pub persistence_generations: u32,
    pub first_seen_at: DateTime<Utc>,
    pub current_status: EvidenceState,
    pub snapshot_generation: u64,
    pub captured_at: DateTime<Utc>,
    pub evidence_hash: String,
}
