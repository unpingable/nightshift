# GAP: Lesson Distillation Boundary

> Status: candidate / proposed doctrine. Filed 2026-05-05 to capture
> a meta-meta boundary: the rules under which Night Shift may extract
> candidate lessons and motifs from reconciliation history *without*
> letting them become auto-applied doctrine, hard-coded rules, or
> authority. **No implementation.** No pattern machinery, no motif
> registry, no auto-classification, no clustering pipeline. A record
> is not authorization to build. This spec exists so the boundary
> has a handle for review.
>
> *(Recursion warning: this GAP is itself a doctrinal record about
> doctrinal records. Filing it inside Night Shift is exactly the
> kind of move it exists to make safe. The discipline is named in
> §"The inaugural seed bank: a hazard to ourselves." Read that
> section before treating any of the motifs it lists as ratified.)*

## Why this is named now (YAGNI posture)

Five candidate GAPs landed in Night Shift's `docs/` in roughly a week
(slice-cycle, workflow-routing-boundary, architectural-promotion-
boundary, solution-family-exhaustion, and now this one). The connective
tissue across them — *Night Shift notices when the kind of work has
changed* — is now in `CLAUDE.md`'s "What This Is."

Motifs are already coalescing in the prose:

```text
failure bucket migration            (keeper in exhaustion)
solution-family exhaustion          (its own GAP)
roadmap acceleration                (architectural-promotion-boundary)
transport is not governance         (caveat in routing-boundary)
deferred obligation ≠ authorization (keeper in routing-boundary)
```

Plus several motifs surfacing in operator notes that don't yet sit in
any document:

```text
stale mitigation authority
logical vs physical reclamation
green-signal scope error
maintenance as workload
```

Without naming the distillation boundary now, two failure modes
become available:

1. The next motif gets ad-hoc lifecycle treatment — sometimes promoted
   into a keeper, sometimes folded into an existing GAP, sometimes
   left in chat-log compost — and Night Shift quietly grows a
   private folklore where motifs accrete unevenly.
2. Worse: a future Night Shift surface looks at the motif list and
   silently treats compression-into-a-known-motif as a *decision*
   rather than as a *hypothesis worth surfacing*. That is the
   "tiny expert system in a trench coat" failure mode.

This is the architectural surface where retrofit cost rises with
usage spread. Naming the candidate is cheap; ratifying it lazily is
the discipline.

## Keeper lines

Six load-bearing lines. If the rest of this doc evolves, these stay:

> **Night Shift may extract candidate lessons from reconciliation
> history. It may not ratify doctrine or apply patterns as authority
> without review.**

> **The motif names are handles for recognition, not handles for control.**

> **Night Shift remembers patterns as warnings, not as rules.**

> **Pattern recognition is competence. Pattern authority is where
> the furniture starts floating.**

> **A new case that compresses cleanly into an old motif is
> suspicious. Show the differences before applying the lesson.**

> **Too soon to apply meta-patterns automatically. Not too soon to
> extract them as candidate lessons.**

Each is unpacked below.

## Problem

Pattern recognition is genuinely useful. After enough reconciliation
runs and incident reviews, a small library of recurring shapes
accumulates: *failure shifted buckets again*, *we're tuning around the
same premise again*, *that mitigation expired before it executed
again*. Surfacing those shapes during a fresh incident — *"this looks
like Pattern X; here's the prior case"* — is a real Night Shift
contribution. It is the difference between a tool that schedules
intent and a tool that helps the operator avoid replaying their own
historical mistakes.

But pattern recognition shades into pattern authority almost
silently:

- *"This looks like X"* → *"This is X"*
- *"We treated X this way last time"* → *"X is treated this way"*
- *"Pattern X recommends review"* → *"Pattern X requires review"*
- *"Surfacing Pattern X"* → *"Acting on Pattern X"*

Each step looks like compression. Each step quietly trades
*hypothesis* for *rule*. Past a certain point Night Shift is no
longer recognizing patterns; it is *enforcing* them. The motif names
have become handles for control. That is where the furniture starts
floating.

This GAP names the boundary so Night Shift can do the useful version
of pattern recognition without sliding into the dangerous one.

## Pattern lifecycle

Five explicit stages. **No skipping.** The whole point of the
lifecycle is that compression earns its way down the stages, rather
than appearing at the bottom because the prose was confident.

