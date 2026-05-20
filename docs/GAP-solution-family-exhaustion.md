# GAP: Solution-Family Exhaustion

> Status: candidate / proposed doctrine. Filed 2026-05-05 to capture
> a class of evidence Night Shift can detect but must not silently
> resolve — the moment when repeated mitigations inside the same
> architectural premise have become evidence against the premise
> itself. **No implementation.** No event class, no mitigation-chain
> ledger, no dispatch code. A record is not authorization to build.
> This spec exists so the boundary has a handle for review.

## Why this is named now (YAGNI posture)

The pattern surfaced unmistakably in the driftwatch SQLite mitigation
chain (~2026, observed across multiple slices on labelwatch-host):
batched writer → rollback accounting → writer-owned WAL truncate →
retention-through-writer → legacy retention scheduler → chunk-budget
tuning → kernel I/O wait → DELETE/UPDATE retention. Each fix solved
the visible symptom. Each fix preserved the same architectural
premise — *one large SQLite store can serve hot ingest, cold history,
retention, longitudinal reads, facts export, and archive lifecycle if
the scheduler is tuned enough.* The system answered each fix with a
new failure bucket.

Without naming exhaustion as a class, the easy local move is for
Night Shift to surface "try the next tuning knob" as just-another-
deferred-obligation. The mitigation chain becomes invisible. The
premise stays unnamed. The frog is in the kettle, holding a
sophisticated thermometer.

This is the architectural surface where retrofit cost rises with
usage spread. If "is this an ordinary follow-up or evidence of
exhaustion" gets answered ad hoc by each workflow that meets a
repeated-failure pattern, Night Shift quietly grows a hole through
which mitigation chains launder themselves into perpetual tuning.

Naming the candidate is cheap; ratifying it lazily is the discipline.

## Keeper lines

Five load-bearing lines. If the rest of this doc evolves, these stay:

> **Repeated repair inside the same context is evidence against the context.**

> **Failure bucket migration is not recovery.**

> **A mitigation chain can become evidence.**

> **Solution-family exhaustion is an escalation signal, not an implementation plan.**

> **Night Shift can detect exhaustion; it cannot resolve exhaustion by silently changing architecture.**

Each is unpacked below.

## Problem

Operational systems often fail by changing failure buckets. A
mitigation may eliminate the visible symptom while preserving the
architectural premise that produced the symptom in the first place.
The premise is not visible in any single failure; it is visible only
in the *shape of the chain*.

Examples of bucket migration inside a preserved premise:

```text
queue overflow         → rollback loss
rollback loss          → writer starvation
writer starvation      → I/O wait
retention success      → physical-reclaim gap
WAL bloat              → DB file growth
chunk-budget tuning    → throughput / accuracy tradeoff
```

Each transition reads as progress when viewed locally. Read as a
chain, the same architectural commitment is still on the table; the
system is steadily explaining that the commitment is the actual
problem.

The keeper "more knobs are not always more control" lives here. Each
knob added to defend a stale premise narrows the operating envelope
while widening the surface that needs defending. Past a point, knob
addition is itself a symptom.

If Night Shift treats every next-tweak proposal as ordinary deferred
work, it actively participates in this failure mode. The agent's job
in this case is not to propose the next tweak; it is to *stop the
operator from boiling the frog with increasingly sophisticated knobs.*

## Trigger conditions

Classify a context as **solution-family exhausted** when several of
these are jointly true:

- multiple mitigations have been attempted in the same context
  (workflow, finding family, subject, or architectural seam)
- mitigations targeted different *symptoms* but preserved the same
  *premise* — same storage substrate, same boundary, same actor
  shape, same authority model
- each mitigation either failed, shifted the failure to a new
  bucket, degraded another axis, or only bought temporary runway
- failure buckets shifted *across* the chain rather than failures
  disappearing
- acceptance criteria for the most recent mitigations have become
  increasingly local, defensive, or instrumentation-shaped (rather
  than user-facing)
- the next plausible *durable* fix would change a boundary, a
  substrate, an authority shape, or a workload-class assumption —
  i.e. it falls into the architectural-promotion class per
  `GAP-architectural-promotion-boundary.md`
- continued same-family tuning is starting to feel absurd — adding
  knobs to defend the previous knob, or naming defects of defects

The last bullet is operator-shaped, not formal. It is included
deliberately. Exhaustion is partly a felt sense, and a doctrine that
pretends otherwise will miss the cases it most needs to catch.

Single-mitigation pressure is *not* exhaustion. One fix that didn't
work is a debugging signal. A *chain* of fixes that preserved the
premise is the trigger. The rough threshold is two or more
bucket-migrating mitigations in the same context, plus the next
viable durable fix being architectural; the precise threshold is
deferred until a real case forces it.

