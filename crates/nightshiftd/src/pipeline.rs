//! Pipeline — capture → reconcile → packet.
//!
//! v1 orchestrator. Reads agenda, captures current evidence into a
//! bundle, reconciles the bundle against live sources, emits a packet.
//! No persistence (commit C adds Store integration). No mutation.

use chrono::Utc;

use crate::agenda::{Agenda, AuthorityLevel, CriticalityClass};
use crate::bundle::{
    Bundle, CaptureInput, CapturePhase, Freshness, InputStatus, InvalidationRule,
};
use crate::coordination::{classify_risky, preflight, scope_key, CoordinationOutcome};
use crate::errors::{NightShiftError, Result};
use crate::finding::{EvidenceState, FindingKey};
use crate::ledger::{RunLedgerEvent, RunLedgerEventKind};
use crate::nq::{evidence_hash, NqSource};
use crate::packet::{
    AlternativeRegime, Attention, AttentionState, AuthorityResult, Confidence, Diagnosis,
    DiagnosisReview, DiagnosisReviewMode, FindingSummary, OperationalUrgency, Packet,
    ProposedAction, ProposedActionKind, ReceiptReferences,
};
use crate::reconciler;
use crate::store::{RunTrigger, Store};

/// Runtime options for a pipeline invocation.
#[derive(Debug, Clone, Default)]
pub struct PipelineOptions {
    pub no_governor: bool,
    /// Whether a Continuity substrate is configured for this
    /// deployment. v1 has no real Continuity integration yet; this
    /// flag controls preflight behavior for risky-class runs so the
    /// contract in `GAP-parallel-ops.md` can be exercised now.
    /// Default: false (standalone).
    pub continuity_configured: bool,
    /// How this run was triggered. Recorded in the run row.
    pub trigger: Option<RunTrigger>,
}

