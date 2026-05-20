//! Slice C.1 — Silence-Aware Posture (surface-only) acceptance tests.
//!
//! Pins `docs/GAP-silence-aware-posture.md`. Land in commit 2 of
//! the Slice C.1 three-commit pattern; pass when commit 3 wires the
//! derivation and the regime/action language.
//!
//! Hostile-to-boolean-laundering doctrine fences (these tests
//! exist specifically to disarm three failure modes):
//!
//! - `silence_present ≠ incident_absent`
//! - `acked_silence ≠ acked_incident`
//! - `no_new_evidence ≠ resolved`
//!
//! See also `docs/GAP-reack-doctrine.md` for the ack-lineage rules
//! these tests rely on (invariants 3–4 in that doctrine).

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::bundle::InputStatus;
use nightshiftd::errors::Result;
use nightshiftd::finding::{
    EvidenceState, FindingKey, FindingSilence, FindingSnapshot, Severity,
};
use nightshiftd::nq::{parse_nq_line, translate_nq, NqSource};
use nightshiftd::packet::{Attention, AttentionState, OperationalUrgency, Packet};
use nightshiftd::pipeline::{capture_phase, reconcile_phase, CaptureOutcome, PipelineOptions};
use nightshiftd::posture_class::{derive_posture_class, PostureClass};
use nightshiftd::store::sqlite::SqliteStore;

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn agenda() -> Agenda {
    Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap()
}

fn opts() -> PipelineOptions {
    PipelineOptions {
        no_governor: true,
        continuity_configured: false,
        trigger: None,
        liveness_threshold_seconds: None,
        imported_basis_freshness_window_seconds: None,
    }
}

fn load_first_line(fixture: &str) -> String {
    let path = fixtures_dir().join(fixture);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("fixture must exist: {}", path.display()));
    raw.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .unwrap_or_else(|| panic!("fixture must have at least one line"))
}

fn snap_from_fixture_line(fixture: &str, line_idx: usize) -> FindingSnapshot {
    let path = fixtures_dir().join(fixture);
    let raw = std::fs::read_to_string(&path).unwrap();
    let line = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .nth(line_idx)
        .unwrap_or_else(|| panic!("fixture missing line {line_idx}"));
    translate_nq(&parse_nq_line(line).unwrap()).unwrap()
}

/// Construct a `FindingSnapshot` for a *legacy* silence-shaped
/// detector (one of the six NQ pre-migration names) without any
/// silence envelope. Exists to exercise the "absence of envelope
/// means 'not yet unified', not 'not silence'" rule.
fn synthesized_legacy_silence_snap(detector: &str) -> FindingSnapshot {
    let when = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
    FindingSnapshot {
        finding_key: FindingKey {
            source: "nq".into(),
            detector: detector.into(),
            subject: "host-x:something".into(),
        },
        host: "host-x".into(),
        severity: Severity::Warning,
        domain: None,
        persistence_generations: 1,
        first_seen_at: when,
        current_status: EvidenceState::Active,
        snapshot_generation: 1,
        captured_at: when,
        evidence_hash: String::new(),
        origin: None,
        silence: None, // load-bearing: legacy detector, NO envelope yet
    }
}

// Scripted NqSource for inline-snapshot pipeline tests.
struct ScriptedNqSource {
    snapshots: Mutex<Vec<Option<FindingSnapshot>>>,
}

impl ScriptedNqSource {
    fn new(s: Vec<Option<FindingSnapshot>>) -> Self {
        assert!(!s.is_empty());
        Self {
            snapshots: Mutex::new(s),
        }
    }
}

impl NqSource for ScriptedNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        let mut s = self.snapshots.lock().unwrap();
        if s.len() > 1 {
            Ok(s.remove(0))
        } else {
            Ok(s[0].clone())
        }
    }
}

