# GAP: Silence-Aware Posture (Slice C of DURABLE_ARTIFACT_SUBSTRATE consumption)

> Status: **design — not ready for implementation.** Filed 2026-05-19
> as the design output of the Slice B closeout audit. Slice C is the
> next deliberate move after Slice B, but it has at least one soft
> doctrine blocker and one explicit naming-collision question that
> must be resolved before code lands. This file is the design surface
> for those decisions; the implementation slices (C.0 / C.1 / C.2)
> follow only after the open questions below are settled.

## The deep rule

> **NQ owns truth and evidence classification. Night Shift owns
> posture, attention, and ack obligation. A silence-shaped finding
> may alter notification/ack posture, but MUST NOT imply absence of
> incident, recovery, safety, or active resolution.**

The keeper:

> **A silence ack is not an active-finding ack.**

Companions (hostile-to-boolean-laundering — these are non-negotiable):

> **`silence_present ≠ incident_absent`** — a finding that says
> *"the producer stopped extracting"* says nothing about whether
> active incidents exist; it only says the channel has gone quiet.

> **`acked_silence ≠ acked_incident`** — acknowledging that you
> know the producer is silent is not acknowledging the underlying
> incident the producer was reporting. The two findings carry
> distinct `finding_key`s; their ack lineages must not merge.

> **`no_new_evidence ≠ resolved`** — absence of new evidence is
> absence of new evidence. It is not recovery. It is not safety.
> It is not active resolution.

## Audit findings (Slice B closeout, 2026-05-19)

### What already exists on the wire

- `nq.finding_snapshot.v1` carries a `silence` envelope
  (`scope`, `basis`, `duration_s`, `expected`) per the
  SILENCE_UNIFICATION shared contract. **V1 populates this only on
  `extraction_stale`.** Six legacy silence detectors (`stale_host`,
  `stale_service`, `*_witness_silent`, `signal_dropout`,
  `log_silence`) omit the envelope pending their own migration.
  Per the SILENCE_UNIFICATION rule: **absence of envelope means
  "not yet unified," not "not silence."**
- Slice A plumbed `FindingSilence` through NS's internal model
  (`FindingSnapshot.silence`, `FindingSummary.silence`, peek
  output, packet receipt). Visibility-only — no posture branching.

### What already exists in NS posture / attention machinery

- `AttentionState` enum:
  `Unowned | Acknowledged | Investigating | HandedOff | WatchUntil | Silenced`
  — note `Silenced` means *operator-silenced*, not
  *evidence-shape-is-silence*. **This is a naming collision Slice C
  must resolve before code lands.**
- `Attention` field kit (per `GAP-attention-state.md`):
  `acknowledged_at`, `ack_expires_at`, `re_alert_after`,
  `silence_reason`, plus the WatchUntil triple (`tolerance_basis_id`,
  `tolerance_basis_hash`, `re_alert_after`).
- `attention_key = finding_key`. **Mechanical ack-lineage separation
  is already in place by construction:** different finding_keys ⇒
  different attention rows ⇒ acks do not bleed across findings.
  Slice C does not need to invent finding-level ack separation; it
  needs to make the *posture distinction visible* so a human or
  downstream system doesn't ack the silence finding under the
  belief that it satisfies an active finding's ack obligation.
- `GAP-attention-state.md` invariants already in repo:
  - *"Silence is not handling"* — about operator suppression, but
    structurally relevant to silence-shape too.
  - *"Ack is not closure"* — TTL discipline applies to silence acks
    as well.
  - *"Suppression needs an expiry or a reason."*

### What's NOT in repo text but is referenced

- **Re-ack doctrine** — "re-ack = mini re-triage with typed
  disposition" (per `memory/project_reack_doctrine.md`). Referenced
  in `GAP-autonomous-execution-boundary.md` and
  `GAP-solution-family-exhaustion.md` but never filed as repo
  doctrine. Slice C wants to extend ack semantics by class; a
  load-bearing extension of an unfiled doctrine is a soft blocker.

### Slice B's relation to Slice C

Slice B handles a different problem and the two must not merge:
- **Slice B (`imported_producer_basis_stale`):** the *clock* on an
  ingested finding is too old. Drives `EvidenceState::Stale` +
  Slice 5 advise(revalidate-only).
