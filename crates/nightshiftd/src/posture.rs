//! Operator-facing posture surface.
//!
//! Reads a run's persisted state through the `Store` and synthesizes
//! a human-legible view answering: what ran, what held, why it held,
//! which finding_key it referred to, and whether reconcile happened.
//!
//! Rendering is plain text. No fancy UI. Fancy is a separate problem.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::errors::Result;
use crate::ledger::{RunLedgerEvent, RunLedgerEventKind};
use crate::packet::Packet;
use crate::store::{RunFilter, RunSummary, Store};

/// Synthesized view of one run assembled from the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunPosture {
    pub summary: RunSummary,
    pub events: Vec<RunLedgerEvent>,
    pub packet: Option<Packet>,
}

impl RunPosture {
    /// True if the run was held before reconcile — preflight
    /// (protected-class hold, coordinate, blocked) or the NQ
    /// liveness gate. Slice 2: liveness halts are also holds; the
    /// run never reached reconcile and the operator surface must
    /// not label it `ok`.
    pub fn is_held(&self) -> bool {
        let had_hold = self.events.iter().any(|e| {
            matches!(
                e.kind,
                RunLedgerEventKind::RunPreflightHold
                    | RunLedgerEventKind::RunPreflightCoordinate
                    | RunLedgerEventKind::RunPreflightBlocked
                    | RunLedgerEventKind::RunLivenessGateFailed
            )
        });
        let had_reconciled = self
            .events
            .iter()
            .any(|e| matches!(e.kind, RunLedgerEventKind::RunReconciled));
        had_hold && !had_reconciled
    }

    /// Name the gate that held the run, if any. Slice 2 hold-cause
    /// taxonomy: `liveness` (NQ liveness witness stale/skewed),
    /// `preflight` (coordination — protected-class, overlapping
    /// scope), or `None` for runs that reconciled normally.
    ///
    /// Horizon-driven escalation is *not* a hold — the run still
    /// reconciles and emits a packet with `Stale → revalidate-only`
    /// or `Invalidated` posture. Surfacing the horizon outcome lives
    /// on the packet's `next check` rendering, not here.
    pub fn hold_gate(&self) -> Option<&'static str> {
        if !self.is_held() {
            return None;
        }
        if self
            .events
            .iter()
            .any(|e| matches!(e.kind, RunLedgerEventKind::RunLivenessGateFailed))
        {
            return Some("liveness");
        }
        if self.events.iter().any(|e| {
            matches!(
                e.kind,
                RunLedgerEventKind::RunPreflightHold
                    | RunLedgerEventKind::RunPreflightCoordinate
                    | RunLedgerEventKind::RunPreflightBlocked
            )
        }) {
            return Some("preflight");
        }
        None
    }

    /// Human-readable hold reason if the run was held. None if not held
    /// or if the reason cannot be recovered from persisted state.
    pub fn hold_reason(&self) -> Option<String> {
        if !self.is_held() {
            return None;
        }
        // First preference: the packet documents the hold explicitly.
        if let Some(pkt) = &self.packet {
            if let Some(first_block) = pkt.reconciliation_summary.blocked.first() {
                return Some(first_block.clone());
            }
            if let Some(reason) = &pkt.attention.silence_reason {
                return Some(reason.clone());
            }
        }
        // Fallback: reconstruct from the hold event payload. Preflight
        // events carry `outcome` + `reasons`; liveness carries a
        // `verdict` string. Both render with the gate as the prefix
        // so the operator sees `<gate>: <detail>` consistently.
        let gate = self.hold_gate().unwrap_or("held");
        for e in &self.events {
            match e.kind {
                RunLedgerEventKind::RunLivenessGateFailed => {
                    let verdict = e
                        .payload
                        .get("verdict")
                        .and_then(|v| v.as_str())
                        .unwrap_or("witness not fresh");
                    return Some(format!("{gate}: {verdict}"));
                }
                RunLedgerEventKind::RunPreflightHold
                | RunLedgerEventKind::RunPreflightCoordinate
                | RunLedgerEventKind::RunPreflightBlocked => {
                    let outcome = e
                        .payload
                        .get("outcome")
                        .and_then(|v| v.as_str())
                        .unwrap_or("held");
                    let reasons = e
                        .payload
                        .get("reasons")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    return Some(if reasons.is_empty() {
                        format!("{gate} {outcome}")
                    } else {
                        format!("{gate} {outcome}: {reasons}")
                    });
                }
                _ => {}
            }
        }
        None
    }

    /// Short one-word status for list rendering.
    pub fn status_label(&self) -> &'static str {
        if self.is_held() {
            "HELD"
        } else if self
            .events
            .iter()
            .any(|e| matches!(e.kind, RunLedgerEventKind::RunCompleted))
        {
            "ok"
        } else {
            "running"
        }
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.summary.completed_at
    }
}

