# Formalization handoff

## Status and source of truth

This is a **formalization specification, not a proof result**. No current
claim of end-to-end Lean/runtime conformance exists. The executable contract
and tests precede the formalization: the runtime is done and green, and this
note only describes the smallest formal work that behavior now justifies.

The normative implementation contract is
[`docs/CANONICAL_RUNTIME_C1.md`](../../CANONICAL_RUNTIME_C1.md). Pre-cutover
correspondence documents —
[`NQ-NIGHTSHIFT-LEAN-CORRESPONDENCE.md`](NQ-NIGHTSHIFT-LEAN-CORRESPONDENCE.md)
and [`CALCULUS-CONFORMANCE.md`](CALCULUS-CONFORMANCE.md) — are historical:
the runtime they describe was deleted or quarantined by the canonical
cutover, and they must not be treated as the runtime model.

Nothing here changes runtime behavior. Nothing here is Lean.

## Runtime boundary being modeled

One governed occurrence walks a closed program counter:

```text
ObservationRequired → ProposalRecorded → StandingRequired
  → AdmissiblePendingAuthorization → AuthorizationConsumed
  → Dispatched → SettledObservationRequired → (continuation) ...
```

The authority-relevant facts, per the C1 contract:

- `record_proposal` is informational: it checks identity/binding only
  (campaign/occurrence, proposal class rules, and the exact prepared-work
  binding), plus observation resolution for the record. It evaluates no
  workflow policy.
- The sole authority-minting event is `AgAuthorizationSpendV1`, reachable
  only from `authorize`, which revalidates every gate fresh: observation
  health, pinned evidence basis, current catalog predicate, current
  standing, exact work binding.
- Docket independently gates the post-spend execution/effect boundary.

## F1 — spend-gating target

Model a small state machine with the gate dimensions kept distinct:

```text
1. observationUsable   -- cited evidence resolves Current
2. basisConsistent     -- currentBasis = recordedBasis (pinned evidence)
3. workflowAllowed     -- current catalog predicate over the basis
4. standing            -- external governance value at judgment time
5. workBound           -- proposalWork = expectedWork
6. spendCount          -- one-use discipline
```

Theorem shape (safety direction only):

```text
Spend occurs
  → observationUsable ∧ basisConsistent ∧ workflowAllowed
    ∧ standing ∧ workBound
```

under the modeled `authorize` transition, with `spendCount ≤ 1` per
occurrence. The model must allow: a proposal recorded while the predicate
fails or standing is absent; standing recovery for the same proposal; an
evidence change refusing (not refreshing) an old proposal; policy change
between `decide` and `authorize`; spend only after authorize-time
revalidation. The converse (every valid situation eventually spends) is not
a target of this tranche.

**Standing treatment.** Standing is an environmental governance predicate —
a Boolean (or equivalently a Current/non-Current status) supplied to the
modeled kernel at each judgment. The theorem is conditional: *given* the
standing value the kernel reads, no spend occurs while it is false, and
standing alone mints no authority. Lean does not model or prove the
authority's truthfulness, mandate semantics, or revocation timeliness.

**Observation-health treatment.** The resolver establishes usability of
cited historical evidence — unique citation, internal integrity, actionable
freshness, latest-qualified-in-lineage, valid basis — not current-world
truth. The five statuses (`Current | Stale | Superseded | Contradictory |
Absent`) may be collapsed to `usable | unusable` for F1; no F1 theorem
depends on the finer distinction. The finer statuses matter only to the F4
adapter that justifies `usable`.

## F2 — provenance / no-rebinding target

A successful spend determines its provenance. Modest theorem shapes:

- **Policy provenance**: `Spend s → s.policy_basis = identity(catalog)` for
  the catalog actually evaluated at that judgment. Digest equality is
  treated as identity equality at the model boundary; collision resistance
  is an external assumption, not a Lean goal.
- **No silent evidence refresh**:
  `recordedBasis = B ∧ judgmentBasis = B' ∧ B' ≠ B → no spend from that
  judgment`.
- **Exact-work binding**: `Spend work=W → W = occurrence.expectedWork`,
  where `expectedWork` was bound at occurrence creation.

Keep the two work-identity domains distinct in the model: Nightshift's
`compiled_work` (compiled-payload identity) and AG's executable-work
identity (executor-plan identity). The runtime guarantee is a sealed
binding, not digest equality — model it as a relation
`PreparedWorkBinding compiledWork agWork` established at proposal
preparation, plus the record-time equality check. Cryptographic
serialization is not reproduced.

## F3 — DecisionBasis adequacy target

The runtime normalization is
`normalize : Posture → DecisionBasis`; the supported decision family is
`WorkPreconditionV1` (`required ⊆ atoms ∧ forbidden ∩ atoms = ∅`); the
intended source semantics is a predicate's direct meaning over posture axes.
The theorem:

```text
normalize(p1) = normalize(p2)
  → ∀ predicate ∈ SupportedFamily, desired(predicate, p1) = desired(predicate, p2)
```

This is decision-relative adequacy. Injectivity is explicitly **not** the
property: benign collisions are allowed whenever no supported decision
distinguishes the source states.

Exact current finite domain (from the executable certificate):

- source states: `ConditionAxis × DeliveryStanding` = 3 × 5 = **15**;
- atom vocabulary: **8** frozen v1 atoms;
- complete predicate family: every disjoint `(required, forbidden)` pair —
  **3^8 = 6561**;
- executable certificate: **98,415** source-level verdict comparisons,
  zero unsafe collisions.

The formalization should mirror this exact enumeration and family, not a
more impressive abstraction.

**Executable certificate → Lean relationship.** The Rust checker
(`crates/nightshiftd/tests/decision_basis_adequacy.rs`) is the
production-facing regression witness and stays after Lean exists. The Lean
side provides the kernel-checked statement that an exhaustive
collision-free enumeration over this domain implies adequacy. One finite
vocabulary/domain specification should feed both; do not maintain two
independent semantic universes.

## Observation lineage/currentness adapter (F4)

Only enough lineage semantics to justify `usable` for F1:

```text
FamilyKey = (policy_id, configuration_version, subject_id, scope_id,
             scheduler_clock_id)
OrderKey  = (occurrence, nominal_due_at, slot_id)   -- logical order
Qualified(cycle) := cycle.observation.is_some()
Superseded(cited) := ∃ later qualified observation in the same family
```

Preserve: unrelated families never supersede; Missed/RecoveryRequired
cycles (no observation) never supersede; later weak/blind-support
observations do supersede; completion time and `evaluated_at` never define
order; no subject-wide latest. A small finite lineage model is sufficient.
Missed-boundary facts (`now == latest_admissible` admissible, `now >`
Missed) enter only as definition-level consequences of `Qualified`.

## Existing formal work worth reusing

| Artifact | Exact names | Status | Reuse |
|---|---|---|---|
| `leantmp/ObservationAdequacy` generic library (frozen 2026-08-08; experimental, never promoted) | `RelSystem`, `Enabled`, `Runs`, `Reachable`; `adequateForNext_iff_exists_classifier`; `adequateForNext_iff_observation_refines_decisionEquivalent`; `next_collision_refutes_adequacy`; `AdequateForConsumer`; `DecisionEquivalent` | Live as generic machinery | F1 state-machine substrate and F3 adequacy meta-theory |
| `ObservationAdequacy.Enumerator` | `findCollision_sound`, `findCollision_complete_on_enumeration`, `findCollision_complete_of_bounded_collision`, `findCollision_none_certifies_adequacy_for` | Live as generic machinery | F3: the kernel-checked bridge from exhaustive enumerated search to adequacy — the exact Lean counterpart of the WO-10 checker |
| `ObservationAdequacy` qualification chain | `QualifiedThrough`, `qualifiedThrough_selected_enabled`, `qualifiedThrough_succ_implies` | Live as generic machinery | Candidate substrate for "gate-open-through-authorize" chaining in F1 |
| `leantmp` Nightshift instances | `Instances.Nightshift.Posture.headline_consumer_relative`, `operator_view_adequate_for_currentness`; `Instances.RuntimeProjection.consumer_relative_adequacy` | Historical as runtime correspondence (grounded in pre-cutover modules); conceptually useful specimens | Patterns for building the F3 instance; not citable against the current runtime |
| Skunkworks `NightshiftOperationalPosture.lean` (pinned commit `5302a092…`) | `OperationalMayRely`, `ProposalApplicableTo`, `different_generation_blocks_proposal_applicability`, `different_dependencies_block_proposal_applicability` | Historical placement (a universal proposal-time gate no longer exists); condition logic survives as a special case | Specialize as one per-workflow predicate instance: `required = {condition.*}` plus a delivery conjunct. The generation/dependency-blocking theorems are structural ancestors of F2's no-silent-refresh target |
| Skunkworks inertness results | `supported_proposal_not_authorized_in_empty_ledger`, `authorization_not_execution_in_empty_ledger`, `empty_authorization_ledger_keeps_proposal_inert` | Structurally still true of the architecture: proposal/admissibility/standing mint no spend | Reusable theorem substrate for F1 inertness lemmas; no source-level runtime correspondence until an adapter exists |

Nothing above is claimed to apply to the current runtime by name alone;
each reuse requires the adapter/instance work described in F1–F4.

## Historical formal work not to resurrect

- The universal `OperationalMayRely` proposal-time gate
  (`observed condition == requiredCondition` for every proposal). Workflow
  condition policy now lives in per-workflow catalog preconditions judged by
  AG at `decide`/`authorize`.
- The claim that the runtime stops before the proposal boundary.
- `CALCULUS-CONFORMANCE.md`'s runtime-side rows (deleted modules).
- Any injectivity framing of normalization adequacy.

