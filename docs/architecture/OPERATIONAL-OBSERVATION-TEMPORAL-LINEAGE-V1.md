# Operational observation temporal lineage v1

Status: EPOCH-LANTERN contract checkpoint.

Nightshift owns admissibility across time. This contract adds the missing temporal owner seam after accepted FIELD-CLOCK without altering either FIELD result.

## Exact predecessor and upstream pins

The campaign starts from exact STILL-CIPHER result 87e39aeaac07c74819a7e20a24cc905ad8929d63.

The immutable lineage contract accepts only the exact FIELD contracts:

- Monitor result b2d52fe34f146774cbf5601819982c267c7fb082;
- NQ result 39b9f84f2f70955dd12e5cbfe798c740f9e52854;
- Monitor schema monitor.operational-acquisition/v1;
- NQ schema nq.operational-observation-qualification/v1.

The Rust module uses fixture-compatible closed types. It has no path or runtime dependency on a mutable Monitor or NQ checkout.

## Two contracts, two time scales

nightshift.operational-observation-lineage/v1 is immutable. It retains:

- exact raw-byte SHA-256 and byte length for Monitor and NQ artifacts;
- the Monitor semantic observation digest and NQ JCS semantic digest;
- typed subject and family-owned stable-basis contract;
- the exact producer principal and key-bound producer identity;
- acquisition outcome, epoch, sequence, and predecessor;
- acquisition start/end, optional producer observation, receiver custody, NQ qualification, and Nightshift admission times;
- the selected NQ claim support, cannot-testify findings, refusals, and subject contradictions without reclassification.

nightshift.operational-reobservation-evaluation/v1 changes with evaluation time. A closed Nightshift profile owns max_age_seconds; current_until is derived from the producer observation time. Currentness uses the half-open interval evaluated_at < current_until. Equality is stale.

The evaluation projects only claim IDs already supported by NQ. It cannot add a claim, convert cannot-testify or refusal into support, select a producer class as preferred, or erase a contradiction.

## Lineage law

Within one typed subject, exact producer, and producer epoch:

- sequence zero has no predecessor;
- later sequences require the retained immediately preceding sequence;
- the predecessor observation digest must equal the prior immutable Monitor semantic digest;
- exact replay converges on the original lineage identity;
- a different observation at an occupied epoch/sequence coordinate is a refused fork;
- a missing, reordered, or substituted predecessor is refused.

A new producer epoch begins again at sequence zero. Its existence does not rewrite an earlier epoch.

## Failure and action boundary

no_response, command_failed, producer_unavailable, receiver_unavailable, malformed_input, and refused acquisitions carry no producer observation time, payload schema, claim support, or current horizon. They evaluate as acquisition failure and request only re-observation.

Re-observation is a request to acquire new testimony. It is not retry of an effect, remediation, dispatch, approval, execution, standing, or authorization. The module opens no listener, starts no subprocess, calls no office, and adds no binary. Canonical nightshiftd remains exactly two production binaries.

No aggregate result exists.