## Required Night Shift behavior

When the trigger conditions are met, Night Shift must emit a
**solution-family-exhausted** signal in place of an execution
proposal. The signal carries (field names sketched, not specified):

```text
solution_family            short identifier for the exhausted context
shared_premise             plain-language statement of what every
                           prior mitigation preserved
mitigations_tried          chain of prior mitigations, in order
failure_bucket_shifts      observed sequence of bucket migrations
containment_status         what mitigation is currently in place,
                           expiring when
runway                     observed budget remaining if exhaustion
                           is not addressed
candidate_boundary_changes one or more named architectural moves
                           that *would* change the premise; not a
                           recommendation, an enumeration
risk_of_continuing         what fails if same-family tuning continues
risk_of_premise_change     what is given up if the premise is
                           abandoned
may_execute                false
requires_human_decision    true
```

`may_execute: false` is the load-bearing field. The signal is an
**escalation event**, not a proposal Governor can authorize. It
exists to make the chain visible and to halt the auto-generation of
"next tweak" tasks inside the exhausted family.

The signal is itself a Night Shift artifact (a packet of class
"exhaustion review request"), not a Governor authorization request
and not an execution plan. Routing it onward follows the claim-type
discipline in `GAP-workflow-routing-boundary.md`.

## Operational impact when exhaustion triggers

When a context is classified as solution-family exhausted, Night
Shift's behavior in that context changes — not by doing more, but by
doing less of a specific kind of thing:

1. **Stop creating ordinary "next tweak" tasks by default.** No more
   "try another timeout," "bump chunk budget," "one more restart"
   inside the exhausted family — *unless* the proposed action is
   explicitly classified as containment per
   `GAP-architectural-promotion-boundary.md`. Containment proposals
   continue. Tuning and architectural promotion both pause behind
   the exhaustion review.

2. **Name the shared premise.** The signal must name the premise
   plainly, not gesture at it. "Single SQLite shared substrate."
   "Manual Continuity update discipline." "Single packet can carry
   mixed authority." The premise is the thing under suspicion;
   leaving it unnamed is the failure mode.

3. **List same-family mitigations as a chain, not a log.** Not
   every retry. The mitigation *chain* — what was tried, in what
   order, and what bucket it shifted to. The chain is the evidence;
   the per-event log is not.

4. **Separate containment from durable fix.** Containment work
   inside the exhausted family is still legitimate — keep ingest
   alive, buy disk runway, hold telemetry up. Durable fix work
   inside the exhausted family is suspect; that's the whole point.
   The signal must show this split explicitly so the operator does
   not have to reconstruct it.

5. **Hand the human a decision, not a plan.** Candidate boundary
   changes are *enumerated*, not recommended. Night Shift's
   competence here is detection and packaging, not architecture
   selection.

These are the operational shape of "Night Shift can detect
exhaustion; it cannot resolve exhaustion by silently changing
architecture."

## Distinction from GAP-architectural-promotion-boundary

The two GAPs name adjacent failure modes and compose:

```text
solution-family exhaustion       architectural-promotion boundary

What kind of decision are we     What class is this proposed action
in given the *history* of        in (containment / tuning /
proposals?                       architectural promotion)?

Diagnostic: have repeated        Diagnostic: was this "later"
same-premise fixes become        yesterday and "now" today, with
evidence against the premise?    only operational pressure changed?

Direction: backward-looking      Direction: forward-looking
(read the chain)                 (classify the current proposal)
```

Exhaustion *can* trigger promotion review:

```text
solution_family_exhausted
  → shared premise named, candidates enumerated
  → human decides: change the boundary?
  → if yes, a specific architectural move is selected
  → that move enters the architectural-promotion-boundary path
  → emit architectural-promotion-required for the chosen candidate
  → human ratification, then ordinary execution under Governor
```

But exhaustion does *not* always trigger promotion. The right answer
to an exhausted family may be:

- buy runway and revisit (a non-architectural escape valve)
- accept the failure mode and stop investing (declared end-of-life)
- restructure the workload upstream so the family is no longer
  load-bearing
- do nothing and document the exhaustion, with the chain frozen as
  durable evidence

These are human decisions Night Shift surfaces, not paths Night
Shift selects.

## Distinction from GAP-workflow-routing-boundary

`GAP-workflow-routing-boundary.md` names *which tool is competent for
which claim*. This GAP names *what kind of claim is in front of us*
based on chain shape. They compose: a `solution-family-exhausted`
signal is a claim type — most naturally a `deferred_obligation`
flavored as review-required, with `may_execute: false` — and routes
per the routing boundary's rules. This GAP does not introduce a new
claim type to that taxonomy; it identifies a particular *trigger* for
an existing one.

