# EPOCH-LANTERN source map

Campaign: EPOCH-LANTERN
Slug: nightshift-operational-observation-temporal-lineage-v1
Track: Nightshift operational evidence spine

| Requirement | Owner source |
|---|---|
| Immutable temporal lineage contract | crates/nightshiftd/src/operational_lineage.rs, OperationalObservationLineageV1 |
| Changing re-observation evaluation | crates/nightshiftd/src/operational_lineage.rs, OperationalReobservationEvaluationV1 |
| Exact raw/semantic custody | ExactArtifactCustodyV1, admit_operational_lineage |
| Typed subject/producer and exact-body Ed25519 reopening | SubjectKindV1, SubjectBasisV1, extract_object_field, validate_monitor |
| Exact FIELD result pins | FIELD_CLOCK_MONITOR_RESULT_HEAD, FIELD_CLOCK_NQ_RESULT_HEAD |
| Claim subset/cannot-testify/refusal/contradiction preservation | validate_nq_findings, claim_value_is, bind_input, findings, validate_against |
| Independent time axes and max-age horizon | OperationalObservationLineageV1::validate, evaluate_reobservation |
| Replay/fork/successor law | admit_history |
| Failure/no-response never current | validate_monitor, evaluate_reobservation |
| Re-observation trigger and next lawful action | ReobservationTriggerV1, NextLawfulActionV1 |
| Closed immutable schema | schemas/nightshift.operational-observation-lineage.v1.schema.json |
| Closed changing schema | schemas/nightshift.operational-reobservation-evaluation.v1.schema.json |
| Executable schema parity | tests/test_operational_lineage_schema.py |
| Fixed FIELD vectors | crates/nightshiftd/tests/fixtures/operational_lineage |
| Structural owner boundary and deterministic controls | scripts/check_operational_lineage_boundary.sh |
| Architecture and nonclaims | docs/architecture/OPERATIONAL-OBSERVATION-TEMPORAL-LINEAGE-V1.md |

The compatibility types are local contract fixtures, not a runtime dependency on Monitor or NQ. FIELD artifacts are not revised.

No AG, Docket, Casework, listener, transport, service, executor, or target-effect surface is present.
