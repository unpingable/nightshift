//! Decision-relative adequacy certificate for `DecisionBasisV1`.
//!
//! For normalization rule `nightshift.posture-normalization` v1 and the
//! supported workflow-predicate family v1 (the mirror of AG's
//! `WorkPreconditionV1`), this checker establishes exhaustively:
//!
//! ```text
//! normalize(p1) == normalize(p2)
//!     ⇒
//! ∀ predicate ∈ family, source_verdict(predicate, p1) == source_verdict(predicate, p2)
//! ```
//!
//! This is deliberately weaker than injectivity. A collision is unsafe only
//! when two source states share a normalized basis but some supported
//! predicate's intended source-level verdict differs between them.
//! Collapsing a distinction no supported predicate can observe is benign and
//! accepted by this checker (see `the_checker_accepts_a_benign_collision`).
//!
//! Source-level ground truth is computed by exhaustive matches against the
//! real `ConditionAxis` / `DeliveryStanding` enums and never consults
//! `normalize_posture` or `DecisionBasisV1`, so a broken normalizer that
//! collapses a decision-relevant distinction is caught (see
//! `the_checker_detects_an_unsafe_collision`).
//!
//! Predicate family identity: `nightshift.workflow-precondition-family`
//! version `1` — the complete finite family of valid v1 preconditions:
//! every `(required, forbidden)` pair of subsets of the frozen v1 atom
//! vocabulary with `required ∩ forbidden = ∅`, evaluated as
//! `required ⊆ basis.atoms ∧ forbidden ∩ basis.atoms = ∅`. This mirrors AG's
//! `WorkPreconditionV1` validation and judgment law; the atom vocabulary is
//! cross-repository pinned by the frozen WO-3 basis vector asserted in both
//! repositories. Nightshift ships no production catalogs, so there is no
//! shipped-catalog consistency surface to check beyond the family itself.
//!
//! Maintenance rule: adding a new workflow predicate does NOT automatically
//! require a normalization change. Extend the family, rerun this checker;
//! only if the new predicate distinguishes states the normalization
//! collapses must the normalization vocabulary/rule version grow. If the
//! normalization rule version grows, this certificate must be rebound
//! deliberately — the rule identity is pinned below.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use chrono::{DateTime, Utc};
use nightshiftd::decision_basis::{
    normalization_rule_v1, normalize_posture, ATOM_VOCABULARY_V1, CONDITION_CLEAN_ATOM_V1,
    CONDITION_PRESENT_ATOM_V1, CONDITION_UNRESOLVED_ATOM_V1, DELIVERY_FAILED_ATOM_V1,
    DELIVERY_NOT_CONFIGURED_ATOM_V1, DELIVERY_NOT_REQUIRED_ATOM_V1, DELIVERY_PARTIAL_ATOM_V1,
    DELIVERY_QUALIFIED_ATOM_V1,
};
use nightshiftd::diagnostic_posture::{
    evaluate_posture, ConditionAxis, DeliveryStanding, DiagnosticInputs, OperationalPosture,
    PosturePolicy, RecurrenceEvidence,
};

/// The supported decision family this certificate covers.
const PREDICATE_FAMILY_ID: &str = "nightshift.workflow-precondition-family";
const PREDICATE_FAMILY_VERSION: &str = "1";

/// The frozen v1 normalization rule identity this certificate is bound to.
const EXPECTED_RULE_ID: &str = "nightshift.posture-normalization";
const EXPECTED_RULE_VERSION: &str = "1";
const EXPECTED_RULE_DIGEST: &str =
    "sha256:5f8bd1a497e034633d6fd465a6834a2ca8e9a4b20158322fd0a4bc36095f8e67";

/// One member of the supported predicate family: the exact mirror of AG's
/// `WorkPreconditionV1` judgment law.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SupportedPredicateV1 {
    required: BTreeSet<&'static str>,
    forbidden: BTreeSet<&'static str>,
}

impl SupportedPredicateV1 {
    /// The AG-side judgment law: `required ⊆ basis.atoms` and
    /// `forbidden ∩ basis.atoms = ∅`.
    fn holds_over(&self, atoms: &BTreeSet<&str>) -> bool {
        self.required.is_subset(atoms) && self.forbidden.is_disjoint(atoms)
    }
}

