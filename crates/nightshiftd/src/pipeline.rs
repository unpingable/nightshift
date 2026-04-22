//! Pipeline — capture → reconcile → packet.
//!
//! Per `GAP-deferred-run-split.md`, the Watchbill pipeline is split
//! into two phases that may run in the same invocation or separated
//! by wall time:
//!
//! - [`capture_phase`] — freezes the baseline observation, runs the
//!   capture-time gates (authority, liveness, preflight), persists
//!   the bundle, and leaves the run *open*. Returns a
//!   [`CaptureOutcome`] that is either `Captured { run_id }` (the
//!   normal path) or `HeldPacket` (liveness / preflight held the run
//!   and the packet is already saved + the run already completed).
//! - [`reconcile_phase`] — loads a captured run, re-checks preflight,
//!   performs exactly one explicit live NQ acquisition step, runs
//!   pure [`crate::reconciler::adjudicate`] over the captured bundle +
//!   acquisition, emits the packet, finalizes the run. One-shot:
//!   reconciling an already-completed run is an error.
//!
//! [`run_watchbill_with_liveness`] composes both phases in one
//! invocation — the same-generation convenience path used by
//! `watchbill run` and legacy callers.

use chrono::Utc;

use crate::agenda::{Agenda, AuthorityLevel, CriticalityClass};
use crate::bundle::{
    Bundle, CaptureInput, CapturePhase, Freshness, InputStatus, InvalidationRule,
    ReconciliationPhase,
};
use crate::coordination::{classify_risky, preflight, scope_key, CoordinationOutcome};
use crate::errors::{NightShiftError, Result};
use crate::finding::{EvidenceState, FindingKey, FindingSnapshot};
use crate::ledger::{RunLedgerEvent, RunLedgerEventKind};
use crate::liveness::{verdict_for, LivenessSource, LivenessVerdict, DEFAULT_STALENESS_THRESHOLD_SECONDS};
use crate::nq::{evidence_hash, NqSource};
use crate::packet::{
    AlternativeRegime, Attention, AttentionState, AuthorityResult, Confidence, Diagnosis,
    DiagnosisReview, DiagnosisReviewMode, FindingSummary, OperationalUrgency, Packet,
    ProposedAction, ProposedActionKind, ReceiptReferences,
};
use crate::reconciler::{self, PolicyFingerprint};
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
    /// Liveness staleness threshold in seconds. Applied by the
    /// pipeline gate against `freshness.age_seconds` from the
    /// liveness DTO. Only consulted when a `LivenessSource` is
    /// supplied to `run_watchbill`. `None` ⇒ default
    /// `DEFAULT_STALENESS_THRESHOLD_SECONDS`. See `liveness.rs`.
    pub liveness_threshold_seconds: Option<u64>,
}

/// Outcome of the capture phase.
///
/// `Captured` means the run is open and awaiting reconcile. `HeldPacket`
/// means a gate (liveness or preflight) held the run, and the packet
/// is already saved + the run already completed; no further pipeline
/// work is required. Boxed to keep the enum small.
#[derive(Debug, Clone)]
pub enum CaptureOutcome {
    Captured { run_id: String },
    HeldPacket(Box<Packet>),
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
    run_watchbill_with_liveness(agenda, target, nq, None, store, opts)
}

/// Same-generation composition of `capture_phase` + `reconcile_phase`.
///
/// Provides the pre-split behavior for callers (like `watchbill run`)
/// that want the full pipeline in one invocation. Deferred callers
/// invoke `capture_phase` and `reconcile_phase` separately — that is
/// the canonical path (see `GAP-deferred-run-split.md`).
pub fn run_watchbill_with_liveness(
    agenda: &Agenda,
    target: &FindingKey,
    nq: &dyn NqSource,
    liveness: Option<&dyn LivenessSource>,
    store: &dyn Store,
    opts: &PipelineOptions,
) -> Result<Packet> {
    match capture_phase(agenda, target, nq, liveness, store, opts)? {
        CaptureOutcome::HeldPacket(packet) => Ok(*packet),
        CaptureOutcome::Captured { run_id } => reconcile_phase(&run_id, nq, store, opts),
    }
}