- **Slice C (silence-shaped):** the *content* of the finding is
  about absence of expected observation. Should not drive Stale;
  should drive a distinct silence posture.

A single producer can simultaneously emit:
- an ingested finding whose `origin.producer_extraction_time` is
  ancient (Slice B's domain), and
- a separate `extraction_stale` finding from NQ's own clock (Slice
  C's domain).

These are two findings with two finding_keys and two ack lineages.
Default: do not merge them.

## v1 scope

**Slice C v1 silence-shaped =** `snap.silence.is_some()`. That is,
exactly the findings NQ surfaces with the SILENCE_UNIFICATION
envelope populated. In NQ V1 that means only `extraction_stale`.

Per the SILENCE_UNIFICATION rule, **absence of the envelope is not
classified as "not silence."** A legacy silence detector
(`stale_host`, etc.) that omits the envelope must be treated as
*posture-unknown / cannot-classify*, not as incident-shaped.

Forward-compat: when NQ migrates a legacy silence detector, the
envelope appears on the wire and Slice C automatically classifies
the finding as silence-shaped. No NS-side allowlist; no per-detector
config.

## Core invariants

1. **NQ owns truth.** Slice C does not change which findings exist,
   which severities they carry, or what conditions trigger them. NS
   reads NQ's wire shape; it does not contest NQ's classification.

2. **Posture distinction does not flip evidence semantics.** A
   silence-shaped finding's `current_status` continues to come from
   NQ. Slice C does not set `EvidenceState` based on silence-shape.
   In particular, silence-shape does NOT set `EvidenceState::Stale`
   — that is Slice B's pathway and stays distinct.

3. **`finding_key` separation is the ack-lineage primitive.** Two
   findings with two finding_keys have two attention rows. Slice C
   does not invent cross-finding ack semantics; it surfaces the
   posture-class so consumers can tell which kind of finding they
   are looking at.

4. **Absence of envelope is not non-silence.** Per the
   SILENCE_UNIFICATION rule, missing envelope means *not yet
   unified*. v1 Slice C surfaces such findings as
   `posture_class = unknown` and refuses to claim they are
   incident-shaped.

5. **Silence does not imply absence, recovery, or safety.** The
   three boolean-laundering refusals above are invariants, not
   stylistic preferences. Slice C surfaces them in the data model
   and in the test contract.

6. **Slice 5 three-axis split is preserved.** NQ owns truth; NS
   owns posture + ack obligation. Slice C extends what NS does in
   the *posture + ack* lane; it does not enter NQ's lane.

7. **No active-finding resolution.** Emergence of a silence-shaped
   finding does not resolve, downgrade, or auto-recover any active
   finding. Each finding lives or dies on its own evidence.

## Proposed data model

The minimum new surface needed to express the distinction without
laundering booleans:

```rust
/// Posture class derived from the finding's *shape*, not its
/// individual content. Read NQ silence envelope to classify.
///
/// Distinct from `AttentionState`, which is the operator's view of
/// the finding's lifecycle. Posture is about *what kind of finding
/// this is* (incident vs absence-shaped); attention is about *what
/// the operator has done about it*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostureClass {
    /// Active-condition findings — the bulk of NQ's emissions.
    /// `silence` envelope absent AND the finding represents a live
    /// observation about a condition that is happening.
    IncidentShape,
    /// Findings whose content is *about* absence of expected
    /// observation. v1: `snap.silence.is_some()`. Notification
    /// language and ack lineage differ from IncidentShape.
    SilenceShape,
    /// Reserved for findings whose shape cannot be inferred from
    /// the current wire surface — primarily legacy NQ silence
    /// detectors that have not yet migrated to the SILENCE_UNIFICATION
    /// envelope. **NOT a default for IncidentShape.** Treats the
    /// absence as `not yet unified`, not as `not silence`.
    Unknown,
}
```

The class is **derived**, not stored on the wire. Derivation rule:

```text
snap.silence.is_some()                              → SilenceShape
known_legacy_silence_detector(snap.finding_key)     → Unknown
                                                      (post-NQ-migration: SilenceShape)
otherwise                                            → IncidentShape
```

The `known_legacy_silence_detector` predicate is a small allowlist
keyed on detector name (`stale_host`, `stale_service`,
`*_witness_silent`, `signal_dropout`, `log_silence`). It exists so
NS does not silently classify legacy silence findings as
incident-shaped. Once NQ migrates a detector, the envelope appears
and the legacy branch becomes unreachable for that detector — the
allowlist entry can then be removed.

## Proposed ack / re-ack semantics

**Mechanical separation is already in place** (different
finding_keys → different attention rows). Slice C's contribution is
making the posture-class visible *on the attention surface* so
consumers can see which class an ack belongs to without inspecting
the underlying finding.

Add one field to `packet::Attention`:

```rust
/// Posture class of the finding this attention row attaches to.
/// Surfaced so consumers (operator UIs, downstream readers, future
/// re-ack tooling) can distinguish silence-shaped acks from
/// incident-shaped acks without re-classifying from the snapshot.
///
/// Per `GAP-silence-aware-posture.md`: a silence ack is not an
/// active-finding ack. The two are separated mechanically by
/// finding_key; this field surfaces the distinction by class.
pub posture_class: PostureClass,
```

**Re-ack semantics under Slice C** (depends on whether re-ack
doctrine lands in repo text — see Open Questions):

- A re-ack on a silence-shaped finding is *a mini re-triage of the
  silence claim*: "do I still believe the producer is meaningfully
  silent, or has the silence become routine and the finding should
  age out?"
- A re-ack on an incident-shaped finding is *a mini re-triage of
  the incident*: "is this still active, has it changed, has it
  recovered?"
- The two re-ack flows produce different `disposition` values when
  filed. Slice C does not invent a re-ack mechanism; if re-ack
  doctrine ratifies into repo text, Slice C extends `disposition`
  with class-aware values. If re-ack remains in memory only, Slice
  C falls back to surface-only (posture_class visible; re-ack
  unchanged).

## Proposed receipt fields

Slice C adds posture-class visibility on three surfaces:

**On `bundle::ReconciliationResult`** (audit trail per result):

```text
posture_class: PostureClass   # always present, derived per-result
```

**On `packet::FindingSummary`** (operator-facing summary):

```text
posture_class: PostureClass   # mirrors the result for the target
```

**On `packet::Attention`** (per-finding attention state):

```text
posture_class: PostureClass   # the class the ack belongs to
```

The freshness receipt from Slice B is unchanged. Posture and
freshness are orthogonal:

| | Posture: Incident | Posture: Silence | Posture: Unknown |
|---|---|---|---|
| Freshness: Fresh | normal advise/propose | silence-shaped notify | cannot-classify notify |
| Freshness: Stale (B.2) | revalidate-only | silence + revalidate-only | revalidate-only |
| Freshness: CannotAssess | normal advise/propose | silence-shaped notify | cannot-classify notify |

The combinations are not exotic — they are the existing Slice 5 +
Slice B postures with the silence label layered on top. Slice C does
not introduce new blocking semantics; it disambiguates the existing
ones.

## Notification language

The regime string (`packet.diagnosis.regime`) gains a silence-shaped
variant. Proposed prefixes (one of these must be picked before
implementation — see Open Questions):

```text
incident-shape regime prefixes: "committed: …", "changed: …",
                                "stale: …", "invalidated: …"
silence-shape regime prefixes:  "silence: …", "silenced-evidence: …",
                                "basis-silent: …", "absence: …"
```

`ProposedAction.steps` for silence-shaped findings should:

- name what is silent (e.g., the producer, the extraction run, the
  detector scope)
- name the silence duration (`silence.duration_s`)
- propose verification of *whether the silence is expected* (e.g.,
  declared maintenance, scheduled snapshot cadence)
- **NOT** propose remediation of the underlying-incident-that-
  might-or-might-not-exist
- **NOT** propose marking findings recovered

## Minimal acceptance test list

Eight tests, in three families. Names use the `c_` prefix.

**Family 1 — posture-class derivation:**

1. `c_silence_envelope_present_classifies_as_silence_shape`
2. `c_active_incident_classifies_as_incident_shape`
3. `c_legacy_silence_detector_without_envelope_classifies_as_unknown`
   — the **anti-laundering sentinel** for the "absence of envelope
   ≠ not silence" rule. Uses a synthesized `stale_host`-shaped
   finding with no silence envelope; asserts `posture_class =
   Unknown`, not `IncidentShape`.

**Family 2 — ack-lineage separation:**

4. `c_silence_ack_does_not_transfer_to_active_finding`
   — two findings with distinct finding_keys; ack on the silence
   finding's attention row does NOT mark the active finding as
   acknowledged.
5. `c_active_ack_does_not_transfer_to_silence_finding`
   — symmetric.
6. `c_posture_class_surfaces_on_attention_for_both_kinds`
   — both attention rows carry the correct `posture_class`.

**Family 3 — boolean-laundering refusals (the doctrine fences):**

7. `c_silence_does_not_resolve_active_findings`
   — emission of an `extraction_stale` finding for producer X does
   NOT alter the `EvidenceState` or `AttentionState` of any active
   finding (from any producer). The sentinel disarms
   `silence_present ⇒ incident_absent`.
8. `c_silence_does_not_imply_recovery_or_safety`
   — silence-shaped finding's `Attention.evidence_state` does not
   become `Recovered`. `ProposedAction.steps` for silence-shaped
   findings do NOT contain "resolved," "safe," "recovered," or
   "no action needed."

## Non-goals (explicit and load-bearing)

Slice C does NOT:

- change NQ truth semantics
- infer recovery, resolution, or safety from any silence-shape
- auto-resolve, downgrade, or close any active finding based on
  silence-shape
- implement support for the six legacy silence detectors (they
  classify as `Unknown` until NQ migrates them)
- generalize Slice D (full two-clock semantics across liveness gate
  / packet emission / horizon)
- alter Slice B behavior in any way — silence-shape and
  stale-imported-basis are orthogonal
- introduce new blocking semantics — `ok_to_proceed` rules are
  Slice 5's domain; Slice C does not flip the bool
- rename or restructure `AttentionState::Silenced` (see Open
  Question 2)
- file the re-ack doctrine into repo text (see Open Question 1)
- change `OperationalUrgency` or `severity` rendering — those come
  from NQ and policy, not from posture-class
- add an `ack_class` enum on top of `posture_class` (the class
  attaches to the attention row, not to a separate ack object)

## Open questions / blockers

These must be answered before implementation. The first is the soft
blocker that justifies "spec-first only" instead of "ready to
implement."

### 1. ~~**Re-ack doctrine: file it or fall back?**~~ *(resolved 2026-05-20)*

**Resolved 2026-05-20:** the maintainer chose the **hybrid path** —
file re-ack doctrine as a prerequisite repo artifact, then run
Slice C.1 as **surface-only** under that doctrine.

`GAP-reack-doctrine.md` was filed 2026-05-20 as a sibling GAP and
this file's references now point at it. Slice C.1 is no longer
blocked by Open Q1; it proceeds under the surface-only fallback
(option b in the original framing): `PostureClass` visible on
`ReconciliationResult`, `FindingSummary`, and `Attention`; regime
prefix and notification language updated; **no disposition-enum
extension** in v1.

Class-aware disposition values (silence-ack vs incident-ack
dispositions on `GAP-reack-doctrine.md`'s six-value enum) remain a
deferred extension conditional on operator review of v1 re-ack
flows. Not in scope for Slice C.1 or any near-term commit.

### 2. **`AttentionState::Silenced` naming collision.** *(naming question)*

The existing `AttentionState::Silenced` variant means
*operator-silenced* (suppression with reason + TTL per
`GAP-attention-state.md`). Slice C's `PostureClass::SilenceShape`
means *evidence-shape-is-silence*. Two different concepts, similar
names.

Options:

- **(a) Accept the collision.** `AttentionState::Silenced` keeps
  its existing meaning; `PostureClass::SilenceShape` is a separate
  field. Document the distinction in both struct docs. *No
  rename.*
- **(b) Rename `AttentionState::Silenced` → `OperatorSilenced`.**
  Clearer but is a breaking rename that touches every
  serialization of attention state.
- **(c) Rename `PostureClass::SilenceShape` → `AbsenceShape` or
  `BasisSilent` or `SilenceContent`.** Preserves
  `AttentionState::Silenced`; renames Slice C's term.

**Recommendation: (a) for v1.** Document carefully in both
locations; if confusion shows up in operator review, revisit.

### 3. **Regime prefix vocabulary.** *(naming question)*

Pick one before implementation:
- `"silence: …"` — short, but collides with `silence_reason` (an
  operator-silencing field on Attention).
- `"silenced-evidence: …"` — explicit but verbose.
- `"basis-silent: …"` — names the structural property.
- `"absence: …"` — emphasizes the anti-incident framing.

**Recommendation: `"silence: …"` with a comment in the regime
prefix table noting the distinction from `silence_reason`.**

### 4. **Should silence-shape affect `OperationalUrgency`?**

Defaults today: severity comes from NQ; urgency is derived per the
existing urgency function. Slice C could either:
- Leave urgency untouched and let posture-class be visibility-only.
- Subtract one urgency level for silence-shape (silence is
  inherently *meta*-evidence and rarely critical-in-itself).

**Recommendation: leave urgency untouched in v1.** Add a comment
flagging the question. If operator review shows silence findings
generating false-positive criticality, revisit.

### 5. **Should the `legacy_silence_detector` allowlist live in code or config?**

Six detector names hardcoded vs. an `agenda::legacy_silence_detectors`
config. Hardcoding is simpler; config is more flexible if NQ adds
new legacy-shape detectors before the migration.

**Recommendation: hardcoded constant array in `freshness.rs` (or
a new `posture.rs`).** Six entries is small; the migration is a
known timeline; YAGNI for a config knob until a forcing case.

### 6. **Posture and Slice B freshness composition: receipt order?**

Both Slice B and Slice C add receipts to `ReconciliationResult`.
Receipt-rendering order matters only for human readability. Picks:
- posture first, then freshness
- freshness first, then posture

**Recommendation: freshness first (existing order), posture second.**
Receipts are listed in slice-landing order; matches commit history.

## Recommendation: Slice C.1 may proceed (surface-only)

**Updated 2026-05-20** after `GAP-reack-doctrine.md` landed.

Open Question 1 is **resolved**: re-ack doctrine is now in repo
text. The hybrid path was chosen — file re-ack first, then run
Slice C as surface-only under that doctrine.

**Slice C.1 (surface-only) may proceed.** Scope:

- `PostureClass` enum (`IncidentShape | SilenceShape | Unknown`)
  added with the derivation rule and the legacy-detector allowlist.
- `posture_class` field added to `ReconciliationResult`,
  `FindingSummary`, and `Attention`.
- Regime prefix and `ProposedAction.steps` language updated for
  `SilenceShape` findings.
- Eight acceptance tests across the three families (derivation /
  ack-lineage / boolean-laundering refusals).
- **No disposition-enum extension.** `GAP-reack-doctrine.md`'s
  six-value enum stays frozen for v1. Class-aware disposition
  (silence-ack vs incident-ack dispositions) is a downstream
  extension conditional on operator review.

Open Questions 2–6 (naming / ordering choices) should be settled
in the Slice C.1 spec commit, not as separate decisions:

- Q2: accept the `AttentionState::Silenced` collision; document
  carefully (recommended).
- Q3: regime prefix `"silence: …"` (recommended).
- Q4: leave `OperationalUrgency` untouched (recommended).
- Q5: hardcoded legacy-detector allowlist (recommended).
- Q6: freshness first, posture second (recommended).

**Recommended next move:** spec-first commit (`GAP-silence-aware-
posture.md` is already that spec — just amend to reflect Q1
resolution if needed), then scaffolding + red tests, then
implementation. Three-commit pattern as for Slice B.

## Provenance

- Filed 2026-05-19 as the design output of the Slice B closeout
  audit.
- Builds on Slice A (visibility) and Slice B (clock freshness).
- Sibling doctrine: `GAP-attention-state.md`,
  `GAP-imported-basis-freshness.md`.
- Sibling doctrine: re-ack (`GAP-reack-doctrine.md`, filed
  2026-05-20). The memory provenance
  (`memory/project_reack_doctrine.md`) is preserved as the
  pre-promotion source.
- No code, no tests, no schema changes in this commit. The design
  surface cools in repo text before any implementation commits.
