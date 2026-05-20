# GAP: Re-Ack Doctrine

> Status: **ratified doctrine, no implementation.** Filed 2026-05-20
> as the explicit prerequisite for Slice C.1 (Silence-Aware Posture)
> per `GAP-silence-aware-posture.md` Open Question 1. Promoted from
> `memory/project_reack_doctrine.md` after three prior GAPs
> (`GAP-autonomous-execution-boundary.md`,
> `GAP-solution-family-exhaustion.md`,
> `GAP-silence-aware-posture.md`) referenced "the re-ack doctrine"
> by name without a discoverable repo home. No code, no schema, no
> tests in this commit — doctrine only.

## Keepers

> **Re-ack is a mini re-triage, not a courtesy tap.**

> **A re-ack without reassessment is just institutionalized self-soothing.**

> **Ack is operator testimony about attention and handling. It is not NQ truth.**

## What ack is, and what re-ack is

Two distinct operations on the same attention row:

- **Ack** — *seen; owner established; immediate next step named.*
  An ack is the operator's first-touch testimony: I observed this,
  I am taking it, this is what happens next. Ack carries a TTL
  (`ack_expires_at`) per `GAP-attention-state.md`; it does not
  carry indefinite force.

- **Re-ack** — *renewed situational understanding.* Required when
  the operative posture or context has changed enough that the
  prior ack no longer testifies to the current obligation. A
  re-ack must resolve to a typed disposition:

  ```
  advanced
  unchanged-waiting-on-X
  blocked-on-Y
  handed-off
  escalated
  resolved/no-longer-valid
  ```

  Missing-disposition on a re-ack is a contract violation, not a
  default. v1 freezes this six-value list. Slice C's class-aware
  disposition extension is a *non-goal* of this doctrine commit
  (see §"What this does not authorize").

## Invariants

The doctrine, as ratified:

1. **Ack-lineage scope.** An ack is scoped to a *finding lineage*
   (the `finding_key` axis per `GAP-attention-state.md`), not to
   an abstract condition. An ack on finding-A does not bind
   finding-B even if both findings describe related conditions.

2. **Re-ack obligation on context change.** A re-ack is required
   when the operative posture or context changes enough that the
   prior ack no longer testifies to the current obligation. The
   six-axis table in §"What ack must not collapse" enumerates the
   axes whose change forces a re-ack.

3. **Silence ack ≠ active-finding ack.** An ack on a silence-shaped
   finding (`PostureClass::SilenceShape` per
   `GAP-silence-aware-posture.md`) does not satisfy the ack
   obligation of any active-incident finding, even when the two
   findings describe the same producer / host / subject /
   timeframe. Different finding_keys, different ack lineages.

4. **Active-finding ack ≠ silence ack.** Symmetric. An ack on an
   active-incident finding does not satisfy the ack obligation of
   a related silence-shaped finding. The two surfaces report
   structurally different things: incident-shape reports what the
   producer said; silence-shape reports that the producer stopped
   saying it.

5. **Stale / revalidate-only ack ≠ invalidated / blocking ack.**
   An ack on a finding in the Slice 5 advise(revalidate-only)
   pathway (`InputStatus::Stale`, including the Slice B
   imported-producer-basis-stale case) does not satisfy the ack
   obligation if that finding later transitions to
   `InputStatus::Invalidated` and starts blocking. The ack lineage
   is preserved per finding_key, but the *posture class* changed
   and a re-ack is required.

6. **Ack ≠ resolution.** Acknowledgement of a finding does not
   mark the underlying condition resolved. Resolution is an NQ
   evidence claim (`EvidenceState::Recovered` arising from NQ's
   own observation), not an operator testimony.

7. **Ack ≠ safety.** Acknowledgement does not testify that the
   condition is safe to ignore, safe to proceed past, or safe to
   exclude from future considerations. Ack records attention; it
   records nothing about risk surface.

8. **Ack ≠ evidence freshness.** Acknowledgement of a finding
   under fresh evidence does not preserve admissibility when the
   evidence goes stale. Slice B's imported-producer-basis-stale
   pathway flips `EvidenceState::Stale` regardless of prior acks;
   a re-ack against the new posture is required for continued
   attention coherence.