/// Freeze the baseline observation for a Watchbill run and leave the
/// run open, ready for `reconcile_phase`. Runs the capture-time
/// gates (authority ceiling, optional liveness, preflight). On gate
/// failure returns `HeldPacket` with the run already finalized.
///
/// See `GAP-deferred-run-split.md` invariant #1: capture happens
/// exactly once per run.
pub fn capture_phase(
    agenda: &Agenda,
    target: &FindingKey,
    nq: &dyn NqSource,
    liveness: Option<&dyn LivenessSource>,
    store: &dyn Store,
    opts: &PipelineOptions,
) -> Result<CaptureOutcome> {
    store.create_agenda(agenda)?;

    let trigger = opts.trigger.unwrap_or(RunTrigger::Manual);
    let run_id = store.create_run(&agenda.agenda_id, trigger, Some(target))?;
    let effective_ceiling = effective_ceiling(agenda.promotion_ceiling, opts.no_governor);

    // 1. Liveness gate (if configured). A stale or skewed witness
    //    terminates the run as Stale before any finding source is
    //    consulted.
    if let Some(liveness_src) = liveness {
        let threshold = opts
            .liveness_threshold_seconds
            .unwrap_or(DEFAULT_STALENESS_THRESHOLD_SECONDS);
        let snapshot = liveness_src.current()?;
        let verdict = verdict_for(&snapshot, threshold);
        if !verdict.is_fresh() {
            let pkt = liveness_gate_failed(
                agenda,
                target,
                &snapshot,
                &verdict,
                store,
                opts,
                effective_ceiling,
                &run_id,
            )?;
            return Ok(CaptureOutcome::HeldPacket(Box::new(pkt)));
        }
        store.append_run_event(&new_event(
            &run_id,
            RunLedgerEventKind::RunLivenessGateCleared,
            serde_json::json!({
                "instance_id": snapshot.instance_id,
                "witness_generation": snapshot.witness.generation_id,
                "age_seconds": snapshot.freshness.age_seconds,
                "threshold_seconds": threshold,
            }),
        ))?;
    }

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
        captured_finding_snapshot: Some(captured_snapshot.clone()),
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

    // 3. Preflight — risky-class coordination gate per
    //    GAP-parallel-ops.md. Reconcile will re-check preflight at
    //    reconcile time per GAP-deferred-run-split.md: coordination
    //    state can move while the run is paused.
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
                    "phase": "capture",
                }),
            ))?;
        }
        _ => {
            let pkt = hold_for_preflight(
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
            )?;
            return Ok(CaptureOutcome::HeldPacket(Box::new(pkt)));
        }
    }

    // 4. Persist the captured bundle. After this point, reconcile_phase
    //    can pick the run up asynchronously.
    store.save_bundle(&run_id, &bundle_pre_reconcile)?;

    Ok(CaptureOutcome::Captured { run_id })
}

