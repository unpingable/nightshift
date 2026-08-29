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
  `sha256:9b4e1a2cea4010dbc620368fe3adfe51a7c1e2551fa7af61f27f94a0a8bc692b`
- Plan reference:
  `nightshift-packet://9b4e1a2cea4010dbc620368fe3adfe51a7c1e2551fa7af61f27f94a0a8bc692b`
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
