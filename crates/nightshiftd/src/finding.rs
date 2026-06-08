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

/// Witness observation lane, per NQ canonical taxonomy
/// (`nq_core::witness::WitnessPosition`). Three values shipped on
/// NQ's wire as of `ad26dc4` (2026-06-08): `substrate`,
/// `application_internal`, `platform`.
///
/// **Render-only.** Night Shift does not branch reconciliation,
/// notification posture, ack obligation, or freshness semantics on
/// this value. The render surface shows it when present; absence
/// renders as an operator-prose absence label — *not* a sixth
/// taxonomy variant. Inference from `witness_type` /
/// `detector` is forbidden by sentinel test.
///
/// See `CLAUDE.md` invariant on NS not branching on NQ witness
/// positions; this enum exists to render NQ's claim, not to
/// re-derive it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessPosition {
    Substrate,
    ApplicationInternal,
    Platform,
}

/// Origin envelope for findings ingested through NQ's
/// `nq.finding_import.v1` substrate (`DURABLE_ARTIFACT_SUBSTRATE_GAP`
/// V1). Present only when the upstream NQ snapshot carries
/// `origin.source = "import"`; absent for native NQ findings.
///
/// **Visibility-only.** Slice A consumes this envelope as
/// evidence-bearing context — Night Shift surfaces it through peek
/// and packet so operators can see both clocks side-by-side. Night
/// Shift does not branch reconciliation, notification posture, ack
/// obligation, or freshness semantics on any field here. See
/// "Deferred semantics" in `tests/durable_artifact_substrate_v1.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingOrigin {
    /// Discriminator string. NQ ships this as `"import"`; the block's
    /// presence is itself the import-vs-native signal per NQ gap doc.
    pub source: String,
    pub producer_id: String,
    pub extraction_run_id: String,
    /// Producer's clock — the basis-recency timestamp for the
    /// underlying corpus state. Distinct from `FindingSnapshot.captured_at`
    /// (which grounds in NQ ingest time). Two-clock provenance per
    /// NQ gap §"Two-clock provenance is load-bearing."
    ///
    /// `None` when the JSON field was absent OR present but
    /// unparseable. The companion field
    /// `producer_extraction_time_raw` distinguishes the two cases:
    /// `None` raw means the field was absent in JSON; `Some(s)` raw
    /// means it was present but failed to parse as RFC3339. This
    /// distinction matters for `GAP-imported-basis-freshness.md`
    /// receipt reasons (`imported_producer_basis_missing` vs
    /// `imported_producer_clock_incoherent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_extraction_time: Option<DateTime<Utc>>,
    /// Raw `producer_extraction_time` string from the NQ wire, used
    /// to distinguish "absent in JSON" (`None`) from "present but
    /// unparseable" (`Some(_)` with `producer_extraction_time =
    /// None`). See `GAP-imported-basis-freshness.md` time semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_extraction_time_raw: Option<String>,
    pub import_contract_version: u32,
}

/// Silence envelope for findings using NQ's SILENCE_UNIFICATION
/// shared contract. V1 only populates this for `extraction_stale`
/// findings; the six legacy silence detectors omit it pending their
/// own migration. Per NQ gap doc: consumers must read missing
/// `silence` as **not yet unified**, not **not silence**.
///
/// **Visibility-only.** Night Shift carries silence through for
/// operator visibility; it does not change notification posture or
/// ack obligation based on these fields in Slice A.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingSilence {
    /// What is silent. `extraction` for V1; future scopes (`host`,
    /// `service`, `witness`, `metric`, `log`) added as legacy
    /// detectors migrate.
    pub scope: String,
    /// Mechanism shape. `age_threshold` for V1; future:
    /// `presence_delta`, `baseline_collapse`.
    pub basis: String,
    /// Seconds since silence began. For `scope = extraction`,
    /// computed against the producer clock.
    pub duration_s: i64,
    /// `none` in V1; future values when silence is operator-declared.
    pub expected: String,
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
    /// Present for findings ingested via NQ's V1 durable-artifact
    /// substrate (`origin.source = "import"`); absent for native NQ
    /// findings. Visibility-only; no NS branching on this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<FindingOrigin>,
    /// Present for silence-shaped findings using NQ's
    /// SILENCE_UNIFICATION contract (V1: `extraction_stale` only).
    /// Absence means **not yet unified**, not **not silence**.
    /// Visibility-only; no NS branching on this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silence: Option<FindingSilence>,
    /// NQ witness observation lane per `nq-witness-api`
    /// `WitnessPosition`. Present when NQ stamps it on the wire;
    /// absent on packets predating the witness.position cut-over
    /// (and absent on every wire surface NS consumes today —
    /// `nq findings export` does not carry it). Render-only;
    /// absent renders as `position: not testified`. No inference
    /// from `witness_type` / detector — guarded by sentinel test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<WitnessPosition>,
}