/// Reconcile a captured (open) run: re-check preflight, perform the
/// explicit reconcile-time live NQ acquisition, run pure adjudication
/// over the captured bundle + acquisition, emit the packet, and
/// finalize the run.
///
/// One-shot: returns `RunAlreadyCompleted` if the run has already
/// been reconciled. If the bundle is missing, returns
/// `RunBundleMissing` — that indicates a bug or a crash between
/// `create_run` and `save_bundle` in capture_phase.
pub fn reconcile_phase(
    run_id: &str,
    nq: &dyn NqSource,
    store: &dyn Store,
    opts: &PipelineOptions,
) -> Result<Packet> {
    // 0. Run must exist and be open.
    let summary = store
        .get_run_summary(run_id)?
        .ok_or_else(|| NightShiftError::RunNotFound(run_id.to_string()))?;
    if summary.completed_at.is_some() {
        return Err(NightShiftError::RunAlreadyCompleted(run_id.to_string()));
    }

    // 1. Load the captured bundle + agenda. Both must be present.
    let captured_bundle = store
        .get_bundle(run_id)?
        .ok_or_else(|| NightShiftError::RunBundleMissing(run_id.to_string()))?;
    let agenda = store.get_agenda(&captured_bundle.agenda_id)?.ok_or_else(|| {
        NightShiftError::AgendaNotFound(captured_bundle.agenda_id.clone())
    })?;
    let effective_ceiling = effective_ceiling(agenda.promotion_ceiling, opts.no_governor);

    // 2. Re-check preflight. Coordination state can move while a
    //    run is paused between capture and reconcile.
    let preflight_outcome = preflight(&agenda, opts.continuity_configured);
    let risky = classify_risky(&agenda);
    let target = parse_target_from_bundle(&captured_bundle)?;

    match preflight_outcome {
        CoordinationOutcome::Clear | CoordinationOutcome::OperatorOverride => {
            store.append_run_event(&new_event(
                run_id,
                RunLedgerEventKind::RunPreflightCleared,
                serde_json::json!({
                    "outcome": "clear",
                    "risky": risky.is_risky(),
                    "reasons": risky.reasons(),
                    "phase": "reconcile",
                }),
            ))?;
        }
        _ => {
            // Preflight moved to a hold between capture and reconcile.
            // Same packet-shape as a capture-time hold, but the bundle
            // is already saved and the run is open — just emit the
            // held packet and close the run.
            let captured_snapshot = captured_bundle
                .capture
                .inputs
                .iter()
                .find_map(|i| i.captured_finding_snapshot.clone())
                .ok_or_else(|| {
                    NightShiftError::InvalidAgenda(format!(
                        "reconcile_phase: captured bundle for run {run_id} has no NQ snapshot"
                    ))
                })?;
            return hold_for_preflight(
                &agenda,
                &target,
                &captured_snapshot,
                &captured_bundle,
                preflight_outcome,
                &risky,
                store,
                opts,
                effective_ceiling,
                run_id,
            );
        }
    }

    // 3. Explicit, single-shot live acquisition. One NQ call per
    //    NQ-backed input. After this event, the run has no further
    //    live NQ dependency.
    let acquisition = reconciler::acquire_current(&captured_bundle, nq)?;
    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunCurrentSnapshotAcquired,
        serde_json::json!({
            "acquired_at": acquisition.acquired_at.to_rfc3339(),
            "inputs": acquisition.current_snapshots.len(),
            "present": acquisition.present_count(),
            "absent": acquisition.absent_count(),
        }),
    ))?;

    // 4. Pure adjudication. Deterministic over bundle + acquisition +
    //    policy fingerprint; no further live reads.
    let reconciliation =
        reconciler::adjudicate(&captured_bundle, &acquisition, &PolicyFingerprint::default());

    let reconciled_bundle = Bundle {
        reconciliation: Some(reconciliation.clone()),
        ..captured_bundle
    };
    store.save_bundle(run_id, &reconciled_bundle)?;

    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunReconciled,
        serde_json::json!({
            "ok_to_proceed": reconciliation.summary.ok_to_proceed,
            "blocked": reconciliation.summary.blocked,
            "downgraded": reconciliation.summary.downgraded,
        }),
    ))?;

    // 5. Build the packet from persisted bundle state — no live reads.
    let packet = build_success_packet(
        &agenda,
        run_id,
        &target,
        &reconciled_bundle,
        &reconciliation,
        effective_ceiling,
        opts,
    )?;

    store.save_packet(run_id, &packet)?;
    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunCompleted,
        serde_json::json!({
            "packet_id": packet.packet_id,
            "requested_authority_level": packet.proposed_action.requested_authority_level,
            "effective_ceiling": effective_ceiling,
        }),
    ))?;
    store.complete_run(run_id)?;

    Ok(packet)
}

/// Reconstruct the target FindingKey from a persisted bundle. v1
/// bundles have exactly one NQ finding input per capture; derive the
/// key from its captured snapshot.
fn parse_target_from_bundle(bundle: &Bundle) -> Result<FindingKey> {
    bundle
        .capture
        .inputs
        .iter()
        .find_map(|i| i.captured_finding_snapshot.as_ref().map(|s| s.finding_key.clone()))
        .ok_or_else(|| {
            NightShiftError::InvalidAgenda(format!(
                "bundle for run {} has no captured NQ snapshot",
                bundle.run_id
            ))
        })
}