9. **Ack is operator testimony, not NQ truth.** The three-axis
   split (NQ owns truth; NS owns posture + ack obligation;
   `GAP-nq-nightshift-contract.md`) extends here. An ack writes
   NS-side attention state; it does not contest, override, or
   confirm NQ's classification.

## What ack must not collapse

Six axes that an ack does NOT carry forward. When any of these
changes between an ack and the next surfacing of the same
finding_key, a re-ack is the only honest operator response.

| Axis | Lives where | Re-ack trigger when… |
|---|---|---|
| `finding_key` lineage | `FindingKey` (stable across regenerations) | a different finding_key surfaces, even with related semantics |
| `posture_class` | `PostureClass` (Slice C, derived) | shape flips between incident / silence / unknown |
| `evidence_state` | `EvidenceState` on the snapshot or Attention | active → stale, fresh → stale, recovered → recurrence |
| `reliance_class` | `RelianceClass` on the result | authoritative → historical, hint → none, etc. |
| `proposed_action` / advice | `ProposedAction.kind` + `.steps` | advise → revalidate-only, advisory → staged, change of recommended steps |
| operator ack / re-ack obligation | `Attention.acknowledged_at` + TTL + disposition | prior ack expired, or any of the five axes above changed |

The table is **declarative**: any of the six axes shifting means
the prior ack no longer testifies to the current obligation. A
prompt-only re-ack ("yes, still seen") that does not name the
shifted axis and its new disposition is the
*institutionalized-self-soothing* failure mode this doctrine
exists to prevent.

## What this does NOT authorize

Per the spec brief and the cooling discipline:

- **No Slice C implementation.** This commit is doctrine only;
  Slice C.1 (surface-only posture-class visibility) follows.
- **No disposition-enum extension.** The six v1 disposition values
  above are frozen. Slice C's class-aware disposition (silence-ack
  disposition vs. incident-ack disposition) is a future extension
  conditional on operator review of v1 re-ack flows; it is *not*
  ratified by this commit.
- **No notification behavior changes.** Notification posture
  vocabulary (regime prefixes, ProposedAction step language) is
  Slice C.1's concern, not this commit.
- **No ack storage schema changes.** The existing field kit on
  `packet::Attention` (`acknowledged_at`, `ack_expires_at`,
  `re_alert_after`, `silence_reason`, plus the WatchUntil triple)
  is sufficient for v1 doctrine. Adding a `disposition` field or
  an ack-history log is a separate slice with its own design
  surface.
- **No two-clock generalization (Slice D).** The Slice B
  `producer_extraction_time` clock and the Slice D full two-clock
  semantics are out of scope here.
- **No NQ truth changes.** NQ owns evidence classification. This
  doctrine governs the NS-side ack lane only.

## Provenance

This doctrine existed as `memory/project_reack_doctrine.md` from
2026-04-23 onward (per memory frontmatter), referenced by name in:

- `GAP-autonomous-execution-boundary.md` §"GAP-attention-state.md
  and the re-ack doctrine" (and three other call sites)
- `GAP-solution-family-exhaustion.md` line 455 ("per the re-ack
  doctrine")
- `GAP-silence-aware-posture.md` Open Question 1 (the slice that
  forced the promotion)

The discoverability gap (referenced ≥ 3 times, never filed in
repo) crossed the threshold the *grep-before-governance*
discipline names: *"if only memory exists, weigh whether row
pressure or discoverability justifies surfacing into repo text."*
Three prior references is the row pressure; Slice C's load-bearing
dependence is the discoverability case. Promotion to repo text is
the consequence.

## Cross-references

- `GAP-attention-state.md` — sibling doctrine. The attention field
  kit (`acknowledged_at`, `ack_expires_at`, `re_alert_after`,
  `silence_reason`) lives there; this file extends it with the
  re-ack semantics and the six-axis collapse-refusal.
- `GAP-nq-nightshift-contract.md` — the three-axis split (NQ owns
  truth, NS owns posture + ack). This doctrine operates inside the
  NS-owned ack lane.
- `GAP-silence-aware-posture.md` — Slice C consumes this doctrine
  to populate `PostureClass` on attention rows and to assert the
  silence-ack ≠ active-finding-ack rule mechanically.
- `GAP-imported-basis-freshness.md` — Slice B's stale pathway
  composes with invariant 5 (stale ack ≠ invalidated ack); a
  re-ack is required when the same finding crosses the
  Stale→Invalidated boundary.
