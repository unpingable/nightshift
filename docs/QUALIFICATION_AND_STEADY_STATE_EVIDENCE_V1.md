# Qualification and steady-state evidence v1

**Status:** canonical Nightshift contract, 2026-08-22.

> Re-observation may refresh what can be learned by looking. It may not
> refresh what can only be learned by intervening.

> A qualification remains evidence of the test that occurred. It does not
> become a fresh observation merely because the deployed system has not
> changed.

This contract is the narrow decision-relative extension of the existing
local-Compose external-evidence path. It distinguishes one governed,
effectful qualification (`Q`) from a separately acquired, read-only
steady-state observation (`S`). Nightshift is the owner of composition,
adequacy, and currentness. Neither adapter nor the acquisition orchestrator
owns those judgments.

## Evidence taxonomy

`nightshift.artifact_qualification_evidence.v1` is derived from an admitted
`maude.local-compose-world-observation/v1` that contains the complete strong
`post_settlement_successor` claim set. It proves that the exact PlanDocument,
compilation, work, subject, and scope survived the exact governed fault test
whose attempt and settlement it names. Its acquisition time remains
historical. It is not assigned a rolling currentness horizon.

`maude.local-compose-steady-state-observation/v1` is produced by the separate
`maude.local-compose-steady-state-observation-adapter` version 1. It contains
only these closed claims:

- `front_door_reachable` (`pn_health`);
- `cache_a_present` (`pn_cache_a`);
- `cache_b_present` (`pn_cache_b`);
- `ordinary_cache_behavior_observed` (`pn_cache_behavior`).

It cannot represent `single_cache_failure_survived` or
`cache_topology_restored`. Its source evidence is acquired with fixed HTTP
GETs and one fixed `docker compose ps` query. There is no container lifecycle,
generic command, remediation, Docket, AG, or currentness surface.

## Applicability is not currentness

Qualification applicability is exact identity equality across:

- locked PlanDocument digest;
- compilation identity;
- exact work identity;
- workflow profile and strong source profile;
- subject and scope;
- source campaign, occurrence, proposal, issuance, attempt, and settlement.

V1 defines no semantic equivalence between different artifact or compilation
digests. A changed artifact therefore needs new governed qualification when a
decision depends on the strong claims. Restart, elapsed time, a new passive
observation, or a similar label cannot rebind `Q1` to `C2`.

Passive steady-state evidence has an exclusive profile horizon:

```text
steady_state_observed_at_unix_ms + max_age_ms
```

Nightshift clips ordinary posture currentness to that horizon and revalidates
the exact composition at consequence time. The qualification acquisition time
is preserved alongside, not merged into, the passive timestamp. There is no
synthetic combined evidence time.

## Decision-adequacy matrix

| Decision | Required evidence | Time law | Artifact law | Refusal |
|---|---|---|---|---|
| `post_settlement_successor` | original four-claim strong application observation | existing strong profile horizon | exact governed predecessor and artifact | unchanged v1 strong-profile refusal |
| `routine_continuation` | exact historical qualification `Q` plus all four passive claims `S` | only `S` must be current under the steady-state profile | `Q` and `S` must bind the same exact plan, compilation, work, subject, and scope | missing/inapplicable `Q` or absent/stale/inadequate `S` refuses |
| re-assert present failure survival | a new governed effectful fault test | new acquisition time belongs to the new test | exact newly qualified artifact | passive observation is categorically insufficient |
| resilience decision after artifact/configuration change | new governed qualification for the new digest | according to that new qualification contract | no cross-digest equivalence in v1 | old `Q` remains historical but inapplicable |

The meaning of `post_settlement_successor` is not weakened. The new
`routine_continuation` purpose is a distinct closed profile and a canonical
`nightshift.composed_decision_relative_evidence.v1` embedded in
`nightshift.observation_record.v4`.

## Composition and identity

