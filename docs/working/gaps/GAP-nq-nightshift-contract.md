# GAP: NQ -> Night Shift Finding Contract

> Status: identified, not specified. First artifact to nail down before
> Watchbill MVP. The seam must be clean before Night Shift internals
> matter.

## Core question

> What does NQ expose that lets Night Shift produce a review packet
> without learning NQ's internals?

That is the dogfood seam. Everything else in Night Shift hangs off it.

## Why this contract first

NQ is the evidence substrate. Night Shift is the consumer. If the
contract is right, Night Shift starts as a tiny consumer instead of a
whole new empire wearing a false mustache.

If the contract is wrong, Night Shift ends up reimplementing NQ logic
upstream — which is the exact failure mode the project's thesis
prohibits.

## Key design rule

NQ findings are **evidence, not commands.**

Even if NQ eventually pushes `wal_bloat persisted for 4 generations`,
Night Shift must treat it as:

```text
activation hint → reconcile current state → produce proposal
```

Not:

```text
alert says bad → run repair
```

The second is pager-driven Calvinball with JSON.

## Minimum finding snapshot

NQ should provide a stable finding snapshot shape. Exact wire format
TBD, but the fields Night Shift needs:

```yaml
finding_key: nq:wal_bloat:labelwatch-host:/var/lib/labelwatch.sqlite
  # stable identity: source + detector + host + subject

finding_id: nq_f_...                    # per-observation id
generation: 12847                       # NQ generation counter
source_db_hash: sha256:...              # NQ snapshot integrity
snapshot_captured_at: 2026-04-17T03:00:00Z

detector: wal_bloat
host: labelwatch-host
subject: /var/lib/labelwatch.sqlite
severity: warning                       # info | warning | critical
status: active | pending | resolving | clear
domain: "Δg"                            # NQ failure domain
regime_hint: accumulation               # optional

first_seen_generation: 12843
last_seen_generation: 12847
persistence_count: 5
recovery_count: 0
recovered_at: null                      # rfc3339 if recovered

summary: "WAL 512.5 MB (12.3% of db)"
value: 512.5
unit: mb

evidence_rows:                          # detector-specific; what NQ saw
  - key: wal_size_mb
    value: 512.5
  - key: db_size_mb
    value: 4156.0
  - key: checkpoint_lag_s
    value: 43200
```

## Pull contract (MVP)

Boring, CLI-friendly:

```bash
nq findings export --format json
nq findings export --detector wal_bloat --host labelwatch-host
nq findings export --changed-since-generation 12800
nq findings export --finding-key nq:wal_bloat:labelwatch-host:/var/lib/labelwatch.sqlite
```

HTTP equivalent later. `GET /api/findings` already exists; this is
asking for a slightly sharper query surface.

Pull first. Push later. Push is seductive. Seductive things usually
have incident reports.

## Stable identity

Finding identity must survive across generations.

```text
finding_key = source + detector + host + subject
```

Same `finding_key` across successive observations = same finding, with
evolving state. This is how Night Shift tells "same thing persisted"
from "new thing wearing a similar costume."

The existing `warning_state` table in NQ tracks `first_seen`,
`consecutive_gens`, `acknowledged`, etc. — that's exactly the right
shape. Night Shift needs a read API that surfaces it.

## Transition events (future)

Eventually NQ should expose transitions as first-class events:

```text
new
persisted
escalated
resolving
recovered
flapped
stale
regime_shift
```

MVP derives these by diffing consecutive snapshots. Push-based transitions
are covered in `GAP-nq-activation.md`.

## `resolving` status — temporal literacy

NQ already exposes a `resolving` state: the condition is still part of
the story but its trajectory has turned. This is not the same as
`clear`, and Night Shift must not treat it that way.

> `resolving` preserves the scar. It says: don't page, don't forget,
> don't pretend nothing happened. Keep watching until the system earns
> closure.

Night Shift's default behavior when encountering `resolving`:

```text
Finding: labelwatch log source silent
Status: resolving
Action bias: observe / no escalation
Packet: note trajectory, continue monitoring, do not propose repair
        unless recovery stalls
```

This avoids both failure modes the alert surface is supposed to prevent:

- **panic automation**: "bad, do thing"
- **greenwashing**: "not currently bad, erase context"

A `resolving` finding should not trigger `recurrence_after_repair`
escalation, because the prior repair is the reason trajectory turned.
If `resolving` stalls (no change over N generations while still
non-clear), Night Shift may escalate with a *stall* trigger, distinct
from recurrence.

## Cross-reference to NQ's own gap specs

Three NQ gaps that pre-structure this contract, flagged by NQ's Claude:

- **DASHBOARD_MODE_SEPARATION_GAP** (live vs snapshot at UI layer) ↔
  Night Shift's `capture` vs `reconcile`. Same discipline, different
  altitude. The NQ spec should cite Night Shift's reconciler as the
  scheduler-layer instance of the pattern.
- **OBSERVER_DISTORTION_GAP (Δq)** (participation manifest) ↔
  Night Shift's authority ladder
  (`observe → advise → stage → request → apply → publish → escalate`).
  Night Shift's ladder is a cleaner articulation; NQ's Δq spec should
  cross-reference.
