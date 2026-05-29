//! `ProposedAction` — Nightshift's terminal emission across the
//! actuation boundary.
//!
//! See `docs/working/decisions/ACTUATION_BOUNDARY.md` (canonical
//! source in `~/git/cartography/doctrine/nightshift-actuation-boundary.md`).
//!
//! ## What this type is
//!
//! The inert artifact NS emits when it has classified a finding,
//! cooked a Wicket admissibility verdict, and decided the operator-
//! visible posture warrants surfacing an option. A `ProposedAction`
//! is a **description**, not an instruction NS can execute.
//!
//! Per the doctrine doc:
//!
//! > A trigger is an emission across a boundary. Actuation is holding
//! > the creds. Nightshift keeps every trigger. It just fires an
//! > *inert proposal* into a separately-privileged executor instead of
//! > pulling the trigger on the substrate itself.
//!
//! ## What this type is NOT
//!
//! - No `enact` field, no command string, no connection target, no
//!   credentials. The type's *shape* makes actuation impossible from
//!   inside NS — there is nothing to execute even if a caller wanted
//!   to.
//! - No reference to an `Executor`. The executor lives on the other
//!   side of a privilege boundary and is not modeled here. Whoever
//!   holds creds consumes `ProposedAction`s and enacts.
//! - Not wired into any pipeline that actually enacts anything. The
//!   v1 surface is producer-side: NS *can* produce a ProposedAction;
//!   nothing in NS *consumes* one to act on.
//!
//! ## Relationship to `crate::packet::ProposedAction`
//!
//! `crate::packet::ProposedAction` is the per-finding advisory field
//! inside the review packet — operator-facing prose describing what
//! NS suggests humans look at. This module's `ProposedAction` is the
//! terminal emission across the actuation boundary: a separately
//! addressable artifact that carries an admitted Wicket verdict and
//! content-addresses the originating NQ finding.
//!
//! Both exist; they serve different roles. The packet field is
//! "what does this run advise"; this module's type is "what crosses
//! the boundary to an executor."

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reversibility of the proposed action. Per the doctrine doc:
/// *irreversible → human, always.* Auto-eligibility is reserved
/// for `Reversible` actions, and only under a Wicket
/// standing-grant scoped to the action kind — that grant is held
/// by the auto-executor, never by NS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    Reversible,
    Irreversible,
}

/// Subject of a `ProposedAction`: content-addresses the originating
/// NQ finding so custody walks back to the observation. Two-field
/// shape:
///
/// - `finding_key` — the stable NQ finding identity NS classified.
/// - `basis_hash` — the content hash of the NQ receipt this action
///   was proposed against (typically the receipt's `content_hash`,
///   `sha256:<hex>`). A verifier who has both this hash and the
///   upstream NQ receipt can confirm subject custody without
///   re-running the pipeline.
///
/// This mirrors the in-toto-style custody chain the rest of the
/// constellation already uses (MVP-A NS posture-packet's
/// `nq_receipt_ref`, Wicket Intent's `prior_receipt` evidence,
/// WLP's `custody.causal_parents`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSubject {
    pub finding_key: String,
    pub basis_hash: String,
}

/// Wicket verdict carried into the proposal. A `ProposedAction`
/// **must** carry an admitted verdict — `authorized` or `gap` — to
/// have meaning. A proposal carrying `denied` is still serializable
/// (the type is honest about all wire outcomes) but is not
/// downstream-enactable, and the executor side is expected to
/// refuse it on those grounds.
///
/// The fields are a deliberate subset of `wicket::Outcome`: enough
/// for an executor to reconstruct the upstream receipt chain without
/// pulling in the full Wicket dependency. Each field maps 1:1 to the
/// Wicket Outcome it was extracted from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Admissibility {
    /// Wicket `SurfaceVerdict` rendered as snake_case string.
    /// Allowed values per Wicket SPEC §8: `authorized`,
    /// `advisory_only`, `denied`, `gap`, `unaccounted`.
    pub surface_verdict: String,
    /// Wicket `OutcomeClass` rendered as snake_case string:
    /// `verdict` or `error`.
    pub outcome_class: String,
    /// Wicket `OperationClass` rendered as snake_case string:
    /// `observe`, `interpret`, `recommend`, `authorize`, `execute`,
    /// `bind`.
    pub operation_class: String,
    /// Wicket reason codes (snake_case strings). Open set.
    pub reason_codes: Vec<String>,
    /// The Wicket receipt's `input_hash`. Walks back to the Intent
    /// canonical bytes, which carry the NQ receipt as
    /// `prior_receipt` evidence. Together with `subject.basis_hash`
    /// this lets a verifier confirm both directions of the chain.
    pub receipt_input_hash: String,
}

