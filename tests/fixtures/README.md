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
