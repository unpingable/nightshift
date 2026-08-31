# Foreman Provider-Capacity Admission V1

Status: GAUGE-LATCH owner-contract correction.

This contract binds normalized provider-capacity evidence to a Nightshift
foreman attempt without turning observation into target-effect authority. The
authority effect is exactly `LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY`. It does not
answer an approval, actuate an operational subject, retry an attempt, or add an
aggregate campaign classification.

## Immutable run requirement

A capacity-aware run is admitted with an exact
`nightshift.foreman-capacity-requirement/v1` record in the same immediate
SQLite transaction as the packet, admission, profile, and `RunAdmitted` event.
The requirement binds:

- packet, foreman-admission, execution-profile, and run identities;
- the profile `budget_policy_ref` to the exact capacity policy ID;
- one exact normalized provider identity; and
- the packet's complete model-class set to the closed mapping `small` and
  `bounded` = `CHEAP`, `medium` and `large` = `EXPENSIVE`.

The policy digest is deliberately not substituted for the policy ID. It is
retained separately in each attempt admission.

Historical stores and runs remain readable. A legacy run without this immutable
requirement retains the legacy `prepare_attempt` transition. A capacity-required
run refuses that transition and admits an attempt only through
`prepare_attempt_with_capacity`. A capacity bundle is likewise refused on a
legacy run. There is no optional capacity path for a capacity-required run.

## Exact attempt admission

`nightshift.foreman-capacity-admission/v1` binds the requirement, packet,
admission, profile, run, work item, adapter, provider, packet/profile model
class, cost class, policy ID, exact observation/policy/decision digests, and the
exact attempt-admission instant. The event retains the exact canonical bytes of
all four records. The attempt and capacity event commit atomically with resource
claims under one immediate transaction.

The normalized FUEL owner contracts remain authoritative for observation,
policy, and decision validity. The foreman additionally requires:

- the explicit provider binding in the requirement (itself bound to the exact
  profile digest), observation, decision, and attempt admission;
- exact policy-ID equality with `budget_policy_ref`, plus exact policy digest;
- exact observation and decision digest relationships;
- full equality with the deterministic FUEL decision recomputed from the exact
  retained observation, policy, and decision time;
- a half-open currentness interval: `observed_at <= evaluated_at < expires_at`;
- decision time equal to attempt-admission time; and
- exact packet/profile model-class equality and the closed cost mapping.

`NO_NEW_WORK` refuses every new attempt. `CHEAP_BOUNDED_ONLY` admits only the
closed cheap classes. `ORDINARY_BOUNDED` admits expensive work only when the
exact decision says `allow_new_expensive_work`. The packet/profile contracts do
not bind a speculative-work request, so V1 fixes `speculative_requested` to
false; any requested speculative admission fails closed. Active work may still
reach exact custody after later capacity becomes critical.

Restart and replay never refresh evidence. They reconstruct the exact retained
requirement and capacity-admission events. Substituted bytes, provider or policy
identity, stale evidence, missing or duplicate events, and a concurrent legacy
transition are refused before an attempt can exist.

## Projection and schemas

The query-only foreman projection exposes the exact recorded admission,
observation, policy, and decision bytes and their typed identities. It adds no
Casework mutation and makes no claim beyond those exact recorded facts.

The recursively closed schemas are:

- `schemas/nightshift.foreman-capacity-requirement.v1.schema.json`
- `schemas/nightshift.foreman-capacity-admission.v1.schema.json`

Domain-separated digests are computed over RFC 8785/JCS bytes with the digest
field removed:

- `nightshift.foreman-capacity-requirement.digest/v1\0`
- `nightshift.foreman-capacity-admission.digest/v1\0`