/// Action kind. Open vocabulary; NS today emits descriptive names
/// (e.g., `advisory`, `revalidate_only`, `request_human_review`),
/// not actuator names. The doctrine doc forbids action kinds that
/// name actuators (`ssh.reboot`, `systemctl.restart`, etc.); the
/// guard script blocks new sites where such names would land in
/// code regardless of how this field is populated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionKind {
    pub name: String,
}

/// Nightshift's inert `ProposedAction`. Canonical-JSON serializable
/// per RFC 8785 via `serde_jcs`.
///
/// **There is no `enact` field on this struct, by construction.**
/// `enactment` is `serde_json::Value::Null` on the wire — the empty
/// shape — and the field exists at all only so the wire envelope is
/// explicit that "no instructions live here." A future version could
/// remove the field entirely; today's posture is to make the
/// emptiness *visible* on the wire so the doctrinal property
/// (*Nightshift never holds the trigger*) is auditable from the
/// artifact alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedAction {
    /// Wire schema identifier. Always `"ns.proposed_action.v1"`.
    pub schema: String,
    /// NS daemon identity that emitted this proposal.
    pub actor: String,
    /// Subject — content-addresses the originating NQ finding.
    pub subject: ActionSubject,
    pub action_kind: ActionKind,
    /// Human-readable single-line summary. Not an instruction; not
    /// executable. Operator-facing prose only.
    pub summary: String,
    /// Rationale lines — why NS chose this action. Operator-facing;
    /// not parsed.
    #[serde(default)]
    pub rationale: Vec<String>,
    pub reversibility: Reversibility,
    /// Wicket admissibility verdict. A consumer that admits the
    /// proposal MUST consult this field before considering the
    /// action well-formed downstream.
    pub admissibility: Admissibility,
    /// Intentionally always `null` on the wire. NS does not encode
    /// commands, connection targets, or credentials. If an action
    /// happens, a *different program* did it.
    pub enactment: serde_json::Value,
    pub issued_at: DateTime<Utc>,
}

impl ProposedAction {
    pub const SCHEMA: &'static str = "ns.proposed_action.v1";

