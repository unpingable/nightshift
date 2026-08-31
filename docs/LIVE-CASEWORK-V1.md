# Nightshift live Casework projection V1

`nightshift.casework-live-run/v1` is a read-only operator projection of one
explicitly registered durable foreman run. It is a separate family from the
immutable `nightshift.casework-run/v1` receipt projection. Neither family is
changed or promoted into the other.

## Source and transaction boundary

The Casework process receives startup-only pairs of an existing SQLite store
path and an exact foreman `run_id`. The run identity is never interpreted as a
pathname. Each HTTP request opens the admitted store through the foreman's
descriptor-retained, `mode=ro`, `query_only` path and takes one deferred read
transaction. The snapshot contains exact packet, admission, profile, ordered
journal, accepted receipt, resource, scheduler, and optional final-snapshot
bytes from one transaction history.

The owner reopens every retained journal row and accepted receipt before it is
projected. Capacity rows additionally pass a foreman-owned accessor that requires
the enclosing canonical internal-event bytes, nested typed capacity record, and
every nested exact source-byte sequence to agree before Casework can project them. It recomputes retained-raw and record digests, binds the raw record
identity to the SQLite row identity, and requires receipt kind, exact state,
classification, and digest to agree with deterministic foreman replay. A
required-table-compatible store with substituted row content is refused.
Packet, admission, and profile bytes are reopened and cross-validated before
replay; redundant run-table digests, times, and concurrency must equal those
exact contracts. A retained final snapshot is accepted only when its raw digest
equals the `RunClosed` digest and its canonical bytes reproduce from the exact
accepted receipts in the same read transaction.

The read path never initializes a store, changes journal mode, starts a write
transaction, or creates a missing database. Symlinks, non-regular files,
partial WAL sidecar custody, and pathname replacement are refused or remain
bound to the originally opened inode by the foreman owner law. Casework imports
only the narrow read snapshot function; it does not import the writer store.

## Projection and digest

The projection digest is SHA-256 over:

```
nightshift.casework-live-run.digest/v1\0 || RFC8785-JCS(projection without projection_digest)
```

The packet, admission, execution profile, append-only event timeline, resource
claims, attempt/adapter/provider identifiers, lane-local questions, and exact
accepted terminal or not-started kind remain separate fields. Scheduler-state
counts describe mechanism state only. They are not campaign classifications.
An accepted outcome is copied only from an accepted exact receipt; otherwise
the projection carries the explicit absence marker
`NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT`.

Provider-capacity evidence is fail-closed and evidence-derived. Legacy runs with no
journal-recorded capacity requirement or admissions retain the exact
`NOT_RECORDED_BY_FOREMAN` absence; a profile policy reference alone never proves
that a policy was evaluated. Capacity-required GAUGE stores project
`EXACT_RECORDED_BY_FOREMAN` only after the read-only consumer independently
reopens the canonical requirement, admission, observation, policy, and decision
bytes, matches each parsed value to the foreman typed duplicate, reproduces the
deterministic FUEL decision, and cross-binds packet, run, work item, attempt,
adapter, provider, model class, policy, journal sequence, and admission time.

The distinct mechanism region orders admissions by exact journal sequence and
shows provider and model identities, capacity state, admission disposition,
source class, confidence, observation disposition, observation/decision times,
view-time currentness, owner-domain record digests, and plain SHA-256 digests of
each exact retained source. Those facts say only what the journal recorded. They
are not a campaign result, a target-effect authority, or evidence that any
unrecorded FUEL observation influenced scheduling.

## Exact raw-source framing

Raw packet, admission, profile, event, accepted-receipt, and optional final
bytes remain downloadable through fixed GET routes. Plain SHA-256 identifies
each exact raw source; those byte digests are distinct from owner-domain record
digests.

The complete journal byte stream is:

```
"NIGHTSHIFT-FOREMAN-JOURNAL-FRAMING-V1\0"
|| repeated(sequence_u64_be || byte_length_u64_be || exact_event_bytes)
```

The accepted-receipt byte stream is:

```
"NIGHTSHIFT-FOREMAN-ACCEPTED-RECEIPTS-FRAMING-V1\0"
|| repeated(work_item_id_length_u64_be || work_item_id_utf8
            || receipt_length_u64_be || exact_receipt_bytes)
```

Receipt frames are ordered by the closed work-item identity map. A terminal
receipt and a not-started receipt retain distinct kinds and exact source bytes.
The Raw view exposes the optional final snapshot and a fixed exact-byte link for
each event in addition to the aggregate journal framing.

## Live and sealed relationship

A closed live run links to a sealed Casework V1 record only when both the exact
packet digest and the exact final-snapshot bytes equal that sealed record's
packet and receipt sources. Packet identity alone is insufficient. The live
projection is resealed after adding the relational link. The sealed V1 bytes
and schema remain unchanged, while the UI exposes reciprocal navigation using
the server-qualified live index.

## HTTP and UI boundary

The live routes are fixed descendants of `/api/v1/active-runs/{navigation_id}`:
detail, events, exact event bytes, and the fixed raw sources. The navigation ID
is a derived lowercase hexadecimal identifier; it is not the run ID or a
filesystem name. Question routes use a second derived identity over the exact
work-item ID plus lane-local question ID, so equal question IDs in different
lanes remain distinct. Every non-GET method returns 405. There is no approval,
answer, dispatch, retry, resume, cancel, execute, close, merge, or promotion
control. The browser renders packet intent, live mechanism state, and accepted
outcome or absence as three visibly separate regions.

Unknown source extensions remain available only through exact raw bytes. An
unknown typed scheduler value cannot be invented by Casework: the owner
foreman contract must first admit it. This projection adds no authority,
currentness extension, overall result, or target effect.
