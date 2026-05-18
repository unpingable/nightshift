//! Imported Basis Freshness — Slice B of `DURABLE_ARTIFACT_SUBSTRATE`
//! consumption. See `docs/GAP-imported-basis-freshness.md`.
//!
//! **The deep rule:** Night Shift may consume NQ findings; it cannot
//! upgrade custody into basis.
//!
//! **Keeper:** NQ lifecycle / custody time cannot launder upstream
//! observation time.
//!
//! **Companion:** `captured_at` may prove when Night Shift saw the
//! finding. It does not prove when the world was observed.
//!
//! Time is treated as a hostile substrate. Clocks are witnesses,
//! not facts; a timestamp is evidence about time, not authority
//! over time. Freshness is not transitive across custody.
//!
//! ## Four clock roles
//!
//! 1. **Producer basis time** — `origin.producer_extraction_time`.
//!    The only imported clock that can support evidence freshness.
//! 2. **Local custody time** — `FindingSnapshot.captured_at` /
//!    `first_seen_at`. Proves custody, not freshness.
//! 3. **Decision time** — `reconciled_at` / wall-clock now. Where
//!    freshness is evaluated.
//! 4. **Monotonic process time** — internal only; never on receipts
//!    and never in cross-system comparisons.

use chrono::{DateTime, Utc};

use crate::finding::{FindingOrigin, FindingSnapshot};

/// Default freshness window for imported findings, in seconds.
/// Independent from `liveness_threshold_seconds` and from NQ's own
/// `extraction_stale` detector threshold — see GAP §"Freshness window."
pub const DEFAULT_IMPORTED_BASIS_FRESHNESS_WINDOW_SECONDS: u64 = 3600;

/// Default skew budget for clock-coherence checks, in seconds.
/// Tolerates small clock drift between producer and Night Shift hosts;
/// violations beyond this become `imported_producer_clock_incoherent`.
pub const DEFAULT_SKEW_BUDGET_SECONDS: u64 = 60;

/// Which clock NS used as the freshness basis for this finding.
///
/// Does not conflate clock source with assessment quality: the
/// "missing" and "incoherent" variants are not valid bases; they
/// are the *absence* of an admissible basis, surfaced explicitly so
/// callers cannot collapse them into the present-basis cases by
/// accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessBasis {
    /// Native NQ finding (no `origin` block). Basis is the
    /// snapshot's own lifecycle / custody clock — the existing
    /// pre-Slice-B path.
    NativeLifecycle { timestamp: DateTime<Utc> },
    /// Imported finding with a parseable, coherent
    /// `producer_extraction_time`. This is the only imported case
    /// that can produce a `Fresh` or `Stale` assessment.
    ProducerExtraction { timestamp: DateTime<Utc> },
    /// Imported finding whose `producer_extraction_time` was absent
    /// from the wire JSON. Reason: `imported_producer_basis_missing`.
    MissingProducerExtraction,
    /// Imported finding whose `producer_extraction_time` was
    /// present but unparseable, in the future of the decision clock,
    /// or after the custody clock beyond the skew budget. Reason:
    /// `imported_producer_clock_incoherent`.
    IncoherentProducerExtraction,
}

/// Coarse freshness verdict the reconciler reads. Maps to existing
/// `EvidenceState` decisions (see GAP §"Stale must not eat clock
/// failures"): `Stale` triggers Slice 5's advise(revalidate-only)
/// path; `CannotAssess` degrades posture without claiming the basis
/// is old.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessAssessment {
    Fresh,
    Stale,
    CannotAssess,
}

/// Outcome of a freshness assessment for one finding. Carries the
/// basis used, the verdict, the audit reason, and the import-lag
/// audit value (when both producer and custody clocks are present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshnessOutcome {
    pub basis: FreshnessBasis,
    pub assessment: FreshnessAssessment,
    /// Audit reason string. Stable values:
    /// - `"none"` for fresh native or fresh imported
    /// - `"imported_producer_basis_stale"`
    /// - `"imported_producer_basis_missing"`
    /// - `"imported_producer_clock_incoherent"`
    pub reason: &'static str,
    /// Custody clock — always present (from `captured_at`).
    pub custody_at: DateTime<Utc>,
    /// `captured_at - producer_extraction_time` when both are
    /// present. Audit material only; never participates in the
    /// assessment itself per the spec ("import lag is audit, not
    /// basis").
    pub import_lag_seconds: Option<i64>,
}

/// Assess imported-basis freshness for a single finding.
///
/// Native findings (no `origin` block) return
/// `FreshnessBasis::NativeLifecycle` + `FreshnessAssessment::Fresh`
/// with reason `"none"` — this is the regression path and must not
/// change Slice B behavior for native findings.
///
/// Imported findings (`origin.source = "import"`) walk the five
/// cases in `docs/GAP-imported-basis-freshness.md` §"Five cases."
pub fn assess_freshness(
    _snap: &FindingSnapshot,
    _reconciled_at: DateTime<Utc>,
    _window_seconds: u64,
    _skew_seconds: u64,
) -> FreshnessOutcome {
    todo!("Slice B implementation lands in commit 3 (see GAP-imported-basis-freshness.md)")
}

/// Helper: classify the producer clock against custody + decision
/// clocks under the skew budget. Returns the basis variant only;
/// the freshness comparison happens separately because the window
/// only applies once we know the basis is admissible.
#[allow(dead_code)]
fn classify_producer_clock(
    origin: &FindingOrigin,
    captured_at: DateTime<Utc>,
    reconciled_at: DateTime<Utc>,
    skew_seconds: u64,
) -> FreshnessBasis {
    let _ = (origin, captured_at, reconciled_at, skew_seconds);
    todo!("Slice B implementation lands in commit 3")
}
