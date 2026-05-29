# GAP: Temporal Obligation Tracking — NS-Side Consumer Design

> **Status:** working GAP. Filed 2026-05-29 by promotion from
> [`../decisions/temporal-obligation-tracking.md`](../decisions/temporal-obligation-tracking.md)
> after NQ minted the upstream witness grammar
> ([`workload_phase_observation.v0`](../../../../notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md))
> and filed the candidate severity-axis decomposition
> ([`PRESSURE_HARM_LOSS_RECOVERABILITY_GAP`](../../../../notquery/docs/working/gaps/PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md))
> on 2026-05-28 / 2026-05-29. The working note's promotion condition
> (1) fired the same day it was written. The note is superseded by
> this GAP; supersession header filed in the note for provenance.
> **No implementation authorized.** **Not authorization to implement
> Path B or proxy-shock recognition** — Path B remains held by
> `~/git/cartography/coordination/POST_MVP_A_CHECKPOINT.md` step 5.

## The deep rule

> **NQ owns the witness grammar. Night Shift watches how those
> observations evolve across time. Workload-phase witnesses may
> qualify NS posture, attention, or urgency; they MUST NOT mint
> action permission. Temporal observation is evidence, not
> authority.**

This is the actuation-boundary doctrine + the NQ-owns-truth /
NS-owns-posture channel split + the
`ok_to_proceed ≠ authorization summary` sentinel composing at a new
layer. Each of the three has already drawn the same line. The
temporal axis just gives that line a new place to be crossed.

Three boundary statements survive distillation; each is the negation
of a specific laundering vector this GAP exists to prevent:

> **Workload-phase witnesses may qualify posture; they do not
> qualify standing.**

> **A trend, a regime shift, or a recoverability-window-closing
> observation may raise NS urgency. It must not raise NS authority.**

> **NS may notice that the kind of work has changed. NS may not
> decide that the change authorizes the next action.**

## Why this is now a GAP

The working note at [`../decisions/temporal-obligation-tracking.md`](../decisions/temporal-obligation-tracking.md)
named three promotion conditions on 2026-05-29:

1. NQ mints `workload_phase_observation.v0` (or its eventual name)
   and NS has a concrete consumer to design.
2. A second labelwatch-shape soak fires in a different ops domain
   and surfaces a temporal-interpretation gap that existing GAPs
   cannot carry.
3. An operator review identifies a real packet / ledger / peek
   surface where NS could expose "this window vs. last window"
   without an enum commitment.

Condition (1) is **partially** fired by the close of business
2026-05-29:

- **NQ minted the grammar.** `~/git/notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md`
  (v0, 2026-05-28) names `workload_phase_observation.v0` exactly as
  the note anticipated, with the keeper *"NQ owns the grammar.
  Apps own the phase map."* and the disciplinary line
  *"A workload-phase witness describes one observed window. It is
  not absolution."*
- **NQ split the severity axes.** `~/git/notquery/docs/working/gaps/PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md`
  (candidate, 2026-05-29, forcing case = labelwatch Day-5) separates
  pressure / harm / loss / recoverability as four distinct evidentiary
  axes — the same decomposition the note's overlap-mapping table
  cited as living only at Governor's tolerability horizon. NQ is now
  the second authoritative home for the axis decomposition.
- **NS has not designed a consumer.** This is the unfired clause of
  condition (1). The note's "Anti-collapse boundaries" section
  remains accurate; what is now missing is a positive description of
  what NS *does* consume, where the consumed observation lands in
  NS's surfaces, and which laundering vectors the consumer design
  must refuse.

The note's promotion condition fired the same day it was authored.
This GAP is the response, not new doctrine.

## What NQ has filed (the upstream substrate)

The packet spine NS would consume:

```json
{
  "packet_type": "workload_phase_observation.v0",
  "system": "<app>",
  "component": "<subsystem or null>",
  "phase": "<named phase>",
  "role": "<read | write | mixed | external_call | gate_decision | operator_surface | other>",
  "priority": "<hot_ingest | derived | operator_surface | retention | export | other>",
  "observed_start": "<RFC3339 UTC>",
  "observed_end": "<RFC3339 UTC>",
  "duration_ms": 12345,
  "outcome": "...",
  "substrates": { ... },
  "harm": { ... },
  "cannot_testify": [ ... ]
}
```

(Full grammar at `~/git/notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md`;
NS does not re-document it here.)

NQ's discipline lines that bound what NS may consume:

- *"NQ owns the grammar. Apps own the phase map."* — NS is an app at
  the consumer end; NS does not get to coin synonyms for the grammar
  fields. (Composes with [[feedback-nq-canonical-terminology]].)