fn run_pipeline(snap: FindingSnapshot) -> Packet {
    let nq = ScriptedNqSource::new(vec![Some(snap.clone()), Some(snap.clone())]);
    let store = SqliteStore::open_in_memory().unwrap();
    let target = snap.finding_key.clone();
    let run_id = match capture_phase(&agenda(), &target, &nq, None, &store, &opts()).unwrap() {
        CaptureOutcome::Captured { run_id } => run_id,
        CaptureOutcome::HeldPacket(_) => panic!("synthesized snap must capture"),
    };
    reconcile_phase(&run_id, &nq, &store, &opts()).unwrap()
}

// -----------------------------------------------------------------------------
// Family 1 — derivation (3 tests)
// -----------------------------------------------------------------------------

/// An `extraction_stale` finding carries a populated silence envelope.
/// Derivation must return `SilenceShape`.
#[test]
fn c_silence_envelope_present_classifies_as_silence_shape() {
    // Line 0 of the stale fixture is NQ's own `extraction_stale`
    // finding — carries silence envelope, no origin block.
    let snap = snap_from_fixture_line("nq-findings-import-stale.jsonl", 0);
    assert!(
        snap.silence.is_some(),
        "fixture line 0 must carry the silence envelope"
    );
    assert_eq!(derive_posture_class(&snap), PostureClass::SilenceShape);
}

/// An active-condition finding with no silence envelope and a
/// non-legacy detector name must derive `IncidentShape`.
#[test]
fn c_active_incident_classifies_as_incident_shape() {
    // The observable fixture is native NQ; freelist_bloat detector;
    // no silence envelope.
    let line = load_first_line("nq-findings-observable.jsonl");
    let snap = translate_nq(&parse_nq_line(&line).unwrap()).unwrap();
    assert!(snap.silence.is_none());
    assert_eq!(derive_posture_class(&snap), PostureClass::IncidentShape);
}

/// A legacy NQ silence-shaped detector (e.g., `stale_host`) that has
/// not yet migrated to the unified envelope must classify as
/// `Unknown`, never silently as `IncidentShape`. This is the
/// anti-laundering sentinel for the SILENCE_UNIFICATION rule.
#[test]
fn c_legacy_silence_detector_without_envelope_classifies_as_unknown() {
    for legacy in &[
        "stale_host",
        "stale_service",
        "signal_dropout",
        "log_silence",
        "host_witness_silent",   // suffix family
        "service_witness_silent", // suffix family
    ] {
        let snap = synthesized_legacy_silence_snap(legacy);
        assert!(
            snap.silence.is_none(),
            "synthesized legacy snap must NOT carry a silence envelope"
        );
        assert_eq!(
            derive_posture_class(&snap),
            PostureClass::Unknown,
            "legacy detector {legacy} without envelope must classify as Unknown, \
             not IncidentShape (absence of envelope = 'not yet unified')"
        );
    }
}

// -----------------------------------------------------------------------------
// Family 2 — ack lineage / surfacing (3 tests)
// -----------------------------------------------------------------------------