The deployment-owned `nightshift.steady_state_evidence_profile.v1` embeds the
exact strong profile and independently pins the passive adapter, producer,
key, runtime, required passive claims, and passive TTL. A producer cannot
select its TTL or adequacy law.

Composition retains:

```text
qualification Q
  -> exact strong source/custody/profile
  -> exact artifact/work/governed execution
  -> historical acquisition time

steady state S
  -> exact passive source/custody/profile
  -> exact artifact/work/qualification relation
  -> passive observation time and exclusive horizon

decision D
  -> exact Q + exact S + target campaign/occurrence/subject/scope
```

Changing either source, a claim, PlanNode binding, producer, time, profile,
artifact, subject, scope, or target changes the content-bound composition and
canonical observation identity or causes refusal.

The existing diagnostic `DecisionBasisV1` and NQ boundary are unchanged.
Application evidence is not relabelled as diagnostic atoms. NQ still admits
diagnostic artifacts where required; external application evidence remains a
separate Nightshift input whose provenance is bound through the canonical
observation.

## Re-observation

Nightshift emits an owner-produced
`nightshift.steady_state_reobservation_basis.v1` with one of `absent`,
`current`, or `stale`. It binds the exact qualification, prior passive source,
profile, artifact, governed execution, evaluation time, and prior exclusive
horizon. The Maude acquisition orchestrator accepts `reobserve_after_stale`
only for a valid `stale` basis and the closed passive adapter. The same adapter
may perform the first bounded `reobserve_for_successor` acquisition only from
an exact owner-produced `absent` basis; that is not an alias for stale refresh.

The new logical acquisition receives a new trigger, request, evidence,
custody, and observation identity. Exact resend of that acquisition reuses its
durable handoff and original `observed_at`; it never calls the probe again.
An incomplete invocation whose outcome cannot be reconciled remains unknown
rather than reacquiring the world under the same identity.

The strong adapter still refuses stale re-observation. Missing or
inapplicable qualification is reported as a requirement for new governed
work, never as permission for the scheduler to inject a fault.

## Conflict, absence, and failure

The passive observation must contain the exact closed subject shape, both
cache identities in `running` state, a reachable front door, and a bounded
MISS/MISS/HIT/HIT sequence through alternating cache identities. A missing
cache, unavailable front door, malformed response, timeout, or contradictory
sequence produces no satisfied evidence and authorizes no remediation.

Authenticated conflicting evidence remains separate provenance. Nightshift
does not average it, overwrite an older object, or let a UI choose a winner.
Settlement without `Q`, and passive evidence without an applicable `Q`, remain
inadequate for decisions that require qualification.

## Cross-probing and inspection

Exact PlanNode bindings remain distinct:

- `pn_health`, `pn_cache_a`, `pn_cache_b`, and `pn_cache_behavior` point to
  passive evidence;
- `pn_continued` and `pn_restore` remain tied to the governed effectful
  qualification.

Phosphor-ng renders historical qualification, current/stale passive evidence,
and decision adequacy as separate sections. Evidence age is display arithmetic
only. The owner-produced Nightshift resolution and exact profile horizon are
displayed separately. `/inspect` remains GET/HEAD-only and `/design` remains a
pre-governed PlanDocument editor.

## Nonclaims and deployment remainder

This contract does not add continuous monitoring, recurring schedules,
alerting, automatic remediation, automatic requalification, standing,
authorization, or execution permission. It qualifies one demand-driven
passive re-observation for one workflow profile.

Host clock trust, real service-principal separation, key rotation, physical
failure, live backup/restore, and executor-to-world correspondence remain
designated-host or physical qualification gates.

The successor-artifact lifecycle, including exact C1/Q1 refusal, ordinary
governed C2 requalification, Q2, and Q2 plus current S continuation, is
specified in
[`ARTIFACT_CHANGE_AND_REQUALIFICATION_V1.md`](ARTIFACT_CHANGE_AND_REQUALIFICATION_V1.md).
