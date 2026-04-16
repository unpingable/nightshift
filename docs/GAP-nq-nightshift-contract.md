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
status: active | pending | clear
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
recovered
flapped
stale
regime_shift
```

MVP derives these by diffing consecutive snapshots. Push-based transitions
are covered in `GAP-nq-activation.md`.

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

## What Night Shift does with the contract

Given a finding snapshot, Night Shift:

1. Reads it into the bundle as `authoritative`
2. Stores the `generation` and `source_db_hash` for reconciliation
3. At run time, re-pulls the current snapshot for the same `finding_key`
4. Compares `generation`, `persistence_count`, `severity`, `status`
5. Marks the input `current` | `changed` | `stale` | `invalidated`
6. Produces a packet that quotes the evidence, notes what changed, and
   proposes next steps within the agenda's promotion ceiling

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
