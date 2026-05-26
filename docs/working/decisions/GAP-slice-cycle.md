# GAP: Slice Cycle (work-cycle cadence)

> Status: candidate / non-binding. Filed 2026-04-30 to capture an
> architectural surface — the cadence under which Night Shift drives
> bounded work units forward — before terminology and enum shape get
> retrofitted across modules. **Do not implement the enum yet.** This
> spec exists so the candidate has a handle for review.

## Why this is named now (YAGNI posture)

Phase vocabulary is exactly the kind of cross-module surface where
retrofit cost rises with usage spread. If "what phase is the work in"
gets answered ad hoc by capture verbs, run states, agenda transitions,
and operator notes — each invented locally — Night Shift quietly grows
four parallel phase machines that almost agree.

That is the failure mode this record is intended to prevent. Naming
the candidate is cheap; ratifying it lazily is the discipline.

## Keeper lines

> **Night Shift preserves cadence. Governor preserves admissibility.**

Or, sharper, from the framing thread:

> **Night Shift decides what phase the work is in. Governor decides
> whether the next transition is allowed.**

These two lines are the load-bearing pieces of this gap. If the enum
shape changes, these stay.

## Load-bearing invariant

> **No phase transition may skip required validation when the next
> phase can mutate state, create reliance, or authorize future work.**

This is the actual point of the cycle: make skipping the boring safety
transitions visible. Most phases are not metaphysically profound; they
exist so that "looks good, ship it" cannot become a religious
experience.

The invariant is symmetric across modes:

- **Build mode** (proposing changes from agendas, slice-derived work).
- **Incident mode** (proposing remediation in response to NQ findings,
  operator pages, or escalation packets).

Both modes pass through the same cadence, even though the entry point
and termination shape differ. The cadence is mode-agnostic; the
admissibility of any given transition is mode-shaped (see
`GAP-incident-modes.md`).

## The candidate cycle

A `Slice` is a bounded unit of intent — the smallest work unit whose
intended effect can be reviewed, executed, and verified as a whole.
Today this is closest to "one agenda's run," but the slice concept is
explicitly broader: a slice can span multiple runs (capture +
reconcile is already two-phase) and can derive successor slices.

The candidate phases:

```text
1.  Proposed                  smallest work unit and intended effect declared
2.  Previewed                 architecture / doctrine review before mutation
3.  Replanned                 scope adjusted based on preview findings
4.  PreconditionsValidated    standing, evidence, freshness, conflicts, repo state
5.  Executed                  bounded mutation performed
6.  QAReviewed                tests, checks, NQ witnesses, diff review
7.  Refined                   defects fixed without widening scope
8.  Revalidated               refinement did not change admissibility basis
9.  Completed                 outcome, receipts, decisions, open gaps recorded
10. NextSliceDerived          forced edge identified and named
```

Governor is invoked at specific gates — not at every phase:

```text
Previewed              → is the proposed slice admissible?
PreconditionsValidated → are preconditions satisfied?
Executed               → is this action authorized?
Completed              → is the result receipted and reliance-safe?
NextSliceDerived       → is the forced edge real or decorative?
```

Phases between gates are Night Shift's bookkeeping. The gates are
where authority is consulted.

## Reconciliation against the lifecycle ladder (open question)

Invariant 5 already names a 7-phase **lifecycle ladder** for runs:

```text
capture → reconcile → plan → review → run → verify → record
```

The Slice Cycle is richer (Propose / Preview / Replan upstream of
capture; Refine / Revalidate as within-run iteration; NextSliceDerived
as a successor seed). It is also potentially broader in scope: the
lifecycle ladder is *per run*, the Slice Cycle is *per unit of
intent*, and a slice may span multiple runs.

Three plausible reconciliations. Pick one before any code learns
either vocabulary:

### Option A — Slice Cycle refines the lifecycle ladder

Treat lifecycle phases as coarse partitions and slice phases as their
internal structure:

```text
capture     ⊇ Proposed, Previewed, Replanned
reconcile   ⊇ PreconditionsValidated
plan        ⊇ (workflow internal)
review      ⊇ (Governor gate, no slice phase)
run         ⊇ Executed
verify      ⊇ QAReviewed, Refined, Revalidated
record      ⊇ Completed, NextSliceDerived
```