/// Execute the v1 Watchbill pipeline for a single finding target.
///
/// Persists through the `Store`: creates the run record, saves the
/// bundle and packet, emits `run.captured` / `run.reconciled` /
/// `run.completed` ledger events. Attention keys on the stable
/// `FindingKey` per GAP-attention-state.md.
pub fn run_watchbill(
    agenda: &Agenda,
    target: &FindingKey,
    nq: &dyn NqSource,
    store: &dyn Store,
    opts: &PipelineOptions,
) -> Result<Packet> {
    // Ensure agenda persisted — the store is the durable record.
    store.create_agenda(agenda)?;

    let trigger = opts.trigger.unwrap_or(RunTrigger::Manual);
    let run_id = store.create_run(&agenda.agenda_id, trigger, Some(target))?;
    // 1. Authority ceiling enforcement (pre-pipeline gate).
    //    Without Governor, ceiling cannot exceed advise.
    let effective_ceiling = effective_ceiling(agenda.promotion_ceiling, opts.no_governor);

    // 2. Capture phase — pull the current NQ snapshot to seed the bundle.
    let captured_snapshot = nq.snapshot(target)?.ok_or_else(|| {
        NightShiftError::InvalidAgenda(format!(
            "target finding not present in NQ source at capture time: {}",
            target.as_string()
        ))
    })?;
    let captured_hash = evidence_hash(&captured_snapshot);
    let captured_at = Utc::now();

    let capture_input = CaptureInput {
        input_id: format!("nq:finding:{}:{}", target.detector, target.subject),
        source: "nq".into(),
        kind: "nq_finding_snapshot".into(),
        status: InputStatus::Observed,
        evidence_hash: captured_hash.clone(),
        freshness: Freshness {
            captured_at,
            expires_at: None,
            invalidates_if: vec![
                InvalidationRule::FindingAbsentForNGenerations { n: 1 },
                InvalidationRule::HostUnreachable,
            ],
        },
        payload_ref: format!("nq:snapshot:{}", captured_snapshot.snapshot_generation),
        admissible_for: vec![],
        inadmissible_for: vec![],
    };

    let bundle_pre_reconcile = Bundle {
        bundle_version: 0,
        agenda_id: agenda.agenda_id.clone(),
        run_id: run_id.clone(),
        capture: CapturePhase {
            captured_at,
            captured_by: "scheduler".into(),
            capture_reason: "watchbill scheduled run".into(),
            inputs: vec![capture_input],
        },
        reconciliation: None,
    };

    store.append_run_event(&new_event(
        &run_id,
        RunLedgerEventKind::RunCaptured,
        serde_json::json!({
            "target_finding_key": target.as_string(),
            "captured_hash": captured_hash,
        }),
    ))?;

    // 2a. Preflight — risky-class coordination gate per
    //     GAP-parallel-ops.md. For non-risky agendas this is always
    //     Clear; for risky agendas without Continuity configured it
    //     holds the run, per CLAUDE.md invariant #18.
    let preflight_outcome = preflight(agenda, opts.continuity_configured);
    let risky = classify_risky(agenda);

    match preflight_outcome {
        CoordinationOutcome::Clear | CoordinationOutcome::OperatorOverride => {
            store.append_run_event(&new_event(
                &run_id,
                RunLedgerEventKind::RunPreflightCleared,
                serde_json::json!({
                    "outcome": "clear",
                    "risky": risky.is_risky(),
                    "reasons": risky.reasons(),
                }),
            ))?;
        }
        _ => {
            // Hold path — the run stops before reconcile.
            return hold_for_preflight(
                agenda,
                target,
                &captured_snapshot,
                &bundle_pre_reconcile,
                preflight_outcome,
                &risky,
                store,
                opts,
                effective_ceiling,
                &run_id,
            );
        }
    }

    // 3. Reconcile — live source check for each NQ input.
    let reconciliation = reconciler::reconcile_bundle(&bundle_pre_reconcile, nq)?;

    let reconciled_bundle = Bundle {
        reconciliation: Some(reconciliation.clone()),
        ..bundle_pre_reconcile
    };
    store.save_bundle(&run_id, &reconciled_bundle)?;

    store.append_run_event(&new_event(
        &run_id,
        RunLedgerEventKind::RunReconciled,
        serde_json::json!({
            "ok_to_proceed": reconciliation.summary.ok_to_proceed,
            "blocked": reconciliation.summary.blocked,
            "downgraded": reconciliation.summary.downgraded,
        }),
    ))?;

    // 4. Build the packet. v1 diagnosis is deterministic — no LLM.
    let nq_result = reconciliation
        .results
        .iter()
        .find(|r| r.input_id.starts_with("nq:finding:"))
        .expect("v1 pipeline seeded a single NQ input; it must be present in reconciliation");

    let current_snapshot = nq
        .snapshot(target)?
        .unwrap_or_else(|| captured_snapshot.clone());

    let finding_summary = FindingSummary {
        source: current_snapshot.finding_key.source.clone(),
        detector: current_snapshot.finding_key.detector.clone(),
        host: current_snapshot.host.clone(),
        subject: current_snapshot.finding_key.subject.clone(),
        severity: current_snapshot.severity,
        domain: current_snapshot.domain.clone(),
        persistence_generations: current_snapshot.persistence_generations,
        first_seen_at: current_snapshot.first_seen_at,
        current_status: current_snapshot.current_status,
    };

    let diagnosis = Diagnosis {
        regime: "v1 deterministic placeholder — LLM workflow deferred".into(),
        evidence: vec![format!(
            "finding {} persisted for {} generations; reconciliation status = {:?}",
            target.as_string(),
            current_snapshot.persistence_generations,
            nq_result.status
        )],
        confidence: Confidence::Low,
        alternatives_considered: vec![AlternativeRegime {
            regime: "alternative regimes not considered in v1".into(),
            ruled_out_by: "no LLM wired; deterministic placeholder".into(),
        }],
    };

    let proposed_action = ProposedAction {
        kind: ProposedActionKind::Advisory,
        steps: vec![
            "human review of the finding and reconciliation summary".into(),
            "no automated action taken in v1 Watchbill".into(),
        ],
        risk_notes: vec![],
        reversible: true,
        blast_radius: "none — advise only".into(),
        requested_authority_level: AuthorityLevel::Advise,
    };

    let ceiling_changed = effective_ceiling != agenda.promotion_ceiling;
    let (governor_verdict, ceiling_note) = if opts.no_governor {
        (
            Some("n/a (--no-governor; ceiling capped at advise)".into()),
            if ceiling_changed {
                Some(format!(
                    "agenda ceiling {:?} → effective {:?} because --no-governor",
                    agenda.promotion_ceiling, effective_ceiling
                ))
            } else {
                None
            },
        )
    } else {
        (Some("not integrated in v1".into()), None)
    };

    let authority_result = AuthorityResult {
        requested: AuthorityLevel::Advise,
        governor_present: !opts.no_governor,
        governor_verdict,
        authority_receipts: vec![],
        ceiling_note,
    };

    let diagnosis_review = DiagnosisReview {
        mode: DiagnosisReviewMode::SelfCheck,
        unsafe_assumptions: vec![],
        stale_context_risks: match nq_result.status {
            InputStatus::Stale => vec!["reconciliation produced stale NQ evidence".into()],
            _ => vec![],
        },
        promotion_overreach: vec![],
        missing_verification: vec!["v1 has no LLM self-check; placeholder review only".into()],
        recommended_downgrade: None,
    };

    let operational_urgency = urgency_from(
        current_snapshot.severity,
        agenda.criticality.class,
        nq_result.status,
    );

    let attention = Attention {
        attention_key: target.clone(),
        evidence_state: match nq_result.status {
            InputStatus::Stale | InputStatus::Invalidated => EvidenceState::Stale,
            _ => current_snapshot.current_status,
        },
        attention_state: AttentionState::Unowned,
        operational_urgency,
        owner: None,
        last_touched_by: None,
        last_touched_at: None,
        acknowledged_at: None,
        ack_expires_at: None,
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: None,
    };

    let packet_id = format!(
        "pkt_{}_{}",
        run_id,
        scope_key(&agenda.scope).chars().take(12).collect::<String>()
    );

    let packet = Packet {
        packet_version: 0,
        packet_id,
        agenda_id: agenda.agenda_id.clone(),
        run_id: run_id.clone(),
        produced_at: Utc::now(),
        finding_summary,
        reconciliation_summary: reconciliation.summary.clone(),
        diagnosis,
        proposed_action,
        authority_result,
        diagnosis_review,
        attention,
        receipt_references: ReceiptReferences {
            run_ledger: Some(format!("ledger://nightshift/runs/{run_id}")),
            governor_receipts: vec![],
            evidence_bundle: Some(format!("bundle://{run_id}")),
        },
    };

    store.save_packet(&run_id, &packet)?;

    store.append_run_event(&new_event(
        &run_id,
        RunLedgerEventKind::RunCompleted,
        serde_json::json!({
            "packet_id": packet.packet_id,
            "requested_authority_level": packet.proposed_action.requested_authority_level,
            "effective_ceiling": effective_ceiling,
        }),
    ))?;

    store.complete_run(&run_id)?;

    Ok(packet)
}

