# Nightshift Worker Adapter Protocol V2

Status: TUNNEL-FINCH generic Nightshift owner contract. It is a bounded local-agent compute protocol and grants no target-effect authority.

The combined closed schema is `schemas/nightshift.worker-adapter.v2.schema.json`. Start requests bind adapter ID, adapter version, protocol, packet/run/work-item/attempt identities, workspace, provider/model class, limits, and the deterministic worker-brief digest. The only admitted commands are `capabilities`, `start`, `resume`, `status`, and `collect`; there is no approval-response operation.

## Exact worker brief

`nightshift.worker-brief-basis/v2` is an RFC 8785/JCS record with a maximum total size of 16 MiB and at most 1,024 direct predecessor receipts. Its digest domain is `nightshift.worker-brief.digest/v2` followed by NUL and the exact canonical brief bytes. V2 has an independent digest namespace.

The brief retains exact orientation-packet source bytes and every exact direct-dependency terminal or not-started receipt byte sequence. Each byte sequence carries a domain-separated retained-raw digest and lowercase hexadecimal encoding. Unknown receipt extensions survive byte-for-byte but acquire no scheduler or adapter semantics. Digest-only predecessor references are not usable worker evidence.

For ergonomics, the brief carries recursively closed recognized-contract wrappers for the selected packet work item, packet global constraints, and execution-profile work-item entry. Each wrapper contains only its exact contract identifier and canonical JSON string. The consuming validator checks the recognized values against the retained packet and start request, verifies packet integrity, requires predecessor keys to equal direct dependencies, validates receipt kind/digest/binding, and enforces total/count bounds. Attempt preparation executes that validator before resource claims or journal mutation.

The store obtains packet and predecessor BLOB lengths first and computes the exact canonical JSON expansion size using closed metadata before loading or hexadecimal expansion of predecessor BLOBs. Oversized input fails closed.

## Canonicalization compatibility domain

Foreman runtime digest production remains solely on the established `serde_jcs` implementation; `serde_json_canonicalizer = 0.3.2` (MIT) is pinned only as an independent qualification oracle. The open extension-value surface is mechanically restricted, recursively, to object keys in the existing ASCII identifier alphabet, at most 64 members per object, and all numeric values in `[-9007199254740991, 9007199254740991]`. Fixed schema keys and dependency identifiers already occupy that same ASCII ordering subset. These bounds exclude the known UTF-16 key-order and integer-rounding divergence surfaces while retaining Unicode string values and finite non-integer numbers. All larger numeric magnitudes are refused, including exact non-integral decimal witnesses that a binary float could round; the admitted subset is canonicalize-parse-validate-recanonicalize closed.

The oracle vector covers exponent formatting, negative zero, Unicode UTF-16 key ordering, control escapes, and both safe-integer boundaries; the established serializer and oracle are byte-identical over the admitted vector. Unicode extension-object keys and integers at plus or minus `9007199254740992` are refused by owner validation. Qualification attaches the oracle to tests only; it is not a second production digest law.

## Exact byte digest laws

Retained packet and receipt source bytes use the preimage: ASCII `nightshift.foreman-retained-raw.digest/v1`, one NUL byte, then the exact source byte sequence without normalization. For exact bytes `{}`, the fixed result is `sha256:defbb1499ef874d99cdf029e5c1dc04dc253d0fc1e0f88f966278cf3934302fe`.

Capability custody uses the preimage: ASCII `nightshift.worker-adapter-capabilities.raw/v1`, one NUL byte, then the exact canonical capability bytes. For exact bytes `{}`, the fixed result is `sha256:4dbc0996b158b29f3e54274c8fd1ccb774422f75fb38b3fd1a1aae0662ff5c4c`. Content-equivalent bytes in a different serialization have a different custody digest and are not admitted by the joint verifier.

## Adapter admission and resource bounds

Start requests are independently bounded at 86,400 seconds and 16 MiB output, matching execution-profile ceilings. A separate adapter-contract verifier consumes exact canonical capability bytes and binds their raw digest plus adapter ID, protocol, version, executable identity, profile digest, work-item execution entry, and start-request digest. A standalone capability shape does not establish admission.

## Lifecycle and authority boundary

Adapter events and terminal receipts retain exact provider, model, session, thread, turn, and queue identity custody. Identity fields freeze when first observed. A provider completion observation or process exit is not a result; only an exact identity-bound terminal receipt can become the worker outcome. A waiting-approval event is testimony only and receives no response or protected effect. Resume retains the same attempt identity, and terminal occurrences are not retried.

Text `maxLength` laws count Unicode scalar values, matching JSON Schema and the Python consumer; independent serialized-record ceilings remain UTF-8 byte bounds.

Receipt timestamp custody is lexical as well as temporal. Terminal `started_at`/`ended_at` and not-started `recorded_at` must equal the exact canonical UTC `Z` serialization produced by the owner `chrono` type: no numeric offset aliases, redundant fractional zero groups, or abbreviated sub-microsecond forms. Canonical fractions use the shortest 3-, 6-, or 9-digit group. The consumer compares the full admitted nanosecond value; host-language microsecond truncation is not permitted.