/// Build the success-path packet from persisted bundle + adjudication
/// result. Pure: reads no live state. The current-state fields come
/// from `reconciliation.results[i].current_finding_snapshot` (the
/// persisted reconcile-time acquisition); when the finding was
/// absent at acquisition time, falls back to the captured baseline
/// so the packet still renders sensible operator-facing context.
#[allow(clippy::too_many_arguments)]
fn build_success_packet(
    agenda: &Agenda,
    run_id: &str,
    target: &FindingKey,
    reconciled_bundle: &Bundle,
    reconciliation: &ReconciliationPhase,
    effective_ceiling: AuthorityLevel,
    opts: &PipelineOptions,
) -> Result<Packet> {
    let nq_result = reconciliation
        .results
        .iter()
        .find(|r| r.input_id.starts_with("nq:finding:"))
        .expect("v1 pipeline seeded a single NQ input; it must be present in reconciliation");

    // Prefer the reconcile-time acquired snapshot for the
    // FindingSummary; fall back to the captured baseline when the
    // finding was absent at acquisition time.
    let captured_snapshot = reconciled_bundle
        .capture
        .inputs
        .iter()
        .find_map(|i| i.captured_finding_snapshot.clone())
        .expect("v1 bundle invariant: one NQ input with captured_finding_snapshot");
    let current_snapshot = nq_result
        .current_finding_snapshot
        .clone()
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

    let (diagnosis, proposed_action) = build_verdict_surfaces(
        target,
        &current_snapshot,
        nq_result.status,
        nq_result
            .notes
            .as_deref()
            .unwrap_or("no reconciler note recorded"),
    );

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

    Ok(Packet {
        packet_version: 0,
        packet_id,
        agenda_id: agenda.agenda_id.clone(),
        run_id: run_id.to_string(),
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
    })
}

