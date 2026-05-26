# Class-aware disposition: design-space exploration

**Status:** candidate / non-binding. Working note. Explores the design space for what *would* become Slice C.2 of the silence-aware-posture chain, **without extending the frozen six-value disposition enum** in `GAP-reack-doctrine.md`. No code, no schema, no contract change is proposed here.

**Filed:** 2026-05-26
**Companion:** `GAP-silence-aware-posture.md` (Slice C.2 deferral), `GAP-reack-doctrine.md` (frozen six-value enum), memory `project_substrate_consumption_chain.md` (three unlock triggers).

## Why this file exists

Slice C.2 is deferred until one of three unlock triggers fires (operator review surfaces a confused-disposition incident / NQ migrates a legacy silence detector / a second posture class lands). The deferral is correct for *shipping*: extending a frozen public-contract enum without a forcing case creates real downstream migration cost.

The deferral is **wrong for thinking**. Exploring the per-class disposition design space in a working note is cheap, brakes survive, and being prepared means the unlock-case doesn't force the design under time pressure. The three sub-questions are concrete and grounded in actual repo state — this is exactly the work that should happen *before* pressure fires, not after.

See `feedback_shipping_vs_thinking_discipline` for the framing correction that produced this file: "wait for forcing case" applies to *shipping* (extending the enum), not to *thinking* (sketching what the extension would want to look like).

## Frozen six-value enum (for grounding)

Per `GAP-reack-doctrine.md` v1, the disposition enum is:

1. `advanced` — work moved forward
2. `unchanged-waiting-on-X` — no progress, blocked on named axis
3. `blocked-on-Y` — actively blocked on Y
4. `handed-off` — re-assigned
5. `escalated` — kicked upward
6. `resolved/no-longer-valid` — closed

These were minted for incident-shape (active condition the operator is managing). The frozen state is load-bearing — Slice C.1 explicitly preserved it as a non-goal of C.1 implementation.

## Sub-question 1: per-class disposition vocabulary

**The question:** Are silence-ack dispositions a subset, superset, or parallel set of incident-ack dispositions? `resolved` works for an active incident — does it work for a silence claim that is itself *about* absence?

### Background

The six values were minted under the implicit assumption that the finding describes an *active condition the operator is managing*. Silence-shape findings invert that assumption: the finding describes *the absence of expected observation*. The operator is not "managing the silence" the same way they are "managing the incident."

Worked example: a `extraction_stale` finding fires because a durable-artifact extractor hasn't run in 30 days. Operator acks with disposition `resolved` — what did they mean?

