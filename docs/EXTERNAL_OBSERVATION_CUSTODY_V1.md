# External observation custody v1

## Purpose

The first exact local-container workflow exposed a deliberate gap after Docket
settlement: the executor retained useful application/world evidence, but that
evidence was neither an NQ diagnostic nor a Nightshift observation. Treating a
successful settlement as health would have erased the boundary.

This contract closes only the source-custody half of that gap:

```text
immutable local-Compose executor evidence
    + exact compilation / PlanNode bindings
    + exact governed occurrence / attempt / settlement bindings
    -> maude.local-compose-world-observation/v1
    -> authenticated nightshift.external_observation_handoff.v1
    -> immutable Nightshift custody projection
```

It does not insert an observation into a canonical Nightshift cycle.

## Owner and producer

The Maude workflow adapter owns the workflow-specific projection because it
knows the closed compiler/output contract and the meaning of the executor's
acceptance evidence. It verifies:

- the canonical executor-evidence bytes and domain-separated receipt;
- the exact Docket outcome embedded in those bytes;
- the exact executor plan and `exact_work_identity`;
- the compilation receipt and stable PlanNode bindings;
- the exact campaign, occurrence, proposal, issuance, attempt, settlement,
  subject, scope, and outcome in the owner-produced governed cross-probe.

The configured observation-adapter service principal then authenticates one
exact candidate to one Nightshift runtime with a dedicated HMAC key. The key
must be a 32-byte, owner-only regular file and is not stored in the repository.
The credential should be distinct from Maude session and authoring-handoff
credentials in deployment.

Nightshift owns acceptance and immutable persistence of the candidate and its
custody receipt. It does not trust a browser or Phosphor projection to mint the
record.

## Closed claim vocabulary

V1 is deliberately local-Compose-specific. Qualify evidence may project only:

- `front_door_reachable` -> `pn_health`;
- `cache_miss_then_hit` -> `pn_cache_behavior`;
- `single_cache_failure_survived` -> `pn_continued`;
- `cache_topology_restored` -> `pn_restore`.

Teardown evidence may project only:

- `campaign_resources_absent` -> `pn_teardown`.

Every claim binds the exact compiled output identity and exact JSON-pointer
paths in the retained executor evidence. A failed or indeterminate executor
result can yield only `unknown` claims. The adapter refuses a purported
successful result whose evidence contradicts the closed acceptance contract.

This is not a generic claims bag. Another workflow needs another explicit
adapter/schema or a deliberate schema evolution.

## Integrity and custody

`observation_id`, every `claim_id`, `handoff_id`, and `custody_id` are
content-derived SHA-256 identities over canonical JSON preimages. The executor
receipt is independently recomputed with the existing
`maude.local-compose.executor-evidence/v1` AG domain hash. The handoff HMAC
binds the complete candidate, producer principal/key identity, target runtime,
and creation time.

Recomputing an outer object digest cannot conceal changed source evidence,
PlanNode bindings, governed identities, or claim paths. Without the configured
producer credential, a changed candidate cannot acquire a valid custody tag.

## Persistence and replay

Nightshift persists the observation and custody receipt in
`canonical_external_observations`, separate from
`canonical_observation_cycles`.

- Exact retransmission returns the first durable custody receipt, including
  its original receipt time.
- The same attempt, occurrence, evidence receipt, handoff, or observation
  cannot be rebound to conflicting facts.
- Restart/reopen recovers the exact record.
- Historical occurrences without a candidate remain absent; no backfill is
  inferred.
- A successor occurrence needs its own exact candidate and handoff.

## Read projection and evidence age

`nightshift external-observation export` supports exact lookup by observation,
campaign plus occurrence, or Docket attempt. The caller supplies an evaluation
instant and evidence TTL. The projection reports one of:

- `fresh_at_evaluation`;
- `stale_at_evaluation`;
- `not_yet_observed`.

This is a transparent arithmetic statement about source-evidence age. It is
not `Current` under the canonical Nightshift observation resolver, does not
select an observation lineage head, and is never sent to AG as an observation
resolution.

## Explicit nonclaims

> Producer authentication establishes custody of exact source evidence. It
> does not establish Nightshift currentness.

> Docket settlement establishes an attempt outcome, not the present state of
> the world.

The candidate and custody record establish neither:

- NQ admission;
- diagnostic coherence or coverage;
- present-support qualification;
- Nightshift currentness;
- standing or admissibility;
- AG authorization or spend;
- Docket execution permission;
- incident closure or successful recovery now.

No canonical cycle code reads this table. A later campaign may define an
explicit composition contract by which an appropriate source observation is
used alongside NQ substrate testimony and present support. Until then, the
candidate remains inspectable source evidence, honestly unpromoted.

## Commands

The Maude adapter creates exact canonical bytes:

```sh
maude-local-compose-observation \
  --executor-evidence /exact/attempt.json \
  --executor-plan /exact/executor-plan.json \
  --compilation-receipt /exact/compilation-receipt.json \
  --governed-bindings /exact/governed-cross-probe.json \
  --producer-key /run/credentials/maude-observation.key \
  --producer-principal-id maude-observer:local \
  --producer-key-id maude-observer-key:primary \
  --target-runtime-id nightshift:local-c1 \
  --created-at 2026-08-22T18:30:00Z \
  --output /bounded/handoff.json
```

Nightshift authenticates and persists it:

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite \
  external-observation import \
  --handoff /bounded/handoff.json \
  --credential /run/credentials/maude-observation.key \
  --producer-principal-id maude-observer:local \
  --producer-key-id maude-observer-key:primary \
  --nightshift-runtime-id nightshift:local-c1 \
  --received-at 2026-08-22T18:31:00Z
```

After a timeout, resend the same exact handoff or query its exact observation,
occurrence, or attempt. Do not invent a new observation to resolve transport
uncertainty.
