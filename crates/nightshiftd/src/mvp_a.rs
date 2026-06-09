//! MVP-A Slices 2/3/4 — NS cook layer + Wicket invocation + WLP wrap.
//!
//! This module is the cross-component edge for the MVP-A demo loop:
//!
//! ```text
//! substrate → NQ → NS (here) → Wicket → WLP → Continuity persistence
//! ```
//!
//! NS takes an NQ `nq.receipt.v1` document plus its own reconciliation
//! packet, cooks a Wicket `Intent`, invokes `wicket::check()`, wraps
//! the Wicket Outcome as a WLP `AuthorizationReceipt`, calls
//! `wlp::handle()`, and writes each artifact to a local sink.
//!
//! ## Hash chain (the load-bearing acceptance)
//!
//! - **NS posture-packet** references NQ `content_hash` in
//!   `receipt_references.evidence_bundle`.
//! - **Wicket Intent.claimed_basis.evidence_refs** contains an entry
//!   with `kind = prior_receipt` whose `ref` is the NQ `content_hash`.
//! - **WLP `AuthorizationReceipt.custody.causal_parents[0]`** is the
//!   Wicket Outcome's `receipt.input_hash` (the hash a verifier walks
//!   to identify the Wicket receipt). This makes the chain walkable:
//!   `HandlingReceipt.custody.causal_parents[0]` → AuthorizationReceipt
//!   artifact hash → AuthorizationReceipt.custody.causal_parents[0] →
//!   Wicket receipt hash.
//!
//! ## Boundaries
//!
//! - Does not modify Wicket or WLP. Read-only consumer per
//!   `MVP_A_SLICES_2_3_4_PACKET.md` constraints.
//! - Does not add subscription, remote emit, or revocation handling.
//!   AuthorizationReceipt path only; WLP context is `&[]`.
//! - Does not change NS posture vocabulary or add new closure verdicts.
//! - Does not treat NS posture as truth (the forbidden cycle from
//!   `NQ-NS-CHANNEL-SPLIT.md` remains structurally absent: NS emits
//!   *to* Wicket/WLP sinks; no code path ingests NS posture as
//!   substrate truth back into NQ).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// Wicket only re-exports a subset of model types at the crate root;
// the others are reached via `wicket::model::*`. Both forms are
// public per the wicket public API.
use wicket::model::{
    ActorStanding, ClaimedBasis, Precedence, Revocation, ScopeAssertion, ValidityStatus,
};
use wicket::{Evidence, EvidenceKind, Intent, OperationClass, Outcome, PrecedenceResolution,
    Provenance, StandingClass};
use wlp::{
    Admissibility, Artifact, Contestability, Custody, HandleOpts, Kind, PolicyRef,
    TemporalEnvelope, Transition,
};

use crate::errors::{NightShiftError, Result};
use crate::governor_client::NonDischargeKind;
use crate::packet::Packet;

/// NS component identity in actor strings.
pub const NS_ACTOR: &str = "ns:nightshiftd";

/// Policy scheme used in WLP `PolicyRef.scheme` for NS-classification
/// decisions. WLP `HandleOpts.supported_policy_schemes` must include
/// this scheme or `wlp::handle()` returns `Unsupported`.
pub const NS_POLICY_SCHEME: &str = "ns-classification";

/// Canonical reference for the policy NS invokes when classifying
/// findings. Pinned to a stable repo-local identifier; the actual
/// policy text lives in `docs/working/decisions/FEATURE-HISTORY.md`
/// under SLICE_4_CLOSURE_CANDIDATE V1.
pub const NS_POLICY_REF: &str = "policy://ns/SLICE_4_CONTRACT";
pub const NS_POLICY_VERSION: &str = "v1";

/// Minimum subset of `nq.receipt.v1` NS needs to construct the chain.
///
/// The full document carries additional fields (`status_reasons`,
/// `verified`, `suggested_weaker_claims`, etc.); this struct names
/// only what NS chains against. Forward-compat: unknown fields are
/// ignored at parse time.
#[derive(Debug, Clone, Deserialize)]
pub struct NqReceiptRef {
    pub schema: String,
    pub claim: String,
    pub subject: String,
    pub status: String,
    /// NQ's enumeration of *why* the receipt's status is what it is.
    /// For `status = verified`, typically `["all_requirements_verified"]`.
    /// For `status = not_verified`, may be `["contradictory_observation"]`,
    /// `["insufficient_evidence"]`, etc. Forward-compat: NS reads the
    /// strings as-is; doesn't enumerate them. Slice A.5 propagates
    /// these into the refusal artifact when admissibility fails.
    #[serde(default)]
    pub status_reasons: Vec<String>,
    pub witnesses: Vec<NqWitnessRef>,
    pub observed_at_min: DateTime<Utc>,
    pub observed_at_max: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
    pub evaluator: NqEvaluator,
    pub content_hash: String,
    /// Self-limiting claim list — NQ explicitly refuses to testify to
    /// these consequences. NS preserves into Wicket Intent context
    /// for traceability; NS does not parse this into action gates.
    #[serde(default)]
    pub cannot_testify: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NqWitnessRef {
    pub witness_type: String,
    pub digest: String,
    pub observed_at: DateTime<Utc>,
    pub custody_basis: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NqEvaluator {
    pub evaluator: String,
    pub version: u32,
}

impl NqReceiptRef {
    /// Parse an `nq.receipt.v1` document from JSON file. Validates the
    /// schema string; rejects anything else (forward-compat at the
    /// envelope level — a schema bump is a deliberate event).
    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path).map_err(|e| {
            NightShiftError::Store(format!(
                "reading nq receipt {}: {e}",
                path.display()
            ))
        })?;
        let parsed: NqReceiptRef = serde_json::from_str(&raw)?;
        if parsed.schema != "nq.receipt.v1" {
            return Err(NightShiftError::InvalidAgenda(format!(
                "nq receipt schema mismatch: expected nq.receipt.v1, got {}",
                parsed.schema
            )));
        }
        Ok(parsed)
    }