```text
1. Observed incident
   A specific incident with receipts. No generalization yet.
   Lives in: run ledger, packet history, operator notes.

2. Candidate lesson
   An incident-local takeaway. Cross-context compression has not yet
   been claimed.
   Example: "Retention protected ingest but did not reclaim disk
   because SQLite DELETE/UPDATE does not return filesystem space
   without VACUUM."
   Lives in: incident postmortem, run notes, Continuity entry
   marked candidate.

3. Candidate meta-pattern
   A *named handle* claiming the same shape applies across multiple
   incidents. Status: candidate. Authority: advisory only.
   Example: "Logical lifecycle control is not physical resource
   reclamation."
   Lives in: a doc like this one, or a memory pointer; not in any
   enforcement code path.

4. Ratified pattern
   A meta-pattern explicitly approved for advisory recognition.
   Either: repeated successful use across genuinely different
   incidents *and* explicit human review, or: one human-approved
   ratification.
   Example: "When a mitigation controls future growth but does not
   reclaim existing pressure, Night Shift surfaces the unreclaimed
   pressure as separate debt."
   Lives in: a Night Shift doctrine doc, possibly an invariant.

5. Applied as advisory recognition
   Night Shift may surface "this looks like ratified pattern X" as a
   *hypothesis with prior cases*. Never as a decision; never as
   authority.
```

The stage names are vocabulary, not yet wire format. The point of
naming them is to make stage-skipping visible. A motif appearing in
operator prose at the *applied advisory* register without having
passed through *candidate meta-pattern* and *ratified pattern* is
the failure this lifecycle exists to prevent.

## Candidate sharpening of the lifecycle: transitions, not nouns

> Status: candidate / not ratified. Filed in repo text rather than
> as its own GAP (premature promotion of this candidate into a
> standalone GAP would itself violate the rule below — the rule
> forbids unwitnessed promotion, not all standalone GAPs) and
> rather than memory-only (see *retrieval-hook discipline* below).
> The location of record is itself part of the discipline this
> section names.

The pattern lifecycle above answers *when* a candidate advances
through stages. This section names a candidate sharpening of the
*entry criterion* — what makes a pattern eligible for the lifecycle
in the first place.

### The candidate rule

> **Do not mint a new primitive until you know what transition it governs.**

Operational form. A candidate primitive earns lifecycle entry only
when it names the transition it blocks, permits, classifies, or
audits. A name that labels a noun without the transition stays
unfiled.

> **Do not promote an ensemble of new primitives faster than the
> transition graph can absorb.**

Volume caution. Each primitive may pass the transitions test
alone; the ensemble forces graph-wide integration that may exceed
what the existing transition graph holds without rewriting.

### Witnesses

- *Labelwatch autonomous-execution incident, 2026-05-06.* After
  the `GAP-autonomous-execution-boundary.md` tightenings landed, a
  fifteen-item enumeration of additional candidate primitives was
  generated (operator-state model, stranded preparation, recovery
  race, read-only cost classes, role identity drift, repeated-alarm
  pressure, evidence-from-silence, and others). Most items
  individually governed transitions. The threat was volume — the
  set would have forced an ensemble-level transition-graph rewrite
  faster than existing transitions could absorb. Three items
  promoted (rollback-as-mutation corollary, keeper-5 reword,
  context-window-poisoning mechanism note); twelve held.
- *Primitive-chase → state-space framing*, the recurring prior
  lesson. The shape rhymes; the rule generalizes.

### Recursive self-check

The rule's own transition is *candidate-pattern → load-bearing-
doctrine* inside the pattern lifecycle above. It passes its own
test, which is what makes it filable here. It does not pass into
ratification by being filable; ratification waits for either
explicit operator ratification or a second independent witness.

### Non-authority

- Does not authorize new GAP creation.
- Not a roadmap, punch list, or benchmark spec.
- Does not modify the pattern lifecycle text above. Lifecycle
  modification is a separate move with its own ratification trigger.

### Retrieval-hook discipline

The pair of operative principles for *where* candidate work lives
between recognition and filing:

> **Record the candidate where it will be reviewed, not where it
> will self-promote.**

> **"Later" is the unsecured S3 bucket of cognition.**

Deferral requires a retrieval hook — repo text, a memory pointer,
an index entry, or an explicit review surface in the GAP it would
eventually amend. Otherwise deferral becomes silent deletion. A
candidate held only in conversational memory or in the operator's
recollection is not deferred; it is leaked.

For this rule specifically, repo text is the primary review surface
(the next GAP-author consults this file before filing). The
original memory pointer (`memory/project_transitions_not_nouns.md`)
is preserved as a backup handle, not as the primary record.

## What Night Shift may safely do

Inside this boundary, Night Shift's pattern-related competence
includes:

- detect repeated *shapes* across incidents already in scope
- cluster related incidents by shared premise, when the cluster is
  evidence-backed and operator-readable
- propose *candidate* meta-patterns by extracting compression
  candidates from incident history
- attach evidence: which incidents, which receipts, which mitigation
  chains support the candidate compression
- suggest a Continuity memory entry capturing a candidate lesson
- suggest a GAP filing if the pattern crosses tools or substrates
- remind a future session: *"this incident resembles prior
  ratified-pattern X — here are the differences"* (advisory only)
- distinguish candidate vs ratified status in any pattern reference
  it surfaces

These are all observational, advisory, and evidence-bearing. None of
them carries authority.

## What Night Shift may not do

Outside this boundary, regardless of pattern confidence, Night Shift
must not:

- mutate doctrine (CLAUDE.md, GAPs, invariants) on the basis of a
  pattern
- create or amend invariants because a motif looks load-bearing
- authorize actions because a pattern recommends them
- auto-promote architecture because a pattern signals containment is
  expiring
- decide a pattern is ratified
- collapse a new case into an old pattern *without showing
  differences* — even when the compression looks tight
- treat motif-name compression as a verdict
- suppress an incident's specific evidence in favor of pattern-level
  generalization

The keeper that anchors all of these:

> **Pattern recognition is competence. Pattern authority is where
> the furniture starts floating.**

## Signal shape (sketched, not specified)

When Night Shift surfaces a pattern recognition, the artifact shape
is review-bearing, not action-bearing. Field names below are
illustrative; this doc does not specify them.

```text
event_kind                candidate_meta_pattern
                            | ratified_pattern_recognition
may_execute               false
requires_human_review     true
authority                 advisory_only

pattern
  name                    short identifier
  status                  candidate | ratified
  source_incidents        [...]                  receipts, run ids
  recognition_features    [...]                  what made this look
                                                  like the pattern
  differences_from_prior  [...]                  required field — what
                                                  does NOT match prior
                                                  cases, even when
                                                  most of it does

compressed_lesson         plain-language statement of the pattern

suggested_destinations
  continuity              durable lesson candidate (yes/no + draft)
  nightshift_gap          possible doctrine (yes/no + draft scope)
  governor                only if a consequence-bearing action
                          follows; default no

not_authorized            explicit non-authorization list:
                          - automatic remediation
                          - architecture promotion
                          - invariant mutation
                          - threshold change
                          - re-classification of past incidents
```

`may_execute: false` is non-negotiable. The seatbelt on the chainsaw,
in the user's framing.

`differences_from_prior` is the load-bearing required field. A
pattern recognition that does not name what is *different* about the
new case is the precise failure mode the keeper *"a new case that
compresses cleanly into an old motif is suspicious"* exists to
catch. Compression without differences is doctrine-as-autopilot.

## The inaugural seed bank: a hazard to ourselves

This section is the recursion warning made concrete. Listing motifs
inside the GAP that names the distillation boundary is *exactly*
the move the boundary exists to govern. The discipline:

- Motifs listed below are at most at *candidate meta-pattern* status.
- Motifs already promoted into existing Night Shift docs are
  cross-referenced by their canonical home; the seed bank does not
  re-ratify them.
- Motifs not yet documented elsewhere are listed as candidate names
  with one-line stubs. The stubs are *recognition handles*, not
  specifications. None is ratified by being listed here.
- Inclusion in this list does **not** authorize Night Shift to apply
  any of these motifs as patterns. The list is doctrinal vocabulary;
  application requires lifecycle progression to *ratified pattern*.

### Already promoted into a Night Shift doc (cross-references)

```text
failure_bucket_migration
  → keeper in GAP-solution-family-exhaustion.md
    "Failure bucket migration is not recovery."

solution_family_exhaustion
  → its own GAP: GAP-solution-family-exhaustion.md

roadmap_acceleration
  → kernel of GAP-architectural-promotion-boundary.md
    keeper: "A roadmap item is not an authorization."

transport_is_not_governance
  → §"Transport is not governance" in GAP-workflow-routing-boundary.md
    keeper: "Queue presence is not work authorization."

deferred_obligation_is_not_deferred_authorization
  → keeper in GAP-workflow-routing-boundary.md
```

These are listed for completeness so the seed bank is honest about
what is already in motion. They are not re-ratified by appearing
here.

### Named candidate, not yet filed (watch list)

```text
stale_mitigation_authority
  Mitigation authority captured at T0 may not survive to execution
  at T1. The restart that was justified can become unjustified before
  it runs. Adjacent to "deferred obligation is not deferred
  authorization" but mitigation-specific.
  → memory: project_kind_of_work_keepers.md (watch list)
  → wait for failing case before filing as own GAP
```

