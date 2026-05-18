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
use serde::{Deserialize, Serialize};

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

/// Wire-format projection of a freshness assessment, suitable for
/// serialization onto `bundle::ReconciliationResult`. Slice B.1
/// surfaces this on the reconciliation receipt **without** changing
/// `EvidenceState` semantics — the seam is made visible before it is
/// made binding.
///
/// Field layout matches `docs/GAP-imported-basis-freshness.md`
/// §"Receipt fields."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshnessReceipt {
    /// `"fresh"`, `"stale"`, or `"cannot_assess"`.
    pub assessment: String,
    /// Stable reason string per `FreshnessOutcome::reason`.
    pub reason: String,
    pub freshness_basis: ClockReference,
    pub custody_basis: ClockReference,
    /// `captured_at - producer_extraction_time` in seconds; omitted
    /// (`None`) when no producer clock is available or when its
    /// coherence with custody is undefined. Audit material only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_lag_seconds: Option<i64>,
}

/// Named reference to one of Slice B's four clock roles. The `kind`
/// string is stable and audit-friendly; `timestamp` is `None` when
/// the clock is missing or incoherent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockReference {
    /// `"native_lifecycle"` | `"producer_extraction_time"` |
    /// `"missing_producer_extraction"` |
    /// `"incoherent_producer_extraction"` |
    /// `"finding_snapshot.captured_at"`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

impl From<&FreshnessOutcome> for FreshnessReceipt {
    fn from(o: &FreshnessOutcome) -> Self {
        let (basis_kind, basis_timestamp) = match &o.basis {
            FreshnessBasis::NativeLifecycle { timestamp } => {
                ("native_lifecycle", Some(*timestamp))
            }
            FreshnessBasis::ProducerExtraction { timestamp } => {
                ("producer_extraction_time", Some(*timestamp))
            }
            FreshnessBasis::MissingProducerExtraction => {
                ("missing_producer_extraction", None)
            }
            FreshnessBasis::IncoherentProducerExtraction => {
                ("incoherent_producer_extraction", None)
            }
        };
        let assessment = match o.assessment {
            FreshnessAssessment::Fresh => "fresh",
            FreshnessAssessment::Stale => "stale",
            FreshnessAssessment::CannotAssess => "cannot_assess",
        };
        FreshnessReceipt {
            assessment: assessment.into(),
            reason: o.reason.into(),
            freshness_basis: ClockReference {
                kind: basis_kind.into(),
                timestamp: basis_timestamp,
            },
            custody_basis: ClockReference {
                kind: "finding_snapshot.captured_at".into(),
                timestamp: Some(o.custody_at),
            },
            import_lag_seconds: o.import_lag_seconds,
        }
    }
}

/// Assess imported-basis freshness for a single finding.
///
/// Native findings (no `origin` block) return
/// `FreshnessBasis::NativeLifecycle` + `FreshnessAssessment::Fresh`
/// with reason `"none"` — this is the regression path and Slice B
/// introduces no new behavior for native findings.
///
/// Imported findings (`origin.source = "import"`) walk the five
/// cases in `docs/GAP-imported-basis-freshness.md` §"Five cases."
pub fn assess_freshness(
    snap: &FindingSnapshot,
    reconciled_at: DateTime<Utc>,
    window_seconds: u64,
    skew_seconds: u64,
) -> FreshnessOutcome {
    let custody_at = snap.captured_at;

    let Some(origin) = snap.origin.as_ref() else {
        // Native finding — Slice B introduces no new behavior.
        // first_seen_at is the audit-bearing basis timestamp; the
        // assessment is Fresh by construction (the pre-existing
        // freshness path applies for any further checks).
        return FreshnessOutcome {
            basis: FreshnessBasis::NativeLifecycle {
                timestamp: snap.first_seen_at,
            },
            assessment: FreshnessAssessment::Fresh,
            reason: "none",
            custody_at,
            import_lag_seconds: None,
        };
    };

    // Imported finding. Classify the producer clock first; only an
    // admissible ProducerExtraction basis is eligible for the
    // age-vs-window comparison.
    let basis = classify_producer_clock(origin, custody_at, reconciled_at, skew_seconds);

    match &basis {
        FreshnessBasis::ProducerExtraction { timestamp } => {
            let evidence_age = reconciled_at - *timestamp;
            let import_lag_seconds = Some((custody_at - *timestamp).num_seconds());
            let window_secs = i64::try_from(window_seconds).unwrap_or(i64::MAX);
            if evidence_age.num_seconds() > window_secs {
                FreshnessOutcome {
                    basis,
                    assessment: FreshnessAssessment::Stale,
                    reason: "imported_producer_basis_stale",
                    custody_at,
                    import_lag_seconds,
                }
            } else {
                FreshnessOutcome {
                    basis,
                    assessment: FreshnessAssessment::Fresh,
                    reason: "none",
                    custody_at,
                    import_lag_seconds,
                }
            }
        }
        FreshnessBasis::MissingProducerExtraction => FreshnessOutcome {
            basis,
            assessment: FreshnessAssessment::CannotAssess,
            reason: "imported_producer_basis_missing",
            custody_at,
            import_lag_seconds: None,
        },
        FreshnessBasis::IncoherentProducerExtraction => FreshnessOutcome {
            basis,
            assessment: FreshnessAssessment::CannotAssess,
            reason: "imported_producer_clock_incoherent",
            custody_at,
            // Lag is undefined when the producer clock is incoherent;
            // do not surface a value that would launder the
            // incoherence into audit material.
            import_lag_seconds: None,
        },
        // NativeLifecycle should never be produced for an imported
        // finding by classify_producer_clock; surfacing here means
        // the classifier broke its contract.
        FreshnessBasis::NativeLifecycle { .. } => FreshnessOutcome {
            basis,
            assessment: FreshnessAssessment::CannotAssess,
            reason: "imported_producer_clock_incoherent",
            custody_at,
            import_lag_seconds: None,
        },
    }
}

/// Classify an imported finding's producer clock against custody +
/// decision clocks under the skew budget. The classification is
/// orthogonal to the window-based freshness check: it determines
/// whether the producer clock is *admissible* as a basis at all.
fn classify_producer_clock(
    origin: &FindingOrigin,
    captured_at: DateTime<Utc>,
    reconciled_at: DateTime<Utc>,
    skew_seconds: u64,
) -> FreshnessBasis {
    let skew = chrono::Duration::seconds(i64::try_from(skew_seconds).unwrap_or(i64::MAX));

    match (
        origin.producer_extraction_time,
        origin.producer_extraction_time_raw.as_ref(),
    ) {
        // Absent in JSON.
        (None, None) => FreshnessBasis::MissingProducerExtraction,
        // Present in JSON but unparseable.
        (None, Some(_)) => FreshnessBasis::IncoherentProducerExtraction,
        // Parsed cleanly — check the two skew relations.
        (Some(t), _) => {
            // Future relative to decision clock.
            if t > reconciled_at + skew {
                return FreshnessBasis::IncoherentProducerExtraction;
            }
            // Producer claims to have extracted after Night Shift
            // captured the finding — impossible ordering beyond
            // skew tolerance.
            if t > captured_at + skew {
                return FreshnessBasis::IncoherentProducerExtraction;
            }
            FreshnessBasis::ProducerExtraction { timestamp: t }
        }
    }
}
