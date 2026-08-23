# External application-evidence composition v1

**Status:** canonical Nightshift contract, 2026-08-22.

> Authenticated application evidence is a historical observation with
> provenance. Nightshift determines whether it is current and adequate for a
> decision.

> Evidence age is not currentness.

This contract composes the existing authenticated
`maude.local-compose-world-observation/v1` custody record into an ordinary
Nightshift observation. It adds no observation authority and no AG authority.

## Owner boundary

The exact path is:

```text
Docket settlement/evidence
  -> Maude workflow observation adapter
  -> authenticated external-observation custody
  -> Nightshift deployment profile admission
  -> nightshift.composed_external_evidence.v1
  -> nightshift.observation_record.v3
  -> nightshift-observation-resolver
  -> ordinary AG proposal/currentness evaluation
```

Custody proves who delivered which exact historical bytes about which exact
execution. Composition proves that those bytes satisfy one closed,
deployment-owned evidence profile for one exact successor decision. The
observation resolver alone produces `Current`, `Stale`, `Superseded`,
`Contradictory`, or `Absent` at consequence time.

## V1 profile and adequacy

`nightshift.external_evidence_profile.v1` is supplied independently by the
Nightshift deployment. The evidence producer cannot select it or its TTL. V1
has one purpose, `post_settlement_successor`, and one closed local-Compose
claim set:

- front door reachable;
- MISS then HIT observed;
- one-cache failure survived;
- cache topology restored.

All four claims must be `satisfied`, the adapter/producer/key/runtime must
match the profile, and the execution outcome must be `success`. This is a
decision-relative adequacy gate, not a general application-health ontology.
Failed or indeterminate execution observations may remain truthful historical
custody records, but this v1 profile does not admit them for successor use.

The existing Nightshift `DecisionBasisV1` remains the frozen diagnostic
condition/delivery projection. Application claims are not relabelled as NQ
atoms. Their exact provenance is reached through the content-bound canonical
observation identity and its embedded composition receipt. AG continues to
evaluate ordinary standing, catalog admissibility, and one-use authorization.

## Temporal law

The retained times have different meanings:

| field | meaning |
|---|---|
| Docket settlement time | when Docket recorded the attempt outcome |
| `source_observed_at_unix_ms` | when the workflow adapter acquired the application evidence |
| custody `received_at` | when Nightshift durably received the authenticated handoff |
| composition `admitted_at` | the sealed Nightshift cycle evaluation time |
| resolver `resolved_at_unix_ms` | consequence-time currentness evaluation |

The profile horizon is the exclusive
`source_observed_at_unix_ms + max_age_ms`. The canonical resolver uses the
earlier of that horizon and its ordinary posture TTL horizon. Receipt time,
settlement time, filesystem mtime, and UI capture time cannot substitute for
observation time. Missing or unrepresentable temporal evidence remains
custodied but cannot establish currentness.

The UI may calculate `now - observed_at` for explanation. That arithmetic is
never the resolver predicate.

## Exact predecessor and subject binding

Composition binds the source campaign, occurrence, proposal, work, issuance,
attempt, settlement, PlanDocument digest, compilation, claim, PlanNode,
compiled output, producer, key, runtime, subject, and scope. The target must be
a distinct successor occurrence in the same campaign.

Nightshift additionally requires the source occurrence to equal the governed
occurrence attached to the latest exact prior governed observation in the
same Nightshift lineage. A closed or inadequate Nightshift observation remains
historical but does not invent a governed predecessor. Campaign ancestry alone
is insufficient. Evidence for occurrence O cannot silently serve O+2 after
O+1 has become the latest governed predecessor.

One historical source may participate in more than one canonical observation
for the **same exact target** while it remains inside the deployment-owned
profile horizon. This permits a later diagnostic slot to reconsider the same
successor after its other decision-relative inputs are corrected. Each
composition binds its own admission/evaluation time and therefore has its own
identity. This does not refresh the source acquisition time or currentness
horizon, consume evidence as though it were authority, or create work by
itself. Exact replay of one canonical slot converges; a conflicting target
retarget refuses.

AG is queried read-only to confirm that the source occurrence is still the
exact `SettledObservationRequired` record with the claimed attempt and
settlement. That query does not make settlement mean health.

## NQ boundary

NQ-NG still owns diagnostic admission for Nightshift's delivered diagnostic
inputs. External application evidence is not forced through NQ because it is
not an NQ diagnostic artifact, but it does not bypass NQ: the ordinary NQ
qualification path still runs before a v3 observation can be persisted.
Authenticated application evidence plus a refusing NQ input yields an NQ
refusal and no successor cycle or AG occurrence.

## Identity, persistence, replay, and restart

The composition and profile are content-derived identities. The canonical
observation identity is domain-separated and binds the complete composition.
Changing a claim, observed time, source occurrence, producer, PlanNode,
PlanDocument, compilation, subject, or target changes identity or refuses.

The composition is persisted inside `nightshift.observation_record.v3` in the
same canonical cycle transaction as the observation. Store insertion
re-resolves the exact custody source and reproduces the composition. Exact
resend cannot duplicate a canonical observation. Restart does not recreate
evidence, alter acquisition time, or refresh the profile horizon. A response
lost after durable persistence is recovered by exact observation export.

Historical evidence remains durable after the resolver reports `Stale`.
Fresh re-observation is a new authenticated observation, composition, and
canonical observation identity; it does not renew the old object or predecessor
authority.

The separate Maude-owned post-settlement acquisition orchestrator closes the
mechanical settlement-to-custody step for the qualified local-Compose
workflow. Its trigger/request/process receipts are not part of canonical
currentness. Nightshift remains the only owner of profile admission,
composition, and resolution.

