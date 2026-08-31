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

Provider-capacity evidence is fail-closed in V1. The foreman profile retains a
budget policy reference, but the journal does not retain an exact FUEL
observation or decision. The projection therefore says
`NOT_RECORDED_BY_FOREMAN` and does not infer that a policy was evaluated.

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
filesystem name. Every non-GET method returns 405. There is no approval,
answer, dispatch, retry, resume, cancel, execute, close, merge, or promotion
control. The browser renders packet intent, live mechanism state, and accepted
outcome or absence as three visibly separate regions.

Unknown source extensions remain available only through exact raw bytes. An
unknown typed scheduler value cannot be invented by Casework: the owner
foreman contract must first admit it. This projection adds no authority,
currentness extension, overall result, or target effect.
