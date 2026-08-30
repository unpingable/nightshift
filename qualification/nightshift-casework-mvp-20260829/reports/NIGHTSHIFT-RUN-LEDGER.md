# Nightshift run ledger

- Packet: `nightshift-casework-mvp-20260829`
- Packet digest: `sha256:9a819dc830b021a38b918029c1e6f0370fb8732572dcc68bedf4c60fa45fb93b`
- Receipt snapshot: `2026-08-30T00:42:01Z`
- Aggregate verdict: none; every campaign retains its own classification.

## Campaign DAG

```text
index-wren-backend <- root
map-cabinet-frontend <- index-wren-backend
```

## Per-workstream state

| Work item | Campaign | Dependencies | State | Classification |
|---|---|---|---|---|
| index-wren-backend | INDEX-WREN / nightshift-run-receipt-casework-read-model-and-api-v1 | none | CLOSEOUT-COMPLETE-INDEPENDENT-DIMENSIONS | SEE-INDEPENDENT-DIMENSIONS-NO-AGGREGATE-CLASSIFICATION |
| map-cabinet-frontend | MAP-CABINET / nightshift-read-only-casework-console-mvp-v1 | index-wren-backend | QUALIFIED | NIGHTSHIFT-READ-ONLY-CASEWORK-CONSOLE-MVP-V1-QUALIFIED |
