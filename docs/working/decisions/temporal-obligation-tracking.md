## Working note: Temporal obligation tracking — NQ owns axes, Night Shift watches them move

> **Status: SUPERSEDED 2026-05-29 (same day).** Promoted to
> [`../gaps/GAP-temporal-obligation-tracking.md`](../gaps/GAP-temporal-obligation-tracking.md).
>
> The note's promotion condition (1) fired the same day the note
> was written. A cross-workspace reconcile pass (per
> [[feedback-cross-workspace-reconcile-before-summary]]) discovered
> NQ had already filed `~/git/notquery/docs/integration/WORKLOAD_PHASE_WITNESSES.md`
> (2026-05-28) and `~/git/notquery/docs/working/gaps/PRESSURE_HARM_LOSS_RECOVERABILITY_GAP.md`
> (2026-05-29). The grammar exists; the open question is the NS-side
> consumer design. The GAP carries that question forward with explicit
> boundary discipline (workload phase may qualify posture / attention /
> urgency; must not mint action permission).
>
> This note is preserved unchanged below for provenance — what
> framing was distilled out of the original cut, which keepers
> mapped to existing corpus vs. were genuinely new, which sketches
> were held as transitions-not-nouns candidates. The GAP carries the
> live content; this note carries the trail.
>
> **Do not read past this header for current state. Read the GAP.**

---

> **Status (original):** candidate working note. Filed 2026-05-29.
> Distilled from an operator-supplied "Nightshift cut" draft on workload-phase
> witnesses and temporal obligation tracking. Most of the cut's substance is
> already covered by existing GAPs and CLAUDE.md framing; this note preserves
> only the *new* pieces and points upward for the rest. **No implementation,
> no enum commitments, no GAP.** A record is not authorization to build
> (per global "YAGNI scope" / project "name early, ratify lazily").

## Why this is named now

The cut named one sentence sharply enough that it deserves a handle:

> **NQ preserves the axes of testimony. Night Shift tracks the temporal
> behavior of those axes.**

Existing corpus has the *individual* failure modes (cascading debt, silence
laundering, recoverability windows, soak-vs-snapshot interpretation) but
does not have the axis statement as a single load-bearing line. Without it,
NS's lane is described case-by-case and the boundary against APM / metrics
DB / Prometheus collapse is unnamed.

Naming the candidate is cheap. Ratifying it lazily is the discipline.

## Provenance: what came in from the cut, what stayed where

The cut's content map relative to corpus, so future readers do not re-mint:

| Cut section / keeper | Already lives at |
|---|---|
| "Notice when the kind of work has changed" | `CLAUDE.md` line 12 (top-level framing) |
| "Work done is not progress made" / spike vs trend vs regime shift / bucket migration | `working/decisions/GAP-solution-family-exhaustion.md` (mitigation chain → bucket migration keepers) |
| Pressure / harm / loss / recoverability as separable axes; recoverability has an expiration date | Governor `GOV_GAP_TOLERABILITY_HORIZON_001` v1 (7-value enum); piped through NS via paired `--horizon-policy` + `--governor-socket` (see `FEATURE-HISTORY` → `HORIZON_CLI_PIPE_THROUGH V1`) |
| Silence ≠ success; degradation must say what degraded | `working/gaps/GAP-silence-aware-posture.md` + Slice C.1 anti-laundering trio (`silence_present ≠ incident_absent`, `acked_silence ≠ acked_incident`, `no_new_evidence ≠ resolved`) |
| Imported-basis time ≠ observation time; "recoverable until when" | `working/gaps/GAP-imported-basis-freshness.md` (`captured_at` cannot launder upstream observation time) |
| Ack ≠ closure; silence needs a reason or expiry | `CLAUDE.md` invariant 14 + `working/gaps/GAP-attention-state.md` + `architecture/GAP-reack-doctrine.md` |
| Refusal ≠ failure; silence ≠ health | Slice C.1 + `working/gaps/GAP-silence-aware-posture.md` |
| Labelwatch as forcing specimen | `working/decisions/GAP-autonomous-execution-boundary.md`, `GAP-solution-family-exhaustion.md`, `GAP-architectural-promotion-boundary.md`, `GAP-workflow-routing-boundary.md` |
| "NS proposes, Governor authorizes" / "trigger ≠ authority" | `CLAUDE.md` invariant 1 + `GAP-autonomous-execution-boundary.md` |