- **PORTABILITY_GAP** (capability honesty) ↔ Night Shift's
  Python-workflow-boundary invariant ("Python can be weird without being
  sovereign"). Same anti-sovereignty discipline.

These three substantiate the "same substrate" reading: Night Shift is
not a bolt-on, it is the consumer that makes NQ's gap specs prove
they're real.

## Labelwatch case: SQLITE_BUSY framing

Live ops example from 2026-04-15 labelwatch incident:
`discovery_stream.py` crashed on `SQLITE_BUSY`, treating recoverable
contention as corruption — "over-applied loudness."

Same failure class as stale-snapshot rendering: "over-applied
present-tense authority." Sibling misframings.

This is the kind of thing Night Shift should produce well:

```text
Finding: sqlite_busy contention
Classification: recoverable contention / loudness mismatch
Reconciled: contention window observed but not persistent
Proposal: inspect writer/reader behavior; add bounded retry/backoff;
          avoid restart unless contention persists past threshold
Authority: advise only
Mutation blocked.
```

Not "restart the service because SQLITE_BUSY looks scary."

## Reconciliation axes: identity, churn, semantic state

A finding snapshot exposes three independent axes. The reconciler
treats them differently. Conflating them is the most reliable way to
make reconciliation operationally useless.

**1. Identity** — `finding_key` (`source + detector + host + subject`).
Stable across generations. Identity drift means a different finding,
or NQ's encoding changed under us; either way, the captured evidence
no longer applies. Reconciler verdict on identity drift: `invalidated`.

**2. Churn** — `snapshot_generation`, `last_seen_at`,
`persistence_generations`, and `evidence_hash`. Advances every NQ
scan cycle that resurfaces the finding. Churn is *evidence the
finding is still live*; it is not evidence its meaning changed. In
steady-state observation against a long-lived finding, churn fields
move every minute (matching NQ's scan cadence) and `evidence_hash`
flips with them.

**3. Semantic state** — `severity`, `evidence_state`, `status`,
`regime_hint`, and the detector-specific evidence rows. These are
the fields that answer "did the meaning of this finding change?"
The reconciler's `changed` verdict reads only this axis.

### What this means for `evidence_hash`

`evidence_hash` is **strict payload integrity**, not semantic-change
detection. It proves the exported byte stream changed; it does not
prove the finding's meaning changed. Because the hash covers the
churn axis as well, it flips on every NQ scan tick (~1/min) for
every long-lived finding. That is correct for an integrity hash
and operationally useless as a standalone `changed` predicate.

Validated against the live `/opt/notquery/nq.db` on 2026-04-20: 8
long-lived findings; back-to-back snapshots ~15 minutes apart showed
every finding's `evidence_hash` flip in lockstep with
`snapshot_generation` advancing exactly 15 ticks, while identity
fields and lifecycle anchors (`first_seen_at`, `severity`,
`evidence_state`) stayed stable. Exactly the shape the three-axis
split predicts.

### Reconciler verdict semantics

Given the three axes, the reconciler emits one of four verdicts per
captured input:

- **`committed`** — same `finding_key`, current snapshot retrieved,
  semantic axis unchanged. Churn may have advanced; that's expected
  and not by itself a reason to escalate.
- **`changed`** — same `finding_key`, current snapshot retrieved,
  **semantic axis** differs from the captured snapshot. The diff
  records which fields moved.
- **`stale`** — current snapshot retrieval failed (NQ unreachable,
  schema mismatch, transport error, query timeout). Capture-time
  evidence is preserved but no longer revalidated; promotion ceiling
  drops to advise.
- **`invalidated`** — finding absent from NQ at current generation,
  `finding_key` no longer present, NQ's identity scheme changed
  under us, or a declared scope assumption no longer holds.

`evidence_hash` participates as a *cheap inequality check*: if it
matches the captured value, the finding is trivially `committed`
without reading the semantic axis. If it differs, the reconciler
must read the semantic axis explicitly to choose between
`committed` (churn-only) and `changed` (real semantic movement).

### Future: a dedicated semantic fingerprint

A `semantic_fingerprint` — a hash over only the semantic-axis fields
— would let the reconciler skip the explicit field comparison when
nothing semantic changed. Not required for the first reconciler
slice; a documented comparison set is enough to start. If introduced
later, it lives alongside `evidence_hash`, never replacing it. They
answer different questions.

## What Night Shift does with the contract

Given a finding snapshot, Night Shift:

1. Reads it into the bundle as `observed`
2. Stores the captured snapshot (including `evidence_hash` and the
   semantic-axis fields) for reconciliation
3. At run time, re-pulls the current snapshot for the same `finding_key`
4. Applies the three-axis comparison above
5. Marks the input `committed` | `changed` | `stale` | `invalidated`
6. Produces a packet that quotes the evidence, notes what changed on
   the semantic axis (and what merely churned), and proposes next
   steps within the agenda's promotion ceiling

No mutation. No direct action on NQ's data.

## MVP build order (inside the Watchbill MVP)

1. NQ export contract (this doc)
2. Night Shift capture bundle
3. Night Shift reconciliation against current NQ state
4. Packet output
5. Governor observe/advise receipt
6. Only then consider stage/request

No push. No repair. No MCP. No Code mode. No "while we're here."

The first demo:

```bash
nightshift watchbill run wal-bloat-review
```

```text
Captured NQ finding.
Reconciled current state.
Finding still active.
Persistence increased from 4 → 5 generations.
Likely regime: accumulation / pinned reader.
Recommended next step: inspect active readers; do not restart yet.
Promotion ceiling: advise.
Mutation blocked.
Receipt written.
```

More than enough.

## Open questions

- CLI surface or HTTP-first? (CLI probably. HTTP is already partly
  there; CLI is cheaper to stabilize.)
- Stable wire format: JSON vs. protobuf? (JSON for v0.)
- Where does `finding_key` canonicalization live? (NQ owns it.)
- Does NQ expose `source_db_hash` today, or does it need to? (Needs to,
  for reconciliation integrity.)

## For NQ's Claude session

This document is Night Shift's ask. Relevant NQ-side work:

- Review the `warning_state` table exposure path — is the read API rich
  enough to support `--changed-since-generation`?
- Add `finding_key` canonicalization (or confirm existing scheme)
- Add `source_db_hash` or equivalent snapshot-integrity field
- Consider a sharper CLI surface for export (`nq findings export ...`)
- Cross-reference from DASHBOARD_MODE_SEPARATION, OBSERVER_DISTORTION,
  PORTABILITY gap specs to Night Shift's reconciler / authority ladder /
  python-boundary invariants

Memory reference: the three-gap resonance with Night Shift is saved in
the observatory-family Continuity workspace (2026-04-16).
