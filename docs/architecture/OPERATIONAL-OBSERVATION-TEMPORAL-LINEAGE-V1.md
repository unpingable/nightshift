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

The Rust module uses fixture-compatible closed types. It verifies signatures over the exact supplied raw body slice, then requires typed subject and producer identity objects to reproduce their exact FIELD digests. It has no path or runtime dependency on a mutable Monitor or NQ checkout.

## Two contracts, two time scales

nightshift.operational-observation-lineage/v1 is immutable. It retains:

- exact raw-byte SHA-256 and byte length for Monitor and NQ artifacts;
- the Monitor semantic observation digest and NQ JCS semantic digest;
- typed subject and family-owned stable-basis contract;
- the exact producer principal and key-bound producer identity;
- acquisition outcome, epoch, sequence, and predecessor;
- acquisition start/end, optional producer observation, receiver custody, NQ qualification, and Nightshift admission times;
- the selected NQ claim support, cannot-testify findings, refusals, and subject contradictions without reclassification.

nightshift.operational-reobservation-evaluation/v1 changes with evaluation time. A closed Nightshift profile owns max_age_seconds; current_until is derived from the producer observation time. Currentness uses the half-open interval evaluated_at < current_until. Equality is stale. Accepted fractional RFC3339 precision is retained through receiver custody, NQ qualification, Nightshift admission, evaluation, and current_until.

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

The compatibility parser mirrors FIELD Monitor v1 limits: closed locator kinds, at most 32 locators and attachments, and at most 512 bytes for Monitor-owned text. This v1 Nightshift compatibility profile deliberately admits only printable ASCII metadata within those owner byte ceilings; that schema-expressible subset never admits text FIELD would refuse. Unicode FIELD testimony remains upstream evidence but is not admitted into this projection. Nightshift and NQ projection text uses the same printable-ASCII subset and remains bounded at 1024 bytes. Executable schemas encode exact subject-family alternatives and direct Unicode refusal witnesses.

Every NQ input is validated, not only the selected input. The accepted graph mirrors NQ qualify_one: a record-reopening refusal has no Monitor identity tuple or other finding; a later refusal has a complete reopened tuple and no support or cannot-testify; a non-refused input has a complete tuple and at least one support or cannot-testify finding; acquisition failures carry cannot-testify only; and contradictions equal the deterministic graph derived from supported claims. Optional identity, outcome, time, and payload-schema fields cannot form partial alternate shapes.

The pinned qualify_one refusal vocabulary is branch-closed: reopening failures cannot be relabeled as post-reopening profile/time/custody refusals, and post-reopening refusals cannot use reopening-failure codes. Every refusal retains the exact raw input digest. Non-refused reopened inputs retain one canonical complete profile claim-ID domain across support and cannot-testify, and each acquisition-failure reason is the exact outcome-derived NQ string. Contradiction fixtures use separately signed Monitor observations with distinct exact payload custody rather than content mutation inside one reopened input.

Unopened refusal details are also owner-closed. Fixed NQ details must match exactly. Parser-derived JSON and timestamp details use a closed grammar limited to the pinned owner messages, exact field names, and the pinned parser error vocabulary. Record and body JSON coordinates are canonical: columns are bounded by the one-MiB Monitor input ceiling, while lines admit one beyond that byte ceiling because an exactly one-MiB all-newline record can fail at the following line with column zero. Exact payload coordinates are canonical `usize` values because pinned NQ does not bound separately supplied payload bytes. Timestamp details admit only errors reachable through the pinned relaxed RFC3339 `DateTime<Utc>::from_str` path; enum-wide but unreachable display strings are refused. A code/detail substitution or invented diagnostic is refused.

No aggregate result exists.
