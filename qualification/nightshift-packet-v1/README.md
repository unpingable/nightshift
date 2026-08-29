# NightshiftPacketV1 qualification

Campaign: VELVET-ORRERY

Canonical slug: `nightshift-immutable-run-packet-v1-20260829`

Base: `999a91d92b56e655906a31a1a4e914ccaf1ecbfb`

The campaign introduces a narrow non-authorizing orientation packet because
the repository had no current orchestration envelope. The historical Review
Packet v0 remains historical and is not repurposed.

## Exact run specimen

- Packet ID: `nightshift-20260829-autonomous-convergence`
- Packet digest:
  `sha256:01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`
- Plan reference:
  `nightshift-packet://01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`
- Schema file SHA-256:
  `6b71b4ec182811c376c4b852bc6ae540e1c063d5db43d1cacefaeead9636c50f`
- Creation: `2026-08-29T17:00:00Z`
- Currentness boundary: `2026-08-30T12:00:00Z`
- Work items: 14
- Authority effect: `NONE`

`velvet-orrery/packet.v1.json` is exact canonical JSON with no trailing
newline. `PACKET-SUMMARY.md` is non-normative.

## Qualification cases

The Rust suite covers:

- valid sealing, validation, and non-authorizing rendering;
- mutation of a normative field changes the digest;
- deterministic packet-specific domain-framed digest vector;
- byte-exact seal-to-file-to-validate interoperability;
- empty strings/collections, substituted field types, and nested unknown
  fields fail closed at the shared schema boundary;
- packet-digest mismatch fails closed;
- plan-reference mismatch fails closed;
- unknown JSON fields fail closed;
- unknown work-item dependencies fail closed;
- dependency cycles fail closed;
- stale packet evaluation fails closed;
- committed positive and negative fixtures reproduce those dispositions.

The Python suite covers receipt-to-report derivation and rejects unknown or
missing receipt work items.

Switchyard owns nonce custody and replay qualification; Nightshift does not
duplicate that state machine.

## Result

`NIGHTSHIFT-ORIENTATION-PACKET-V1-IMPLEMENTATION-QUALIFIED`

This classification is limited to the packet implementation and local
deterministic fixtures. It grants no continuation or execution authority.
