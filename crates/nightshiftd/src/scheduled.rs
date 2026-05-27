//! Scheduled-trigger idempotency: skip if same NQ generation already
//! reconciled.
//!
//! Slice 1 close-out — runtime ladder roadmap acceptance #5:
//! re-running within the same NQ generation against the same
//! `finding_key` either finds the existing run and reports it, or
//! opens a new run with an explicit reason. Never silently
//! double-counts.
//!
//! Policy:
//! - **Scheduled** trigger consults this check. If the most recent
//!   *completed* run for `(agenda_id, finding_key)` captured the
//!   same NQ `snapshot_generation` as the current NQ snapshot, the
//!   invocation skips and reports the prior run. Otherwise it
//!   proceeds normally.
//! - **Manual** and **Event** triggers never skip. The operator
//!   asked; the event fired; both are explicit. Idempotency is for
//!   timer-driven invocations, where re-running the same
//!   reconciliation costs work and confuses the operator surface
//!   without producing fresh information.
//!
//! The skip path is observation-only: no new run row is created, no
//! ledger events written. The prior run remains the canonical
//! receipt for this generation. Operators looking for "what did the
//! daemon do at 03:00" follow the skip line to the prior run_id.

use chrono::{DateTime, Utc};

use crate::agenda::Agenda;
use crate::errors::Result;
use crate::finding::FindingKey;
use crate::nq::NqSource;
use crate::store::{RunFilter, Store};

/// Result of the scheduled-trigger idempotency check.
#[derive(Debug, Clone)]
pub enum ScheduledOutcome {
    /// The current NQ generation has already been reconciled in a
    /// prior completed run; the caller should not open a new run.
    Skipped(SkipReport),
    /// No prior run captured the current generation, or the current
    /// finding is absent from NQ — the caller should proceed with
    /// the normal pipeline.
    Proceed,
}

/// Detail for a skipped invocation. Surfaced to the operator as a
/// single line so timer logs stay scannable.
#[derive(Debug, Clone)]
pub struct SkipReport {
    pub prior_run_id: String,
    pub prior_completed_at: Option<DateTime<Utc>>,
    pub snapshot_generation: u64,
}

impl SkipReport {
    /// One-line operator-facing skip message.
    pub fn message(&self) -> String {
        let completed = self
            .prior_completed_at
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| "(unknown)".into());
        format!(
            "scheduled-skip: nq snapshot_generation={} already reconciled in run {} at {}",
            self.snapshot_generation, self.prior_run_id, completed,
        )
    }
}

/// Inspect prior completed runs to decide whether a Scheduled
/// invocation should skip.
///
/// Caller contract: invoke this only when the run trigger is
/// `Scheduled`. Manual and Event triggers should bypass it.
///
/// Returns `Proceed` when:
/// - the current finding is absent from NQ (let the pipeline emit
///   its usual not-present error),
/// - no prior completed run exists for `(agenda_id, finding_key)`,
/// - the most recent prior completed run captured a different
///   `snapshot_generation`,
/// - or the prior run's bundle cannot be read (fail open; new run
///   carries its own receipt).
pub fn check_scheduled_idempotency(
    agenda: &Agenda,
    target: &FindingKey,
    nq: &dyn NqSource,
    store: &dyn Store,
) -> Result<ScheduledOutcome> {
    let Some(current_snapshot) = nq.snapshot(target)? else {
        return Ok(ScheduledOutcome::Proceed);
    };
    let current_gen = current_snapshot.snapshot_generation;

    let target_key = target.as_string();
    let runs = store.list_runs(RunFilter {
        agenda_id: Some(agenda.agenda_id.clone()),
        target_finding_key: Some(target_key),
        limit: None,
    })?;

    let mut completed: Vec<_> = runs
        .into_iter()
        .filter(|r| r.completed_at.is_some())
        .collect();
    completed.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

    for prior in completed {
        let Some(bundle) = store.get_bundle(&prior.run_id)? else {
            continue;
        };
        let prior_gen = bundle
            .capture
            .inputs
            .iter()
            .find_map(|i| i.captured_finding_snapshot.as_ref())
            .map(|s| s.snapshot_generation);
        match prior_gen {
            Some(g) if g == current_gen => {
                return Ok(ScheduledOutcome::Skipped(SkipReport {
                    prior_run_id: prior.run_id,
                    prior_completed_at: prior.completed_at,
                    snapshot_generation: current_gen,
                }));
            }
            _ => {
                // Different generation (or unreadable): the new run
                // is the canonical reconciliation for `current_gen`.
                // Stop scanning earlier runs — they can only be
                // older generations.
                return Ok(ScheduledOutcome::Proceed);
            }
        }
    }
    Ok(ScheduledOutcome::Proceed)
}