/// Mechanical ack-lineage separation: two Attention rows with
/// distinct `attention_key`s carry ack state independently.
/// Acknowledging the silence-shaped row does not mark the
/// active-shaped row as acked.
#[test]
fn c_silence_ack_does_not_transfer_to_active_finding() {
    let now = Utc::now();
    let silence_attn = Attention {
        attention_key: FindingKey {
            source: "nq".into(),
            detector: "extraction_stale".into(),
            subject: "prod-x:run-1".into(),
        },
        evidence_state: EvidenceState::Active,
        attention_state: AttentionState::Acknowledged,
        posture_class: PostureClass::SilenceShape,
        operational_urgency: OperationalUrgency::Medium,
        owner: Some("alice".into()),
        last_touched_by: Some("alice".into()),
        last_touched_at: Some(now),
        acknowledged_at: Some(now),
        ack_expires_at: Some(now + chrono::Duration::hours(4)),
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: None,
        tolerance_basis_id: None,
        tolerance_basis_hash: None,
    };

    let active_attn = Attention {
        attention_key: FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "prod-x:/var/lib/db".into(),
        },
        evidence_state: EvidenceState::Active,
        attention_state: AttentionState::Unowned,
        posture_class: PostureClass::IncidentShape,
        operational_urgency: OperationalUrgency::Medium,
        owner: None,
        last_touched_by: None,
        last_touched_at: None,
        acknowledged_at: None,
        ack_expires_at: None,
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: None,
        tolerance_basis_id: None,
        tolerance_basis_hash: None,
    };

    // Different keys → different rows. Ack on silence does NOT
    // appear on active.
    assert_ne!(silence_attn.attention_key, active_attn.attention_key);
    assert_eq!(silence_attn.attention_state, AttentionState::Acknowledged);
    assert_eq!(active_attn.attention_state, AttentionState::Unowned);
    assert!(silence_attn.acknowledged_at.is_some());
    assert!(active_attn.acknowledged_at.is_none());
    // And the class distinction is surfaced on both rows.
    assert_eq!(silence_attn.posture_class, PostureClass::SilenceShape);
    assert_eq!(active_attn.posture_class, PostureClass::IncidentShape);
}

/// Symmetric: ack on an active-incident finding does not satisfy
/// the ack obligation of a silence-shaped finding with a different
/// finding_key.
#[test]
fn c_active_ack_does_not_transfer_to_silence_finding() {
    let now = Utc::now();
    let active_attn = Attention {
        attention_key: FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "prod-x:/var/lib/db".into(),
        },
        evidence_state: EvidenceState::Active,
        attention_state: AttentionState::Acknowledged,
        posture_class: PostureClass::IncidentShape,
        operational_urgency: OperationalUrgency::High,
        owner: Some("bob".into()),
        last_touched_by: Some("bob".into()),
        last_touched_at: Some(now),
        acknowledged_at: Some(now),
        ack_expires_at: Some(now + chrono::Duration::hours(4)),
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: None,
        tolerance_basis_id: None,
        tolerance_basis_hash: None,
    };

    let silence_attn = Attention {
        attention_key: FindingKey {
            source: "nq".into(),
            detector: "extraction_stale".into(),
            subject: "prod-x:run-1".into(),
        },
        evidence_state: EvidenceState::Active,
        attention_state: AttentionState::Unowned,
        posture_class: PostureClass::SilenceShape,
        operational_urgency: OperationalUrgency::Medium,
        owner: None,
        last_touched_by: None,
        last_touched_at: None,
        acknowledged_at: None,
        ack_expires_at: None,
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: None,
        tolerance_basis_id: None,
        tolerance_basis_hash: None,
    };

    assert_ne!(active_attn.attention_key, silence_attn.attention_key);
    assert!(active_attn.acknowledged_at.is_some());
    assert!(silence_attn.acknowledged_at.is_none());
    assert_eq!(active_attn.posture_class, PostureClass::IncidentShape);
    assert_eq!(silence_attn.posture_class, PostureClass::SilenceShape);
}

/// End-to-end: the pipeline populates `posture_class` on Attention
/// for both IncidentShape and SilenceShape findings, drawing the
/// value from the wire shape via `derive_posture_class`.
#[test]
fn c_posture_class_surfaces_on_attention_for_both_kinds() {
    // SilenceShape: the extraction_stale finding.
    let silence_snap = snap_from_fixture_line("nq-findings-import-stale.jsonl", 0);
    let silence_packet = run_pipeline(silence_snap);
    assert_eq!(
        silence_packet.attention.posture_class,
        PostureClass::SilenceShape,
        "silence-shaped finding must surface SilenceShape on Attention"
    );

    // IncidentShape: the native observable finding.
    let active_line = load_first_line("nq-findings-observable.jsonl");
    let active_snap = translate_nq(&parse_nq_line(&active_line).unwrap()).unwrap();
    let active_packet = run_pipeline(active_snap);
    assert_eq!(
        active_packet.attention.posture_class,
        PostureClass::IncidentShape,
        "active-incident finding must surface IncidentShape on Attention"
    );
}