/// The intended source-level meaning of each v1 atom, defined by exhaustive
/// matches against the real source enums. This is the independent ground
/// truth of the certificate: it never consults `normalize_posture` or any
/// `DecisionBasisV1`, so a normalizer that maps a source variant to the
/// wrong atom is caught here.
fn atom_source_truth(atom: &str, condition: &ConditionAxis, delivery: &DeliveryStanding) -> bool {
    match atom {
        CONDITION_CLEAN_ATOM_V1 => matches!(condition, ConditionAxis::Clean),
        CONDITION_PRESENT_ATOM_V1 => matches!(condition, ConditionAxis::ConditionPresent),
        CONDITION_UNRESOLVED_ATOM_V1 => matches!(condition, ConditionAxis::Unresolved),
        DELIVERY_QUALIFIED_ATOM_V1 => matches!(delivery, DeliveryStanding::Qualified),
        DELIVERY_PARTIAL_ATOM_V1 => matches!(delivery, DeliveryStanding::PartialDelivery),
        DELIVERY_FAILED_ATOM_V1 => matches!(delivery, DeliveryStanding::Failed),
        DELIVERY_NOT_CONFIGURED_ATOM_V1 => matches!(delivery, DeliveryStanding::NotConfigured),
        DELIVERY_NOT_REQUIRED_ATOM_V1 => matches!(delivery, DeliveryStanding::NotRequired),
        other => panic!("predicate atom outside the frozen v1 vocabulary: {other}"),
    }
}

/// The intended source-level verdict of one supported predicate over one
/// source state, computed only from the state's semantic axes.
fn source_verdict(
    predicate: &SupportedPredicateV1,
    condition: &ConditionAxis,
    delivery: &DeliveryStanding,
) -> bool {
    predicate
        .required
        .iter()
        .all(|atom| atom_source_truth(atom, condition, delivery))
        && predicate
            .forbidden
            .iter()
            .all(|atom| !atom_source_truth(atom, condition, delivery))
}

/// The complete supported predicate family v1: every `(required, forbidden)`
/// pair of subsets of the frozen v1 atom vocabulary with
/// `required ∩ forbidden = ∅`. Enumerated exhaustively by giving each atom
/// three roles (required / forbidden / absent), so the family size is
/// `3^|vocabulary|` by construction — no heuristic reduction.
fn supported_predicate_family_v1() -> Vec<SupportedPredicateV1> {
    let mut family = Vec::new();
    let mut roles = [0u8; ATOM_VOCABULARY_V1.len()];
    loop {
        let mut required = BTreeSet::new();
        let mut forbidden = BTreeSet::new();
        for (atom, role) in ATOM_VOCABULARY_V1.iter().zip(roles.iter()) {
            match role {
                0 => {}
                1 => {
                    required.insert(*atom);
                }
                2 => {
                    forbidden.insert(*atom);
                }
                _ => unreachable!(),
            }
        }
        family.push(SupportedPredicateV1 {
            required,
            forbidden,
        });
        let mut index = 0;
        loop {
            if index == roles.len() {
                return family;
            }
            roles[index] += 1;
            if roles[index] < 3 {
                break;
            }
            roles[index] = 0;
            index += 1;
        }
    }
}

/// What the checker established about one (normalizer, family) pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AdequacyReport {
    states: usize,
    predicates: usize,
    equal_basis_pairs: usize,
    verdict_comparisons: usize,
}

/// A certified failure: two source states share a normalized basis, but a
/// supported predicate's intended source verdict separates them.
#[derive(Clone, Debug)]
struct UnsafeCollision {
    state_a: String,
    state_b: String,
    basis: String,
    predicate: String,
    verdict_a: bool,
    verdict_b: bool,
}

impl std::fmt::Display for UnsafeCollision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unsafe DecisionBasis collision\n\nstate A:\n  {}\n\nstate B:\n  {}\n\nnormalized basis:\n  {}\n\ndistinguishing predicate:\n  {}\n\nsource verdict A = {}\nsource verdict B = {}",
            self.state_a,
            self.state_b,
            self.basis,
            self.predicate,
            self.verdict_a,
            self.verdict_b,
        )
    }
}

fn describe_predicate(predicate: &SupportedPredicateV1) -> String {
    let mut out = String::from("required = {");
    let mut first = true;
    for atom in &predicate.required {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "{atom}");
    }
    out.push_str("}, forbidden = {");
    let mut first = true;
    for atom in &predicate.forbidden {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "{atom}");
    }
    out.push('}');
    out
}