## Environmental assumptions

External to the formal kernel — name them as assumptions, never smuggle
them into axioms with vague names:

| Assumption | Suggested name |
|---|---|
| Observation/source evidence honesty | `EvidenceSourceHonest` |
| Observation resolver honesty | `ObservationResolverHonest` |
| Standing authority honesty | `StandingAuthorityHonest` |
| Docket execution-standing authority honesty | `ExecutionStandingHonest` |
| Request-sealer integrity, incl. caller-supplied `evaluated_at` | `RequestSealerIntegrity` |
| Revocation propagation/timeliness within configured TTL bounds | `RevocationTimely` |
| NQ/upstream artifact admission correctness | `UpstreamAdmissionCorrect` |
| SHA-256 collision resistance (digest equality = identity equality) | `DigestCollisionResistant` |
| Deployment identity/TTL configuration correctness | `DeploymentConfigCorrect` |
| External clock/receiver-clock semantics where not modeled | `ClockModelAdequate` |

## Proposed formal objects

A sketch to prevent a blank page — not a mandated representation:

```text
structure GovernedState where
  pc : ProgramCounter            -- ProposalRecorded | AdmissiblePendingAuthorization | AuthorizationConsumed | ...
  proposalWork expectedWork : AgWork
  recordedBasis currentBasis : BasisRef
  observationUsable : Bool       -- abstracting Current vs non-Current
  workflowAllowed : Bool         -- catalog predicate over currentBasis, current policy
  standing : Bool                -- environmental
  policyBasis : PolicyRef        -- identity of the catalog evaluated
  spendCount : Nat

canAuthorize s :=
  s.observationUsable
  ∧ s.currentBasis = s.recordedBasis
  ∧ s.workflowAllowed
  ∧ s.standing
  ∧ s.proposalWork = s.expectedWork

spend_gate_theorem :=
  ∀ s s', Step s authorize s' → producesSpend s' → canAuthorize s

inertness_lemmas :=
  ProposalRecorded ↛ spend   ∧   AdmissiblePendingAuthorization ↛ spend
  ∧ standing alone ↛ spend
```

For F3, instantiate the `ObservationAdequacy` relational system with
`observe := normalize`, the decision/relevance class := the 6561-predicate
family, and the source semantics := the axis-level evaluator; the Lean
certificate corresponds to `findCollision_none_certifies_adequacy_for` over
the same finite enumeration the Rust checker walks.

## Suggested proof order

1. **F1** — small governed authorization state machine; spend-gating and
   inertness.
2. **F2** — expected-work binding, pinned evidence, policy and standing
   provenance; no silent rebinding.
3. **F3** — DecisionBasis adequacy instance over the 15-state /
   6561-predicate domain using the `ObservationAdequacy` machinery.
4. **F4** — supersession/currentness adapter: just enough lineage to
   justify `observationUsable`.

If source inspection reveals a better dependency order, take it — but keep
the program finite. No F5 is justified by the three target families above.

## Executable qualification evidence

Preconditions the formalization models (all green at handoff):

- Nightshift workspace: 174 passed / 0 failed;
- AG workspace: 408 passed / 0 failed;
- full-chain cross-repo integration
  (`crates/nightshiftd/tests/ag_governed_integration.rs`): 8/8, real
  subprocess boundaries;
- Docket process integration: 1/1;
- adequacy certificate
  (`crates/nightshiftd/tests/decision_basis_adequacy.rs`): 6/6, zero unsafe
  collisions over the pinned domain;
- canonical contract frozen: `docs/CANONICAL_RUNTIME_C1.md`.

If a future runtime change invalidates any of these, correspondence must be
re-qualified, not silently retained.

## Correspondence maintenance rule

Formal/runtime correspondence is **versioned evidence, not a permanent
declaration**. A future material runtime cutover must:

1. identify the affected formal correspondences;
2. rerun the executable conformance vectors;
3. update or retire the correspondence records;
4. only then claim theorem applicability to the new runtime.

Do not recreate the situation where frozen Lean evidence silently pointed
at deleted runtime modules.

## Non-goals

Hard constraints on the future Lean tranche:

- do not restore the pre-cutover runtime model;
- do not model a universal Clean proposal gate;
- do not make proposal existence authorization;
- do not make standing an authorization token;
- do not let new evidence refresh an old proposal;
- do not replace decision-relative adequacy with injectivity;
- do not model subject-wide observation supersession;
- do not collapse the Nightshift and AG work-identity domains;
- do not prove external authority truthfulness;
- do not expand into generic workflow-policy research;
- do not change runtime code to make proofs easier without a separate
  architectural decision;
- no liveness/eventual-authorization targets; no open-world completeness;
  no SHA-256 collision proof.

## Stop condition

The next task after this handoff is implementation of the smallest F1
formal model/proof against the frozen C1 contract. No further architecture
qualification is required before beginning that work unless current
executable tests or source inspection contradict this handoff.