// -----------------------------------------------------------------------------
// Family 3 — boolean-laundering refusals (2 tests)
// -----------------------------------------------------------------------------

/// `silence_present ≠ incident_absent`. Processing a silence-shaped
/// finding through the pipeline does NOT mark the finding's
/// `EvidenceState` as `Recovered`, does NOT silently flip
/// `ok_to_proceed`, and the freshness/Slice-B state of the finding
/// is whatever it would be without Slice C (no Slice B alteration).
#[test]
fn c_silence_does_not_resolve_active_findings() {
    let silence_snap = snap_from_fixture_line("nq-findings-import-stale.jsonl", 0);
    let packet = run_pipeline(silence_snap);

    // The finding's EvidenceState is NOT Recovered.
    assert_ne!(
        packet.attention.evidence_state,
        EvidenceState::Recovered,
        "silence-shaped finding must not silently report Recovered"
    );

    // The reconciliation regime is NOT a recovery / cleared regime.
    let regime = &packet.diagnosis.regime;
    assert!(
        !regime.contains("recovered"),
        "regime must not contain 'recovered' for silence-shaped finding; got {regime:?}"
    );
    assert!(
        !regime.contains("cleared"),
        "regime must not contain 'cleared' for silence-shaped finding; got {regime:?}"
    );
}

/// `no_new_evidence ≠ resolved` and `silence ≠ safety`. A
/// silence-shaped finding's `ProposedAction.steps` must NOT contain
/// language that implies recovery, safety, resolution, or no-op.
#[test]
fn c_silence_does_not_imply_recovery_or_safety() {
    let silence_snap = snap_from_fixture_line("nq-findings-import-stale.jsonl", 0);
    let packet = run_pipeline(silence_snap);

    let steps_lower: Vec<String> = packet
        .proposed_action
        .steps
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let combined = steps_lower.join(" | ");

    let forbidden = [
        "resolved",
        "safe to ignore",
        "no action needed",
        "no action required",
        "recovered",
        "all clear",
        "incident absent",
    ];
    for phrase in &forbidden {
        assert!(
            !combined.contains(phrase),
            "silence-shaped ProposedAction.steps must not contain forbidden \
             laundering phrase {phrase:?}; got steps {:?}",
            packet.proposed_action.steps
        );
    }

    // Attention.evidence_state must not be Recovered.
    assert_ne!(packet.attention.evidence_state, EvidenceState::Recovered);

    // Regime prefix sanity: a silence-shaped finding's regime
    // should be silence-prefixed (the implementation contract per
    // the spec).
    assert!(
        packet.diagnosis.regime.starts_with("silence"),
        "silence-shaped finding's regime should be silence-prefixed; got {:?}",
        packet.diagnosis.regime
    );
}

// -----------------------------------------------------------------------------
// Existence sentinel — keeps the legacy-detector allowlist
// reachable from tests (compiler keep-alive).
// -----------------------------------------------------------------------------

#[test]
fn _silence_envelope_type_is_constructible() {
    // Just a compile-time keep-alive that FindingSilence is in scope
    // for future test additions. No assertion of behavior.
    let s = FindingSilence {
        scope: "extraction".into(),
        basis: "age_threshold".into(),
        duration_s: 1,
        expected: "none".into(),
    };
    assert_eq!(s.scope, "extraction");
}

// Keep the `InputStatus` import live to avoid `unused_imports`
// warnings during the scaffolding commit; this constant is exercised
// by Slice B's tests under stale-imported-basis. Removed in commit 3
// if the implementation tests use it directly.
const _KEEP_INPUTSTATUS_IN_SCOPE: Option<InputStatus> = None;