/// The generic adequacy check. For every ordered pair of source states with
/// equal normalized basis, every supported predicate's intended source-level
/// verdict must agree. The source verdict is computed from the state's
/// semantic axes via `axes`, never from the normalized basis, so the check
/// is a real cross-evaluation, not a tautology over the projection output.
fn check_adequacy<S, B: Eq>(
    states: &[(String, S)],
    axes: impl Fn(&S) -> (ConditionAxis, DeliveryStanding),
    normalize: impl Fn(&S) -> B,
    describe_basis: impl Fn(&B) -> String,
    predicates: &[SupportedPredicateV1],
) -> Result<AdequacyReport, UnsafeCollision> {
    let bases: Vec<B> = states.iter().map(|(_, state)| normalize(state)).collect();
    let mut equal_basis_pairs = 0usize;
    let mut verdict_comparisons = 0usize;
    for left in 0..states.len() {
        for right in 0..states.len() {
            if bases[left] != bases[right] {
                continue;
            }
            equal_basis_pairs += 1;
            let (condition_a, delivery_a) = axes(&states[left].1);
            let (condition_b, delivery_b) = axes(&states[right].1);
            for predicate in predicates {
                verdict_comparisons += 1;
                let verdict_a = source_verdict(predicate, &condition_a, &delivery_a);
                let verdict_b = source_verdict(predicate, &condition_b, &delivery_b);
                if verdict_a != verdict_b {
                    return Err(UnsafeCollision {
                        state_a: states[left].0.clone(),
                        state_b: states[right].0.clone(),
                        basis: describe_basis(&bases[left]),
                        predicate: describe_predicate(predicate),
                        verdict_a,
                        verdict_b,
                    });
                }
            }
        }
    }
    Ok(AdequacyReport {
        states: states.len(),
        predicates: predicates.len(),
        equal_basis_pairs,
        verdict_comparisons,
    })
}

fn example_policy_inputs_recurrence() -> (PosturePolicy, DiagnosticInputs, RecurrenceEvidence) {
    (
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/policy.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/recurrence.json"
        ))
        .unwrap(),
    )
}

/// One canonical valid posture from the checked-in specimen. The
/// enumeration varies only the two axes normalization rule v1 reads; every
/// other field is held at this canonical value and is outside the v1
/// decision domain.
fn canonical_posture() -> OperationalPosture {
    let (policy, inputs, recurrence) = example_policy_inputs_recurrence();
    evaluate_posture(
        &policy,
        &inputs,
        &recurrence,
        DateTime::parse_from_rfc3339("2026-07-27T20:00:10Z")
            .unwrap()
            .with_timezone(&Utc),
    )
    .unwrap()
}

const ALL_CONDITIONS: [ConditionAxis; 3] = [
    ConditionAxis::Clean,
    ConditionAxis::ConditionPresent,
    ConditionAxis::Unresolved,
];

const ALL_DELIVERIES: [DeliveryStanding; 5] = [
    DeliveryStanding::Qualified,
    DeliveryStanding::PartialDelivery,
    DeliveryStanding::Failed,
    DeliveryStanding::NotConfigured,
    DeliveryStanding::NotRequired,
];

/// The complete exportable source domain of DecisionBasis v1: the Cartesian
/// product of the two axes `normalize_posture` reads. Match exhaustiveness
/// on the real enums plus these explicit arrays keeps the enumeration
/// pinned to the actual type definitions; a new variant forces a deliberate
/// update here.
fn source_states() -> Vec<(String, OperationalPosture)> {
    let canonical = canonical_posture();
    let mut states = Vec::new();
    for condition in ALL_CONDITIONS {
        for delivery in ALL_DELIVERIES {
            let mut posture = canonical.clone();
            posture.condition = condition;
            posture.delivery = delivery;
            states.push((
                format!("condition = {condition:?}\n  delivery = {delivery:?}"),
                posture,
            ));
        }
    }
    states
}