Pro: keeps the existing 7-phase ladder canonical; slice phases become
sub-states. No vocabulary collision; one ladder, one refinement.
Con: forces "Refine" and "Revalidate" inside a single `verify` phase,
which is the within-run iteration loop the framing wants to make
*visible*. Risks burying the safety transitions the invariant was
meant to surface.

### Option B — Slice Cycle is orthogonal to the lifecycle ladder

Treat them as different axes. Lifecycle = where this run is. Slice
phase = where this unit of intent is, across however many runs it
spans.

Pro: a slice can span multiple runs (capture run, reconcile run,
followup remediation run). Refine / Revalidate / NextSliceDerived are
genuinely cross-run and don't fit cleanly inside one run's lifecycle.
Con: now there are two phase machines, and the project already warns
against terminology drift across the three ladders. Adding a fourth
axis (lifecycle, authority, artifact, *slice*) is a real cost.

### Option C — Slice Cycle replaces the lifecycle ladder

Promote slice phases to be *the* phase vocabulary. Recast the
7-phase lifecycle ladder as a projection of the slice cycle onto a
single run.

Pro: one ladder. The richer one. Captures the framing thread's
intent directly.
Con: invasive. Invariant 5, multiple GAPs, and existing run ledger
events (`run_captured`, `run_reconciled`, `run_completed`) all carry
the lifecycle vocabulary. Replacement is a doctrine refactor, not a
local change.

**Default recommendation, pending review**: Option B, with the
explicit constraint that the Slice Cycle is *not* recorded in the run
ledger. The run ledger continues to use lifecycle phases. Slice
phases live one level up — in the agenda / work-cycle layer that
sequences runs. This keeps the ledger boring and the cadence
expressive.

But this is the load-bearing question. It should be answered
deliberately, not accreted.

## What this does not authorize

- **No enum yet.** No `SlicePhase` type in any crate. No phase column
  in any table. No CLI verb that names a slice phase.
- **No new authority.** The cycle is bookkeeping; it does not raise,
  lower, or reroute promotion ceilings.
- **No new ledger events.** The run ledger is not the slice ledger.
  If a slice ledger ever exists, that is its own GAP.
- **No new schema.** Agenda, bundle, packet schemas are unchanged by
  this record.

A record is not authorization to build.

## Trigger conditions for ratification

Ratify (and pick an option above) when one of these happens:

1. A second module starts inventing a parallel phase vocabulary
   (e.g. agenda transitions that don't map to lifecycle phases).
2. Cross-run iteration ("refine and re-run this slice") becomes a
   real workflow rather than a one-off.
3. Forced-edge derivation ("what's the next slice this exposed?")
   needs to be a first-class output rather than an operator
   afterthought.
4. Incident-mode and build-mode work-cycles diverge enough that a
   shared cadence vocabulary would prevent drift.

Until one of those triggers fires, this stays a candidate.

## Open questions (not load-bearing for the record itself)

- **Slice identity vs run identity.** If a slice spans runs, what
  carries the slice id? Agenda is too coarse; run is too fine.
- **Forced-edge representation.** Is `NextSliceDerived` an artifact
  (a packet field), a ledger event, or just operator notes?
- **Refine within authority ceiling.** Refine implies within-run
  iteration that re-touches the system. Does each refine pass
  re-consult Governor, or does the original `apply` authorization
  cover bounded follow-up under a declared budget?
- **Preview vs review.** "Preview" (phase 2) and "review" (lifecycle
  phase) sound similar but mean different things — one is
  doctrine/architecture review before mutation, the other is the
  Governor authority gate. If the cycle survives, these names need
  to stop colliding.

## Why this isn't blocking

The current slice-of-work (deferred capture/reconcile split, NQ
liveness consumer, tolerability horizon) does not need the Slice
Cycle to land. The cycle becomes load-bearing only when the second
parallel phase vocabulary appears, or when cross-run iteration stops
being one-off. Filing now buys cheap retrofit insurance; building now
would be speculative expansion.