/// Handle a preflight hold — emit the appropriate event, save a
/// bundle with no reconciliation, build a packet that documents the
/// hold, and finalize the run. Returns the packet so the caller can
/// render it.
#[allow(clippy::too_many_arguments)]
fn hold_for_preflight(
    agenda: &Agenda,
    target: &FindingKey,
    captured_snapshot: &crate::finding::FindingSnapshot,
    bundle: &Bundle,
    outcome: CoordinationOutcome,
    risk: &crate::coordination::RiskyClassification,
    store: &dyn Store,
    opts: &PipelineOptions,
    effective_ceiling: AuthorityLevel,
    run_id: &str,
) -> Result<Packet> {
    let (event_kind, outcome_label) = match outcome {
        CoordinationOutcome::HoldForContext => {
            (RunLedgerEventKind::RunPreflightHold, "hold_for_context")
        }
        CoordinationOutcome::Coordinate => {
            (RunLedgerEventKind::RunPreflightCoordinate, "coordinate")
        }
        CoordinationOutcome::BlockForResolution => {
            (RunLedgerEventKind::RunPreflightBlocked, "block_for_resolution")
        }
        // Clear and OperatorOverride are handled on the main path and
        // do not enter this function. Treat as a bug if they do.
        other => {
            return Err(NightShiftError::PreflightBlocked(format!(
                "hold_for_preflight called with non-hold outcome: {other:?}"
            )));
        }
    };

    store.save_bundle(run_id, bundle)?;
    store.append_run_event(&new_event(
        run_id,
        event_kind,
        serde_json::json!({
            "outcome": outcome_label,
            "risky": risk.is_risky(),
            "reasons": risk.reasons(),
            "continuity_configured": opts.continuity_configured,
        }),
    ))?;

    let hold_reason = format!(
        "preflight {outcome_label}: {}",
        risk.reasons().join(", ")
    );

    let finding_summary = FindingSummary {
        source: captured_snapshot.finding_key.source.clone(),
        detector: captured_snapshot.finding_key.detector.clone(),
        host: captured_snapshot.host.clone(),
        subject: captured_snapshot.finding_key.subject.clone(),
        severity: captured_snapshot.severity,
        domain: captured_snapshot.domain.clone(),
        persistence_generations: captured_snapshot.persistence_generations,
        first_seen_at: captured_snapshot.first_seen_at,
        current_status: captured_snapshot.current_status,
    };

    let reconciliation_summary = crate::bundle::ReconciliationSummary {
        ok_to_proceed: false,
        blocked: vec![hold_reason.clone()],
        ..Default::default()
    };

    let diagnosis = Diagnosis {
        regime: "held — coordination preflight did not clear".into(),
        evidence: vec![format!(
            "risky-class reasons: {}",
            if risk.reasons().is_empty() {
                "(none declared)".to_string()
            } else {
                risk.reasons().join(", ")
            }
        )],
        confidence: Confidence::Low,
        alternatives_considered: vec![],
    };

    let proposed_action = ProposedAction {
        kind: ProposedActionKind::Advisory,
        steps: vec![
            "coordinate with overlapping actor(s) via Continuity".into(),
            "or invoke operator_override with a named reason".into(),
            "no pipeline action taken; run halted before reconcile".into(),
        ],
        risk_notes: vec![hold_reason.clone()],
        reversible: true,
        blast_radius: "none — run halted".into(),
        requested_authority_level: AuthorityLevel::Observe,
    };

    let authority_result = AuthorityResult {
        requested: AuthorityLevel::Observe,
        governor_present: !opts.no_governor,
        governor_verdict: Some("not consulted — preflight halted the run".into()),
        authority_receipts: vec![],
        ceiling_note: Some(format!(
            "ceiling {:?} → held at observe by preflight",
            effective_ceiling
        )),
    };

    let diagnosis_review = DiagnosisReview {
        mode: DiagnosisReviewMode::SelfCheck,
        unsafe_assumptions: vec![],
        stale_context_risks: vec![
            "no reconciliation performed — captured evidence not verified against current state"
                .into(),
        ],
        promotion_overreach: vec![],
        missing_verification: vec![hold_reason.clone()],
        recommended_downgrade: None,
    };

    let attention = Attention {
        attention_key: target.clone(),
        evidence_state: captured_snapshot.current_status,
        attention_state: AttentionState::Unowned,
        operational_urgency: urgency_from(
            captured_snapshot.severity,
            agenda.criticality.class,
            InputStatus::Observed,
        ),
        owner: None,
        last_touched_by: None,
        last_touched_at: None,
        acknowledged_at: None,
        ack_expires_at: None,
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: Some(hold_reason),
    };

    let packet_id = format!(
        "pkt_{}_{}",
        run_id,
        scope_key(&agenda.scope).chars().take(12).collect::<String>()
    );

    let packet = Packet {
        packet_version: 0,
        packet_id,
        agenda_id: agenda.agenda_id.clone(),
        run_id: run_id.to_string(),
        produced_at: Utc::now(),
        finding_summary,
        reconciliation_summary,
        diagnosis,
        proposed_action,
        authority_result,
        diagnosis_review,
        attention,
        receipt_references: ReceiptReferences {
            run_ledger: Some(format!("ledger://nightshift/runs/{run_id}")),
            governor_receipts: vec![],
            evidence_bundle: Some(format!("bundle://{run_id}")),
        },
    };

    store.save_packet(run_id, &packet)?;
    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunCompleted,
        serde_json::json!({
            "packet_id": packet.packet_id,
            "held": true,
            "outcome": outcome_label,
        }),
    ))?;
    store.complete_run(run_id)?;

    Ok(packet)
}

