//! HISTORICAL WATCHBILL SPECIMEN. Slice 3 acceptance — operator attention persists across runs and
//! shows up at reconcile time through the read-time projection.
//!
//! Invariants exercised:
//! - Ack persists across NQ generations; the next reconcile sees
//!   `attention_state=Acknowledged` until `ack_expires_at`.
//! - Ack with elapsed TTL re-surfaces as `Unowned` with urgency
//!   bumped by one step.
//! - Silence requires both `until` and `reason` (enforced by the
//!   `AttentionRow::silence` constructor; CLI also enforces).
//! - Silence is not handling: a silenced finding remains visible in
//!   `runs list` (it does not get hidden).
//! - Attention never raises authority: `requested_authority_level`
//!   and the effective ceiling are unchanged by ack/silence.
//! - Attention transitions emit `RunAttentionChanged` ledger events.
//! - Different `(agenda, finding)` pairs are isolated — one agenda
//!   acking does not silence another.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::{Duration, TimeZone, Utc};

use nightshiftd::agenda::Agenda;
use nightshiftd::attention::{AttentionRow, PersistedAttentionState, ReAckDisposition};
use nightshiftd::errors::Result;
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
use nightshiftd::ledger::RunLedgerEventKind;
use nightshiftd::nq::NqSource;
use nightshiftd::packet::{AttentionState, OperationalUrgency};
use nightshiftd::pipeline::{run_watchbill, PipelineOptions};
use nightshiftd::store::sqlite::SqliteStore;
use nightshiftd::store::{RunFilter, Store};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn target() -> FindingKey {
    FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    }
}

fn agenda() -> Agenda {
    Agenda::from_yaml_file(&fixtures_dir().join("wal-bloat-review.yaml")).unwrap()
}

fn snapshot_at(generation: u64) -> FindingSnapshot {
    FindingSnapshot {
        finding_key: target(),
        host: "labelwatch-host".into(),
        severity: Severity::Warning,
        domain: Some("delta_g".into()),
        persistence_generations: 3,
        first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: generation,
        captured_at: Utc::now(),
        evidence_hash: String::new(),
        origin: None,
        silence: None,

        position: None,    }
}

/// A scripted NQ source whose returned snapshot can be mutated
/// between runs to simulate generation advances.
struct MutableNqSource {
    snap: Mutex<FindingSnapshot>,
}

impl MutableNqSource {
    fn new(snap: FindingSnapshot) -> Self {
        Self {
            snap: Mutex::new(snap),
        }
    }
    fn advance(&self) {
        let mut s = self.snap.lock().unwrap();
        s.snapshot_generation += 1;
    }
}

impl NqSource for MutableNqSource {
    fn snapshot(&self, _key: &FindingKey) -> Result<Option<FindingSnapshot>> {
        Ok(Some(self.snap.lock().unwrap().clone()))
    }
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

#[test]
fn ack_persists_into_next_reconcile_and_packet_shows_acknowledged() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    // Initial reconcile — packet attention is Unowned (no prior).
    let pkt_v1 = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    assert_eq!(pkt_v1.attention.attention_state, AttentionState::Unowned);

    // Operator acks with a 4-hour TTL.
    let exp = Utc::now() + Duration::hours(4);
    let row = AttentionRow::ack(
        ag.agenda_id.clone(),
        target(),
        "alice".into(),
        Some(exp),
        Some("looking".into()),
        None,
    );
    store.save_attention(&row).unwrap();

    // Next NQ generation, next reconcile — packet now Acknowledged.
    nq.advance();
    let pkt_v2 = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    assert_eq!(pkt_v2.attention.attention_state, AttentionState::Acknowledged);
    assert!(pkt_v2.attention.acknowledged_at.is_some());
    assert_eq!(pkt_v2.attention.ack_expires_at.map(|t| t.timestamp()), Some(exp.timestamp()));
    // Operator ack also populates the next-check surface from Slice 2.
    assert_eq!(pkt_v2.attention.re_alert_after.map(|t| t.timestamp()), Some(exp.timestamp()));
}

#[test]
fn ack_with_elapsed_ttl_resurfaces_with_urgency_bumped() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    // Establish a baseline reconciled packet so we know the
    // un-bumped urgency for the same finding under this agenda.
    let baseline = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    let baseline_urgency = baseline.attention.operational_urgency;

    // Persist an ack whose TTL has already elapsed.
    let row = AttentionRow::ack(
        ag.agenda_id.clone(),
        target(),
        "alice".into(),
        Some(Utc::now() - Duration::hours(1)),
        None,
        None,
    );
    store.save_attention(&row).unwrap();

    nq.advance();
    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    // Expiry does NOT change attention_state (the default — typically
    // Unowned — stands), but it bumps urgency one step.
    assert_eq!(pkt.attention.attention_state, AttentionState::Unowned);
    let expected = match baseline_urgency {
        OperationalUrgency::Low => OperationalUrgency::Medium,
        OperationalUrgency::Medium => OperationalUrgency::High,
        OperationalUrgency::High => OperationalUrgency::Critical,
        OperationalUrgency::Critical => OperationalUrgency::Critical,
    };
    assert_eq!(pkt.attention.operational_urgency, expected);
}