/// Load a single run's posture.
pub fn load_posture(store: &dyn Store, run_id: &str) -> Result<Option<RunPosture>> {
    let runs = store.list_runs(RunFilter {
        agenda_id: None,
        target_finding_key: None,
        limit: None,
    })?;
    let summary = match runs.into_iter().find(|r| r.run_id == run_id) {
        Some(s) => s,
        None => return Ok(None),
    };
    let events = store.list_events(run_id)?;
    let packet = store.get_packet(run_id)?;
    Ok(Some(RunPosture {
        summary,
        events,
        packet,
    }))
}

/// Filter knobs for the `runs list` surface.
#[derive(Debug, Default, Clone)]
pub struct PostureFilter {
    pub agenda_id: Option<String>,
    pub target_finding_key: Option<String>,
    pub held_only: bool,
    pub limit: Option<usize>,
}

/// List run postures according to the filter.
///
/// The store is queried with a coarse filter (agenda, finding), then
/// `held_only` is applied client-side since "held" is derived from
/// the event stream.
pub fn list_postures(store: &dyn Store, filter: &PostureFilter) -> Result<Vec<RunPosture>> {
    let runs = store.list_runs(RunFilter {
        agenda_id: filter.agenda_id.clone(),
        target_finding_key: filter.target_finding_key.clone(),
        limit: None,
    })?;

    let mut out = Vec::with_capacity(runs.len());
    for summary in runs {
        let events = store.list_events(&summary.run_id)?;
        let packet = store.get_packet(&summary.run_id)?;
        let posture = RunPosture {
            summary,
            events,
            packet,
        };
        if filter.held_only && !posture.is_held() {
            continue;
        }
        out.push(posture);
    }

    if let Some(n) = filter.limit {
        out.truncate(n);
    }
    Ok(out)
}

/// Render one run as a list-row string (stable, operator-friendly).
pub fn render_list_row(posture: &RunPosture) -> String {
    let status = posture.status_label();
    let started = posture.summary.started_at.format("%Y-%m-%d %H:%M:%SZ");
    let finding = posture
        .summary
        .target_finding_key
        .clone()
        .unwrap_or_else(|| "(none)".into());
    let mut row = format!(
        "{run_id}  {started}  {status:<7}  {agenda:<24}  {finding}",
        run_id = posture.summary.run_id,
        agenda = posture.summary.agenda_id,
    );
    if let Some(reason) = posture.hold_reason() {
        row.push_str(&format!("\n    hold: {reason}"));
    }
    row
}