The keeper "deferred obligation is not deferred authorization"
(routing-boundary) and the keeper "solution-family exhaustion is an
escalation signal, not an implementation plan" (this GAP) are the
same shape applied at different altitudes. Routing-boundary forbids
silently promoting a remembered obligation into an authorized
action. This GAP forbids silently promoting a chain of mitigations
into a license to keep tuning.

## Distinction from the agenda-reconciler trap

`feedback_agenda_reconciler_trap.md` warns against Night Shift
growing *upward* into operator pickup truth — agendas, next-actions,
the operator's PM substrate. Exhaustion detection is *not* that
trap, but it gets close enough to be worth flagging.

Exhaustion reads Night Shift's *own* mitigation history in a
declared context (workflow, finding family, subject), not the
operator's broader work state. Different surface. The premise being
named is the *system's* architectural commitment, not the
*operator's* prioritization.

The line stays bright if Night Shift restricts exhaustion detection
to chains it has receipts for in its own ledger, on findings or
agendas already in scope. The trap returns if Night Shift starts
asking "is the *operator* exhausted on this problem family?" — that
is operator pickup truth, and it requires the same failing-case
discipline the trap memo names.

## Worked example (cited, not absorbed)

Driftwatch retention slice, ~2026, observed on labelwatch-host:

```text
Shared premise (under suspicion)
  A single large SQLite store can serve hot ingest, cold history,
  retention, longitudinal rechecks, facts export, health, and archive
  lifecycle if the scheduler is tuned carefully enough.

Mitigation chain
  1. Batched writer / persistent connection — addressed queue
     pressure; rollback losses then surfaced.
  2. rollback_lost accounting — exposed lock-conflict loss;
     visibility improved, the loss did not.
  3. Writer-owned WAL truncate — bounded WAL growth; retention
     contention surfaced behind it.
  4. Retention-through-writer — removed lock conflict; ingest
     starvation appeared.
  5. Legacy retention scheduler — protected ingest; reclaim
     progress became partial.
  6. stream_lag and chunk-budget tuning — improved classification;
     disk runway pressure remained.
  7. Kernel I/O wait observation — storage-layer contention
     surfaced as the ceiling.
  8. DELETE/UPDATE retention with auto_vacuum=0 — physical reclaim
     still requires VACUUM/rebuild.

Failure-bucket trace
  queue overflow → rollback loss → writer starvation → WAL pressure →
  ingest starvation → reclaim gap → I/O wait → physical-extent gap

Conclusion
  The SQLite tuning family is not exhausted because SQLite is bad.
  It is exhausted because the hot/cold workload boundary is wrong.
  The premise — one store, all workloads — kept losing in different
  rooms.

Correct Night Shift action
  Emit solution-family-exhausted with the chain, the premise, and
  enumerated candidate boundary changes (hot/cold split, durable
  spool, DuckDB/Parquet cold path, workload isolation, runway buy,
  EoL of one workload). may_execute: false. Stop generating "next
  tuning knob" tasks against this family by default.

Incorrect Night Shift action
  Schedule the next tuning knob as an ordinary deferred obligation
  and let the chain continue.
```

The example is cited as motivation. Its specific vocabulary (SQLite,
WAL, auto_vacuum, DuckDB/Parquet, chunk budget, the specific
mitigation order) is **not absorbed** into Night Shift doctrine.
Driftwatch keeps its own vocabulary; this GAP is generalized over
the *shape* of mitigation-chain exhaustion, not over that incident.

> A cited example is a witness for the doctrine. Absorbed
> vocabulary is doctrine that has quietly contracted to the witness.

## What this does not authorize

Per YAGNI posture and the no-implementation constraint:

- **No event class shipped.** The `solution_family_exhausted`
  shape sketched above is doctrinal vocabulary, not a wire format.
  No enum, no JSON schema, no packet field.
- **No mitigation-chain ledger.** The "chain of prior mitigations,
  in order" requires durable history of past Night Shift proposals
  inside a context. This GAP does not authorize building that
  ledger. If chain history needs a home, that is its own GAP,
  triggered by a real failing case for chain reconstruction.
- **No specific architectures.** Cold paths, brokers, durable
  spools, hot/cold splits, workload-class boundaries — none of
  these are ratified by this doc. Candidate enumeration is *part
  of the signal*, not part of doctrine.
- **No automatic suppression of follow-up tasks.** The "stop
  creating ordinary next-tweak tasks" behavior is required-on-
  ratification, not required-on-filing. Until ratified, Night
  Shift behaves as it does today; the doc is a handle for review.
- **No new authority.** This doc does not raise, lower, or reroute
  any tool's authority ceiling. It clarifies what Night Shift may
  *propose* and *refuse to propose* in a particular evidence
  pattern.
- **No CLAUDE.md invariants added** by filing this. If an
  exhaustion invariant becomes load-bearing later, it is added
  when ratified.