Anything not in the table is genuinely new and lives in the next section.

## What is genuinely NS-native here

### 1. The axis statement

> **NQ preserves the axes of testimony. Night Shift tracks the temporal
> behavior of those axes.**

This is the NS-native lane in one sentence. It composes with — does not
replace — the channel split (`working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md`,
"health is a subject, not an axis") by giving NS a positive description of
what it adds on top of NQ's witness grammar: *temporal interpretation of
axis movement*, not new axes.

Candidate. Held here, not promoted to `CLAUDE.md` until a second forcing
case lands that the existing framing cannot carry.

### 2. Anti-collapse boundaries

Night Shift must not become:

- an APM product
- a metrics database
- a host telemetry exporter
- a re-implementation of NQ
- a Prometheus replacement
- a generic alert manager
- a per-application dashboard (e.g. labelwatch's soak board)

These are listed because the gravity well exists: once NS consumes
workload-phase witnesses and tracks them across windows, every adjacent
need ("can it also export metrics?", "can it host the soak board?") is one
incremental yes away. The list is anti-yes scaffolding.

This list is candidate; it is not yet promoted to the `CLAUDE.md` "Don't"
section. Promotion condition: an actual edit or feature request lands in
the repo that would step over one of these lines.

### 3. Workload-phase witness consumption (consumed, not minted)

Future input shape candidate from NQ / app emitters:

```text
workload_phase_observation.v0
```

Per `feedback_nq_canonical_terminology.md`: NQ mints canonical names for
shared concepts; NS translates to language convention; NS does not coin
local synonyms for shared concepts. The grammar of this witness — phase,
observed_start/end, outcome, progress/pressure/harm/loss/recoverability
indicators, substrates touched, `cannot_testify` — is **NQ's to define**.

NS-side concern is only: what would NS *consume* off such a witness, and
what temporal-interpretation surface would it expose to the operator
(packet fields, ledger events, peek output)? That design is not done.

This subsection records the consumption-side question without designing
the witness. If/when NQ files the grammar, NS files its consumption GAP
and references this note as the pre-positioned breadcrumb.

### 4. Labelwatch soak as temporal-interpretation specimen

Existing GAPs cite labelwatch as an *autonomous-execution* witness, an
*exhaustion* witness, a *promotion-boundary* witness, and a *routing*
witness. None cite it as a *temporal-interpretation* witness, which the
cut's Day-1 / Day-3 / Day-4 / Day-5 walk-through does cleanly:

```text
Day-1:  structural fix landed; drops zero; checkpoint debt isolated.
Day-3:  wal_busy up; 2-pass drain ratio up; last_completed stalled.
Day-4:  Day-3 trend reverted; last_completed advanced.
Day-5:  raw drops high; unique affected subjects low; recoverability intact.
```

NS's value is not knowing any single day. NS's value is preserving the
temporal interpretation across them:

- Day-3 was not a regime shift (because Day-4 reverted).
- Day-5 raw drops were pressure, not broad unrecoverable loss
  (because unique-DID loss stayed low and recoverability stayed intact).
- Residual checkpoint debt remained named but did not earn action.

Held here as a forcing specimen for whatever consumption-side GAP this
note eventually justifies. Not promoted into any existing GAP because
the temporal-interpretation framing is the new piece.

## Class vocabularies: sketches, not committed surface

The cut proposed posture, trend, debt, progress, and recoverability
class enums. Holding these explicitly as **sketches**, not committed
surface, per `feedback_transitions_not_nouns.md` ("do not mint a
primitive without naming the transition it governs") and the
narrowing-posture precedent (refused `work_phase` / `narrowing_role`
enum fields for the same reason).

Sketches preserved for retrieval; transitions named where they exist;
gaps flagged where they do not.

### posture (sketch)

```text
HOLDING   WATCH   ESCALATE   EXPIRED   CANNOT_TESTIFY
```

Transition candidates:
- `HOLDING → WATCH` when a window-over-window delta crosses a named
  threshold (which one? not yet pinned).
- `WATCH → ESCALATE` when a tripwire fires; tripwire shape is
  Governor-bound, not NS-minted.
- `* → EXPIRED` when a recoverability horizon closes without recheck.
- `* → CANNOT_TESTIFY` when upstream evidence is `Stale` or
  `Invalidated` per Slice 5 contract.

`CANNOT_TESTIFY` overlaps `PostureClass::Unknown` from Slice C.1 — if
this sketch ever ratifies, that overlap must be resolved, not duplicated.

### trend_class (sketch)

```text
stable   spike_then_revert   monotonic_degradation
step_change   oscillating_within_band   unknown
```

`spike_then_revert` is already a *transition* (between two windows), not a
noun — keep as-is. The others are nouns describing what a window-sequence
*is* and beg for transitions: "when does monotonic_degradation become
step_change?", "when does oscillating_within_band become regime shift?".
Do not ratify until those transitions have witnesses.

### debt_class (sketch)

```text
none   isolated   bounded_recurring   cascading   unbounded
```

The transitions `isolated → bounded_recurring → cascading` are the
load-bearing ones; the *category labels* are scaffolding. The
solution-family-exhaustion GAP already names the bucket-migration
transition explicitly. If this ever ratifies, it likely composes with
that GAP rather than standing alone.

### progress_class (sketch)

```text
completed   advanced   repeated_prefix   stalled   drained
abandoned_due_to_window_close   unknown
```

`repeated_prefix` is the labelwatch `update_author_day` discovery
(processed=2/7 every cycle was repetition, not progress). The keeper
"Work done is not progress made" is the transition rule; the enum is
the way to express the rule on the wire if/when wire-format need lands.

### recoverability_class (sketch)

```text
not_needed   recoverable   recovery_pending
recovery_window_closing   recovery_expired   unknown
```

Overlaps directly with `GOV_GAP_TOLERABILITY_HORIZON_001`'s 7-value
enum — likely subsumed there, not duplicated here. Held as a sketch so
the overlap is documented, not as a parallel surface.

## Keepers that survived distillation

Already promoted; do not re-mint:

> **Night Shift notices when the kind of work has changed.**
> (`CLAUDE.md` line 12)

> **Night Shift stops unresolved work from pretending it resolved itself.**
> (close paraphrase of `CLAUDE.md` line 12 "prevents unresolved work
> from masquerading as resolved")

Held here as candidate keepers; not promoted yet:

> **NQ preserves the axes. Night Shift watches them move.**

> **A daily witness says what happened. A soak witness says whether
> yesterday's worry survived contact with today.**

> **Recoverability has an expiration date.**
> (subsumed by tolerability horizon; held here as a retrieval handle
> for the NS-side framing)

> **Pressure is not harm. Harm is not loss. Loss is not unrecoverability.**

> **Debt is not failure until it compounds faster than the system can
> pay it.**

## Non-goals

- Defining `workload_phase_observation.v0`. That is NQ's grammar to mint.
- Adding any posture / trend / debt / progress / recoverability enum to
  packet, ledger, or wire format. These are sketches.
- Promoting "NQ owns axes; NS watches them move" to `CLAUDE.md` until a
  second forcing case demands it.
- Adding an integration file at `docs/integration/`. No `docs/integration/`
  directory exists; the project uses `architecture/` for ratified design,
  `working/gaps/` for open spec-shaped GAPs, and `working/decisions/` for
  candidate doctrine. This note belongs in `working/decisions/`; if a
  consumption GAP is later justified, it belongs in `working/gaps/`.
- Implementing a metrics DB, soak board, or any host telemetry surface.

## Promotion conditions

This note becomes a GAP when:

1. NQ mints `workload_phase_observation.v0` (or its eventual name) and
   NS has a concrete consumer to design, **or**
2. A second labelwatch-shape soak fires in a different ops domain and
   surfaces a temporal-interpretation gap that the existing GAPs cannot
   carry, **or**
3. An operator review identifies a real packet / ledger / peek surface
   where NS could expose "this window vs. last window" without an enum
   commitment, and the design space justifies a GAP.

Until then: candidate. Held for retrieval, not implementation.