#[test]
fn silence_applies_silenced_state_and_carries_reason() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    let until = Utc::now() + Duration::hours(2);
    let row = AttentionRow::silence(
        ag.agenda_id.clone(),
        target(),
        "alice".into(),
        until,
        "rolling restart underway".into(),
    );
    store.save_attention(&row).unwrap();

    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    assert_eq!(pkt.attention.attention_state, AttentionState::Silenced);
    assert_eq!(
        pkt.attention.silence_reason.as_deref(),
        Some("rolling restart underway")
    );
    assert_eq!(
        pkt.attention.re_alert_after.map(|t| t.timestamp()),
        Some(until.timestamp())
    );
}

#[test]
fn silence_does_not_hide_finding_from_runs_list() {
    // GAP-attention-state invariant: silence is not handling. A
    // silenced finding still surfaces in `runs list`.
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    let row = AttentionRow::silence(
        ag.agenda_id.clone(),
        target(),
        "alice".into(),
        Utc::now() + Duration::hours(2),
        "rolling restart".into(),
    );
    store.save_attention(&row).unwrap();

    let _ = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();
    let runs = store
        .list_runs(RunFilter {
            target_finding_key: Some(target().as_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(runs.len(), 1, "silenced finding's run must still be queryable");
    assert!(runs[0].completed_at.is_some());
}

#[test]
fn attention_never_raises_authority_under_ack_or_silence() {
    // CLAUDE.md invariant — attention state never grants additional
    // ceiling. Compared against a baseline run with no attention,
    // both ack and silence must leave `requested_authority_level`
    // and `governor_verdict` unchanged.
    let store_baseline = SqliteStore::open_in_memory().unwrap();
    let store_ack = SqliteStore::open_in_memory().unwrap();
    let store_silence = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    let baseline = run_watchbill(&ag, &target(), &nq, &store_baseline, &opts()).unwrap();

    store_ack
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(Utc::now() + Duration::hours(4)),
            None,
            None,
        ))
        .unwrap();
    let acked = run_watchbill(&ag, &target(), &nq, &store_ack, &opts()).unwrap();

    store_silence
        .save_attention(&AttentionRow::silence(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Utc::now() + Duration::hours(4),
            "maintenance".into(),
        ))
        .unwrap();
    let silenced = run_watchbill(&ag, &target(), &nq, &store_silence, &opts()).unwrap();

    assert_eq!(
        acked.proposed_action.requested_authority_level,
        baseline.proposed_action.requested_authority_level,
        "ack must not change requested authority"
    );
    assert_eq!(
        silenced.proposed_action.requested_authority_level,
        baseline.proposed_action.requested_authority_level,
        "silence must not change requested authority"
    );
    assert_eq!(
        acked.authority_result.governor_verdict,
        baseline.authority_result.governor_verdict,
        "ack must not change governor verdict surface"
    );
}

#[test]
fn attention_transitions_emit_run_attention_changed_event() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let nq = MutableNqSource::new(snapshot_at(1));

    store
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(Utc::now() + Duration::hours(4)),
            None,
            None,
        ))
        .unwrap();
    let pkt = run_watchbill(&ag, &target(), &nq, &store, &opts()).unwrap();

    let events = store.list_events(&pkt.run_id).unwrap();
    let changed = events
        .iter()
        .find(|e| matches!(e.kind, RunLedgerEventKind::RunAttentionChanged));
    let ev = changed.expect("attention application must emit a RunAttentionChanged event");
    let applied = ev.payload.get("applied").and_then(|v| v.as_str()).unwrap();
    assert_eq!(applied, "acknowledged");
}

#[test]
fn ack_in_one_agenda_does_not_silence_another_agenda() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag_a = agenda();
    let mut ag_b = agenda();
    ag_b.agenda_id = "wal-bloat-postmortem".into();
    let nq = MutableNqSource::new(snapshot_at(1));

    // Operator acks under agenda A only.
    store
        .save_attention(&AttentionRow::ack(
            ag_a.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(Utc::now() + Duration::hours(4)),
            None,
            None,
        ))
        .unwrap();

    let pkt_a = run_watchbill(&ag_a, &target(), &nq, &store, &opts()).unwrap();
    assert_eq!(pkt_a.attention.attention_state, AttentionState::Acknowledged);

    let pkt_b = run_watchbill(&ag_b, &target(), &nq, &store, &opts()).unwrap();
    assert_eq!(
        pkt_b.attention.attention_state,
        AttentionState::Unowned,
        "ack on agenda A must not be projected onto agenda B for the same finding"
    );
}

#[test]
fn save_attention_upsert_replaces_prior() {
    let store = SqliteStore::open_in_memory().unwrap();
    let ag = agenda();
    let first_exp = Utc::now() + Duration::hours(2);
    let second_exp = Utc::now() + Duration::hours(8);

    store
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(first_exp),
            None,
            None,
        ))
        .unwrap();
    store
        .save_attention(&AttentionRow::ack(
            ag.agenda_id.clone(),
            target(),
            "alice".into(),
            Some(second_exp),
            None,
            Some(ReAckDisposition::UnchangedWaiting),
        ))
        .unwrap();

    let row = store.get_attention(&ag.agenda_id, &target()).unwrap().unwrap();
    assert_eq!(row.state, PersistedAttentionState::Acknowledged);
    assert_eq!(
        row.ack_expires_at.map(|t| t.timestamp()),
        Some(second_exp.timestamp())
    );
    assert_eq!(row.disposition, Some(ReAckDisposition::UnchangedWaiting));
}