/// Render full detail for `runs show <run_id>`.
pub fn render_show(posture: &RunPosture) -> String {
    let mut out = String::new();
    out.push_str(&posture.summary.run_id);
    out.push('\n');
    out.push_str(&format!("  agenda:     {}\n", posture.summary.agenda_id));
    out.push_str(&format!("  trigger:    {:?}\n", posture.summary.trigger));
    out.push_str(&format!(
        "  target:     {}\n",
        posture
            .summary
            .target_finding_key
            .as_deref()
            .unwrap_or("(none)")
    ));
    out.push_str(&format!(
        "  started:    {}\n",
        posture.summary.started_at.to_rfc3339()
    ));
    out.push_str(&format!(
        "  completed:  {}\n",
        match posture.summary.completed_at {
            Some(t) => t.to_rfc3339(),
            None => "(running)".into(),
        }
    ));
    out.push_str(&format!("  status:     {}\n", posture.status_label()));

    if let Some(reason) = posture.hold_reason() {
        if let Some(gate) = posture.hold_gate() {
            out.push_str(&format!("  hold gate:  {gate}\n"));
        }
        out.push_str(&format!("  hold cause: {reason}\n"));
    }

    if let Some(pkt) = &posture.packet {
        out.push_str(&format!(
            "  ceiling:    requested={:?} governor_present={}\n",
            pkt.proposed_action.requested_authority_level, pkt.authority_result.governor_present
        ));
        if let Some(note) = &pkt.authority_result.ceiling_note {
            out.push_str(&format!("              {note}\n"));
        }
        out.push_str(&format!(
            "  urgency:    {:?}\n",
            pkt.attention.operational_urgency
        ));
        out.push_str(&format!(
            "  evidence:   {:?}\n",
            pkt.attention.evidence_state
        ));
        // Slice 2 — attention + watch context. Operator scans this
        // block to see "what should happen next, and when." Lines
        // appear only when populated; absent fields are silently
        // skipped to keep the output dense.
        out.push_str(&format!(
            "  attention:  {:?}\n",
            pkt.attention.attention_state
        ));
        out.push_str(&format!(
            "  posture:    {:?}\n",
            pkt.attention.posture_class
        ));
        out.push_str(&format!(
            "  proposed:   {:?}\n",
            pkt.proposed_action.kind
        ));
        if let Some(owner) = &pkt.attention.owner {
            out.push_str(&format!("  owner:      {owner}\n"));
        }
        if let Some(t) = pkt.attention.re_alert_after {
            out.push_str(&format!("  next check: {}\n", t.to_rfc3339()));
        }
        if let Some(basis) = &pkt.attention.tolerance_basis_id {
            out.push_str(&format!("  watch basis: {basis}\n"));
        }
        if let Some(t) = pkt.attention.ack_expires_at {
            out.push_str(&format!("  ack expires: {}\n", t.to_rfc3339()));
        }
        if let Some(t) = pkt.attention.follow_up_by {
            out.push_str(&format!("  follow up:  {}\n", t.to_rfc3339()));
        }
        if !pkt.receipt_references.governor_receipts.is_empty() {
            out.push_str(&format!(
                "  gov receipts: {}\n",
                pkt.receipt_references.governor_receipts.join(", ")
            ));
        }
    } else {
        out.push_str("  (no packet saved)\n");
    }

    out.push_str("\nevents:\n");
    if posture.events.is_empty() {
        out.push_str("  (none)\n");
    } else {
        for e in &posture.events {
            let when = e.at.format("%Y-%m-%d %H:%M:%SZ");
            let kind = serde_json::to_value(e.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_else(|| format!("{:?}", e.kind));
            out.push_str(&format!("  [{when}] {kind}"));
            if let Some(obj) = e.payload.as_object() {
                if !obj.is_empty() {
                    let mut pairs: Vec<_> = obj.iter().collect();
                    pairs.sort_by_key(|(k, _)| k.to_string());
                    let rendered: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| format!("{k}={}", short_json(v)))
                        .collect();
                    out.push_str(&format!("  {}", rendered.join(" ")));
                }
            }
            out.push('\n');
        }
    }
    out
}