    /// Construct a new ProposedAction with `enactment` forced to
    /// `Null`. Even if a caller tries to pass a value, the
    /// constructor refuses to thread it through.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor: String,
        subject: ActionSubject,
        action_kind: ActionKind,
        summary: String,
        rationale: Vec<String>,
        reversibility: Reversibility,
        admissibility: Admissibility,
        issued_at: DateTime<Utc>,
    ) -> Self {
        Self {
            schema: Self::SCHEMA.to_string(),
            actor,
            subject,
            action_kind,
            summary,
            rationale,
            reversibility,
            admissibility,
            enactment: serde_json::Value::Null,
            issued_at,
        }
    }

    /// Serialize to canonical JCS bytes. Same canonicalization the
    /// rest of the constellation uses (NQ receipts, Wicket Intents,
    /// WLP artifacts), so a verifier hashing this byte stream gets
    /// the same content hash a producer computed.
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_jcs::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_admissibility() -> Admissibility {
        Admissibility {
            surface_verdict: "authorized".to_string(),
            outcome_class: "verdict".to_string(),
            operation_class: "interpret".to_string(),
            reason_codes: vec![
                "BASIS_OK".to_string(),
                "STANDING_CALLER_ASSERTED_UNVERIFIED".to_string(),
            ],
            receipt_input_hash:
                "sha256:cb86eca82645c6f55fabc1e7d74ff75f5750c018c274f2ac5f280ea1cb4ab789"
                    .to_string(),
        }
    }

    fn example_proposed_action() -> ProposedAction {
        ProposedAction::new(
            "ns:nightshiftd".to_string(),
            ActionSubject {
                finding_key: "nq:disk_pressure:sushi-k:".to_string(),
                basis_hash:
                    "sha256:93eba183f059652b041c374d64f1087233385faea0beda10a48a9ed402567a0c"
                        .to_string(),
            },
            ActionKind {
                name: "request_human_review".to_string(),
            },
            "Operator review of disk_pressure on sushi-k".to_string(),
            vec![
                "NQ disk_state receipt verified for host:sushi-k.".to_string(),
                "Slice 4 closure predicate refuses with \
                 UnassessableMissingConsequenceWitness."
                    .to_string(),
            ],
            Reversibility::Reversible,
            example_admissibility(),
            chrono::Utc.with_ymd_and_hms(2026, 5, 28, 19, 0, 0).unwrap(),
        )
    }

    use chrono::TimeZone;

    #[test]
    fn enactment_is_null_after_construction() {
        let pa = example_proposed_action();
        assert_eq!(pa.enactment, serde_json::Value::Null);
    }

    #[test]
    fn schema_constant_matches_struct_field() {
        let pa = example_proposed_action();
        assert_eq!(pa.schema, ProposedAction::SCHEMA);
        assert_eq!(pa.schema, "ns.proposed_action.v1");
    }

    #[test]
    fn type_has_no_actuation_fields() {
        // Static check: the JSON wire shape names every field NS
        // exposes on a proposal. If a future change adds a command
        // string, connection target, or credential field, this test
        // will need to be deliberately updated to allow it — at
        // which point the doctrine check catches the change in
        // review.
        let pa = example_proposed_action();
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&pa).unwrap()).unwrap();
        let allowed_keys: std::collections::HashSet<&str> = [
            "schema",
            "actor",
            "subject",
            "action_kind",
            "summary",
            "rationale",
            "reversibility",
            "admissibility",
            "enactment",
            "issued_at",
        ]
        .into_iter()
        .collect();
        let actual_keys: std::collections::HashSet<&str> =
            json.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert_eq!(
            actual_keys, allowed_keys,
            "ProposedAction wire shape changed; if intentional, audit \
             against ACTUATION_BOUNDARY.md before accepting"
        );
        // Forbidden field names that would themselves be a doctrinal
        // smell. Adding any of these is exactly the kind of drift
        // the boundary protects against.
        for forbidden in [
            "enact",
            "command",
            "command_str",
            "exec",
            "execute",
            "connection",
            "connection_target",
            "credentials",
            "creds",
            "ssh_target",
            "host_to_run_on",
            "actuator",
        ] {
            assert!(
                !actual_keys.contains(forbidden),
                "actuation-shaped field `{forbidden}` must not appear on ProposedAction"
            );
        }
    }

    #[test]
    fn canonical_json_round_trips_deterministically() {
        let pa = example_proposed_action();
        let s1 = pa.to_canonical_json().unwrap();
        let s2 = pa.to_canonical_json().unwrap();
        assert_eq!(s1, s2, "JCS canonical bytes must be deterministic");
        let parsed: ProposedAction = serde_json::from_str(&s1).unwrap();
        assert_eq!(parsed, pa);
        // JCS key ordering: `action_kind` < `actor` < `admissibility`
        // < `enactment` < `issued_at` < `rationale` < `reversibility`
        // < `schema` < `subject` < `summary` in lexicographic order.
        let pos_actor = s1.find("\"actor\":").unwrap();
        let pos_enactment = s1.find("\"enactment\":").unwrap();
        let pos_schema = s1.find("\"schema\":").unwrap();
        assert!(pos_actor < pos_enactment);
        assert!(pos_enactment < pos_schema);
    }

    #[test]
    fn fixture_round_trips_through_canonical_json() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = std::path::Path::new(manifest_dir)
            .join("tests")
            .join("fixtures")
            .join("proposed_action.example.json");
        let raw = std::fs::read_to_string(&fixture_path)
            .expect("proposed_action.example.json must exist alongside tests/fixtures/");
        let parsed: ProposedAction =
            serde_json::from_str(&raw).expect("fixture must parse as ProposedAction");
        assert_eq!(parsed.schema, ProposedAction::SCHEMA);
        assert_eq!(parsed.enactment, serde_json::Value::Null);
        // The fixture round-trips: serialize back to canonical
        // JCS, parse again, assert deep-equal.
        let canonical = parsed.to_canonical_json().unwrap();
        let reparsed: ProposedAction = serde_json::from_str(&canonical).unwrap();
        assert_eq!(parsed, reparsed);
    }
}