/// Construct a ledger event with a fresh UUID-based id.
fn new_event(
    run_id: &str,
    kind: RunLedgerEventKind,
    payload: serde_json::Value,
) -> RunLedgerEvent {
    RunLedgerEvent {
        event_id: format!("ev_{}", uuid::Uuid::new_v4().simple()),
        run_id: run_id.to_string(),
        kind,
        at: Utc::now(),
        payload,
    }
}

/// The ceiling after applying --no-governor degradation.
pub fn effective_ceiling(declared: AuthorityLevel, no_governor: bool) -> AuthorityLevel {
    if no_governor && declared > AuthorityLevel::Advise {
        AuthorityLevel::Advise
    } else {
        declared
    }
}

fn urgency_from(
    severity: crate::finding::Severity,
    criticality: CriticalityClass,
    status: InputStatus,
) -> OperationalUrgency {
    use crate::finding::Severity::*;
    let base = match (severity, criticality) {
        (Critical, _) => OperationalUrgency::Critical,
        (Warning, CriticalityClass::Protected | CriticalityClass::Safety) => {
            OperationalUrgency::High
        }
        (Warning, CriticalityClass::BusinessCritical) => OperationalUrgency::High,
        (Warning, CriticalityClass::Standard) => OperationalUrgency::Medium,
        (Low, _) => OperationalUrgency::Low,
    };
    // Stale / invalidated evidence bumps urgency floor — we can't tell
    // whether it got worse while we weren't looking.
    match status {
        InputStatus::Stale | InputStatus::Invalidated => base.max(OperationalUrgency::Medium),
        _ => base,
    }
}