V1 does not automate fresh re-observation of this **strong** profile. The
required `single_cache_failure_survived` claim is acquired by effectfully
stopping and restoring a cache. Repeating that operation requires new governed
exact work; it cannot be hidden in a scheduler.

The separate
[`QUALIFICATION_AND_STEADY_STATE_EVIDENCE_V1.md`](QUALIFICATION_AND_STEADY_STATE_EVIDENCE_V1.md)
contract now permits a closed passive profile to combine fresh read-only
steady-state evidence with this exact historical qualification. That path
never emits or renews `single_cache_failure_survived`, and it refuses if the
qualified artifact identity changes.

## Qualified synthetic feedback circuit

The disposable local-cache workload exercised the uninterrupted exact circuit
on 2026-08-22:

```text
locked PlanDocument
  -> governed qualification occurrence O0
  -> Docket attempt and settlement
  -> authenticated local-Compose application observation
  -> Nightshift v3 observation and profile composition
  -> canonical Current resolution for successor O1
  -> ordinary AG standing, one-use authorization, dispatch, and teardown settlement
```

The retained uninterrupted acquisition-orchestrated run is
`/tmp/ag-synthetic-cache-orchestrated-v3`; its acquisition trigger, acquisition
request, source observation, composition, canonical observation, and successor
occurrence are respectively:

- `sha256:302ebefe28e704427231d54a7a49f4420fac87af1bd46179f840fe8009ed43d9`;
- `sha256:5d35fe258770b83c8cd8a570d19784f7f9a88858e04aa85c1c4dfd03d19f4598`;
- `sha256:b619b7fef7460326f5d70d13397cd7d147b1727630d9cbe58f6d6a4e33fae81d`;
- `sha256:eb3d207355c1a9818c40190fe97b1036900081b2fb3b8026f51c3276e00d24de`;
- `sha256:12c191dae06c296dbe1c02566dbff766fee6ba75d53da36908a37ade5ca4a76a`;
- `00000000-0000-4000-8000-000000000001`.

The same ignored integration test then issued ordinary fresh standing and a
new one-use spend for O1, governed the exact teardown work, and closed with two
Docket attempts, two settlements, and no campaign containers or networks.
Exact orchestration replay after completion retained the original evidence
time and the same five event records; it neither invoked the adapter again nor
created another Nightshift custody record.

The resolver returned `Current` before the exclusive profile horizon and
`Stale` at that exact horizon without changing the observation or its basis.
The application observation itself includes the bounded changed-world test:
cache A is stopped, continued service through cache B is observed, and the
two-cache topology is then observed after restoration. Its four exact claims
retain PlanNode bindings for `pn_health`, `pn_cache_behavior`, `pn_continued`,
and `pn_restore`.

An earlier recovery cut also proved that packaging failure after successful
qualification does not redispatch the effect. The exact retained Docket result
was later observed and admitted. Its first Nightshift composition closed as
decision-inadequate because the diagnostic recurrence basis was wrong; a
later diagnostic slot recomposed the same historical source for the same O1
target and succeeded. No evidence, execution, or authority was recreated.

## Conflict and absence

Custody refuses conflicting records for one exact attempt/settlement slot.
Evidence about different exact executions remains separate. Nightshift never
averages claims, chooses a winner by UI timestamp, or overwrites history. If
the relevant lineage is contradictory or the requested observation is
ambiguous, the resolver reports contradiction and consequence fails closed.

Settlement without an authenticated application observation proves only
settlement. A decision whose profile requires application evidence remains
uncomposed; no application health, currentness, standing, or authorization is
inferred.

## Read inspection and cross-probing

`nightshift cycle export-observation` retains the complete v3 record. Its
composition provides the exact chain:

```text
PlanNode
  -> compilation/output
  -> exact work/proposal/occurrence
  -> issuance/attempt/settlement
  -> application claim/custody
  -> canonical Nightshift observation
  -> successor occurrence
```

Phosphor-ng displays source evidence time and arithmetic age separately from
the owner-produced Nightshift resolution and profile horizon. `/inspect`
remains GET/HEAD-only. `/design` may consume exact owner-produced links for
read navigation; it cannot mutate Nightshift, AG, or Docket.

## One-shot operation

After custody import, a base successor cycle request references only the exact
source observation, custody, and independently configured profile identities.
The read-only packager resolves those objects and emits the sealed canonical
request whose observation identity binds the composition:

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite \
  external-observation prepare-cycle \
  --request /bounded/successor-cycle-base.jcs.json \
  --profile /etc/nightshift/external-evidence-profile.jcs.json \
  > /bounded/successor-cycle-composed.json
```

The exact profile file is provisioned by the deployment as RFC 8785/JCS bytes;
the checked-in example documents its fields and content identity but repository
text-file newline conventions are not a credential or deployment artifact.
The canonical run path then receives the same profile independently:

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle run \
  --request /bounded/successor-cycle-composed.json \
  --external-evidence-profile /etc/nightshift/external-evidence-profile.jcs.json \
  ...ordinary NQ, present-support, AG, and Maude-custody options...
```

`prepare-cycle` writes no cycle and contacts no AG process. `cycle run`
re-resolves custody, latest predecessor, AG settlement coordinates, NQ
admission, and present support before persistence or proposal opening.

## Operational nonclaims

- There is no monitoring scheduler in this contract. The qualified
  post-settlement adapter is one-shot. Demand-driven stale re-observation is
  available only through the separate closed passive profile; it is not a
  periodic loop and cannot renew strong qualification.
- The local-Compose profile is not a generic evidence policy system.
- Physical clock trust, host principal isolation, real secret rotation, and
  executor/world correspondence remain deployment qualifications.
- Application evidence does not create standing, admissibility,
  authorization, Docket custody, execution permission, or retry eligibility.