/// Handle a preflight hold — emit the appropriate event, save a
/// bundle with no reconciliation, build a packet that documents the
/// hold, and finalize the run. Returns the packet so the caller can
/// render it.
#[allow(clippy::too_many_arguments)]
fn hold_for_preflight(
    agenda: &Agenda,
    target: &FindingKey,
    captured_snapshot: &FindingSnapshot,
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

/// Terminate a run that failed the NQ liveness gate. No NQ snapshot
/// is captured; no reconciliation is run. Emits a Stale-shape packet
/// per the slice-5 contract (`GAP-nq-nightshift-contract.md`):
/// revalidate-only proposal, no remediation. The packet exists so
/// the operator sees what was attempted and why it stopped.
#[allow(clippy::too_many_arguments)]
fn liveness_gate_failed(
    agenda: &Agenda,
    target: &FindingKey,
    snapshot: &crate::liveness::LivenessSnapshot,
    verdict: &LivenessVerdict,
    store: &dyn Store,
    opts: &PipelineOptions,
    effective_ceiling: AuthorityLevel,
    run_id: &str,
) -> Result<Packet> {
    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunLivenessGateFailed,
        serde_json::json!({
            "instance_id": snapshot.instance_id,
            "witness_generation": snapshot.witness.generation_id,
            "age_seconds": snapshot.freshness.age_seconds,
            "verdict": verdict.explain(),
        }),
    ))?;

    // No bundle gets persisted: nothing was captured. The packet is
    // the operator-facing record of the gate failure.

    let finding_summary = FindingSummary {
        source: target.source.clone(),
        detector: target.detector.clone(),
        host: snapshot.instance_id.clone(),
        subject: target.subject.clone(),
        severity: crate::finding::Severity::Warning,
        domain: None,
        persistence_generations: 0,
        first_seen_at: snapshot.export.exported_at,
        current_status: EvidenceState::Stale,
    };

    let reconciliation_summary = crate::bundle::ReconciliationSummary {
        ok_to_proceed: false,
        blocked: vec![format!("liveness_gate: {}", verdict.explain())],
        ..Default::default()
    };

    let diagnosis = Diagnosis {
        regime: "stale: NQ liveness gate did not clear; no findings consulted".into(),
        evidence: vec![
            verdict.explain(),
            format!(
                "witness instance_id={} generation_id={} generated_at={}",
                snapshot.instance_id,
                snapshot.witness.generation_id,
                snapshot.witness.generated_at.to_rfc3339()
            ),
        ],
        confidence: Confidence::Low,
        alternatives_considered: vec![],
    };

    let proposed_action = ProposedAction {
        kind: ProposedActionKind::Advisory,
        // Revalidate-only — same shape as the slice-5 Stale path.
        steps: vec![
            "revalidate the NQ liveness artifact: confirm the publisher/aggregator is healthy and the artifact path is reachable"
                .into(),
            "if witness clock is skewed, resolve clock sync before retrying"
                .into(),
            "rerun this watchbill once liveness is current"
                .into(),
        ],
        risk_notes: vec![
            "no remediation proposed: liveness gate failure is not a basis for action".into(),
            "no NQ findings were consulted on this run".into(),
        ],
        reversible: true,
        blast_radius: "none — gate halted before capture".into(),
        requested_authority_level: AuthorityLevel::Advise,
    };

    let authority_result = AuthorityResult {
        requested: AuthorityLevel::Advise,
        governor_present: !opts.no_governor,
        governor_verdict: Some("not consulted — liveness gate halted the run".into()),
        authority_receipts: vec![],
        ceiling_note: Some(format!(
            "ceiling {effective_ceiling:?} → held at advise by liveness gate",
        )),
    };

    let diagnosis_review = DiagnosisReview {
        mode: DiagnosisReviewMode::SelfCheck,
        unsafe_assumptions: vec![],
        stale_context_risks: vec![
            "no reconciliation performed — finding source not consulted".into(),
        ],
        promotion_overreach: vec![],
        missing_verification: vec![verdict.explain()],
        recommended_downgrade: None,
    };

    let attention = Attention {
        attention_key: target.clone(),
        evidence_state: EvidenceState::Stale,
        attention_state: AttentionState::Unowned,
        operational_urgency: OperationalUrgency::Medium,
        owner: None,
        last_touched_by: None,
        last_touched_at: None,
        acknowledged_at: None,
        ack_expires_at: None,
        follow_up_by: None,
        handoff_note: None,
        re_alert_after: None,
        silence_reason: Some(verdict.explain()),
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
            evidence_bundle: None,
        },
    };

    store.save_packet(run_id, &packet)?;
    store.append_run_event(&new_event(
        run_id,
        RunLedgerEventKind::RunCompleted,
        serde_json::json!({
            "packet_id": packet.packet_id,
            "held": true,
            "outcome": "liveness_gate_failed",
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

/// Build the verdict-aware diagnosis + proposed action for a packet.
///
/// Per `GAP-nq-nightshift-contract.md` (three-axis split) and the
/// slice-5 design rulings:
///
/// - **Committed (cheap path)** — quiet acknowledgment; no churn note.
/// - **Committed (churn-only)** — explicitly note generations advanced
///   without semantic change, so the operator does not mistake quiet
///   for unobserved.
/// - **Changed** — the diagnosis evidence enumerates which semantic
///   fields moved; the proposal remains advise-only in v1.
/// - **Stale** — proposal is *revalidate-only*: restore evidence,
///   inspect the NQ path, rerun once current. No remediation steps.
/// - **Invalidated** — proposal is "captured premise no longer holds;
///   no remediation proposed." A packet still emits so the
///   transition is visible (silent disappearance is the failure mode
///   that turns into folklore).
/// - **Observed** — should not appear at packet build time; treat as
///   an internal contract bug.
fn build_verdict_surfaces(
    target: &FindingKey,
    current: &FindingSnapshot,
    status: InputStatus,
    reconciler_note: &str,
) -> (Diagnosis, ProposedAction) {
    let target_str = target.as_string();
    let advise_only = AuthorityLevel::Advise;

    match status {
        InputStatus::Committed => {
            let (regime, evidence) = if reconciler_note.contains("churn-only") {
                (
                    "committed (churn-only): finding still live, semantic axis unchanged".into(),
                    vec![
                        format!(
                            "finding {target_str} persisted for {} generations",
                            current.persistence_generations
                        ),
                        reconciler_note.to_string(),
                    ],
                )
            } else {
                (
                    "committed: captured evidence matches current NQ snapshot byte-for-byte".into(),
                    vec![format!(
                        "finding {target_str} persisted for {} generations",
                        current.persistence_generations
                    )],
                )
            };
            (
                Diagnosis {
                    regime,
                    evidence,
                    confidence: Confidence::Low,
                    alternatives_considered: vec![AlternativeRegime {
                        regime: "alternative regimes not considered in v1".into(),
                        ruled_out_by: "no LLM wired; deterministic placeholder".into(),
                    }],
                },
                ProposedAction {
                    kind: ProposedActionKind::Advisory,
                    steps: vec![
                        "human review of the finding and reconciliation summary".into(),
                        "no automated action taken in v1 Watchbill".into(),
                    ],
                    risk_notes: vec![],
                    reversible: true,
                    blast_radius: "none — advise only".into(),
                    requested_authority_level: advise_only,
                },
            )
        }
        InputStatus::Changed => (
            Diagnosis {
                regime: "changed: semantic axis moved since capture".into(),
                evidence: vec![
                    format!(
                        "finding {target_str} persisted for {} generations",
                        current.persistence_generations
                    ),
                    reconciler_note.to_string(),
                ],
                confidence: Confidence::Low,
                alternatives_considered: vec![AlternativeRegime {
                    regime: "alternative regimes not considered in v1".into(),
                    ruled_out_by: "no LLM wired; deterministic placeholder".into(),
                }],
            },
            ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec![
                    "review the semantic-axis change and decide whether the captured proposal still applies"
                        .into(),
                    "no automated action taken in v1 Watchbill".into(),
                ],
                risk_notes: vec![reconciler_note.to_string()],
                reversible: true,
                blast_radius: "none — advise only".into(),
                requested_authority_level: advise_only,
            },
        ),
        InputStatus::Stale => (
            Diagnosis {
                regime: "stale: captured evidence could not be revalidated".into(),
                evidence: vec![
                    format!("captured finding: {target_str}"),
                    format!("revalidation failed: {reconciler_note}"),
                ],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            ProposedAction {
                kind: ProposedActionKind::Advisory,
                // Revalidate-only. Per chatty's slice-5 ruling: stale
                // evidence may not propose remediation. The only safe
                // moves are restoration of evidence currency.
                steps: vec![
                    "revalidate the NQ source: confirm the publisher/aggregator is healthy and the snapshot path is reachable"
                        .into(),
                    "fix transport or schema-version issues if revalidation surfaced any"
                        .into(),
                    "rerun this watchbill once current evidence is available"
                        .into(),
                ],
                risk_notes: vec![
                    "no remediation proposed: stale evidence is not a basis for action".into(),
                ],
                reversible: true,
                blast_radius: "none — revalidation only".into(),
                requested_authority_level: advise_only,
            },
        ),
        InputStatus::Invalidated => (
            Diagnosis {
                regime: "invalidated: captured premise no longer holds".into(),
                evidence: vec![
                    format!("captured finding: {target_str}"),
                    format!("invalidation: {reconciler_note}"),
                ],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec![
                    "no remediation proposed: the captured premise is no longer current".into(),
                    "if the finding's disappearance was unexpected, investigate why upstream"
                        .into(),
                ],
                risk_notes: vec![
                    "silent disappearance flagged: this packet exists so the transition is visible"
                        .into(),
                ],
                reversible: true,
                blast_radius: "none — finding cleared since capture".into(),
                requested_authority_level: advise_only,
            },
        ),
        InputStatus::Observed => (
            // An `Observed` status at packet build time means the
            // reconciler did not run for this input. Treat as a
            // pipeline contract bug; emit a clearly-labelled packet
            // rather than panicking.
            Diagnosis {
                regime: "internal: input still Observed at packet build time".into(),
                evidence: vec![
                    "expected the reconciler to assign Committed/Changed/Stale/Invalidated"
                        .into(),
                    reconciler_note.to_string(),
                ],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec!["investigate pipeline wiring; do not act on this packet".into()],
                risk_notes: vec![
                    "packet emitted from an unreconciled input; treat as evidence-of-bug, not evidence-of-finding"
                        .into(),
                ],
                reversible: true,
                blast_radius: "none — diagnostic only".into(),
                requested_authority_level: advise_only,
            },
        ),
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
