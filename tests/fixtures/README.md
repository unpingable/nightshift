# Test fixtures

## NQ finding-export fixtures

### `nq-findings-observable.jsonl`

**Live capture.** Real `nq findings export` output from the live VM
(`root@nq.neutral.zone`, NQ binary `/opt/notquery/nq` mtime 2026-05-01
18:58, schema 43, contract `nq.finding_snapshot.v1`). Captured
2026-05-01.

The chosen finding (`freelist_bloat` on `labelwatch-host`,
sqlite-path subject) is the closest organic match to the
`wal-bloat-review` agenda's scope shape. `admissibility.state ==
"observable"`, which is the happy-path admission case for the
Night Shift consumer.

This is real evidence. Treat it as a captured wire snapshot.

### `nq-findings-suppressed-derived.jsonl`

**Derived contract fixture, NOT live evidence.** Constructed from
`nq-findings-observable.jsonl` by mutating only the `admissibility`
block:

- `state`: `observable` → `suppressed_by_ancestor`
- `reason`: `none` → `testimony_dependency`
- added `ancestor_finding_key`:
  `local/labelwatch-host/node_unobservable/labelwatch-host`

Every other field is unchanged. The ancestor key is plausible-shaped
(matches NQ's canonical key format) but does not refer to a real
finding on the live VM — none was needed for the contract test, which
exercises parse-time refusal of non-observable testimony, not ancestor
resolution.

Used exclusively by the V1.2 negative test in
`tests/nq_integration.rs` to assert: *given a wire-shaped NQ finding
whose admissibility state is non-observable, the Night Shift consumer
refuses it at parse time with a recognizable error.*

This fixture must not be cited as evidence of suppression behavior on
the live system.

### `nq-findings-import-clean.jsonl`

**Live capture.** Real `nq findings export` output captured 2026-05-12
from NQ's `DURABLE_ARTIFACT_SUBSTRATE_GAP` V1 substrate slice
(`crates/nq-db/tests/fixtures/synthetic_producer_import.json` →
ingest → export). NQ schema 46, contract `nq.finding_snapshot.v1` v1
(preserved — the V1 substrate is additive on the v1 contract).

Two ingested findings, both with the new `origin` envelope populated
(`origin.source = "import"`, `producer_id`,
`producer_extraction_time`, etc.). Both `admissibility.state ==
"observable"`. Used by the V1-substrate consumer-alignment dry run
in `tests/nq_integration.rs` to assert: *Night Shift consumes
ingested findings through the same admissibility path as native NQ
findings (substrate admission ratified).*

### `nq-findings-import-stale.jsonl`

**Live capture.** Real `nq findings export` output captured 2026-05-12
from NQ's `synthetic_producer_stale.json` fixture (producer
extraction time 2026-01-01, ingest 2026-05-12 → V1 substrate's
`extraction_stale` SILENCE_UNIFICATION composition fires).

Two lines:

1. `extraction_stale` — NQ's own testimony about the stale producer.
   Carries the new `silence` envelope (`scope: "extraction"`,
   `basis: "age_threshold"`, `duration_s`). No `origin` block —
   NQ-internal finding, not ingested.
2. The ingested finding from the stale producer. Carries `origin`
   (with `producer_extraction_time: "2026-01-01T00:00:00Z"`). No
   `silence` block — silence is NQ's testimony about the producer,
   not about the finding itself.

Both lines `admissibility.state == "observable"`. Used to verify
NS tolerates the silence envelope and the `extraction_stale` finding
without inventing new admissibility states.