### Inaugural candidate motifs (this doc's contribution)

```text
logical_vs_physical_reclamation
  Logical lifecycle control (DELETE, UPDATE, retention windows) does
  not necessarily reclaim physical resources (disk extents, memory
  pages, file handles). Cross-cuts: storage, memory, network buffers.
  Recognition feature: a green retention/cleanup signal coexists with
  a degrading underlying-resource signal.

green_signal_scope_error
  A green signal proves what its scope says it proves, not the
  adjacent thing the operator wants it to prove. "rollback_lost=0"
  proved ingest rollback safety, not retention health. Recognition
  feature: a metric named for one concern is being read as evidence
  for a related-but-distinct concern.

maintenance_as_workload
  Retention, export, longitudinal recheck, archive lifecycle, backup,
  and reconciliation are *workloads*, not "background vibes." They
  compete for the same locks, I/O, and CPU as primary workload, and
  fail in workload-shaped ways. Recognition feature: scheduling
  decisions treat maintenance as free; failure modes recapitulate
  primary-workload failure modes inside the maintenance class.
```

These three are the genuinely new contribution of this GAP to the
seed bank. They are *candidate meta-patterns*: named handles, advisory
authority, no application machinery, no ratified status. Each may
later be promoted into its own GAP, folded into an existing one, or
quietly retired if it does not survive operational reality.

The seed bank is **not** a backlog. It does not commit Night Shift to
ratifying any of these. Per the agenda-reconciler trap, treating
motif lists as a roadmap to drain is exactly the failure mode this
project keeps not committing.

## Distinction from agenda-reconciler trap and solo-platform-PM-substrate

This GAP sits in a hazard family with two memory pointers:

- `feedback_agenda_reconciler_trap.md` — Night Shift must not grow
  *upward* into operator pickup truth without a failing case.
- `project_solo_platform_pm_substrate.md` — the recursive pattern is
  recognizable across personal / project-local / system-runtime
  altitudes; recognition handle, not artifact to ship.

This GAP names a third hazard adjacent to both:

- Night Shift must not grow *downward* into doctrine-from-pattern,
  treating motif compression as a substitute for the failing-case
  discipline.

All three are stay-in-your-lane doctrines. They cover different
failure modes:

```text
agenda-reconciler trap            distillation boundary (this GAP)
  failure: NS grows into             failure: NS extracts patterns and
  operator PM substrate              applies them as authority
  direction: upward                  direction: downward into doctrine

solo-platform-PM-substrate
  failure: confusing the recursive
  pattern's recognition value with
  authorization to productize
  direction: across altitudes
```

The three compose. None authorizes the others. All three exist
because Night Shift's surface is rich enough that several different
over-productizing trajectories are available, and each needs its own
named-and-resisted handle.

## Distinction from GAP-solution-family-exhaustion

`GAP-solution-family-exhaustion.md` names *one specific motif*
(failure bucket migration as evidence against a preserved premise)
and the Night Shift behavior required when that motif is detected.
This GAP names the *meta-rules* under which any motif (including
exhaustion) is allowed to be extracted, named, ratified, and applied.

They compose:

- Exhaustion is one of the inaugural ratified-promoted motifs in
  this GAP's seed bank.
- This GAP is meta to exhaustion: it constrains what Night Shift
  may do with the exhaustion motif (and others) in general.
- A future motif's promotion path passes through *this* lifecycle;
  exhaustion's promotion (incident → candidate → its own GAP) was
  the *first* trip through it, made explicit here only after the
  fact.

The relationship is: exhaustion is an instance; distillation is the
governance for the class.

## What this does not authorize

Per YAGNI posture and the no-implementation constraint:

- **No pattern machinery.** No motif registry. No auto-classifier.
  No clustering pipeline. No similarity engine. No "did this match
  Pattern X" lookup function.
- **No pattern lifecycle code.** The five lifecycle stages are
  doctrinal vocabulary. They are not states in any state machine.
- **No application of any motif.** Listing motifs in the seed bank
  does not ratify them, does not authorize Night Shift to apply
  them, does not commit Night Shift to building the recognition
  for them.
- **No pattern-driven mutations.** Doctrine, invariants, schemas,
  CLAUDE.md, and GAPs are not modified on the basis of patterns
  produced by Night Shift. Mutations require human review under
  the ordinary failing-case discipline.
- **No authority elevation via pattern confidence.** Pattern
  ratification status does not raise Night Shift's promotion
  ceiling, lower its review requirements, or substitute for any
  existing gate.