/// Compact a JSON value for inline event-row rendering.
fn short_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => {
            let s = serde_json::to_string(other).unwrap_or_else(|_| "?".into());
            if s.len() > 80 {
                format!("{}…", &s[..80])
            } else {
                s
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::RunLedgerEventKind;
    use crate::store::RunTrigger;
    use chrono::TimeZone;

    fn summary(run_id: &str) -> RunSummary {
        RunSummary {
            run_id: run_id.into(),
            agenda_id: "a".into(),
            trigger: RunTrigger::Manual,
            target_finding_key: Some("nq:d:s".into()),
            started_at: Utc.with_ymd_and_hms(2026, 4, 17, 0, 0, 0).unwrap(),
            completed_at: Some(Utc.with_ymd_and_hms(2026, 4, 17, 0, 0, 1).unwrap()),
        }
    }

    fn event(kind: RunLedgerEventKind, payload: serde_json::Value) -> RunLedgerEvent {
        RunLedgerEvent {
            event_id: format!("ev_{:?}", kind),
            run_id: "r".into(),
            kind,
            at: Utc.with_ymd_and_hms(2026, 4, 17, 0, 0, 0).unwrap(),
            payload,
        }
    }

    #[test]
    fn held_run_has_hold_reason_from_event_when_packet_missing() {
        let p = RunPosture {
            summary: summary("r1"),
            events: vec![
                event(RunLedgerEventKind::RunCaptured, serde_json::Value::Null),
                event(
                    RunLedgerEventKind::RunPreflightHold,
                    serde_json::json!({"outcome": "hold_for_context", "reasons": ["protected-class service in scope"]}),
                ),
                event(RunLedgerEventKind::RunCompleted, serde_json::Value::Null),
            ],
            packet: None,
        };
        assert!(p.is_held());
        let reason = p.hold_reason().unwrap();
        assert!(reason.contains("hold_for_context"));
        assert!(reason.contains("protected-class service in scope"));
        assert_eq!(p.status_label(), "HELD");
    }

    #[test]
    fn reconciled_run_is_not_held() {
        let p = RunPosture {
            summary: summary("r2"),
            events: vec![
                event(RunLedgerEventKind::RunCaptured, serde_json::Value::Null),
                event(RunLedgerEventKind::RunPreflightCleared, serde_json::Value::Null),
                event(RunLedgerEventKind::RunReconciled, serde_json::Value::Null),
                event(RunLedgerEventKind::RunCompleted, serde_json::Value::Null),
            ],
            packet: None,
        };
        assert!(!p.is_held());
        assert_eq!(p.status_label(), "ok");
        assert!(p.hold_reason().is_none());
    }

    #[test]
    fn running_label_when_not_completed() {
        let mut s = summary("r3");
        s.completed_at = None;
        let p = RunPosture {
            summary: s,
            events: vec![event(RunLedgerEventKind::RunCaptured, serde_json::Value::Null)],
            packet: None,
        };
        assert_eq!(p.status_label(), "running");
    }

    // ---- Slice 2 unit tests --------------------------------------

    #[test]
    fn liveness_failed_run_is_held_with_liveness_gate() {
        // Without a packet — falling back to event-payload reading.
        let p = RunPosture {
            summary: summary("r-liv"),
            events: vec![
                event(
                    RunLedgerEventKind::RunLivenessGateFailed,
                    serde_json::json!({
                        "instance_id": "labelwatch-host",
                        "verdict": "stale (age_seconds=600 > threshold 60)",
                    }),
                ),
                event(RunLedgerEventKind::RunCompleted, serde_json::Value::Null),
            ],
            packet: None,
        };
        assert!(p.is_held());
        assert_eq!(p.status_label(), "HELD");
        assert_eq!(p.hold_gate(), Some("liveness"));
        let reason = p.hold_reason().unwrap();
        assert!(reason.starts_with("liveness:"), "{reason}");
        assert!(reason.contains("stale"), "{reason}");
    }

    #[test]
    fn preflight_hold_gate_is_named_in_render() {
        // Confirms the prefix-rename from "preflight ..." to
        // "preflight <outcome>: <reasons>" still leads with the gate
        // word so `render_show` can label `hold gate: preflight`.
        let p = RunPosture {
            summary: summary("r-pre"),
            events: vec![
                event(RunLedgerEventKind::RunCaptured, serde_json::Value::Null),
                event(
                    RunLedgerEventKind::RunPreflightHold,
                    serde_json::json!({
                        "outcome": "hold_for_context",
                        "reasons": ["protected-class service in scope"],
                    }),
                ),
                event(RunLedgerEventKind::RunCompleted, serde_json::Value::Null),
            ],
            packet: None,
        };
        assert_eq!(p.hold_gate(), Some("preflight"));
        let show = render_show(&p);
        assert!(show.contains("hold gate:  preflight"), "{show}");
    }

    /// Build a minimal reconciled packet with `re_alert_after` and
    /// `tolerance_basis_id` set — the WatchUntil shape produced by
    /// the horizon path. Used to confirm `render_show` surfaces the
    /// next-check / watch-basis lines.
    fn mk_watch_until_posture() -> RunPosture {
        let pkt_json = serde_json::json!({
            "packet_version": 0,
            "packet_id": "pkt_test",
            "agenda_id": "a",
            "run_id": "r-watch",
            "produced_at": "2026-05-27T03:00:00Z",
            "finding_summary": {
                "source": "nq",
                "detector": "wal_bloat",
                "host": "h",
                "subject": "s",
                "severity": "warning",
                "persistence_generations": 3,
                "first_seen_at": "2026-05-01T00:00:00Z",
                "current_status": "active"
            },
            "reconciliation_summary": {
                "ok_to_proceed": true,
                "admissible_for_authorization": [],
                "admissible_for_proposal": [],
                "admissible_for_diagnosis": [],
                "blocked": [],
                "demoted": [],
                "stale": [],
                "invalidated": []
            },
            "diagnosis": {
                "regime": "committed",
                "evidence": [],
                "confidence": "medium",
                "alternatives_considered": []
            },
            "proposed_action": {
                "kind": "advisory",
                "steps": [],
                "risk_notes": [],
                "reversible": true,
                "blast_radius": "none",
                "requested_authority_level": "advise"
            },
            "authority_result": {
                "requested": "advise",
                "governor_present": false,
                "authority_receipts": []
            },
            "diagnosis_review": {
                "mode": "self_check"
            },
            "attention": {
                "attention_key": {"source": "nq", "detector": "d", "subject": "s"},
                "evidence_state": "active",
                "attention_state": "watch_until",
                "operational_urgency": "medium",
                "re_alert_after": "2026-05-27T15:30:00Z",
                "tolerance_basis_id": "maintenance-window-2026-q2"
            },
            "receipt_references": {
                "governor_receipts": ["receipt_abc123"]
            }
        });
        let pkt: Packet = serde_json::from_value(pkt_json).expect("test packet must deserialize");
        RunPosture {
            summary: summary("r-watch"),
            events: vec![
                event(RunLedgerEventKind::RunCaptured, serde_json::Value::Null),
                event(RunLedgerEventKind::RunReconciled, serde_json::Value::Null),
                event(RunLedgerEventKind::RunCompleted, serde_json::Value::Null),
            ],
            packet: Some(pkt),
        }
    }

    #[test]
    fn render_show_surfaces_next_check_and_watch_basis() {
        let p = mk_watch_until_posture();
        let show = render_show(&p);
        assert!(
            show.contains("next check: 2026-05-27T15:30:00+00:00"),
            "render_show missing next-check line: {show}"
        );
        assert!(
            show.contains("watch basis: maintenance-window-2026-q2"),
            "render_show missing watch-basis line: {show}"
        );
        assert!(
            show.contains("attention:  WatchUntil"),
            "render_show missing attention-state: {show}"
        );
        assert!(
            show.contains("gov receipts: receipt_abc123"),
            "render_show missing governor-receipts line: {show}"
        );
    }
}
