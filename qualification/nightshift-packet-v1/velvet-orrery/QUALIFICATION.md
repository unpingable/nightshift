# VELVET-ORRERY qualification

Campaign: VELVET-ORRERY

Canonical slug: `nightshift-immutable-run-packet-v1-20260829`

Predecessor base: `999a91d92b56e655906a31a1a4e914ccaf1ecbfb`

## Commands

```text
cargo test --locked --test nightshift_packet_v1
python3 -B -m unittest tests/test_render_nightshift_reports.py
target/debug/nightshift packet validate --packet qualification/nightshift-packet-v1/velvet-orrery/packet.v1.json --evaluated-at 2026-08-29T17:30:00Z
scripts/check_no_actuation_surface.sh
```

## Observed results

- Rust packet tests: 16 passed, 0 failed.
- Receipt/report tests: 3 passed, 0 failed.
- Exact packet validation:
  `VALID_NON_AUTHORIZING_ORIENTATION_PACKET`, authority effect `NONE`,
  14 work items.
- Packet digest:
  `sha256:01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`.
- Schema file SHA-256:
  `6b71b4ec182811c376c4b852bc6ae540e1c063d5db43d1cacefaeead9636c50f`.
- Seal output is exact JCS with no trailing byte; redirected output validates
  without normalization.
- Independent checked-in JSON Schema validation: passed.
- Structural no-actuation gate and its deterministic negative control: passed.

## Classification

`NIGHTSHIFT-ORIENTATION-PACKET-V1-IMPLEMENTATION-QUALIFIED`

The result covers deterministic canonicalization, content addressing,
validation, rendering, fixtures, and receipt-derived report generation. It
does not authorize any packet work item or qualify Switchyard nonce custody.
