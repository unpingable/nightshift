# GAP: Architectural Promotion Boundary

> Status: candidate / proposed doctrine. Filed 2026-05-05 to capture
> a class of decision Night Shift can detect but must not silently
> resolve — the moment when containment is expiring and the next
> viable move is to ship a roadmapped-but-unratified architecture.
> **No implementation.** No packet schema, no enum, no dispatch code.
> A record is not authorization to build. This spec exists so the
> boundary has a handle for review.

## Why this is named now (YAGNI posture)

The decision class — "containment is running out and the next move is
to promote a roadmapped architecture into the active path" — already
showed up in driftwatch retention pressure (~2026-05, observed on
labelwatch-host: SQLite hot DB approaching disk runway, retention
unable to physically reclaim space, `auto_vacuum=0`, planned
DuckDB/Parquet cold path not yet implemented). Without this boundary
named, the easy local move is for Night Shift to surface "promote the
cold path" as just-another-next-action and hand it to whichever loop
happens to be on the path.

That is exactly the kind of architectural surface where retrofit cost
rises with usage spread. If "is this an ordinary maintenance proposal
or an architectural promotion under pressure" gets answered ad hoc by
each workflow that meets a roadmap item, Night Shift quietly grows a
hole through which incident pressure launders roadmap material into
authorized work.

Naming the candidate is cheap; ratifying it lazily is the discipline.

## Keeper lines

Six load-bearing lines. If the rest of this doc evolves, these stay:

> **A roadmap item is not an authorization.**

> **Incident pressure may accelerate review; it must not replace review.**

> **Containment expiring is evidence for review, not permission to build.**

> **"Expected eventually" does not mean "authorized now."**

> **Night Shift can identify promotion pressure; it cannot ratify architecture.**

> **Night Shift may detect that containment has expired into
> architecture. It may not pretend that architecture is just a larger
> maintenance task.**

Each is unpacked below.

## Problem

When a system is under pressure, planned architecture can appear to
be the obvious next step. This is especially dangerous when the
design was already expected, because it feels less like a new
decision than like cashing in a chip.

Operational urgency changes the decision surface, even for work that
has been on the roadmap for months:

- rollback requirements change (under runway pressure, "we'll roll
  back if it goes wrong" stops being free)
- migration risk changes (a calm migration is not a hot migration)
- data integrity requirements change (acceptance criteria written
  for a planned cutover do not survive a pressured one)
- owner-of-consequence becomes immediate (a future architect's
  decision is now an on-call engineer's decision)
- partial implementation may become load-bearing (a half-built
  cold path under pressure is not the cold path that was designed)

Night Shift must prevent incident pressure from laundering roadmap
material into authorized work. The roadmap entry was a *plan*; it
was not a *standing authorization* to ship under any conditions.

## Three action classes

The same observed pressure can produce three categorically different
proposals. They are not points on a single severity scale; they have
different review surfaces.

### Containment

Bounded, temporary, reversible. Buys time without changing the
system's shape.

```text
delete stale archives
disable retention
gate retention behind a threshold
pause longitudinal rechecks
controlled restart
manual VACUUM if disk allows
```

Night Shift can track containment, schedule its expiry as a
reconciliation check, and surface "containment expiring soon" as a
recheck trigger. Authority requirements depend on the action's
consequence class but follow ordinary deferred-ops rules.

### Tuning

Parameter or bounded-scheduler adjustment. Changes the system's
operating point without changing its architecture.

```text
chunk budget
timeout
gate threshold
retry classification
health surface field
```

Night Shift can recommend tuning, queue review, or — for purely
internal parameters under a declared budget — propose at `advise`
ceiling. Same rules as ordinary remediation work.

### Architectural promotion

A roadmapped but unratified architecture is proposed for promotion
into the active path because containment is expiring. Changes the
shape of the system, not just its operating point.

```text
introduce a new storage substrate (e.g. DuckDB/Parquet cold path)
add a durable ingest spool
split hot/cold stores
add a broker, queue, or durable transport
rebuild the hot DB with a new bounded shape
move a derived export off the primary store
change the hot/cold data boundary
```

This class is the subject of this GAP. Night Shift's correct response
is to emit an **architectural-promotion-required** signal, not an
execution proposal. Promotion is a human decision; Night Shift's
contribution is identifying that the decision has become live.

## Trigger conditions for architectural promotion

The action class is `architectural promotion` when the proposed next
action does any of:

- introduces a new storage substrate, broker, queue, or durable
  transport
- changes the hot/cold data boundary or routing semantics
- changes packet, claim, or admissibility shape
- changes authority, standing, or verification flow
- requires a data migration, rebuild, or schema cutover
- changes rollback shape (rollback path different post-change than
  pre-change)
- creates a new operational dependency (another service must remain
  available for this one to function)
- cannot be validated by existing acceptance criteria, because the
  pre-change criteria assumed the prior architecture
- was previously roadmapped as "later" and is now proposed because
  containment is expiring

The last bullet is the diagnostic one. A roadmap item that was
"later" yesterday and is "now" today, where the only thing that
changed is operational pressure, is the precise signature this GAP
exists to make visible.

## Required behavior

When the trigger conditions are met, Night Shift must emit an
**architectural-promotion-required** signal in place of an execution
proposal. The signal carries:

```text
candidate_architecture       what is being proposed for promotion
pressure                     what is forcing the question now
containment_status           what mitigation is in place, expiring when
runway                       observed time/space/event budget remaining
known_alternatives           other moves on the table (containment,
                             tuning, doing nothing, buying runway)
risk_of_doing_nothing        what fails if no action is taken
risk_of_promoting_early      what is given up by ratifying now
required_human_decision      explicit — this is not a Night Shift
                             call, and is not a pure Governor call
required_inputs_before_act   Standing / Verifier / Governor verdicts
                             that must land before any execution
```

Field names above are sketched, not specified. The point is the
*shape* of the signal: a request for review framed by pressure and
runway, not a proposal framed by execution plan.

The signal is itself a Night Shift artifact (a packet of class
"promotion review request"), not a Governor authorization request.
Routing it onward is a separate decision; see
`GAP-workflow-routing-boundary.md` for claim-type discipline.

## Distinction from GAP-incident-modes

`GAP-incident-modes.md` already names three modes (`incident /
remediation / architecture`) with distinct objectives, allowed
actions, and exit criteria. The mode-`architecture` definition is:

> **Objective**: decide whether the incident exposed a bad assumption.
> **Allowed actions**: amend spec, split concepts, add invariants,
> redefine interfaces, redefine roles or phases.

That is *constitutional* architecture work — re-examining the model
because the incident showed the model was wrong.

This GAP names a different decision: bringing forward an
already-decided architecture under containment pressure, when the
model is *not* in question. The ontology was already correct; what
changed is the timeline.

The two compose:

- A run can be in `incident` mode and still encounter architectural
  promotion pressure (containment expiring during stabilization).
- A run can be in `remediation` mode and propose architectural
  promotion (the fix that was on the roadmap is now the fix being
  shipped).
- A run in `architecture` mode is doing constitutional work; it is
  the natural home for *deciding* whether to ratify a promotion, but
  it is not the place where promotion silently happens.

The new boundary applies across all three incident modes: when the
proposed action falls into the *architectural promotion* class above,
Night Shift surfaces the promotion-required signal regardless of
which mode the run is in.

Mode transition preflight (per `GAP-parallel-ops.md` and
`GAP-incident-modes.md`) still applies independently. Promotion
review is not a substitute for preflight; preflight is not a
substitute for promotion review.

## Distinction from the workflow-routing boundary

`GAP-workflow-routing-boundary.md` (filed earlier today) names how
claims route across tools by competence. This GAP names *what kind of
decision* is being made before routing happens.

```text
workflow-routing boundary       architectural-promotion boundary
  (which tool is competent        (what decision class are we in)
   for this claim)

Direction: across tools         Direction: across decision classes
                                 (containment / tuning / promotion)

Failure mode: NS routes by      Failure mode: NS proposes execution
tool identity, consumes         when it should propose review;
claims it isn't competent       roadmap material gets laundered
for.                             into authorized work under
                                 pressure.
```

They compose: an `architectural-promotion-required` signal is itself
a claim of a particular type (probably a `deferred_obligation` framed
as review-required, not action-required), and routes per the routing
boundary.

## Distinction from the agenda-reconciler trap

The companion memo `feedback_agenda_reconciler_trap.md` warns against
Night Shift growing *upward* into operator pickup truth. This GAP
warns against Night Shift growing *forward* into architectural
ratification under pressure. Different lanes. Both are stay-in-your-
lane doctrines. Neither authorizes the other.

## Worked example (cited, not absorbed)

Driftwatch retention pressure, ~2026-05, observed on labelwatch-host:

```text
Observation
  Hot SQLite DB approaching disk runway. Retention can protect ingest
  but cannot reclaim physical disk space (auto_vacuum=0; DELETE/UPDATE
  controls logical lifecycle, not physical extent). Facts export,
  longitudinal rechecks, and history reads compete with hot ingest
  for the same store.

Pressure
  Disk runway shrinking. Retention is succeeding at its declared job
  and still not solving the underlying pressure.

Candidate architectural promotion
  Promote roadmapped DuckDB/Parquet cold path earlier than planned.

Why this is not ordinary ops
  Changes the hot/cold data boundary. Introduces a new storage
  substrate. Creates a new operational dependency. Pre-existing
  acceptance criteria assumed a calm cutover, not a pressured one.

Correct Night Shift action
  Emit architectural-promotion-required. Surface candidate, pressure,
  containment status, runway, alternatives, and the explicit fact
  that this is a human decision. Do not generate an execution
  proposal. Do not silently advance the cold-path implementation
  because "it was already on the roadmap."

Incorrect Night Shift action
  Treat "promote DuckDB/Parquet" as the next deferred obligation in
  the retention slice and queue it for execution.
```

The example is cited as motivation. Its specific vocabulary (hot DB,
DuckDB/Parquet, retention scheduler, runway thresholds, the specific
SQLite pragma) is not absorbed into Night Shift doctrine. Driftwatch
keeps its own vocabulary; this GAP is generalized over the shape of
the problem, not the shape of that incident.

> A cited example is a witness for the doctrine. Absorbed
> vocabulary is doctrine that has quietly contracted to the witness.

## What this does not authorize

Per YAGNI posture and the no-implementation constraint:

- **No specific architectures.** No DuckDB/Parquet cold path. No
  broker. No durable spool. No database migration. No schema
  changes. The driftwatch shape is illustrative; nothing in this
  doc ratifies any of those moves on any system.
- **No automatic ratification of roadmapped work.** The roadmap is
  not a queue this GAP is draining. Each promotion is a fresh
  decision under current conditions.
- **No code.** No `ActionClass` enum, no promotion-review packet
  schema, no dispatch logic. The three-class taxonomy
  (containment / tuning / architectural promotion) is doctrinal
  vocabulary.
- **No new ledger events.** Run-ledger events stay where they are.
- **No new authority.** This doc does not raise, lower, or reroute
  any tool's authority ceiling. It clarifies what Night Shift may
  *propose* in a particular failure mode; it does not change what
  Night Shift may *do*.
- **No CLAUDE.md invariants added** by filing this. If a promotion
  invariant becomes load-bearing later, it is added when ratified.

A record is not authorization to build.

## Trigger conditions for ratification

Ratify (and consider building the promotion-review packet) when one
of these happens:

1. A second Night Shift surface meets the same decision class —
   containment-expiring-into-architecture — and improvises a local
   answer that doesn't match driftwatch's.
2. A real failing case where Night Shift proposed execution (or
   silently consumed a roadmap item as a deferred obligation) and
   the promotion should have been surfaced for human review.
3. A workflow encounters two of the three action classes
   (containment + tuning, or tuning + promotion) in the same packet
   and needs to type them explicitly to keep them from collapsing.
4. Standing or Governor begins consuming Night Shift signals, and
   "is this a promotion review or an execution request" becomes a
   wire-level distinction the receiver needs.

Until one of those triggers fires, this stays a candidate. The six
keeper lines and the three-class taxonomy are the load-bearing part;
the packet shape is deliberately deferred.

## Vocabulary overlaps with existing Night Shift docs

Called out so we know what is reused vs introduced:

- **`incident_mode`** — `GAP-incident-modes.md` defines `incident /
  remediation / architecture` modes. This GAP's three action classes
  (`containment / tuning / architectural promotion`) are orthogonal
  to mode; the same architectural-promotion class can appear inside
  any of the three modes. No collision; new vocabulary.
- **`change_envelope`** — `GAP-incident-modes.md` defines a pre-
  change declaration with `verify_after` checks. A promoted
  architecture, once ratified by humans, would still author a
  change envelope under that GAP's rules. This GAP is upstream of
  envelope authorship: it answers "is this a maintenance change or
  an architectural promotion" before the envelope is written.
- **`protected` role class** — `GAP-incident-modes.md` defines
  `protected` services that resist casual turn-down. An
  architectural-promotion proposal that touches a `protected`
  service is doubly gated (promotion review + operator confirmation
  for the protected turn-down). The two gates compose; neither
  substitutes for the other.
- **`claim_type`** — `GAP-workflow-routing-boundary.md` introduces
  the claim-type taxonomy. The
  `architectural-promotion-required` signal is a claim of type
  `deferred_obligation` (review-required), routed per that GAP's
  rules. This GAP does not introduce a new claim type.
- **`deferred obligation is not deferred authorization`** —
  `GAP-workflow-routing-boundary.md` keeper. This GAP's "roadmap
  item is not an authorization" is the promotion-shaped twin of
  that line: prior planning state does not survive into current
  execution authority, just as prior captured authority doesn't
  survive a reconciliation gap. Same shape, different gap.
- **`Slice` phases** — `GAP-slice-cycle.md` candidate vocabulary.
  Action class is orthogonal: a slice in any phase may carry any
  action class. The cycle's `Previewed` phase is the natural place
  for promotion-class detection to surface; the action-class
  taxonomy is not itself a phase. No collision.

No vocabulary in this doc renames or replaces existing terminology.
New introductions are: the three action classes (containment /
tuning / architectural promotion), the six keeper lines, the
trigger conditions for the architectural-promotion class, and the
shape of the promotion-required signal.

## Open questions (not load-bearing for the record)

- **Where does the action-class typing live when it ratifies?**
  In the packet header, the bundle, or as a derived classification
  on the proposed-actions list? Probably the latter — derived from
  trigger conditions, not declared by the workflow.
- **Boundary between tuning and architectural promotion.**
  "Increase chunk budget" is tuning. "Add a chunk-budget feedback
  loop with state" is closer to promotion. The boundary is the
  trigger-condition list; edge cases will need adjudication when
  they appear.
- **Promotion-required signal lifecycle.** Is the signal a one-shot
  artifact emitted into the run ledger, or a durable open state
  that survives until human decision lands? Probably the latter
  (ack-with-TTL per `GAP-attention-state.md`), but defer until a
  real case forces it.
- **Cross-run promotion accumulation.** Several runs each surface
  promotion pressure for the same candidate architecture. Does
  Night Shift coalesce these, or does each run emit independently?
  Probably coalesce-by-candidate, but the de-duplication key shape
  is deferred.
- **Interaction with `--no-governor` degraded mode.** A degraded
  run already lowers promotion ceiling per
  `GAP-governor-contract.md`. Does it also automatically
  disqualify architectural-promotion signals, or are these
  exactly the signals a degraded run *should* still surface? The
  GAP's posture is: surfacing review pressure does not require
  Governor; ratifying promotion does. So degraded mode keeps the
  detection and loses the advancement. Confirm when ratified.