- (a) The extractor ran and the silence ended (basis-condition cleared)?
- (b) The silence is confirmed expected (the corpus is dormant; this finding should not have fired)?
- (c) The witness has been removed from coverage (the silence is no longer NQ's concern)?
- (d) Something else?

The three semantics are operationally distinct. Today, `resolved` collapses them — which is the kind of confused-disposition incident that would fire the first unlock trigger.

### Candidate shapes (not ranked)

**(A) Subset.** Silence-ack uses fewer of the six. The unused ones simply don't fire for silence-shape findings.

- What would have to be true: most of the six values translate cleanly to silence-shape, and only one or two (e.g., `advanced`) don't apply. Likely if "managing the silence" is mostly the same workflow as "managing the incident" minus a few cases.
- Cost: small. Acceptance is "this disposition can't fire for this posture class." Validation at the boundary.
- Risk: hides ambiguity in the values that *do* translate (e.g., `resolved` still collapses the three semantics above).

**(B) Parallel set.** Silence-ack has its own six values; incident-ack keeps its six. Different vocabulary, possibly same arity.

- Candidate silence-ack values (sketch): `silence-confirmed-expected` / `silence-pending-restoration` / `silence-out-of-scope` / `silence-elapsed` / `escalated` / `cleared`.
- What would have to be true: silence workflows are *operationally distinct enough* that reusing incident vocabulary would be misleading more often than it would be informative.
- Cost: medium. Two enums to maintain. Cross-class flow under migration (Sub-Q2) becomes more involved.
- Risk: drift — the two enums evolve independently, the relationship between them becomes folklore.

**(C) Superset.** Incident-ack uses the existing six; silence-ack uses those six *plus* additional silence-specific values. The six remain the lingua franca.

- Candidate additions: `silence-confirmed-expected`, `silence-pending-restoration`, `silence-out-of-scope`.
- What would have to be true: there's a small handful of silence-only cases that don't fit any of the six, but the six otherwise translate fine.
- Cost: small to medium. The enum grows; downstream consumers may or may not branch on the new values.
- Risk: enum bloat — the discipline that minted six values in the first place erodes as classes accrue.

**(D) Same names, class-aware semantics.** The six values are kept; their meaning is documented per class. `resolved` on incident-shape means "condition cleared"; `resolved` on silence-shape means "silence ended OR silence expected OR scope changed" with a required free-text note.

- What would have to be true: the *operational* difference between incident-ack and silence-ack is too small to warrant new vocabulary, but the *semantic* difference matters for audit.
- Cost: small in code (no enum change); large in documentation discipline.
- Risk: collapses the audit signal — "all our silence findings get resolved" becomes uninformative.

### Open questions (not for resolution here)

- Does the operator's *mental* model distinguish silence-ack from incident-ack today, or does C.1's `PostureClass` surfacing already give them the distinction without enum extension?
- Does NQ's `silence_expected` field already discriminate the (a)/(b)/(c) cases in §Background? If so, the disposition may not need to.
- Cross-shape: what does an `Unknown` posture-class finding's disposition look like? Same as incident by default? Or "unknown" gets its own value?

## Sub-question 2: cross-class re-ack flow under finding migration

**The question:** If a finding's `PostureClass` flips between regenerations (e.g., NQ migrates a legacy silence detector, and an existing `Unknown` becomes `SilenceShape`), does the prior ack survive, force a re-ack, or get archived?

### Background

C.1 invariant: `PostureClass::Unknown` is "not yet unified" — absence of the SILENCE_UNIFICATION envelope, not absence of silence. When NQ migrates one of the six legacy silence detectors to emit the envelope, every existing `Unknown` finding from that detector flips to `SilenceShape` on the next snapshot. Existing acks on those findings were minted against incident-ack semantics (the only ack flow C.1 surfaces). What happens to them?

The re-ack doctrine v1 invariant "re-ack-on-context-change" is load-bearing here: a class flip is unambiguously a context change. But the doctrine doesn't specify *whether* a class flip *itself* invalidates the prior ack, or only flags it for the operator's attention.

### Candidate shapes (not ranked)

**(A) Class change → forced re-ack.** Any `PostureClass` flip invalidates the prior ack. Operator must re-ack with class-appropriate disposition before the finding is considered handled.

- What would have to be true: class is part of the ack's identity — an incident-ack literally cannot apply to a silence-shape finding.
- Cost: operator burden during migration windows (every flipped finding requires re-ack).
- Risk: alert-storm during the migration itself; operator fatigue causes blind-re-ack and the discipline degrades.

**(B) Preserve ack + flag.** Prior ack survives with a `class_changed_since_ack: true` flag. The flag surfaces in operator UI; operator chooses whether to re-ack.

- What would have to be true: the prior ack is meaningful even under the new class — e.g., "I acknowledged this finding 3 weeks ago and haven't acted; the class flip doesn't change that I know about it."
- Cost: small in code; clarity-cost in the UI (flag-not-action).
- Risk: flag rot — same shape as the "ack without TTL" problem the re-ack doctrine v1 already refused. Composes with [[shipping-vs-thinking-discipline]] — flag is a working signal, not an action gate, so it doesn't ship discipline.

**(C) Archive prior + fresh state.** Class change archives the prior ack into a history record; the finding starts fresh as an unhandled item with the new class.

- What would have to be true: ack's class is so tightly bound to its meaning that crossing the class boundary breaks the ack's premise entirely.
- Cost: archive shape needs to exist (Sub-Q3). Operator loses continuity.
- Risk: "fresh state" hides the migration event — operator sees a new finding, may not connect it to the prior one.

**(D) Asymmetric: silence → incident forces re-ack; incident → silence preserves with flag.** Escalation-shaped class transitions require re-ack (treat as new); de-escalation-shaped preserve (treat as continuity).

- What would have to be true: class transitions have implicit severity-direction. Plausible — incident is "more urgent" than silence in most operator mental models, so an incident-emerging-from-silence demands fresh attention more than the reverse.
- Cost: more semantics to remember; the "direction" of class flips is a new concept.
- Risk: introduces a hidden ordering on classes. Once N classes exist (third posture class is one of the unlock triggers), the asymmetric matrix grows.

### Open questions (not for resolution here)

- Are the six legacy silence detectors *expected* to all flip during a single NQ migration window, or one at a time? Cadence affects which candidate is operationally viable.
- Does the `PostureClass::Unknown → SilenceShape` flip happen *atomically* (same snapshot generation) or with a gap (the finding stops appearing, then reappears with new class)? Affects whether the prior ack is still in scope at the moment of flip.
- Composition with re-ack doctrine v1's "ack-lineage-scope" invariant: does ack lineage *include* class, or does it cross class boundaries?

## Sub-question 3: disposition-history retention

**The question:** v1 stores latest ack only. A class-aware disposition extension may want history for audit. Storage-schema change.

### Background

`Attention` today carries `acknowledged_at`, `ack_expires_at`, `re_alert_after`, and (per the C.1 surface-only build) `PostureClass`. There is no history of *prior* acks. If a finding has been acked four times across two class flips, only the most recent ack is visible.

For audit, the latest-only shape is lossy. For operational use, it's clean. The question is whether class-aware disposition makes the history load-bearing.

### Candidate shapes (not ranked)

**(A) Status quo: latest only.** Class change overwrites; no history. C.1 ships this shape.

- What would have to be true: history is never load-bearing for operator action. Audit is satisfied by the run-ledger event trail (re-ack events are already logged there per re-ack doctrine).
- Cost: zero.
- Risk: the run ledger is per-run, not per-finding. "How many times has *this finding* been re-acked?" requires cross-run aggregation.

**(B) Append-only log of ack events.** Every ack appends to an ack-history table (or JSON column). Carries `acked_at`, `disposition`, `posture_class`, `actor`, optionally `note`.

- What would have to be true: audit value of per-finding ack history exceeds the storage / query cost. Operator review use case is plausible (per unlock trigger 1).
- Cost: storage schema change (new table or column). Per-finding query becomes cheap; cross-finding aggregation is independent.
- Risk: ack-spam (one finding with 50 acks) — needs bounding or compaction.

**(C) Bounded ring buffer.** Last N acks (e.g., N=5) retained; older acks discarded. Compromise between (A) and (B).

- What would have to be true: history is useful but recency matters more than completeness. Most operator review will reference the last 1–3 acks.
- Cost: schema change is bounded (fixed-width JSON column or N-row table).
- Risk: discards exactly the kind of ack-pattern that would surface "this finding has been chronically re-acked for six months" — the case that probably needs the audit most.

**(D) Per-class latest.** Keep latest-per-class rather than latest-overall. If finding has been incident-shape, then silence-shape, then incident-shape again, keep three latest acks (one per class transition).

- What would have to be true: class is the primary axis the audit cares about — the operator reviewing wants to know "how did this finding get acked in its silence phase vs its incident phase," not "every ack ever."
- Cost: schema change is moderate (per-class lookup).
- Risk: aligned to a current class model that may not survive the third posture class landing.

### Open questions (not for resolution here)

- Does the run-ledger already carry enough ack-event history that a per-finding history is redundant? Cross-run aggregation cost is the variable.
- If we adopt (B) or (D), is the history mutable (operator can edit a prior ack note) or strictly append-only? Append-only composes better with audit doctrine; mutable composes better with operator workflow.
- Retention policy: does ack history get GC'd when the finding's `EvidenceState` transitions to `Invalidated` (Slice 5)? Or does it persist for some retention window beyond the finding's life?

## Composition notes

- **C.2 is a doctrine extension on `GAP-reack-doctrine.md`, not a new doctrine.** All candidates here preserve the re-ack doctrine v1 invariants: re-ack-on-context-change, silence-ack ≠ active-ack, stale-ack ≠ invalidated-ack, ack ≠ resolution/safety/freshness/truth.
- **Posture-class derivation stays NQ-shaped.** C.1's invariant "NQ owns truth, NS owns posture + ack" is preserved across all candidates. Per-class disposition is an NS-side ack extension; it does not affect NQ's classification.
- **C.1 surface-only is unchanged.** Whatever C.2 looks like, it does not retroactively change C.1's wire shape — Slice C.1 ships `PostureClass` visible on `ReconciliationResult` / `FindingSummary` / `Attention` without disposition extension, and that ship is permanent.

## Pickup cheat sheet

When one of the three unlock triggers fires:

1. **Operator review surfaces a confused-disposition incident.** The incident itself names which Sub-Q is biting. Read the candidate shapes for that Sub-Q; the operator's confusion almost certainly maps to one of (A)/(B)/(C)/(D). The remaining work is verifying which.
2. **NQ migrates a legacy silence detector.** Sub-Q2 is now live (existing `Unknown` findings flip to `SilenceShape`). Read the four candidates for cross-class re-ack flow; pick the one that matches the operational shape of the migration cadence (one detector at a time vs all-six-at-once).
3. **Second posture class lands.** All three Sub-Qs need re-evaluation under N ≥ 3 classes. Sub-Q1's vocabulary debate becomes more involved; Sub-Q2's transition matrix grows; Sub-Q3's per-class history (candidate D) gets a forcing case.

In every case: the design space is captured. The forcing case picks within it; it does not start from scratch.

## What this file is NOT

- Not a spec. No acceptance criteria, no schema, no enum extension proposed.
- Not a ratification of any candidate. All four candidates per sub-question are co-equal sketches.
- Not a roadmap. No commitment to implement Slice C.2 in any form.
- Not a hidden enum extension. The frozen six-value enum stays frozen.

If a future commit converts this file into a spec, that commit should be explicit about the forcing case (which unlock trigger fired, what the incident was), and the file's name and status should change. Until then, this is annex work — explore freely, name uncertainty, do not commit to public surface.

## Provenance

- 2026-05-26: filed as design-space exploration. Trigger: the C.2 deferral was about to be re-relayed in a doc-reorg discussion as "wait for forcing case" / "fake rigor," which is the wrong gate for annex work. See `feedback_shipping_vs_thinking_discipline` for the correction. No code/schema/contract change in this commit.
