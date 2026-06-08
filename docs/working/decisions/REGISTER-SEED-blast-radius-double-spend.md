# Register seed: blast-radius / deployment-budget double-spend

**Filed:** 2026-06-03
**Status:** register seed. Not a GAP, not a theorem, not a build plan;
no Lean specimen yet, no NQ implementation yet, no NS code change.

**Premise.** Disciplined premise production has two species:

- **Source / temporal.** Blocks self-originating authority,
  retroactivity, stale or revoked basis, revoked standing, missing
  independent premise.
- **Multiplicity / resource.** Blocks duplicated use of a valid
  premise or resource.

`ContractionHinge`
(`~/git/lean/LeanProofs/Admissibility/ContractionHinge.lean`) is the
first known multiplicity / resource specimen. This note is a
register seed for a candidate second specimen drawn from real-world
deployment-at-scale failures.

---

## 1. Specimen name candidates

- `BlastRadiusDoubleSpend`
- `SharedSafetyBudgetDoubleSpend`
- `DeploymentBudgetDoubleSpend`
- `ConcurrentRolloutBudgetReuse`

Working handle for the rest of this note: **BlastRadiusDoubleSpend**.

## 2. Failure shape

A deployment system enforces a shared safety budget — for example:
no more than N hosts may mutate concurrently; no more than N racks /
cells / POPs may be in rollout; no more than N risky operations may
consume the same maintenance window or blast-radius budget.

Multiple admissible actors or jobs observe the same valid
remaining-budget reading and each treats the reading as reusable
authority to act. The premise — "budget N, currently k consumed,
headroom N − k" — is valid, fresh, and correctly sourced. The
failure is not bad data. The failure is duplicated use of a single
observation: each consumer treats `N − k` as their own headroom and
acts independently. Total consumption exceeds capacity. The shared
budget has been double-spent.

## 3. Why this is multiplicity / resource, not source / temporal

- The premise is not self-originating. The budget reading comes from
  real authority the consumer is allowed to read.
- The premise is not retroactive. The reading is anchored to the
  pre-state of the consumer's intended mutation.
- The premise is not stale. The reading is fresh by any
  source / temporal measure available at the consumer.
- Standing is not the issue. Each consumer has standing to read and
  to act within the budget.

The failure is that the same fresh, properly sourced premise gets
reused by N parallel consumers, each consuming as if alone. The
discipline that blocks this is uniqueness-of-use at the resource
boundary — not freshness-of-basis or source-validity at the
observation boundary.

## 4. Allowed shape

- A budget / reservation / token / lease is consumed once per
  mutation.
- Total mutation across all admitted consumers remains within
  capacity.

## 5. Blocked shape

- The same observed budget authorizes multiple independent
  mutations.
- Total consumption exceeds the premise's capacity.

## 6. Fix family

Members of the same fix family; this seed does not pick among them.

- Reservation.
- Lease.
- Token consumption.
- Accounting.
- Uniqueness of use.
- Atomic claim / check / use boundary.

## 7. Relation to ContractionHinge

**Same species.** Refusal of a single warrant occurrence being
reused as multiple distinct warrants.

**Different substrate / application altitude.**

- `ContractionHinge` is the structural-rule kernel:
  `¬ Derivable [A] (A ⊗ A)`. One warrant cannot be admitted as two.
- `BlastRadiusDoubleSpend` is the operational deployment-scale form.
  One observation of shared headroom cannot be consumed as multiple
  admissions to act.

Kin shapes; not the same kernel. ContractionHinge denies contraction
at the proof-theoretic layer. BlastRadiusDoubleSpend denies it at
the operational coordination layer, where the warrant is a budget
reading and the contraction is concurrent reuse.

## 8. Non-goals

- No Lean specimen yet.
- No NQ implementation yet.
- No new taxonomy. The source / temporal × multiplicity / resource
  split stands; this seed does not add a third species or refine the
  existing two.
- No source / temporal rewrite. Existing source / temporal-discipline
  registers stay structurally untouched. In particular: do not
  introduce `Int` / count / `≤` into unrelated source / temporal
  registers because this seed mentions capacity arithmetic.

## 9. Promotion gate

This seed converts to a register entry, a sibling Lean specimen, or
an implementation consumer only when one of the following lands:

- A second concrete operational example beyond the deployment-budget
  shape.
- A failing test, at any layer, that names a real consumer of the
  discipline.
- An implementation consumer asking for the shape — for example, a
  Governor reservation primitive, an NQ coordination claim, or an NS
  run-ownership invariant whose proof shape this would absorb.

Until one of those, this entry holds the candidate.

## Hard constraints (carried forward)

- Do not generalize into all deployment safety.
- Do not open implementation work from this seed.
- Do not introduce `Int` / count / `≤` into unrelated
  source / temporal registers.
- Keep this parked as a register seed.