    /// Stable issuer string for evidence and artifact attribution.
    pub fn issuer(&self) -> String {
        format!("nq:{}:v{}", self.evaluator.evaluator, self.evaluator.version)
    }
}

/// The MVP-A posture-sink wrapper. Carries the existing `Packet`
/// flattened at the top level plus the NQ receipt reference fields.
///
/// Why a wrapper rather than a `Packet` schema change: the in-store
/// `Packet` already populates `receipt_references.evidence_bundle`
/// with the run-ledger URL (`bundle://run/<id>`); we want to *add* a
/// NQ-receipt reference to the sink view without overwriting that.
/// `serde(flatten)` puts `Packet`'s fields at the top level and the
/// MVP-A fields beside them.
#[derive(Debug, Clone, Serialize)]
struct PosturePacketSink {
    #[serde(flatten)]
    packet: Packet,
    /// `nq-receipt://<content_hash>` — the canonical reference
    /// downstream consumers use to retrieve the NQ receipt this run
    /// chained against.
    nq_receipt_ref: String,
    /// `<evaluator>@v<version>` — pins which NQ evaluator produced
    /// the receipt. Independent of `content_hash` so a verifier can
    /// reject mismatch without re-fetching.
    nq_evaluator: String,
    /// Subject string from the NQ receipt — the subject-boundary
    /// anchor (host:sushi-k filesystem state) pinned alongside the
    /// content hash for grep-friendliness.
    nq_subject: String,
}

/// Outcome of `run_pipeline`. Either:
/// - **Cooked** — admissibility check passed; Wicket Intent was
///   cooked and the full five-artifact chain emitted.
/// - **Refused** — admissibility check failed; NS refused to cook
///   the Intent and emitted a refusal artifact at the known sink.
///   No Wicket / WLP artifacts are produced in this case.
///
/// A.5 introduced this split. Before A.5, the pipeline assumed every
/// NQ receipt was `verified`; an honest representation of
/// `not_verified` against Wicket Intent v0.3's closed
/// `Evidence.status` enum (`valid | stale | unavailable`) was not
/// reachable, so NS refuses rather than hard-coding `valid` and
/// laundering the contradiction.
///
/// The enum is intentionally two-variant only — `Cooked` and
/// `Refused` exhaust the operator-meaningful outcomes for v1. Adding
/// more variants would re-introduce ambiguity at the cook boundary.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum MvpAResult {
    Cooked(MvpAOutcome),
    Refused(MvpARefusal),
    /// WLP3 receiver-side refusal. Cooking succeeded — Wicket Intent and
    /// Wicket Outcome were both written to disk — but `packet.unsettled`
    /// carries a kind that triggers refusal to mint a WLP
    /// `AuthorizationReceipt`. NS does NOT invoke `wrap_authorization`
    /// or `wlp::handle` on this path. The classification surface
    /// survives; the warranty does not.
    ///
    /// Hard fence: this is not "everything unsettled is forbidden."
    /// One ratified kind triggers refusal per slice. Currently:
    /// `Freshness`. Other `NonDischargeKind` values are carried-but-
    /// -not-adjudicated until they earn their own ratification.
    WlpAuthorizationRefused(WlpAuthorizationRefusal),
}

/// Refusal record returned when NS will not cook a Wicket Intent.
///
/// The disk artifact at `refusal_artifact_path` carries additional
/// fields (status_reasons, cannot_testify, timestamps, agenda/finding/
/// run identifiers); this in-memory shape is the minimum the caller
/// needs to report the refusal honestly without re-reading the file.
#[derive(Debug, Clone, Serialize)]
pub struct MvpARefusal {
    pub refusal_artifact_path: PathBuf,
    pub reason_code: String,
    pub nq_content_hash: String,
    pub nq_subject: String,
    pub nq_status: String,
}

/// WLP3 receiver-side refusal record. The packet cooked + classified
/// cleanly (Wicket Intent + Outcome exist on disk), but
/// `packet.unsettled` carried a ratified refusal kind, so NS did not
/// invoke `wrap_authorization` / `wlp::handle`. The chain remains
/// walkable to the Wicket side via `wicket_intent_path` and
/// `wicket_outcome_path`; the WLP-side warranty is intentionally absent.
#[derive(Debug, Clone, Serialize)]
pub struct WlpAuthorizationRefusal {
    pub posture_packet_path: PathBuf,
    pub wicket_intent_path: PathBuf,
    pub wicket_outcome_path: PathBuf,
    pub refusal_artifact_path: PathBuf,
    pub reason_code: String,
    /// Kinds from `packet.unsettled` that triggered the refusal.
    /// One-element vec in the current slice (Freshness only); the
    /// type carries a Vec so future ratified kinds don't force a
    /// shape change.
    pub unsettled_kinds: Vec<NonDischargeKind>,
    pub governor_receipt_ids: Vec<String>,
}

/// Closed reason-code vocabulary for WLP3 refusals. One code per
/// ratified `NonDischargeKind`. Only `WLP_AUTHORIZATION_REFUSED_
/// FRESHNESS_UNSETTLED` is wired into dispatch in the current slice.
/// Other codes for other kinds remain candidates pending their own
/// ratification slices (see witness 2026-06-09 WLP2 design audit).
pub const WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED: &str =
    "WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED";

/// Scan `packet.unsettled` for kinds that trigger WLP3 refusal.
/// Returns the ordered list of triggering kinds (empty when none).
///
/// **Hard fence:** this function returns ONLY kinds that have been
/// individually ratified. Adding a new kind here is the only honest
/// way to extend WLP3's teeth — and requires its own slice + reason
/// code + tests. Do NOT replace this with a `!packet.unsettled.is_empty()`
/// catch-all; that would be the laundering move WLP3 exists to refuse.
fn wlp3_refusal_triggering_kinds(packet: &Packet) -> Vec<NonDischargeKind> {
    packet
        .unsettled
        .iter()
        .filter(|s| matches!(s.kind, NonDischargeKind::Freshness))
        .map(|s| s.kind)
        .collect()
}