/// The production certificate: DecisionBasis v1 is decision-relatively
/// adequate for the complete supported predicate family v1.
#[test]
fn decision_basis_v1_is_adequate_for_predicate_family_v1() {
    // The certificate is bound to exactly this normalization rule.
    let rule = normalization_rule_v1();
    assert_eq!(rule.id, EXPECTED_RULE_ID);
    assert_eq!(rule.version, EXPECTED_RULE_VERSION);
    assert_eq!(rule.digest, EXPECTED_RULE_DIGEST);

    let states = source_states();
    let predicates = supported_predicate_family_v1();
    let report = check_adequacy(
        &states,
        |posture| (posture.condition, posture.delivery),
        normalize_posture,
        |basis| basis.digest().unwrap(),
        &predicates,
    );
    let report = match report {
        Ok(report) => report,
        Err(collision) => panic!("{collision}"),
    };
    assert_eq!(report.states, 15, "ConditionAxis × DeliveryStanding");
    assert_eq!(
        report.predicates,
        3usize.pow(ATOM_VOCABULARY_V1.len() as u32),
        "every valid (required, forbidden) combination"
    );
    assert_eq!(report.predicates, 6561);
    // Current v1 maps each axis variant to its own atom, so all 15 enumerated
    // bases are distinct and only diagonal pairs compare. This is a count pin
    // that makes any future collapse visible — it is NOT an injectivity
    // requirement; the adequacy property itself is the theorem.
    assert_eq!(report.equal_basis_pairs, 15);
    assert_eq!(report.verdict_comparisons, 15 * 6561);

    // Per-state faithfulness, the pointwise form of the same certificate:
    // for every enumerated source state and every supported predicate, the
    // basis-level verdict (the mirror of AG's judgment over
    // `normalize_posture` output) equals the independent source-level
    // verdict.
    for (label, posture) in &states {
        let basis = normalize_posture(posture);
        let atoms: BTreeSet<&str> = basis.atoms.iter().map(String::as_str).collect();
        for predicate in &predicates {
            assert_eq!(
                predicate.holds_over(&atoms),
                source_verdict(predicate, &posture.condition, &posture.delivery),
                "basis-level and source-level verdicts diverge at {label} for {}",
                describe_predicate(predicate)
            );
        }
    }
}

/// The checker logic does not require injectivity: a state-space distinction
/// that normalization collapses but no supported predicate can observe is
/// accepted.
#[test]
fn the_checker_accepts_a_benign_collision() {
    // A phantom flag outside the v1 decision domain doubles the state space;
    // the (test-local) normalization collapses it by construction.
    #[derive(Clone)]
    struct PhantomState {
        condition: ConditionAxis,
        delivery: DeliveryStanding,
        phantom: bool,
    }
    let canonical = canonical_posture();
    let mut states = Vec::new();
    for condition in ALL_CONDITIONS {
        for delivery in ALL_DELIVERIES {
            for phantom in [false, true] {
                states.push((
                    format!(
                        "condition = {condition:?}\n  delivery = {delivery:?}\n  phantom = {phantom}"
                    ),
                    PhantomState {
                        condition,
                        delivery,
                        phantom,
                    },
                ));
            }
        }
    }
    let predicates = supported_predicate_family_v1();
    let report = check_adequacy(
        &states,
        |state| (state.condition, state.delivery),
        |state| {
            let mut posture = canonical.clone();
            posture.condition = state.condition;
            posture.delivery = state.delivery;
            normalize_posture(&posture)
        },
        |basis| basis.digest().unwrap(),
        &predicates,
    );
    let report = match report {
        Ok(report) => report,
        Err(collision) => panic!("a phantom-flag collision is benign:\n{collision}"),
    };
    assert_eq!(report.states, 30);
    // Each of the 15 distinct bases is shared by the two phantom variants:
    // 2 × 2 ordered pairs per basis.
    assert_eq!(report.equal_basis_pairs, 60);
    assert_eq!(report.verdict_comparisons, 60 * 6561);
}

/// The checker detects loss of decision-relevant information: a deliberately
/// broken normalizer that reports every condition as clean collapses `Clean`
/// and `ConditionPresent` source states into one basis, and the
/// `required = {condition.clean}` predicate separates them at source level.
#[test]
fn the_checker_detects_an_unsafe_collision() {
    let states = source_states();
    let predicates = supported_predicate_family_v1();
    // Test-local broken projection: every condition claims the clean atom.
    // Production `normalize_posture` is untouched.
    let broken_normalize = |posture: &OperationalPosture| {
        let delivery_atom = match posture.delivery {
            DeliveryStanding::Qualified => DELIVERY_QUALIFIED_ATOM_V1,
            DeliveryStanding::PartialDelivery => DELIVERY_PARTIAL_ATOM_V1,
            DeliveryStanding::Failed => DELIVERY_FAILED_ATOM_V1,
            DeliveryStanding::NotConfigured => DELIVERY_NOT_CONFIGURED_ATOM_V1,
            DeliveryStanding::NotRequired => DELIVERY_NOT_REQUIRED_ATOM_V1,
        };
        BTreeSet::from([CONDITION_CLEAN_ATOM_V1, delivery_atom])
    };
    let collision = check_adequacy(
        &states,
        |posture| (posture.condition, posture.delivery),
        broken_normalize,
        |atoms| format!("{atoms:?}"),
        &predicates,
    )
    .expect_err("a broken normalizer must produce an unsafe collision");
    let report = collision.to_string();
    assert!(report.contains("condition = Clean"));
    assert!(report.contains("condition = ConditionPresent"));
    assert!(report.contains("required = {condition.clean}"));
    assert!(report.contains("source verdict A = true"));
    assert!(report.contains("source verdict B = false"));
}

