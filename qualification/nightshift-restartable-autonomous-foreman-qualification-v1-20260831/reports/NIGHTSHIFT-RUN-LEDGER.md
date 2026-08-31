# Nightshift run ledger

- Packet: `foreman-fixture`
- Packet digest: `sha256:ac6b42310384c00a6eaaf7b8565f9ae7009e073bfcaac1d170b50bb63c739743`
- Receipt snapshot: `2026-08-29T16:00:15Z`
- Aggregate verdict: none; every campaign retains its own classification.

## Campaign DAG

```text
root-a <- root
root-b <- root
root-c <- root
dependent <- root-a
```

## Per-workstream state

| Work item | Campaign | Dependencies | State | Classification |
|---|---|---|---|---|
| root-a | ROOT-A / fixture-root-a | none | IMPLEMENTATION-EVIDENCE-RETAINED | MIDNIGHT-DETERMINISTIC-IMPLEMENTATION-FIXTURE |
| root-b | ROOT-B / fixture-root-b | none | AUDIT-EVIDENCE-RETAINED | MIDNIGHT-DETERMINISTIC-AUDIT-FIXTURE |
| root-c | ROOT-C / fixture-root-c | none | BLOCKED-HUMAN-EXACT | MIDNIGHT-LANE-LOCAL-QUESTION |
| dependent | DEPENDENT-D / fixture-dependent | root-a | ENTRY-EVALUATED-EXACT-PREDECESSOR | MIDNIGHT-DETERMINISTIC-DEPENDENT-FIXTURE |
