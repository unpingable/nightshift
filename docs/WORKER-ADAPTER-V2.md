# Nightshift Worker Adapter Protocol V2

Status: TUNNEL-FINCH generic Nightshift owner contract. It is a bounded local-agent compute protocol and grants no target-effect authority.

The combined closed schema is `schemas/nightshift.worker-adapter.v2.schema.json`. Start requests bind adapter ID, adapter version, protocol, packet/run/work-item/attempt identities, workspace, provider/model class, limits, and the deterministic worker-brief digest. The only admitted commands are `capabilities`, `start`, `resume`, `status`, and `collect`; there is no approval-response operation.

## Exact worker brief

`nightshift.worker-brief-basis/v2` is an RFC 8785/JCS record with a maximum total size of 16 MiB and at most 1,024 direct predecessor receipts. Its digest domain is `nightshift.worker-brief.digest/v2` followed by NUL and the exact canonical brief bytes. V2 has an independent digest namespace.

The brief retains exact orientation-packet source bytes and every exact direct-dependency terminal or not-started receipt byte sequence. Each byte sequence carries a domain-separated retained-raw digest and lowercase hexadecimal encoding. Unknown receipt extensions survive byte-for-byte but acquire no scheduler or adapter semantics. Digest-only predecessor references are not usable worker evidence.

For ergonomics, the brief carries recursively closed recognized-contract wrappers for the selected packet work item, packet global constraints, and execution-profile work-item entry. Each wrapper contains only its exact contract identifier and canonical JSON string. The consuming validator checks the recognized values against the retained packet and start request, verifies packet integrity, requires predecessor keys to equal direct dependencies, validates receipt kind/digest/binding, and enforces total/count bounds. Attempt preparation executes that validator before resource claims or journal mutation.

The store obtains packet and predecessor BLOB lengths first and computes the exact canonical JSON expansion size using closed metadata before loading or hexadecimal expansion of predecessor BLOBs. Oversized input fails closed.

## Adapter admission and resource bounds

Start requests are independently bounded at 86,400 seconds and 16 MiB output, matching execution-profile ceilings. A separate adapter-contract verifier consumes exact canonical capability bytes and binds their raw digest plus adapter ID, protocol, version, executable identity, profile digest, work-item execution entry, and start-request digest. A standalone capability shape does not establish admission.

## Lifecycle and authority boundary

Adapter events and terminal receipts retain exact provider, model, session, thread, turn, and queue identity custody. Identity fields freeze when first observed. A provider completion observation or process exit is not a result; only an exact identity-bound terminal receipt can become the worker outcome. A waiting-approval event is testimony only and receives no response or protected effect. Resume retains the same attempt identity, and terminal occurrences are not retried.