// Intentionally suppress unused import warnings that surface only when
// the `tests` module is compiled. These items are used by tests below.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_ceiling_lowers_to_advise_without_governor() {
        assert_eq!(
            effective_ceiling(AuthorityLevel::Apply, true),
            AuthorityLevel::Advise
        );
        assert_eq!(
            effective_ceiling(AuthorityLevel::Publish, true),
            AuthorityLevel::Advise
        );
        assert_eq!(
            effective_ceiling(AuthorityLevel::Stage, true),
            AuthorityLevel::Advise
        );
    }

    #[test]
    fn effective_ceiling_preserves_advise_and_below() {
        assert_eq!(
            effective_ceiling(AuthorityLevel::Advise, true),
            AuthorityLevel::Advise
        );
        assert_eq!(
            effective_ceiling(AuthorityLevel::Observe, true),
            AuthorityLevel::Observe
        );
    }

    #[test]
    fn effective_ceiling_is_noop_with_governor() {
        assert_eq!(
            effective_ceiling(AuthorityLevel::Apply, false),
            AuthorityLevel::Apply
        );
        assert_eq!(
            effective_ceiling(AuthorityLevel::Publish, false),
            AuthorityLevel::Publish
        );
    }

    #[test]
    fn urgency_bumps_on_stale_evidence() {
        let u = urgency_from(
            crate::finding::Severity::Low,
            CriticalityClass::Standard,
            InputStatus::Stale,
        );
        assert_eq!(u, OperationalUrgency::Medium);
    }

    #[test]
    fn urgency_critical_passes_through() {
        let u = urgency_from(
            crate::finding::Severity::Critical,
            CriticalityClass::Standard,
            InputStatus::Committed,
        );
        assert_eq!(u, OperationalUrgency::Critical);
    }
}
