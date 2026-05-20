//! Silence-Aware Posture — Slice C.1 (surface-only).
//!
//! See `docs/GAP-silence-aware-posture.md`.
//!
//! **The deep rule:** NQ owns truth and evidence classification.
//! Night Shift owns posture, attention, and ack obligation. A
//! silence-shaped finding may alter notification/ack posture, but
//! MUST NOT imply absence of incident, recovery, safety, or active
//! resolution.
//!
//! **Keeper:** *"A silence ack is not an active-finding ack."*
//!
//! Three hostile-to-boolean-laundering invariants this module exists
//! to keep alive:
//!
//! - `silence_present ≠ incident_absent`
//! - `acked_silence ≠ acked_incident`
//! - `no_new_evidence ≠ resolved`
//!
//! ## Distinct from `crate::posture` (run-level posture)
//!
//! The neighboring `posture` module synthesizes *run posture* (an
//! operator-facing view of one Watchbill run's state). This module
//! is about *finding posture-class* (the shape of a single finding —
//! incident vs absence-shaped). Different concept, different scope.
//!
//! ## Distinct from `AttentionState::Silenced`
//!
//! `AttentionState::Silenced` means *operator-silenced* (suppression
//! with reason + TTL per `GAP-attention-state.md`). `PostureClass::
//! SilenceShape` means *evidence-shape-is-silence*. Two different
//! concepts that happen to use the word "silence." Per the Slice C
//! design Open Q2: accept the collision; document carefully.

use serde::{Deserialize, Serialize};

use crate::finding::FindingSnapshot;

/// Posture class derived from a finding's *shape*, not its
/// individual content. Read NQ's `silence` envelope to classify.
///
/// `Unknown` is the safe default: per the SILENCE_UNIFICATION rule
/// from NQ, **absence of the envelope means "not yet unified," not
/// "not silence."** Night Shift refuses to silently classify legacy
/// silence detectors as `IncidentShape` while their envelopes are
/// pending migration; it surfaces them as `Unknown` instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostureClass {
    /// Reserved for findings whose shape cannot be inferred from
    /// the current wire surface — primarily legacy NQ silence
    /// detectors that have not yet migrated to the
    /// SILENCE_UNIFICATION envelope. **NOT a default for
    /// IncidentShape.** Surfaces the absence as *not yet unified*.
    /// Also the literal default when posture has not been derived
    /// yet (e.g., test mocks that don't care).
    #[default]
    Unknown,
    /// Active-condition findings — the bulk of NQ's emissions. NQ
    /// emitted no `silence` envelope, and the detector is not on
    /// the legacy-silence allowlist.
    IncidentShape,
    /// Findings whose content is *about* absence of expected
    /// observation. v1: `snap.silence.is_some()`. Notification
    /// language and ack lineage differ from `IncidentShape`.
    SilenceShape,
}

/// Hardcoded allowlist of NQ V1 legacy silence-shaped detectors
/// whose findings do NOT yet carry the unified `silence` envelope.
/// Until NQ migrates them, Night Shift classifies them as `Unknown`.
///
/// Per Slice C design Open Q5: hardcoded constant, not config. Six
/// entries is small; the migration is a known timeline; YAGNI for a
/// config knob until a forcing case.
const LEGACY_SILENCE_DETECTORS: &[&str] = &[
    "stale_host",
    "stale_service",
    "signal_dropout",
    "log_silence",
];

/// Suffix match for the `*_witness_silent` family of legacy silence
/// detectors. Captures `host_witness_silent`,
/// `service_witness_silent`, etc. without enumerating every
/// possibility.
const LEGACY_WITNESS_SILENT_SUFFIX: &str = "_witness_silent";

/// True if `detector` is in the legacy silence-shaped detector
/// allowlist (exact match or `_witness_silent` suffix).
pub fn is_legacy_silence_detector(detector: &str) -> bool {
    LEGACY_SILENCE_DETECTORS.contains(&detector)
        || detector.ends_with(LEGACY_WITNESS_SILENT_SUFFIX)
}

/// Derive the `PostureClass` for a `FindingSnapshot` from its
/// wire-shape (silence envelope presence + detector name).
///
/// Derivation rule (per `docs/GAP-silence-aware-posture.md`):
///
/// ```text
/// snap.silence.is_some()                         → SilenceShape
/// is_legacy_silence_detector(snap.finding_key.detector)
///                                                → Unknown
///                                                  (post-NQ-migration: SilenceShape)
/// otherwise                                      → IncidentShape
/// ```
///
/// The function is pure and deterministic; same snapshot always
/// derives the same posture class. Slice C.1 wires this into the
/// pipeline at three surfaces (ReconciliationResult, FindingSummary,
/// Attention).
pub fn derive_posture_class(snap: &FindingSnapshot) -> PostureClass {
    // Slice C.1 derivation rule. Order matters: envelope presence
    // wins over detector-name heuristic, so a future NQ migration
    // that adds the envelope to a legacy detector promotes it from
    // Unknown to SilenceShape without code changes here.
    if snap.silence.is_some() {
        return PostureClass::SilenceShape;
    }
    if is_legacy_silence_detector(&snap.finding_key.detector) {
        // Legacy NQ silence detector whose unified envelope hasn't
        // arrived yet. Surface as Unknown rather than silently
        // classifying as IncidentShape.
        return PostureClass::Unknown;
    }
    PostureClass::IncidentShape
}