/// MVP-A pipeline output: the five artifacts plus the canonical
/// hashes a verifier walks.
///
/// The chain reads end-to-end from disk:
///
/// ```text
/// NQ receipt content_hash
///   ↓  (posture-packet.nq_receipt_ref)
/// NS posture packet
///   ↓  (intent.claimed_basis.evidence_refs[prior_receipt])
/// Wicket Intent  (sha256(canonical-jcs) == wicket_outcome.receipt.input_hash)
///   ↓  (wicket_outcome.receipt.input_hash)
/// Wicket Outcome
///   ↓  (authorization.custody.causal_parents[0])
/// WLP AuthorizationReceipt
///   ↓  (handling.custody.causal_parents[0])
/// WLP HandlingReceipt
/// ```
///
/// Each arrow is a hash reference written into the next artifact.
/// Once on disk, the chain reconstructs deterministically without
/// keeping any in-memory state around.
#[derive(Debug, Clone, Serialize)]
pub struct MvpAOutcome {
    pub posture_packet_path: PathBuf,
    pub wicket_intent_path: PathBuf,
    pub wicket_outcome_path: PathBuf,
    pub wlp_authorization_path: PathBuf,
    pub wlp_handling_path: PathBuf,
    /// `Receipt.input_hash` from the Wicket Outcome — the canonical
    /// identifier a downstream consumer uses to refer to the Wicket
    /// receipt. Equals `sha256:<hex(sha256(serde_jcs::to_vec(intent)))>`
    /// per Wicket SPEC §8.1; the on-disk Wicket Intent file's bytes
    /// are exactly the input over which this hash was computed.
    pub wicket_receipt_input_hash: String,
    /// Hash of the WLP AuthorizationReceipt artifact (computed via
    /// `wlp::artifact_hash`). The HandlingReceipt's
    /// `causal_parents[0]` is set to this by `wlp::handle()`.
    pub wlp_authorization_artifact_hash: String,
    /// Hash of the WLP HandlingReceipt artifact (computed by
    /// `wlp::handle()` self-stamp).
    pub wlp_handling_artifact_hash: String,
}

/// Build the Wicket Intent from an NS packet + NQ receipt reference.
///
/// Intent shape is type-guaranteed by `wicket::Intent`; field
/// population follows the spec in
/// `docs/working/decisions/MVP_A_SLICES_2_3_4_PACKET.md` Slice 2.
///
/// `reference_time` is used for the call timestamp and evidence
/// validity windows so the integration test can pin determinism.
pub fn cook_intent(packet: &Packet, nq_receipt: &NqReceiptRef, reference_time: DateTime<Utc>) -> Intent {
    let finding_key = packet.attention.attention_key.as_string();
    let target = finding_key.clone();
    let policy_evidence_ref = format!("{NS_POLICY_REF}#{NS_POLICY_VERSION}");
    let nq_validity_floor = nq_receipt.observed_at_min;
    // NS does not assert NQ receipt expiry beyond the +24h MVP-A
    // window — NQ does not encode a freshness horizon on the receipt
    // itself; the Wicket call_timestamp + this window are the
    // local-state pin for admissibility purposes.
    let nq_validity_ceiling = reference_time + chrono::Duration::hours(24);

    Intent {
        actor: NS_ACTOR.to_string(),
        actor_standing: ActorStanding {
            // NS classifying a finding is interpret-class: takes
            // observation, produces a classification. SPEC §5.1 puts
            // interpret > observe; either is admissible for this
            // operation class. Selecting interpret matches the
            // semantic action (classify_finding).
            class: StandingClass::Interpret,
            provenance: Provenance::CallerAsserted,
        },
        intended_action: "classify_finding".to_string(),
        operation_class: OperationClass::Interpret,
        target: target.clone(),
        scope_assertion: Some(ScopeAssertion {
            scope_includes_target: true,
            provenance: Provenance::CallerAsserted,
            evidence_refs: vec![format!("agenda://ns/{}", packet.agenda_id)],
        }),
        claimed_basis: ClaimedBasis {
            rule: format!("{NS_POLICY_REF}@{NS_POLICY_VERSION}"),
            evidence_refs: vec![
                // The chain link: NS cites NQ's content_hash as
                // prior_receipt evidence. A verifier walks
                // Intent.claimed_basis.evidence_refs[k].ref →
                // NQ receipt.
                Evidence {
                    reference: nq_receipt.content_hash.clone(),
                    kind: EvidenceKind::PriorReceipt,
                    issuer: nq_receipt.issuer(),
                    subject: nq_receipt.subject.clone(),
                    valid_from: nq_validity_floor.to_rfc3339(),
                    valid_until: nq_validity_ceiling.to_rfc3339(),
                    status: ValidityStatus::Valid,
                },
                // The NS-side policy reference. Names which
                // classification rule NS is invoking; pinned to
                // SLICE_4_CONTRACT so a future ratification change
                // doesn't silently re-cook receipts under a different
                // rule.
                Evidence {
                    reference: policy_evidence_ref,
                    kind: EvidenceKind::PolicyRef,
                    issuer: NS_ACTOR.to_string(),
                    subject: target.clone(),
                    valid_from: reference_time.to_rfc3339(),
                    valid_until: nq_validity_ceiling.to_rfc3339(),
                    status: ValidityStatus::Valid,
                },
            ],
        },
        precedence: Precedence {
            resolution: PrecedenceResolution::Active,
            superseded_by: None,
            provenance: Provenance::CallerAsserted,
            evidence_refs: vec![format!("ns:reconciler:{NS_POLICY_VERSION}")],
        },
        revocation: Revocation {
            basis_revoked: false,
            standing_forbidden: false,
            provenance: Provenance::CallerAsserted,
            evidence_refs: vec![],
        },
        expected_effect: format!(
            "Non-actuating classification of finding {target}; NS emits posture packet \
             and closure verdict. NS does not authorize closure or mutation; closure \
             remains refused per SLICE_4_CONTRACT."
        ),
        call_timestamp: reference_time.to_rfc3339(),
        prev_receipt_hash: None,
    }
}