A record is not authorization to build.

## Trigger conditions for ratification

Ratify (and consider building the exhaustion-review packet) when one
of these happens:

1. A real failing case where Night Shift proposed the next tuning
   knob inside an exhausted family, and the chain *should* have
   been surfaced as evidence against the premise.
2. A second context (beyond driftwatch retention) hits the same
   pattern and improvises a local answer that doesn't match — i.e.
   parallel exhaustion taxonomies start forming.
3. Operator workflow asks "what mitigations have we tried for this
   family" and Night Shift has no first-class answer — the
   mitigation-chain ledger gap becomes load-bearing.
4. An architectural-promotion-required signal lands without an
   exhaustion signal upstream, and the missing chain visibility
   makes the promotion review under-evidenced.

Until one of those triggers fires, this stays a candidate. The five
keeper lines and the trigger-condition list are the load-bearing
part; the signal shape is deliberately deferred.

## Vocabulary overlaps with existing Night Shift docs

Called out so we know what is reused vs introduced:

- **`containment` / `tuning` / `architectural promotion`** —
  `GAP-architectural-promotion-boundary.md` action classes.
  Exhaustion uses these classes directly: containment continues
  inside an exhausted family; tuning and promotion both pause
  behind the exhaustion review. No collision; reused vocabulary.
- **`may_execute: false`** — new field name in this doc, but the
  semantic appears in `GAP-workflow-routing-boundary.md`'s "deferred
  obligation is not deferred authorization" and in
  `GAP-incident-modes.md`'s "deployed ≠ verified." Same shape; an
  artifact can carry information without carrying permission to act
  on it. If the field name ever ships, alignment with both prior
  docs is required.
- **`incident_mode`** — `GAP-incident-modes.md` distinguishes
  incident / remediation / architecture modes. Exhaustion is
  orthogonal: a chain inside any mode can become evidence against
  its premise. No collision.
- **`claim_type`** — `GAP-workflow-routing-boundary.md` claim-type
  taxonomy. The exhaustion signal routes as a `deferred_obligation`
  (review-required), not as a new claim type.
- **`reconciliation_check`** — `GAP-workflow-routing-boundary.md`
  later-check taxonomy. An exhaustion review is *not* a
  reconciliation check — it is a meta-claim about the *pattern* of
  prior reconciliation checks. Worth distinguishing so the
  reconciler doesn't grow an "exhaustion check" alongside its
  ordinary recheck queue.
- **Driftwatch worked example** — also cited (not absorbed) in
  `GAP-architectural-promotion-boundary.md` and
  `GAP-workflow-routing-boundary.md`. Three GAPs filed within ~5
  days all draw motivation from the same incident shape; the cited-
  not-absorbed practice is what keeps Night Shift doctrine
  generalizable instead of contracting to driftwatch's vocabulary.

No vocabulary in this doc renames or replaces existing terminology.
New introductions are: solution-family / shared-premise framing,
mitigation-chain framing, failure-bucket migration framing, the five
keeper lines, and the `may_execute: false` field on review-shaped
signals.

## Open questions (not load-bearing for the record)

- **Where does mitigation-chain history live?** The signal requires
  reconstructing prior mitigations inside a context. Night Shift's
  current run ledger captures runs, not mitigation-chain semantics.
  Probably a derived view keyed on (workflow, subject) with
  proposal-class tagging; defer until ratification.
- **Context scoping.** "Same context" is intentionally fuzzy
  (workflow, finding family, subject, architectural seam). When
  ratified, one of these will become the canonical chain key, or a
  composite. Defer.
- **De-duplication across runs and operators.** Two runs in two
  sessions each detect exhaustion in the same family. Does Night
  Shift coalesce, suppress one, or emit both with cross-references?
  Likely coalesce-by-family with operator-visible chain. Defer.
- **Exhaustion expiry.** Once an exhaustion signal is emitted and
  the chain is named, does the signal expire? On what — a human
  decision landing, runway extending, the premise being explicitly
  reaffirmed? Probably operator decision lands a typed disposition
  (per `GAP-reack-doctrine.md`) that closes the signal; defer until
  a real case.
- **Interaction with `--no-governor` degraded mode.** Per
  `GAP-governor-contract.md`, degraded mode lowers promotion
  ceiling. Exhaustion detection is observational; the doc's posture
  is that detection survives degraded mode (it is exactly the
  signal a degraded run *should* still surface) while ratification
  of any architectural response does not. Confirm when ratified.
- **Felt-sense calibration.** The "starting to feel absurd"
  trigger is operator-shaped. If Night Shift becomes the thing that
  surfaces absurdity, it must do so without becoming the thing that
  *adjudicates* absurdity. Boundary needs articulation when a real
  case forces a precise rule.