- *"A workload-phase witness describes one observed window. It is
  not absolution."* — NS may not collapse a sequence of windows into
  a single verdict that asserts more than each window observed.
- *"NQ should not merely ingest workload-phase witnesses. It should
  survive one."* — the dogfood line. Composes with NS's
  same-shape obligation: NS must consume workload-phase witnesses
  in a way that survives them, not merely emit them through.

The severity-axis decomposition NQ filed in PHLR — pressure /
harm / loss / recoverability — names the axes NS will watch evolve
across time. NS does not own those axes; NS owns *how they move*.

## What NS is NOT authorized to do (the load-bearing boundary)

This section is the GAP's most important content. Every item below
is a refusal — a thing the consumer design must *not* be:

- **NS does not mint workload-phase observations.** NQ mints,
  apps emit, NS consumes. NS is not an emitter and not a
  re-emitter. Per [[feedback-nq-canonical-terminology]]: NS does
  not coin local synonyms; NS does not author parallel surfaces
  for shared concepts.
- **NS does not derive action permission from temporal observation.**
  A regime shift, a closing recoverability window, a trend turning
  from `oscillating_within_band` to `monotonic_degradation` may
  raise NS urgency, may raise NS attention salience, may change
  posture class. **None of these mints action authority.** The
  actuation-boundary doctrine holds: NS proposes; Governor (or
  whoever holds creds) authorizes; an executor across a privilege
  boundary enacts. Temporal evidence does not bypass that ladder.
- **NS does not promote candidate posture transitions into
  ratified state machines without a forcing case.** The note's
  sketches (trend_class / debt_class / progress_class /
  recoverability_class / posture: HOLDING-WATCH-ESCALATE-EXPIRED-
  CANNOT_TESTIFY) remain sketches, not committed surface. Per
  [[project-transitions-not-nouns]]: a primitive without a named
  transition it governs is enum-shaped speculation. Forcing cases
  are the unlock.
- **NS does not become an APM product / metrics DB / Prometheus /
  per-app dashboard.** The note's anti-collapse boundary list
  transfers wholesale into this GAP. The gravity well is real:
  once NS consumes workload-phase witnesses and tracks them across
  windows, every adjacent "could you also..." is one incremental
  yes away. The list exists as anti-yes scaffolding.
- **NS does not cycle posture back into NQ truth ingestion.** Per
  the channel-split forbidden-cycle invariant
  ([`NQ_NS_CHANNEL_SPLIT_NS_SIDE.md`](NQ_NS_CHANNEL_SPLIT_NS_SIDE.md)),
  there is no path from NS posture to NQ truth. Temporal-evidence-
  driven posture changes (e.g., NS classifying a window-sequence
  as `monotonic_degradation`) MUST NOT be readable back as
  workload-phase witness input to NQ. Structural absence, not a
  flag.
- **NS does not collapse workload-phase observation into the
  closure predicate.** Per `SLICE_4_CLOSURE_CANDIDATE V1`:
  closure-candidate refuses closure; it does not authorize closure.
  A workload-phase window that observes recovery is evidence that
  may move `EvidenceState`, not a positive eligibility signal for
  closure on its own. The combinatorial sweep test on
  `EligibleForClosureReview` unreachability still holds.

## What NS owes (the consumer design — the open question)

The remaining live question, per the deep rule: **what does an
admissible workload-phase consumer in NS look like?**

Open sub-questions, each non-trivial and each a candidate for its
own slice when an unlock fires:

### Q1. Where does the workload-phase witness enter the reconciler?

Candidates (sketches; not committed):

- **As a sidecar to the NQ finding.** Workload-phase observations
  travel alongside the finding shape NS already consumes; the
  reconciler treats them as additional evidence under the same
  freshness / basis-staleness discipline (Slice B). Cleanest fit
  with existing primitives; gives every workload-phase witness a
  bound `FreshnessBasis`.
- **As a separate input class.** Workload-phase observations enter
  on their own pipe; the reconciler composes the two streams. More
  honest about the "axes vs. their temporal behavior" framing; more
  surfaces to keep coherent.

Either way: Slice 5 contract's three-axis split (truth NQ-owned /
notification posture NS-owned / ack obligation NS-owned) holds.
Workload-phase evidence sits on the truth axis (NQ-owned) at the
boundary; how NS *projects* sequences of workload-phase windows is
on the posture axis (NS-owned).

### Q2. What NS surfaces expose temporal interpretation?

The note named three candidate surfaces; all remain candidate:

- `packet.attention.next_check_at` — already exists; could carry a
  workload-phase-derived next-window timing without surface change.