/// Wrap a Wicket Outcome as a WLP AuthorizationReceipt artifact.
///
/// The `custody.causal_parents[0]` is set to the Wicket Outcome's
/// `receipt.input_hash` — that is the chain link a downstream WLP
/// consumer follows. The artifact's own hash (via
/// `wlp::artifact_hash`) becomes the WLP `HandlingReceipt`'s
/// `causal_parents[0]` when `wlp::handle()` runs.
///
/// `reference_time` pins all envelope timestamps for determinism.
pub fn wrap_authorization(
    outcome: &Outcome,
    packet: &Packet,
    nq_receipt: &NqReceiptRef,
    reference_time: DateTime<Utc>,
) -> Artifact {
    let finding_key = packet.attention.attention_key.as_string();
    let wicket_input_hash = outcome.receipt.input_hash.clone();
    // WLP1 observational carry-forward.
    //
    // `ns_unsettled` and `governor_receipt_ids` are preserved into
    // the AuthorizationReceipt payload so receiver-side consumers
    // (Continuity persistence, future receiver gates) have custody
    // of the non-discharge signal NS surfaced on the packet.
    //
    // **Invariant: forwarding is preservation, not adjudication.**
    // Carrying `ns_unsettled` does NOT accept the unsettled claim,
    // does NOT reject it, and does NOT imply automatic refusal of
    // the AuthorizationReceipt. It records that the upstream Defer
    // verdict explicitly did not settle these conditions, so a
    // downstream receiver can decide what reliance is appropriate.
    // Future gremlins will try to treat this as decorative metadata
    // or as an automatic denial. Both wrong. Equal-opportunity
    // wrongness. Don't.
    //
    // Both fields are always emitted (possibly empty arrays). An
    // empty `ns_unsettled: []` is the positive claim "no unsettled
    // claims surfaced," not silence — mirroring the v4 GateReceipt
    // schema's same discipline.
    let payload = serde_json::json!({
        "wicket_surface_verdict": outcome.surface_verdict,
        "wicket_outcome_class": outcome.class,
        "wicket_operation_class": outcome.operation_class,
        "wicket_dimensions": outcome.dimensions,
        "wicket_reason_codes": outcome.reason_codes,
        "wicket_allowed": outcome.allowed,
        "wicket_forbidden": outcome.forbidden,
        "wicket_receipt_id": outcome.receipt.receipt_id,
        "wicket_receipt_input_hash": outcome.receipt.input_hash,
        "ns_packet_id": packet.packet_id,
        "ns_run_id": packet.run_id,
        "ns_agenda_id": packet.agenda_id,
        "ns_finding_key": finding_key,
        "ns_closure_candidate": packet.closure_candidate,
        "ns_attention_state": packet.attention.attention_state,
        "ns_posture_class": packet.attention.posture_class,
        "ns_evidence_state": packet.attention.evidence_state,
        "ns_unsettled": packet.unsettled,
        "governor_receipt_ids": packet.receipt_references.governor_receipts,
        "nq_content_hash": nq_receipt.content_hash,
        "nq_claim": nq_receipt.claim,
        "nq_subject": nq_receipt.subject,
        "nq_evaluator": format!("{}@v{}",
            nq_receipt.evaluator.evaluator,
            nq_receipt.evaluator.version),
        "nq_cannot_testify": nq_receipt.cannot_testify,
    });

    let artifact = Artifact {
        kind: Kind::AuthorizationReceipt,
        version: "0.1".to_string(),
        subject: nq_receipt.subject.clone(),
        actor: NS_ACTOR.to_string(),
        transition: Some(Transition {
            actor: NS_ACTOR.to_string(),
            subject: nq_receipt.subject.clone(),
            object: Some(finding_key.clone()),
            operation: "classify_finding".to_string(),
            target: Some(finding_key),
            payload: Some(payload),
        }),
        admissibility: Admissibility {
            standing: Some("interpret".to_string()),
            scope: Some(packet.agenda_id.clone()),
            basis: Some(format!("{NS_POLICY_REF}@{NS_POLICY_VERSION}")),
            evidence_refs: vec![
                nq_receipt.content_hash.clone(),
                wicket_input_hash.clone(),
            ],
            policy_refs: vec![PolicyRef {
                scheme: NS_POLICY_SCHEME.to_string(),
                id: NS_POLICY_REF.to_string(),
                version: Some(NS_POLICY_VERSION.to_string()),
            }],
            // AuthorizationReceipt does not carry a HandlingVerdict;
            // those are HandlingReceipt-only per WLP SPEC §6.
            verdict: None,
            reason_codes: outcome.reason_codes.clone(),
        },
        temporal_envelope: TemporalEnvelope {
            issued_at: reference_time,
            valid_from: reference_time,
            valid_until: reference_time + chrono::Duration::hours(24),
            revalidation_after: None,
            invalidation_conditions: None,
        },
        custody: Custody {
            issuer: NS_ACTOR.to_string(),
            // Pre-stamp computed below.
            artifact_hash: None,
            signature: None,
            // The chain link a verifier follows from this artifact
            // back to Wicket. WLP `handle()` sets the HandlingReceipt's
            // own `causal_parents[0]` to this artifact's hash, which
            // makes the full chain walkable:
            // HandlingReceipt → (this artifact hash) → Wicket receipt.
            causal_parents: vec![wicket_input_hash],
            receipt_hash: None,
        },
        contestability: Contestability {
            revocation_refs: vec![],
            revocation_endpoints: vec![],
            contest_refs: vec![],
            contest_endpoints: vec![],
            supersession_refs: vec![],
            degradation_behavior: None,
        },
        // AuthorizationReceipts do not record an `acted` bit; only
        // HandlingReceipts do.
        acted: None,
    };

    // Pre-stamp the artifact hash so callers can reference it without
    // recomputing. WLP's canonical form normalizes hash-null +
    // signature-null before computing, so this stamp survives a wire
    // round-trip.
    let hash = wlp::artifact_hash(&artifact);
    Artifact {
        custody: Custody {
            artifact_hash: Some(hash),
            ..artifact.custody
        },
        ..artifact
    }
}

