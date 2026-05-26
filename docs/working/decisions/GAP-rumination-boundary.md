# GAP: Rumination Boundary

> Status: candidate / proposed doctrine. Filed 2026-05-06 with
> **no Night-Shift-internal witness.** Filed in repo text rather
> than memory-only because the doctrine warns against trusting
> synthesized memory as authority, and the doctrine should not
> live exclusively inside the synthesized layer it warns about.
> **No implementation.** Skeleton scope only — output-class
> taxonomy and migration phases live in
> `memory/project_rumination_boundary.md` until a witness in hand
> justifies promoting them to repo text.

## Why this is named now (no-witness posture)

Anthropic shipped a compaction daemon called "Dreams" — past
sessions → offline summarizer/critic → proposed memory updates
("insights") → future agent behavior. The naming does
anthropomorphic + authority-laundering work, but the *shape* is
sound: any agent stack with persistent memory eventually grows
something synthesis-shaped. The boundary discipline matters
before Night Shift ships its own version, not after.

This GAP cites Anthropic's Dreams as motivating observation, not
as absorbed doctrine. The discipline generalizes over the *shape*
of the failure — synthesis becoming standing — not over the
specific implementation. Per the cited-not-absorbed practice in
the other May-series GAPs.

The discipline of *name early, ratify lazily* applies. The keeper
lines below are load-bearing now; implementation is deferred until
a Night-Shift-internal witness fires.

## Keeper lines

Three load-bearing lines.

> **A dream may propose memory. It may not become memory without standing.**

> **The sleep job does not get commit rights.**

> **Nightshift decides when the past is ready to be reviewed.
> Ruminate proposes what the past might mean. Governor or
> Continuity decides what survives as memory.**

The first is the keeper-grade compression. The second is the
version that survives marketing. The third is the architecture in
one sentence.

## The three-actor split

Memory-domain separation of powers, parallel to the splits already
in place at other altitudes:

- *Action domain:* NS proposes / Governor authorizes (CLAUDE.md
  invariant 1).
- *Evidence domain:* NQ owns truth / NS owns notification posture +
  ack obligation (three-axis split, slice 5 contract).
- *Memory domain (this GAP):* NS triggers retrospect / Ruminate
  synthesizes / Governor or Continuity grants memory-standing.

Underneath all three: *the actor that proposes is not the actor
that grants standing.*

**Nightshift owns:**

- *when* a retrospective pass runs (trigger and timing)
- *what* run/session/window is being reviewed (scope)
- *where* the resulting artifact is recorded (ledger)
- *whether* the artifact later becomes stale, superseded, or
  contested (lifecycle)

**Nightshift does not own** the synthesis itself, and does not
own the standing decision. Nightshift is the shift supervisor; it
does not become the dream goblin.

**Ruminate** (logically separate, regardless of physical packaging
— sidecar binary, shelled module, or eventual integration) owns:

- synthesis of past-run history into structured proposals
- output classes whose transitions are named, not implied

Ruminate does not own direct memory mutation, standing for any
synthesized claim, or the trigger or ledger.

**Governor or Continuity** (per claim type) owns:

- the standing decision: does this claim earn durable memory under
  what scope?

## Structural mapping to existing doctrine

This GAP is the memory-domain analogue of
`GAP-autonomous-execution-boundary.md`. That GAP's master keeper —
*"trigger conditions authorize a question. They do not authorize
an actor"* — has the same structural shape as this GAP's master
keeper: *"a dream may propose memory. It may not become memory
without standing."* Synthesis ≠ standing is the memory-domain
parallel of trigger ≠ execution authority. Same goblin door, two
rooms.

**Plant role.** Per the *Test contract* section in
`GAP-autonomous-execution-boundary.md`, Ruminate is plant-shaped:
watch the past, classify, emit structured proposals, do not
self-promote into operator. Negative-space scoring applies — a
synthesis pass that *attempts* to mutate memory is a plant failure
even if a guardrail catches it. This GAP plausibly anchors the
first concrete instance of the plant role outside its originating
doctrine.

**Lesson-distillation lifecycle.** Per
`GAP-lesson-distillation-boundary.md`, Ruminate output is a
candidate-stage artifact. It must flow through the five-stage
lifecycle (observed → candidate lesson → candidate meta-pattern
→ ratified pattern → applied advisory). It does not enter the
lifecycle at *applied advisory* by virtue of being synthesized.
Per the candidate sharpening *transitions, not nouns* in that
GAP, each Ruminate output class must name the transition it
governs.

## What this does not authorize

Per the no-witness posture and the volume caution from
*transitions, not nouns*:

- **No implementation.** No `ruminate` module, no synthesis pass,
  no retrospective queue, no candidate-memory wire format, no
  Governor/Continuity standing-grant RPC.
- **No final output enums.** A draft taxonomy lives in
  `memory/project_rumination_boundary.md`. One candidate
  (`pickup_context`) is flagged as suspicious — it survives only
  if defined as the transition *next-session-start →
  advisory-orientation context-load*. Otherwise it is a noun in
  disguise.
- **No migration phases as ratified plan.** The sidecar →
  NS-invokes → NS-scheduler-not-synthesizer progression in memory
  is a build-order claim, not doctrine.
- **No anthropomorphic naming.** *Retrospective compaction*,
  *memory proposal pass*, *post-session review*, *rumination
  queue*, *untrusted synthesis pass* — all acceptable. *Dreams*,
  *insights*-as-privileged-object, *reflection*-as-action — all
  do authority-laundering work and must not enter Night Shift
  vocabulary.
- **No new authority.** This GAP lowers the *default* posture of
  any future synthesis pass (no standing absent positive grant);
  it does not raise any tool's ceiling, does not grant Ruminate
  any authority class, and does not change Governor's role.

A record is not authorization to build.

## Trigger conditions for ratification

Promote to non-candidate (and lift the output-class taxonomy and
migration phases from memory into this GAP) when one of these
fires:

1. Night Shift first generates synthesis-shaped output —
   candidate memory, retrospective summary, "lessons" — in any
   artifact.
2. An adjacent agent proposes treating retrospective output as
   authority: memory mutation, policy update, configuration
   change.
3. A second independent witness of the synthesis-becomes-standing
   collapse in any project Night Shift's doctrine touches.
4. Continuity grows enough memory-mutation surface that the
   *standing* decision becomes a queryable state.

Until one of those fires, this GAP carries only the keeper lines,
the three-actor split, and the structural mapping. The
elaborations (output-class taxonomy with the `pickup_context`
sharpening, migration phases, vocabulary refusals) live in
`memory/project_rumination_boundary.md` until a witness in hand
justifies promotion.