- **No new ledger events.** The run ledger does not record motif
  matches.
- **No CLAUDE.md invariants added** by filing this. If a
  distillation invariant becomes load-bearing later, it is added
  when ratified.

A record is not authorization to build.

## Trigger conditions for ratification

Ratify (and consider building motif machinery) when one of these
happens:

1. A real failing case where Night Shift applied a motif as
   authority (or near-as-makes-no-difference) and the operator had
   to unwind the consequence.
2. The motif seed bank crosses ~12-15 candidate entries and the
   *absence* of structured tracking is producing collisions
   (different names for the same shape, same name for different
   shapes).
3. A future Night Shift surface (e.g., the watchbill UI) needs
   first-class motif references and the only options are "ratify
   the lifecycle and machinery" or "improvise locally."
4. Continuity grows enough cross-incident memory that motif
   compression becomes operator-visible work, and Night Shift's
   doctrinal stance on pattern application needs to be wire-level
   rather than doc-level.

Until one of those triggers fires, this stays a candidate. The six
keeper lines, the five-stage lifecycle, the safe/unsafe lists, and
the seed bank's candidate-grade discipline are the load-bearing
parts; the machinery is deliberately deferred.

## Vocabulary overlaps with existing Night Shift docs

Called out so we know what is reused vs introduced:

- **`may_execute: false`** — introduced in
  `GAP-solution-family-exhaustion.md` for the exhaustion signal.
  Reused here for the candidate-meta-pattern signal. Same
  semantics; same load-bearing role. If the field ever ships,
  alignment with both GAPs is required.
- **Cited-not-absorbed** practice — established across
  `GAP-workflow-routing-boundary.md`,
  `GAP-architectural-promotion-boundary.md`, and
  `GAP-solution-family-exhaustion.md`. Reused here for the seed
  bank: motifs are cited as recognition handles, not absorbed as
  doctrine.
- **Action classes** (containment / tuning / architectural
  promotion) — `GAP-architectural-promotion-boundary.md`. The
  inaugural `maintenance_as_workload` candidate motif is adjacent
  to that taxonomy but distinct: action classes classify a
  *current proposal*; maintenance-as-workload classifies a
  *whole class of work* the system has been treating as
  category-free. Worth flagging if maintenance-as-workload ever
  ratifies; not collapsing them now.
- **Recognition vs control** keeper — first surfaced informally
  in chatty's framing. Not yet a keeper in any prior GAP.
  Promoted to keeper status in this doc.
- **Lifecycle / authority / artifact** ladders (CLAUDE.md
  invariant 5) — the pattern lifecycle is *not* a fourth ladder.
  It is a lifecycle for *ideas about Night Shift's behavior*, not
  for runs or artifacts. No collision; explicit non-overlap.

No vocabulary in this doc renames or replaces existing
terminology. New introductions are: the five-stage pattern
lifecycle, the six keeper lines, the seed bank with candidate /
promoted / watch-list buckets, and the three inaugural candidate
motifs (logical-vs-physical reclamation, green-signal scope error,
maintenance-as-workload).

## Open questions (not load-bearing for the record)

- **Where does the motif name space live when ratified?** A
  Continuity workspace? A Night Shift doc directory? A GAP-per-
  motif convention? Probably the latter when motifs are
  individually consequential, with a registry index when the count
  justifies it. Defer.
- **Boundary between candidate lesson and candidate meta-pattern.**
  The distinction is "incident-local" vs "claims to apply across
  incidents." Edge cases (one-incident motif that *feels*
  general but lacks a second case) need adjudication when they
  appear. Defer.
- **Differences-from-prior as required field, formally.** The
  signal shape lists `differences_from_prior` as required. If the
  signal ever ships, a compression hypothesis without that field
  is treated as malformed. The exact enforcement shape is
  deferred.
- **Continuity memory candidates.** A candidate lesson may suggest
  a Continuity entry. Continuity is optional for authorization
  per CLAUDE.md invariant 7. Is it optional for lesson durability?
  Probably yes — Night Shift surfaces the candidate; the operator
  decides whether Continuity is the right home. Confirm when
  Continuity wire surface stabilizes.
- **Self-reference handling.** This GAP itself is a doctrinal
  artifact emitted by a Night Shift session. Future sessions
  reading it must distinguish "filed candidate doctrine" from
  "filed candidate motif applied as doctrine." The recursion
  warning at the top is the current best answer; whether more
  structural treatment is needed depends on how this GAP is read
  by the next several Night Shift sessions.