/// Write any JSON-serializable value to `path` using RFC 8785 JCS
/// canonicalization. Object members are sorted lexicographically;
/// number formatting follows JCS rules. The bytes a downstream
/// verifier hashes are exactly the bytes written.
pub fn write_canonical_json<P: AsRef<Path>, T: Serialize>(path: P, value: &T) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                NightShiftError::Store(format!(
                    "creating sink parent {}: {e}",
                    parent.display()
                ))
            })?;
        }
    }
    let canonical = serde_jcs::to_string(value).map_err(NightShiftError::from)?;
    fs::write(path, canonical.as_bytes()).map_err(|e| {
        NightShiftError::Store(format!("writing sink {}: {e}", path.display()))
    })?;
    Ok(())
}

/// Resolved sink paths for MVP-A artifacts in `out_dir`.
///
/// The posture sink and the refusal sink are emitted on both the
/// Cooked and Refused paths; the four Wicket/WLP sinks are emitted
/// only on the Cooked path. The refusal sink is empty on the Cooked
/// path (file is never written there).
#[derive(Debug, Clone)]
pub struct SinkPaths {
    pub posture: PathBuf,
    pub wicket_intent: PathBuf,
    pub wicket_outcome: PathBuf,
    pub wlp_authorization: PathBuf,
    pub wlp_handling: PathBuf,
    /// A.5 refusal artifact. Written only when `evaluate_basis_admissibility`
    /// returns `Some(reason_code)`.
    pub refusal: PathBuf,
    /// WLP3 receiver-side refusal artifact. Written when the packet
    /// cooks + classifies cleanly but `packet.unsettled` contains a
    /// kind that triggers refusal-to-wrap (currently only
    /// `Freshness`). The Wicket Intent/Outcome still exist; this
    /// artifact records the receiver's refusal to mint a WLP
    /// `AuthorizationReceipt` warranty.
    pub wlp_refusal: PathBuf,
}

/// Compute sink paths for the MVP-A artifacts in `out_dir`.
pub fn sink_paths(out_dir: &Path, run_id: &str) -> SinkPaths {
    SinkPaths {
        posture: out_dir.join(format!("ns-posture-{run_id}.json")),
        wicket_intent: out_dir.join(format!("ns-wicket-intent-{run_id}.json")),
        wicket_outcome: out_dir.join(format!("ns-wicket-outcome-{run_id}.json")),
        wlp_authorization: out_dir.join(format!("ns-wlp-authorization-{run_id}.json")),
        wlp_handling: out_dir.join(format!("ns-wlp-handling-{run_id}.json")),
        refusal: out_dir.join(format!("ns-refusal-{run_id}.json")),
        wlp_refusal: out_dir.join(format!("ns-wlp-refusal-{run_id}.json")),
    }
}

/// A.5 admissibility check: can NS honestly represent this NQ
/// receipt as a Wicket Intent basis?
///
/// Returns `None` when the receipt status is `verified` (admissible
/// — proceed to cook). Returns `Some(reason_code)` otherwise; NS
/// will refuse to cook and emit a refusal artifact.
///
/// Why this is a hard refusal rather than a soft downgrade: Wicket
/// Intent v0.3's `Evidence.status` is a closed enum (`valid | stale
/// | unavailable`), none of which honestly represents `not_verified`.
/// Mapping `not_verified → valid` would launder the contradiction
/// (path (c), forbidden); mapping to `stale` or `unavailable` would
/// misrepresent the NQ verdict. Refusing at the cook layer is the
/// only honest path until Wicket Intent gains a `not_verified` or
/// equivalent basis status.
///
/// See `~/git/cartography/audit/2026-05-28-path-a5-plan.md` for the
/// operator-stated (a)/(b)/(c) decision and the cartographer's Wicket
/// schema reading.
pub fn evaluate_basis_admissibility(nq_receipt: &NqReceiptRef) -> Option<&'static str> {
    if nq_receipt.status == "verified" {
        None
    } else {
        // Single reason code for v1. Future receipt-status values
        // that *can* be honestly mapped (when Wicket extends its
        // status enum) would route through different codes; for now
        // every non-`verified` status is unrepresentable in the same
        // way.
        Some("BASIS_NOT_VERIFIED_UNREPRESENTABLE")
    }
}

/// Write the canonical refusal artifact for an A.5 refused run.
///
/// The shape carries enough context for an operator (or audit) to
/// reconstruct *why* NS refused without re-fetching the NQ receipt:
/// the NQ content_hash and subject pin the upstream artifact; the
/// nq_status / nq_status_reasons / nq_cannot_testify carry NQ's
/// self-limiting statements that NS chose to honor rather than
/// launder; the agenda_id / finding_key / run_id place the refusal
/// in NS's own pipeline; refused_at + reason_code name *what* and
/// *when* refusal happened.
#[allow(clippy::too_many_arguments)]
pub fn write_refusal_artifact(
    path: &Path,
    nq_receipt: &NqReceiptRef,
    reason_code: &str,
    agenda_id: &str,
    finding_key: &str,
    run_id: &str,
    refused_at: DateTime<Utc>,
) -> Result<()> {
    let artifact = serde_json::json!({
        "schema": "ns.refusal.v1",
        "reason_code": reason_code,
        "refused_at": refused_at.to_rfc3339(),
        "agenda_id": agenda_id,
        "finding_key": finding_key,
        "run_id": run_id,
        "nq_content_hash": nq_receipt.content_hash,
        "nq_subject": nq_receipt.subject,
        "nq_status": nq_receipt.status,
        "nq_status_reasons": nq_receipt.status_reasons,
        "nq_cannot_testify": nq_receipt.cannot_testify,
        "nq_evaluator": format!(
            "{}@v{}",
            nq_receipt.evaluator.evaluator, nq_receipt.evaluator.version
        ),
        "ns_actor": NS_ACTOR,
        "ns_policy_ref": format!("{NS_POLICY_REF}@{NS_POLICY_VERSION}"),
    });
    write_canonical_json(path, &artifact)
}

