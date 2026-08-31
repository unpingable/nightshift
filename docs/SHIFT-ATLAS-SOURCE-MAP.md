# SHIFT-ATLAS source map

Campaign: SHIFT-ATLAS

Slug: nightshift-operational-condition-casework-v1

Track: nightshift-operator-surface

Exact integration parents:

- LEDGER-FOX result `eea4472342581925d9bde71ed04937039aee78d4`, qualified
  subject `ef6b11ef7514ab11074c297dc09cf796adea7e32`;
- EPOCH-LANTERN result `1b3497066d3f7d3da9284e4d0c9ff7fbd6a8e296`,
  qualified subject `7d467fa8613318f5e5886c082740c4e030a20c2c`.

| Requirement | Owner source |
|---|---|
| Separate closed operational-condition projection | `crates/nightshift-casework/src/operational_model.rs` |
| Fixed explicit condition directories and exact five-file custody | `crates/nightshift-casework/src/operational_loader.rs` |
| Descriptor-relative no-follow reads and before/after metadata binding | `read_regular_at_after_open`, `stable_metadata` |
| Exact Monitor/NQ owner rederivation | `admit_operational_lineage` call in `operational_loader.rs` |
| Exact EPOCH evaluation/profile binding | `OperationalReobservationEvaluationV1::validate_against` |
| Subject-local replay/fork/successor grouping | `same_temporal_branch` |
| Stable projection, condition, and question identities | `operational_model.rs`, `project`, `question` |
| Exact raw source retention | `LoadedOperationalCondition`, `OperationalRawSourcesV1` |
| Closed Draft 2020-12 projection schemas | `schemas/nightshift.casework-operational-condition*.json` |
| Executable schema parity and substitutions | `tests/test_casework_operational_schema.py` |
| Fixed loopback GET/HEAD API and raw routes | `crates/nightshift-casework/src/server.rs` |
| CLI `--condition-dir` admission | `crates/nightshift-casework/src/bin/nightshift_casework.rs` |
| Typed browser contract and same-origin GET adapter | `ui/casework/src/contract.ts`, `api.ts` |
| Operational index, detail, question, and raw views | `ui/casework/src/OperationalViews.tsx` |
| Stable direct routes and keyboard journey | `ui/casework/src/router.ts`, `OperationalViews.test.tsx` |
| Structural boundary and deterministic negative controls | `scripts/check_casework_operational_read_only_surface.sh` |
| Architecture and non-authorizing semantics | `docs/architecture/OPERATIONAL-CONDITION-CASEWORK-V1.md` |

EPOCH-LANTERN remains the owner of temporal lineage, claim preservation, currentness,
trigger, and next-lawful-action semantics. Casework derives and presents those facts;
it does not widen them. LEDGER-FOX remains the owner of the separate live-run
projection and read-only foreman snapshot. Neither predecessor artifact is revised.

No listener, process, browser, service, timer, secret, provider session, or temporary
profile is created by the projection library.