/// The source-level evaluator's ground truth is pinned atom by atom against
/// the real source enums. It is structurally independent of
/// `normalize_posture`: it matches on the source axes directly.
#[test]
fn source_evaluator_ground_truth_is_the_source_enums() {
    let canonical = canonical_posture();
    for condition in ALL_CONDITIONS {
        for delivery in ALL_DELIVERIES {
            let mut posture = canonical.clone();
            posture.condition = condition;
            posture.delivery = delivery;
            for atom in ATOM_VOCABULARY_V1 {
                let expected = match atom {
                    CONDITION_CLEAN_ATOM_V1 => condition == ConditionAxis::Clean,
                    CONDITION_PRESENT_ATOM_V1 => condition == ConditionAxis::ConditionPresent,
                    CONDITION_UNRESOLVED_ATOM_V1 => condition == ConditionAxis::Unresolved,
                    DELIVERY_QUALIFIED_ATOM_V1 => delivery == DeliveryStanding::Qualified,
                    DELIVERY_PARTIAL_ATOM_V1 => delivery == DeliveryStanding::PartialDelivery,
                    DELIVERY_FAILED_ATOM_V1 => delivery == DeliveryStanding::Failed,
                    DELIVERY_NOT_CONFIGURED_ATOM_V1 => delivery == DeliveryStanding::NotConfigured,
                    DELIVERY_NOT_REQUIRED_ATOM_V1 => delivery == DeliveryStanding::NotRequired,
                    other => panic!("atom outside the v1 vocabulary: {other}"),
                };
                assert_eq!(
                    atom_source_truth(atom, &posture.condition, &posture.delivery),
                    expected,
                    "ground truth for {atom} at {condition:?}/{delivery:?}"
                );
            }
        }
    }
}

/// The enumerated predicate family is exactly the valid `WorkPreconditionV1`
/// decision class: every member is disjoint and names only frozen v1 atoms,
/// and the family size is 3^|vocabulary| by construction.
#[test]
fn predicate_family_v1_is_complete_and_vocabulary_bound() {
    let family = supported_predicate_family_v1();
    assert_eq!(ATOM_VOCABULARY_V1.len(), 8);
    assert_eq!(family.len(), 3usize.pow(8));
    assert_eq!(
        PREDICATE_FAMILY_ID,
        "nightshift.workflow-precondition-family"
    );
    assert_eq!(PREDICATE_FAMILY_VERSION, "1");
    for predicate in &family {
        assert!(predicate.required.is_disjoint(&predicate.forbidden));
        for atom in predicate.required.union(&predicate.forbidden) {
            assert!(ATOM_VOCABULARY_V1.contains(atom));
        }
    }
    // Every frozen atom is realizable by some enumerated source state, so no
    // family member is vacuously untestable.
    let states = source_states();
    for atom in ATOM_VOCABULARY_V1 {
        assert!(
            states.iter().any(|(_, posture)| atom_source_truth(
                atom,
                &posture.condition,
                &posture.delivery
            )),
            "atom {atom} is not realizable by any enumerated source state"
        );
    }
}

/// Every enumerated source pair normalizes successfully and satisfies the
/// v1 cardinality invariant (exactly one condition and one delivery atom).
/// This is a normalization-coverage pin, not the adequacy theorem.
#[test]
fn all_fifteen_source_states_normalize_with_v1_cardinality() {
    let mut bases = BTreeSet::new();
    for (_, posture) in source_states() {
        let basis = normalize_posture(&posture);
        basis.validate().unwrap();
        assert_eq!(basis.atoms.len(), 2);
        assert_eq!(
            basis
                .atoms
                .iter()
                .filter(|atom| atom.starts_with("condition."))
                .count(),
            1
        );
        bases.insert(basis.digest().unwrap());
    }
    assert_eq!(bases.len(), 15);
}