/// Write the WLP3 receiver-side refusal artifact.
///
/// **Schema distinct from `ns.refusal.v1`.** `ns.refusal.v1` is for
/// upstream-data-unsuitable refusals (e.g., unverified NQ receipt) and
/// is emitted BEFORE Wicket sees the packet. `ns.wlp_refusal.v1` is
/// emitted AFTER Wicket has classified the packet successfully — the
/// classification surface is honest; the refusal is the receiver-side
/// gate's choice not to warrant downstream reliance while a ratified
/// unsettled condition remains open.
///
/// References (not inlines) the Wicket Intent and Outcome artifacts.
/// A verifier walking from this refusal to the Wicket side reads the
/// path fields and loads the canonical JSON; the refusal artifact
/// itself stays a compact record, not a scrapbook of every upstream
/// shape.
#[allow(clippy::too_many_arguments)]
pub fn write_wlp_refusal_artifact(
    path: &Path,
    packet: &Packet,
    nq_receipt: &NqReceiptRef,
    reason_code: &str,
    unsettled_kinds: &[NonDischargeKind],
    governor_receipt_ids: &[String],
    wicket_intent_path: &Path,
    wicket_outcome_path: &Path,
    refused_at: DateTime<Utc>,
) -> Result<()> {
    let finding_key = packet.attention.attention_key.as_string();
    let artifact = serde_json::json!({
        "schema": "ns.wlp_refusal.v1",
        "reason_code": reason_code,
        "refused_at": refused_at.to_rfc3339(),
        "agenda_id": packet.agenda_id,
        "finding_key": finding_key,
        "run_id": packet.run_id,
        "unsettled_kinds": unsettled_kinds,
        "ns_unsettled": packet.unsettled,
        "governor_receipt_ids": governor_receipt_ids,
        "wicket_intent_path": wicket_intent_path
            .to_str()
            .unwrap_or("<non-utf8 path>"),
        "wicket_outcome_path": wicket_outcome_path
            .to_str()
            .unwrap_or("<non-utf8 path>"),
        "nq_content_hash": nq_receipt.content_hash,
        "nq_subject": nq_receipt.subject,
        "ns_actor": NS_ACTOR,
        "ns_policy_ref": format!("{NS_POLICY_REF}@{NS_POLICY_VERSION}"),
    });
    write_canonical_json(path, &artifact)
}

