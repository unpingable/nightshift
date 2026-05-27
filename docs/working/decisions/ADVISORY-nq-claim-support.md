# ADVISORY: NQ claim-support classification

> **Status:** **closed via outcome (2) — recognition** (2026-05-27). See §"Closeout 2026-05-27" at the bottom. The seam stays filed as the durable cross-project record; NS will encode the substrate-side mapping over `cannot_testify + claim_kind + finding_kind` as a follow-on. No NQ wire change.
> **Owner:** nightshift (consumer; this repo).
> **Recipient:** nq (producer; `~/git/nq`).
> **Filed:** 2026-05-27, following Slice 4 close-out (`SLICE_4_CLOSURE_CANDIDATE V1` in [FEATURE-HISTORY](FEATURE-HISTORY.md#slice_4_closure_candidate-v1-partial-gate-1)).
> **Tracking edge:** NS ↔ NQ producer/consumer contract. **Not an AG authority edge.** Agent-Governor is two rungs further out on the escalation ladder (see §"The full escalation ladder" below), not adjacent — pulling AG into a witness-shape conversation routes ambiguity through the wrong governor. The narrow gate before AG is Wicket; before either, the missing piece is honest testimony, which is producer/consumer, not authority.

## What this advisory is

A named seam. NS has shipped Gate 1's refusal side (`SLICE_4_CLOSURE_CANDIDATE`), and one of the refusal classes is `UnassessableMissingChannelClassification` — the verdict NS emits for `IncidentShape` findings when it cannot tell whether the evidence supports a closure judgment. To make the `EligibleForClosureReview` path reachable, NS needs richer producer-side claim-support information on findings.

The point of filing this *now*, before NQ moves, is the YAGNI doctrine on architectural surfaces with retrofit-cost asymmetry: **wire formats and cross-module vocabulary are exactly the kind of surface where naming is cheap and adding-later is expensive**, because consumers (NS, possibly future consumers) start depending on whatever the producer currently emits, and every change becomes a coordinated migration. (Global `CLAUDE.md` §"YAGNI scope.")

## What this advisory is NOT

- **Not authorization to build.** NQ owns producer-side decisions including whether, when, and in what shape this lands.
- **Not a prescription of NQ's wire shape.** A possible shape is mentioned at the bottom of this doc as a starting point only; NQ owns the actual definition.
- **Not an ask for NQ to mint closure eligibility.** NS owns the closure predicate. NQ should never emit `eligible_for_closure` or any synonym — that would push NQ into workflow governance. NQ testifies; NS judges.
- **Not blocking on the NS side.** Slice 4 ships with `EligibleForClosureReview` defined-but-unreachable. That is the honest shape until the wire supports the distinction. No NS work is gated on this advisory landing.
- **Not propagating beyond this advisory until NQ ratifies.** Until NQ engages, NS does not add fields, change defaults, or pretend the distinction exists.

## The mismatch, in two vocabularies

NS describes the gap in **consumer vocabulary**, naming what the predicate cannot decide:

> Closure-eligibility requires distinguishing **proxy-channel** evidence (dashboard normalization, "the alert is quiet") from **consequence-channel** evidence (substrate witness, "customer impact / downstream effect"). NS's wire-shape input today does not carry that distinction. Without it, every IncidentShape finding is `UnassessableMissingChannelClassification` — the conservative refusal.

NQ likely sees the same gap through **producer vocabulary**, naming what the testimony *can support*:

> Findings need enough claim-support classification for consumers to distinguish what kind of claim a finding can and cannot support: proxy observation, target-state observation, consequence observation, or `cannot_testify`.

Both vocabularies point at the same underlying axis. NQ's framing is closer to the producer-contract truth: a finding doesn't *have* a channel, it *testifies about* some shape of claim. Naming the testimony shape is producer-side; deciding whether that shape suffices for closure is consumer-side.

## Load-bearing invariants

The reason this distinction is structurally important is the family of refusals it enables. If NS cannot ask "is this consequence-bearing testimony?" then it cannot enforce:

- **`proxy quiet ≠ consequence resolved`** (Gate 1 doctrine, [pre-positioned-doctrine-gates.md](pre-positioned-doctrine-gates.md) §"Gate 1: Incident-closure predicate"; cousin of Slice C.1's `silence_present ≠ incident_absent`).
- **`ack ≠ resolution`** (per [`architecture/GAP-reack-doctrine.md`](../../architecture/GAP-reack-doctrine.md) — already enforced, but the closure side is where it matters most).
- **`green liveness ≠ closure evidence`** (liveness witness fresh ≠ substrate recovered; cousin of the wrinkle visibility Slice B and the liveness consumer surface).

These are recurring refusal classes across the agent-governor / NQ / NS family. Naming the testimony shape on NQ findings would let NS enforce them at the closure edge, not just at the posture and ack edges where they already fire.

## What NS owns vs what NQ owns

Pinned because the temptation is to drift:

| NS owns | NQ owns |
|---|---|
| The closure predicate and its outcomes (Slice 4) | The shape of evidence findings testify to |
| What testimony suffices to be `EligibleForClosureReview` | What testimony shape exists on a finding |
| Operator-attention application (Slice 3) | Whether a finding is alive at all (liveness, Slice 5 InputStatus) |
| Refusal-vocabulary on the NS side | Producer-vocabulary that NS reads (without remapping) |

If NQ exposes a claim-support shape, NS will read it through the existing `closure::assess` function, with an added check that gates `EligibleForClosureReview` on consequence-bearing testimony. The NS-side change is small; the contract is the load-bearing piece.

## The full escalation ladder

Recording this because the temptation when "closure" is the active word is to immediately reach for Agent-Governor, since closure smells like an authority-bearing mutation. **That's two rungs too early.** The actual ladder, in dependency order:

```text
1. NQ
   What can this finding honestly testify to?
   (Producer-side claim-support classification — this advisory.)

2. Nightshift
   Given that testimony, is closure review assessable?
   (Consumer-side predicate — Slice 4 shipped the refusal half;
    NQ-side movement unlocks the eligible half.)

3. Wicket
   Before acting, is the proposed closure operation admissible?
   (Narrow preflight gate; cousin of the existing preflight
    coordination check. Wicket is named in
    `pre-positioned-doctrine-gates.md` Gate 4 as the missing
    refusal-propagation edge once it joins NS's dependency graph.
    Not in NS's graph today.)

4. Agent-Governor
   Govern durable authority-bearing mutation across the broader
   agent/runtime surface.
   (Right answer eventually, but only when the action surface
    outgrows Wicket's narrow gate and a constitutional layer is
    actually needed.)
```

Doctrine line — worth pinning so the AG-gravity-well doesn't pull future versions of this advisory off-shape:

> Do not route witness-shape ambiguity through Agent-Governor. First make the testimony honest (NQ), then judge whether closure is assessable (NS), then preflight the action (Wicket), then promote to AG only if the action surface outgrows the small gate.

Tiny gate before big governor. The current seam is rung 1; AG is rung 4. The advisory is correctly scoped at rung 1 ↔ rung 2.

## What NQ is asked to consider

A producer-side classification on each finding (or on each detector's findings) that lets consumers ask: *what kind of claim can this finding support?*

NQ owns whether this is one field or several, an open enum or a closed enum, on every finding or some, etc. Specifically:

- **Not asked**: a `closure_eligibility` field, a `proxy_channel` boolean, or any workflow-governance shape. NS's predicate is NS's; NQ's testimony shape is NQ's.
- **Asked**: enough structural distinction in the wire shape that consumers can tell `"this finding observes a proxy"` from `"this finding observes the consequence-bearing subject"` from `"this finding cannot testify to that question."`

The `cannot_testify` case may already be partially present in NQ's existing refusal lane (per memory `project_nq_witness_daemon_trajectory` and the consequence-refusal lane). nq-claude's recognition pass at the time of ratification will determine whether this advisory is asking for *new* shape or naming an *existing* shape consumers haven't been reading.

## Forcing conditions for NQ to act

This advisory is non-binding. NQ should consider acting on it when one of these triggers fires:

1. **A second consumer** (not just NS) asks for the same distinction. Two independent consumers wanting the same wire-shape feature is strong evidence the testimony shape is real, not an NS-specific artifact.
2. **An operational case** where `EligibleForClosureReview` would have made a real call — e.g., an operator wants to close an incident, NS refuses with `Unassessable`, the operator notes that real consequence-witness evidence existed but couldn't be surfaced.
3. **A pre-Gate-1 incident** that turns on collapsed channel/witness classification — a closure decision made on proxy-only evidence that should have been refused but wasn't, because the system couldn't tell the difference.

Any one is sufficient. None is required for NS to keep operating today.

## A possible shape (offered, not prescribed)

nq-claude (in `~/git/nq`) floated a shape during the cross-project conversation that produced this advisory:

```text
claim_support_kind:
  proxy_observation
  target_state_observation
  consequence_observation
  cannot_testify
  unknown
```

This is **not the advisory's ask**. It is a starting point for the producer-vocabulary mapping and is recorded here so future readers of this doc see what was on the table at filing time. NQ owns whether this shape, a different shape, or no new shape is the right move. NS will read whatever NQ ships.

**One known risk in this particular sketch:** the `target_state_observation` ↔ `consequence_observation` boundary can turn into theology if defined by object category rather than by claim-support. ("Database writable again" — is that target-state or consequence-state? Once a clipboard-bearing goblin shows up to adjudicate, the axis is wrong.) The discipline is to define the variants by *what claim they may support*, not by what kind of thing they observe. If two operators can disagree on which bucket a real finding goes in, the axis is operator-philosophy, not claim-support — and the predicate downstream will inherit the ambiguity.

## Recognition vs. refactor

Per nq-claude's commitment on the receiving end: the first move on the producer side is recognition, not refactor. If the requested distinction is *already implicit* in NQ's existing surfaces (e.g., `witness_packet.witness_type`, `coverage[].witness`, the closed-enum patterns in slice-6 substrate audits, or the `cannot_testify` refusal kernel), the cleaner outcome is naming where it already lives and offering NS a producer-side framing to act on the existing shape — not adding a new field.

**Framing nudge for the recognition pass.** Before deciding whether to extend or recognize, NQ should answer the prior question:

> Is claim-support orthogonal to NQ's existing axes (directness, freshness, witness-type) — or can one of those existing axes already answer *"what claim may this finding support?"*

A reasonable suspicion from the consumer side: it is probably orthogonal. The `directness` axis (`direct / derived / temporal / aggregate`) describes *how a finding was produced*, not *what it may be claimed to support*. Those can come apart:

- A proxy-observation can be `direct` (a dashboard widget firing on its own raw query is "direct" in production but "proxy" in claim-support — `direct` does not promote it to consequence-bearing).
- A consequence-observation can be `derived` (a downstream-impact finding inferred from aggregated user-error logs is "derived" in production but "consequence" in claim-support).

If `directness` (or any existing axis) can in fact answer the claim-support question for *every* finding NQ emits today, then the right outcome is recognition: name the field, NS reads it, no new wire shape. If it cannot, the right outcome is a new orthogonal axis. The advisory expects NQ to make that call; it does not prescribe the answer.

This advisory does not prejudge whether NQ's response is "recognize" or "extend." Either outcome is valid; only refusing to engage with the seam — or collapsing claim-support into an existing axis that cannot actually carry the weight — would be a problem. There is no sign of either.

## Cross-references

- [FEATURE-HISTORY § SLICE_4_CLOSURE_CANDIDATE V1](FEATURE-HISTORY.md#slice_4_closure_candidate-v1-partial-gate-1) — the slice that filed this advisory and the place where the eligible path stays unreachable until NQ moves.
- [pre-positioned-doctrine-gates.md § Gate 1](pre-positioned-doctrine-gates.md) — the doctrinal frame for what Gate 1 ultimately needs to enforce.
- `crates/nightshiftd/src/closure.rs` — the predicate that will pick up the new testimony shape once the wire carries it. Slice 4 doc comments name the dependency explicitly.
- NQ-side memory pointers (consumer-side references; nq-claude has the producer-side equivalents):
  - `project_nq_witness_daemon_trajectory` — NQ's four-verb layering (witness / evaluator / consequence / cannot_testify).
  - `project_nq_directness_axis` — the direct/derived/temporal/aggregate axis already on NQ findings; possibly orthogonal to claim-support, possibly overlapping.

## How this advisory is closed

One of:

1. **NQ ratifies and ships a wire shape.** NS responds by adding the consequence-bearing check inside `closure::assess`, flipping `EligibleForClosureReview` from unreachable to reachable, and updating Slice 4 FEATURE-HISTORY with the unlock receipt.
2. **NQ ratifies the recognition that the distinction already exists** in an existing field. NS responds by reading that field directly and updating `closure::assess` to consult it.
3. **NQ declines to act and the forcing conditions never fire.** The advisory stays in `working/decisions/` as a named seam; NS continues operating with `Unassessable` as the default. Retirement only if NQ explicitly closes the seam as not-going-to-build.

Until one of those, this doc stays here. It is mutable until NQ engages; once NQ has read it, future amendments cite NQ's response.

---

## Closeout 2026-05-27 — outcome (2) with a layer-boundary refinement

NQ engaged. The response sharpened the seam in a way that closes the advisory but also forces a small correction back on the NS side.

### NQ's response (paraphrased; NQ-side record at `docs/working/gaps/NQ_CLAIM_SUPPORT_RECOGNITION.md`)

> NQ findings are by design substrate-state observations. NQ does NOT produce "consequence-bearing testimony" or "downstream-effect" testimony — those are application-layer or above. If NS's predicate needs consequence-bearing testimony, NQ findings may not be the right input for that role at all. They can be substrate-state inputs that NS's assessment builds on top of, but the consequence-bearing claim sits one layer up.
>
> Practical answer if NS agrees: read `cannot_testify + claim_kind + finding_kind`; encode the mapping table on NS's side; no NQ wire change needed today.

### NS's response

**NS agrees with the disagreeable claim.** The advisory's original framing leaked across a layer boundary that the NQ side now makes visible:

- NQ is a substrate-witness daemon. Its job is to testify about substrate state.
- Gate 1's "consequence-channel witness (customer-impact, downstream-effect)" is application-layer or above. That is not NQ's lane and should not become NQ's lane — pushing NQ to mint consequence claims would turn it into a workflow-governance surface, not a witness daemon.
- "Closure-eligibility requires distinguishing proxy-channel evidence from consequence-channel evidence" is true. But the consequence-channel half is a different layer's testimony, not a different shape of NQ finding.

So the originally suspected blocker — "NQ needs to add channel classification on findings" — was a misdiagnosis. The real blockers are two distinct layers:

| Layer | What NS needs | Available today? |
|---|---|---|
| Substrate-side claim-support | A local mapping over NQ's existing `cannot_testify + claim_kind + finding_kind` so NS can name what kind of substrate support a NQ finding provides | **Yes** — NQ already exposes these. NS owes the mapping. Small follow-on slice; not on the runtime ladder today but cheap when a forcing case fires. |
| Consequence-side witness | A non-NQ evidence source carrying application/business-layer claims | **No** — no such source exists in NS's input set. Adding one is a larger architectural question (input-source contract, who produces it, how it's reconciled, how it interacts with the existing Slice 5 contract). Not on the runtime ladder; parked. |

### What changes on the NS side

1. **Closure-candidate variant renamed.** `ClosureCandidate::UnassessableMissingChannelClassification` is renamed to `UnassessableMissingConsequenceWitness`. Render label updated to `"unassessable (missing consequence-witness)"`. The old name encoded a falsified theory ("NQ needs a field"); the new name names what's actually missing (consequence-witness from a non-NQ source). Backward-compat preserved via `#[serde(alias = "unassessable_missing_channel_classification")]` so any Slice-4-era stored packets still deserialize.
2. **FEATURE-HISTORY § SLICE_4 "Deferred"** refined: the two parallel unlocks (substrate-side mapping; consequence-witness source) are named explicitly, with the consequence-witness unlock parked as the larger architectural question.
3. **Gate 1 annotation in `pre-positioned-doctrine-gates.md`** updated: the "still missing" piece is consequence-witness from a non-NQ source, not NQ wire shape.

### What does NOT change

- NQ wire shape. `EligibleForClosureReview` stays unreachable; the variant remains in the enum as the honest design-space anchor.
- AG involvement. Still two rungs out (NQ → NS → Wicket → AG), still not in this room.
- The advisory's filed posture. Mutable for amendments citing further cross-project conversation, but the question this advisory asked is answered.

### Why this is outcome (2) and not (1) or (3)

- **Not (1)** — NQ does not ratify shipping a new wire shape. None is needed today.
- **Outcome (2) holds** because the distinction NS wanted is *partially* implicit in existing NQ fields (`cannot_testify + claim_kind + finding_kind`) — specifically the substrate-side half. The consequence-witness half was a different question that NQ correctly refused to absorb into its surface.
- **Not (3)** because NQ engaged. The forcing conditions stay live as future triggers if the cross-project picture shifts.

### Tripwire for re-opening this advisory

If, in future cross-project work, a non-NQ consequence-witness source becomes real (incident-tracking import, customer-impact telemetry, business-layer dashboard adapter, etc.), the `EligibleForClosureReview` path becomes reachable and `closure::assess` will need a new input. That is a *new* advisory, not a re-opening of this one — the producer/consumer edge will be different (NS ↔ new-source, not NS ↔ NQ).
