# VELVET-ORRERY qualification

Campaign: VELVET-ORRERY

Canonical slug: `nightshift-immutable-run-packet-v1-20260829`

Predecessor base: `999a91d92b56e655906a31a1a4e914ccaf1ecbfb`

## Commands

```text
cargo test --locked --test nightshift_packet_v1
python3 -B -m unittest tests/test_render_nightshift_reports.py
target/debug/nightshift packet validate --packet qualification/nightshift-packet-v1/velvet-orrery/packet.v1.json --evaluated-at 2026-08-29T17:30:00Z
```

## Observed results

- Rust packet tests: 9 passed, 0 failed.
- Receipt/report tests: 3 passed, 0 failed.
- Exact packet validation:
  `VALID_NON_AUTHORIZING_ORIENTATION_PACKET`, authority effect `NONE`,
  14 work items.
- Packet digest:
  `sha256:9b4e1a2cea4010dbc620368fe3adfe51a7c1e2551fa7af61f27f94a0a8bc692b`.

## Classification

`NIGHTSHIFT-ORIENTATION-PACKET-V1-IMPLEMENTATION-QUALIFIED`

The result covers deterministic canonicalization, content addressing,
validation, rendering, fixtures, and receipt-derived report generation. It
does not authorize any packet work item or qualify Switchyard nonce custody.