/// Run the MVP-A cook → Wicket → WLP pipeline end-to-end.
///
/// Inputs:
/// - `packet`: NS reconciliation packet (Slice 1 dogfood proven path).
/// - `nq_receipt`: parsed `nq.receipt.v1` reference.
/// - `out_dir`: directory where the three sink files are written.
/// - `reference_time`: deterministic timestamp for envelope + call
///   stamps. Production: `packet.produced_at` is the natural value;
///   tests pin to a known constant.
///
/// Side-effects:
/// - Writes three JSON files to `out_dir`. All bytes are RFC 8785 JCS
///   canonical (deterministic for tests).
///
/// Returns the [`MvpAOutcome`] with all sink paths and the hashes a
/// verifier walks.
pub fn run_pipeline(
    packet: &Packet,
    nq_receipt: &NqReceiptRef,
    out_dir: &Path,
    reference_time: DateTime<Utc>,
) -> Result<MvpAResult> {
    let paths = sink_paths(out_dir, &packet.run_id);

    // Slice 2b: NS posture-packet outbound. Emitted on **both** the
    // Cooked and Refused paths so the operator-visible posture record
    // exists regardless of whether NS proceeded to the cross-component
    // chain. The sink is a wrapper around the existing `Packet` plus
    // a top-level `nq_receipt_ref` field naming the NQ content_hash
    // this run chained against. The wrapper avoids a schema change to
    // `Packet` (which already populates
    // `receipt_references.evidence_bundle` with the bundle://run/<id>
    // ref for the run-ledger lookup — we don't overwrite that). A
    // downstream verifier reads `nq_receipt_ref` to find the upstream
    // NQ receipt without having to inspect the run ledger.
    let posture_sink = PosturePacketSink {
        packet: packet.clone(),
        nq_receipt_ref: format!("nq-receipt://{}", nq_receipt.content_hash),
        nq_evaluator: format!(
            "{}@v{}",
            nq_receipt.evaluator.evaluator, nq_receipt.evaluator.version
        ),
        nq_subject: nq_receipt.subject.clone(),
    };
    write_canonical_json(&paths.posture, &posture_sink)?;

    // A.5: admissibility check. If the NQ receipt's basis cannot be
    // honestly represented in a Wicket Intent (path (b)), NS refuses
    // here. No Wicket Intent / Outcome / WLP artifacts are produced
    // on this branch; the refusal artifact at `paths.refusal` is the
    // honest answer.
    if let Some(reason_code) = evaluate_basis_admissibility(nq_receipt) {
        let finding_key = packet.attention.attention_key.as_string();
        write_refusal_artifact(
            &paths.refusal,
            nq_receipt,
            reason_code,
            &packet.agenda_id,
            &finding_key,
            &packet.run_id,
            reference_time,
        )?;
        return Ok(MvpAResult::Refused(MvpARefusal {
            refusal_artifact_path: paths.refusal,
            reason_code: reason_code.to_string(),
            nq_content_hash: nq_receipt.content_hash.clone(),
            nq_subject: nq_receipt.subject.clone(),
            nq_status: nq_receipt.status.clone(),
        }));
    }

    // Slice 2a: cook Intent. Persist the Intent **before** invoking
    // Wicket so the on-disk bytes are the canonical input over which
    // Wicket's `receipt.input_hash` was computed. Per Wicket
    // `src/receipt.rs::hash_value`, `input_hash` =
    // `sha256:<hex(sha256(serde_jcs::to_vec(intent)))>`. Our
    // `write_canonical_json` uses the equivalent `serde_jcs::to_string`,
    // so `sha256(file_bytes)` reproduces the hash a verifier reads
    // from the Wicket Outcome.
    let intent = cook_intent(packet, nq_receipt, reference_time);
    write_canonical_json(&paths.wicket_intent, &intent)?;

    // Slice 3: invoke Wicket. `wicket::check` consumes the Intent
    // by reference; the input_hash it computes equals
    // sha256(intent canonical-jcs), which equals sha256 of the bytes
    // just written above.
    let outcome = wicket::check(&intent);
    write_canonical_json(&paths.wicket_outcome, &outcome)?;

    // WLP3 receiver-side gate: refuse to mint a WLP AuthorizationReceipt
    // when `packet.unsettled` contains a ratified refusal kind. The
    // classification surface (Wicket Intent + Outcome on disk above)
    // is untouched — NS+Wicket understood the packet correctly. What
    // NS refuses here is the *warranty for downstream reliance*: no
    // AuthorizationReceipt is minted, so no downstream receiver can
    // accept the WLP HandlingReceipt's `Accepted` verdict as a basis
    // for state mutation.
    //
    // Trap to avoid: do NOT short-circuit on "any non-empty
    // packet.unsettled." Each ratified kind earns its teeth through
    // its own slice. `wlp3_refusal_triggering_kinds` is the closed-
    // vocabulary filter. Currently: Freshness only.
    let refusal_kinds = wlp3_refusal_triggering_kinds(packet);
    if !refusal_kinds.is_empty() {
        let governor_receipt_ids = packet
            .receipt_references
            .governor_receipts
            .clone();
        write_wlp_refusal_artifact(
            &paths.wlp_refusal,
            packet,
            nq_receipt,
            WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED,
            &refusal_kinds,
            &governor_receipt_ids,
            &paths.wicket_intent,
            &paths.wicket_outcome,
            reference_time,
        )?;
        return Ok(MvpAResult::WlpAuthorizationRefused(WlpAuthorizationRefusal {
            posture_packet_path: paths.posture,
            wicket_intent_path: paths.wicket_intent,
            wicket_outcome_path: paths.wicket_outcome,
            refusal_artifact_path: paths.wlp_refusal,
            reason_code: WLP_AUTHORIZATION_REFUSED_FRESHNESS_UNSETTLED.to_string(),
            unsettled_kinds: refusal_kinds,
            governor_receipt_ids,
        }));
    }

    // Slice 4: wrap as WLP AuthorizationReceipt, call handle().
    let auth_artifact = wrap_authorization(&outcome, packet, nq_receipt, reference_time);
    let auth_artifact_hash = auth_artifact
        .custody
        .artifact_hash
        .clone()
        .unwrap_or_else(|| wlp::artifact_hash(&auth_artifact));
    // Persist the AuthorizationReceipt so the chain is walkable from
    // disk alone: HandlingReceipt → AuthorizationReceipt → Wicket
    // receipt. Without this file, the only chain link a verifier
    // could check is HandlingReceipt → AuthorizationReceipt hash
    // (opaque); they'd have no way to reach the Wicket receipt
    // without re-running the pipeline.
    write_canonical_json(&paths.wlp_authorization, &auth_artifact)?;

    let opts = HandleOpts {
        consumer: NS_ACTOR.to_string(),
        reference_time,
        supported_policy_schemes: vec![NS_POLICY_SCHEME.to_string()],
    };
    let handling_receipt = wlp::handle(&auth_artifact, &[], &opts);
    let handling_hash = handling_receipt
        .custody
        .artifact_hash
        .clone()
        .unwrap_or_else(|| wlp::artifact_hash(&handling_receipt));
    write_canonical_json(&paths.wlp_handling, &handling_receipt)?;

    Ok(MvpAResult::Cooked(MvpAOutcome {
        posture_packet_path: paths.posture,
        wicket_intent_path: paths.wicket_intent,
        wicket_outcome_path: paths.wicket_outcome,
        wlp_authorization_path: paths.wlp_authorization,
        wlp_handling_path: paths.wlp_handling,
        wicket_receipt_input_hash: outcome.receipt.input_hash.clone(),
        wlp_authorization_artifact_hash: auth_artifact_hash,
        wlp_handling_artifact_hash: handling_hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixture_nq_receipt() -> NqReceiptRef {
        NqReceiptRef {
            schema: "nq.receipt.v1".to_string(),
            claim: "disk_state".to_string(),
            subject: "host:sushi-k".to_string(),
            status: "verified".to_string(),
            status_reasons: vec!["all_requirements_verified".to_string()],
            witnesses: vec![NqWitnessRef {
                witness_type: "disk_pressure_legacy_projection".to_string(),
                digest: "sha256:bf01423b49b5f56336780174eae5f76e5ceec0bfcc8ca47d70ea0e1b13e8c492"
                    .to_string(),
                observed_at: Utc.with_ymd_and_hms(2026, 5, 28, 18, 44, 12).unwrap(),
                custody_basis: "legacy_projection".to_string(),
            }],
            observed_at_min: Utc.with_ymd_and_hms(2026, 5, 28, 18, 44, 12).unwrap(),
            observed_at_max: Utc.with_ymd_and_hms(2026, 5, 28, 18, 44, 12).unwrap(),
            generated_at: Utc.with_ymd_and_hms(2026, 5, 28, 18, 44, 43).unwrap(),
            evaluator: NqEvaluator {
                evaluator: "disk_state".to_string(),
                version: 1,
            },
            content_hash:
                "sha256:93eba183f059652b041c374d64f1087233385faea0beda10a48a9ed402567a0c"
                    .to_string(),
            cannot_testify: vec!["Incident closure readiness".to_string()],
        }
    }

    #[test]
    fn nq_receipt_ref_issuer_includes_evaluator_version() {
        let r = fixture_nq_receipt();
        assert_eq!(r.issuer(), "nq:disk_state:v1");
    }

    #[test]
    fn cook_intent_carries_nq_content_hash_in_prior_receipt_evidence() {
        // Direct unit check that the chain link is present without
        // standing up a Packet — the integration test below walks the
        // full pipeline end-to-end.
        let nq = fixture_nq_receipt();
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 19, 0, 0).unwrap();
        // Hand-construct a minimal packet stub for the cook call.
        let packet = mock_packet();
        let intent = cook_intent(&packet, &nq, now);

        let prior_receipts: Vec<_> = intent
            .claimed_basis
            .evidence_refs
            .iter()
            .filter(|e| matches!(e.kind, EvidenceKind::PriorReceipt))
            .collect();
        assert_eq!(
            prior_receipts.len(),
            1,
            "exactly one prior_receipt evidence ref expected"
        );
        assert_eq!(
            prior_receipts[0].reference, nq.content_hash,
            "prior_receipt evidence ref must be NQ content hash"
        );
        assert_eq!(prior_receipts[0].issuer, "nq:disk_state:v1");
    }

    #[test]
    fn cook_intent_includes_policy_ref_evidence() {
        let nq = fixture_nq_receipt();
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 19, 0, 0).unwrap();
        let packet = mock_packet();
        let intent = cook_intent(&packet, &nq, now);
        let policy_refs: Vec<_> = intent
            .claimed_basis
            .evidence_refs
            .iter()
            .filter(|e| matches!(e.kind, EvidenceKind::PolicyRef))
            .collect();
        assert_eq!(
            policy_refs.len(),
            1,
            "exactly one policy_ref evidence expected"
        );
        assert!(
            policy_refs[0].reference.contains(NS_POLICY_REF),
            "policy_ref must point at SLICE_4_CONTRACT"
        );
    }

    #[test]
    fn write_canonical_json_is_deterministic() {
        // JCS = RFC 8785; given the same input value, byte output is
        // identical. The integration test relies on this for cross-run
        // hash stability.
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("a.json");
        let p2 = dir.path().join("b.json");
        let value = serde_json::json!({
            "z": [1, 2, 3],
            "a": {"nested": true, "alpha": "x"},
            "m": "middle",
        });
        write_canonical_json(&p1, &value).unwrap();
        write_canonical_json(&p2, &value).unwrap();
        let b1 = std::fs::read(&p1).unwrap();
        let b2 = std::fs::read(&p2).unwrap();
        assert_eq!(b1, b2);
        // JCS sorts keys; the produced bytes should have `a` before
        // `m` before `z` in object key order. Spot-check via string
        // contains: substring "\"a\":" appears before "\"m\":".
        let s = String::from_utf8(b1).unwrap();
        let pos_a = s.find("\"a\":").unwrap();
        let pos_m = s.find("\"m\":").unwrap();
        let pos_z = s.find("\"z\":").unwrap();
        assert!(pos_a < pos_m && pos_m < pos_z, "JCS key order broken");
    }

    fn mock_packet() -> Packet {
        // A minimal packet that satisfies the field set the cook +
        // wrap layers actually read. Other tests exercise the full
        // pipeline via fixtures; this is for unit-level checks where
        // we don't want to stand up the entire reconciler.
        use crate::agenda::AuthorityLevel;
        use crate::bundle::ReconciliationSummary;
        use crate::closure::ClosureCandidate;
        use crate::finding::{EvidenceState, FindingKey, Severity};
        use crate::packet::{
            Attention, AttentionState, AuthorityResult, Confidence, Diagnosis, DiagnosisReview,
            DiagnosisReviewMode, FindingSummary, OperationalUrgency, ProposedAction,
            ProposedActionKind, ReceiptReferences,
        };
        let key = FindingKey {
            source: "nq".to_string(),
            detector: "disk_pressure".to_string(),
            subject: "sushi-k:".to_string(),
        };
        let now = Utc.with_ymd_and_hms(2026, 5, 28, 19, 0, 0).unwrap();
        Packet {
            packet_version: 0,
            packet_id: "pkt_mock".to_string(),
            agenda_id: "sushi-k-disk-pressure".to_string(),
            run_id: "run_mock".to_string(),
            produced_at: now,
            finding_summary: FindingSummary {
                source: key.source.clone(),
                detector: key.detector.clone(),
                host: "sushi-k".to_string(),
                subject: key.subject.clone(),
                severity: Severity::Critical,
                domain: None,
                persistence_generations: 100,
                first_seen_at: now,
                current_status: EvidenceState::Active,
                origin: None,
                silence: None,
                position: None,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
            },
            reconciliation_summary: ReconciliationSummary::default(),
            diagnosis: Diagnosis {
                regime: "mock".to_string(),
                evidence: vec![],
                confidence: Confidence::Low,
                alternatives_considered: vec![],
            },
            proposed_action: ProposedAction {
                kind: ProposedActionKind::Advisory,
                steps: vec![],
                risk_notes: vec![],
                reversible: true,
                blast_radius: "none".to_string(),
                requested_authority_level: AuthorityLevel::Advise,
            },
            authority_result: AuthorityResult {
                requested: AuthorityLevel::Advise,
                governor_present: false,
                governor_verdict: None,
                authority_receipts: vec![],
                ceiling_note: None,
            },
            diagnosis_review: DiagnosisReview {
                mode: DiagnosisReviewMode::SelfCheck,
                unsafe_assumptions: vec![],
                stale_context_risks: vec![],
                promotion_overreach: vec![],
                missing_verification: vec![],
                recommended_downgrade: None,
            },
            attention: Attention {
                attention_key: key,
                evidence_state: EvidenceState::Active,
                attention_state: AttentionState::Unowned,
                posture_class: crate::posture_class::PostureClass::IncidentShape,
                operational_urgency: OperationalUrgency::Critical,
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
            },
            receipt_references: ReceiptReferences::default(),
            unsettled: vec![],
            closure_candidate: ClosureCandidate::UnassessableMissingConsequenceWitness,
        }
    }
}