- `runs show` peek output — could expose a "this window vs. last
  window" comparison without enum commitment if the surface stays
  observational ("disk pressure rose 12pp between window-N and
  window-N+1") rather than verdictful ("trend: monotonic
  degradation").
- A new ledger event class for window-over-window deltas — the
  most invasive; requires forcing case to justify.

Discipline: surfaces that describe what was observed are easier to
build honestly than surfaces that classify what it means. *Observed
in window-N+1* is cheap; *trend = monotonic_degradation* is an
enum-commit, requires forcing case.

### Q3. How does the consumer compose with PHLR's four axes?

NQ's PHLR GAP separates pressure / harm / loss / recoverability.
NS's temporal interpretation is *per-axis*, not on a collapsed
severity scalar. The consumer design must keep the four axes
distinct as it tracks them across windows. Specifically:

- *Pressure was high in window-N+1* is not the same as *harm was
  high in window-N+1.*
- *Loss occurred in window-N+1* is not the same as *recoverability
  expired in window-N+1.*
- A trend on the pressure axis does not imply a trend on the harm
  axis. A trend on the loss axis does not imply a trend on the
  recoverability axis.

This is *not* a new NS doctrine — it's the consumer-side discipline
of NQ's axis decomposition. If NS collapses the four PHLR axes into
a single posture-class signal, NS has effectively re-collapsed what
NQ separated, and the labelwatch Day-5 forcing case happens again.

### Q4. What is the smallest admissible consumer slice?

Candidate (not committed): **a peek-only surface**. NS reads
workload-phase observations from NQ findings (or sidecar input),
surfaces them in `nightshift workload-phase peek` (analogous to
`nq peek` and `liveness peek`) with no reconciler effect. Observer
visibility before action. Same shape Slice C.1 used to ship
silence-aware posture surface-only before any downstream effect.

If this slice ever ships, it is observer-only. Reconciler effect is
a separate slice with its own forcing case.

## Class vocabularies: sketches, not committed surface (carried forward)

The note's sketches transfer to this GAP unchanged, with the same
discipline: per [[project-transitions-not-nouns]], a primitive
without a named transition is enum-shaped speculation. Sketches
preserved here for retrieval; transitions named where they exist.

### `posture` (sketch)

```
HOLDING   WATCH   ESCALATE   EXPIRED   CANNOT_TESTIFY
```

`CANNOT_TESTIFY` overlaps `PostureClass::Unknown` from Slice C.1 —
if this sketch ever ratifies, that overlap must be resolved, not
duplicated.

### `trend_class` (sketch)

```
stable   spike_then_revert   monotonic_degradation
step_change   oscillating_within_band   unknown
```

`spike_then_revert` is a transition (between two windows), keep as-is.
The others are nouns describing what a window-sequence *is* and beg
for transitions. Do not ratify until transitions have witnesses.

### `debt_class` (sketch)

```
none   isolated   bounded_recurring   cascading   unbounded
```

The transitions `isolated → bounded_recurring → cascading` are the
load-bearing ones; the category labels are scaffolding. Likely
composes with `GAP-solution-family-exhaustion.md` if it ever
ratifies.

### `progress_class` (sketch)

```
completed   advanced   repeated_prefix   stalled   drained
abandoned_due_to_window_close   unknown
```

`repeated_prefix` is the labelwatch `update_author_day` discovery
(processed=2/7 every cycle was repetition, not progress).

### `recoverability_class` (sketch)

```
not_needed   recoverable   recovery_pending
recovery_window_closing   recovery_expired   unknown
```

**Likely subsumed by NQ's PHLR recoverability axis** — held as a
sketch so the overlap is documented, not as a parallel surface. If
PHLR ratifies a recoverability vocabulary, NS adopts it; NS does
not coin a parallel one.

## Forcing case

The labelwatch Day-1 / Day-3 / Day-4 / Day-5 soak remains the
temporal-interpretation specimen. The note recorded it; this GAP
adopts it unchanged:

```
Day-1:  structural fix landed; drops zero; checkpoint debt isolated.
Day-3:  wal_busy up; 2-pass drain ratio up; last_completed stalled.
Day-4:  Day-3 trend reverted; last_completed advanced.
Day-5:  raw drops high; unique affected subjects low; recoverability intact.
```

NS's value across these windows would not be classifying any single
day. NS's value would be preserving the temporal interpretation
across them: Day-3 was not a regime shift; Day-5 raw drops were
pressure not loss; residual checkpoint debt remained named but did
not earn action.

The cartography PHLR GAP cites the same Day-5 soak as its forcing
case. NS and NQ are looking at the same forcing case from two
different consumer positions — NQ is asking *what axes does the
finding shape preserve?*; NS is asking *what temporal interpretation
does the consumer place on sequences of windows across those axes?*

## Non-goals (load-bearing)

Explicitly NOT authorized by this GAP, even when the next slice
fires:

- **Path B / proxy-shock recognition / Slice 5.** Held by
  `~/git/cartography/coordination/POST_MVP_A_CHECKPOINT.md` step 5
  ("Then Path B ← held; not now"). This GAP files consumer design
  questions; it does not authorize the gates Path B unlocks.
- **Implementation of any class-vocabulary enum on packet, ledger,
  or wire surface.** Sketches stay sketches until forcing cases
  unlock them per [[project-transitions-not-nouns]].
- **Defining a workload-phase witness format.** NQ owns the
  grammar. If amendment is needed, feedback goes back to NQ; NS
  does not fork the witness format.
- **A metrics database, soak board, host telemetry surface, or APM
  posture.** The anti-collapse boundary list is constitutional, not
  decorative.
- **Cycle-closing channel from NS posture back to NQ truth.** Per
  channel-split: structural absence, not a flag.
- **Authorizing closure of an IncidentShape finding because
  workload-phase observation suggests recovery.** Per Slice 4:
  predicate refuses; does not authorize.

## Promotion to architecture

This GAP becomes ratified architecture when:

1. NS files a concrete consumer slice (Q4-candidate or successor)
   with a forcing case beyond labelwatch Day-5, and the slice ships
   under the boundary discipline above.
2. *Or* a second labelwatch-shape soak fires in a different ops
   domain that surfaces a temporal-interpretation gap, and the
   consumer design ships against that case.

Until either fires: working GAP. Read on relevance, ratify lazily.

## Composes with

- [`../decisions/temporal-obligation-tracking.md`](../decisions/temporal-obligation-tracking.md)
  — superseded working note (provenance only after this GAP lands).
- [`../../../../notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md`](../../../../notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md)
  — upstream witness grammar (NQ-owned).
- [`../../../../notquery/docs/working/gaps/PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md`](../../../../notquery/docs/working/gaps/PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md)
  — upstream axis decomposition (NQ-side candidate).
- [`NQ_NS_CHANNEL_SPLIT_NS_SIDE.md`](NQ_NS_CHANNEL_SPLIT_NS_SIDE.md)
  — channel-split forbidden-cycle invariant.
- [`GAP-silence-aware-posture.md`](GAP-silence-aware-posture.md)
  — Slice C.1 surface-only precedent; this GAP's Q4 candidate
  follows the same shape (peek surface before reconciler effect).
- [`GAP-imported-basis-freshness.md`](GAP-imported-basis-freshness.md)
  — Slice B `FreshnessBasis` discipline; any workload-phase witness
  entering as truth-axis evidence must compose with it.
- `../decisions/ACTUATION_BOUNDARY.md` — the constitutional refusal
  that temporal observation does not mint action permission.
- `../decisions/GAP-solution-family-exhaustion.md` — bucket-migration
  pattern; debt_class transitions likely compose here.
- `../decisions/GAP-autonomous-execution-boundary.md` — the
  trigger-vs-creds discipline at the action layer.
- `../../../../cartography/coordination/POST_MVP_A_CHECKPOINT.md`
  — operator stabilization sequence; Path B held.
- [[project-temporal-obligation-tracking]] — memory pointer for the
  superseded working note's distillation history.
- [[project-substrate-consumption-chain]] —
  `ok_to_proceed ≠ authorization summary` sentinel at the truth-vs-
  authorization boundary; this GAP applies the same discipline at
  the temporal-evidence-vs-action-permission boundary.

## Provenance

- 2026-05-29 (this session, mid-day) — operator handed an
  exploratory "Nightshift cut" draft on workload-phase witnesses
  and temporal obligation tracking. Distilled into the working
  note at `../decisions/temporal-obligation-tracking.md`. The
  note's promotion conditions named NQ's filing of
  `workload_phase_observation.v0` as condition (1).
- 2026-05-29 (later same session) — cross-workspace reconcile
  pass on a "what's next" question (per
  [[feedback-cross-workspace-reconcile-before-summary]]) discovered
  NQ had already filed `WORKLOAD_PHASE_WITNESSES.md` (2026-05-28)
  and `PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md` (2026-05-29). The
  note's promotion condition (1) had fired the same day the note
  was written.
- 2026-05-29 (this filing) — operator chose option (b) supersession
  with explicit boundary statements. GAP filed; note amended with
  supersession header pointing here.
