# CENSUS-MIRROR — declared-inventory observation divergence qualification

> **Track:** `operational-evidence-confounds`
> **Codename:** `CENSUS-MIRROR`
> **Canonical slug:** `declared-inventory-observation-divergence-qualification`
> **Status:** **PLANNED / NOT STARTED**
> **Result classification:** none
> **Filed:** 2026-08-30
> **Authority:** documentation only; this record authorizes no implementation, NetBox access, instance, worker, or infrastructure mutation.

## Roadmap boundary

CENSUS-MIRROR is a future read-only qualification campaign for adding NetBox
as an independently identified source of declared infrastructure state. It has
no active dependency edge in the sealed Nightshift V2 packet and does not block
or reprioritize that run.

The intended evidence relationship is:

```text
NetBox declared state
        ↘
          NQ claim qualification → Nightshift temporal lineage → Casework
        ↗
Monitor observed state
```

NetBox is not ground truth, authority, remediation, or a substitute for
observation. Monitor testimony is not declared inventory. Neither source
automatically dominates the other, and neither source grants target-effect
authority.

This roadmap creates no branch, schema, fixture, worker, NetBox instance,
container, listener, service, or implementation. Future activation requires
separate explicit authority after the foundational operational-subject and
observation contracts qualify.

## Source and office boundaries

A NetBox adapter may testify only to declarations supported by its exact source
contract and custody. Monitor may testify only to observations supported by its
exact acquisition contract. NQ owns claim qualification, including disagreement,
identity ambiguity, incomplete support, and cannot-testify outcomes. Nightshift
owns temporal applicability and re-observation attention without widening NQ
claim support. Casework renders exact owner artifacts read-only and creates no
disposition.

The future adapter must retain raw source identity, query/profile identity,
source revision or change marker where available, acquisition time, receiver
custody time, and exact response digest. Cache or transport custody must not be
promoted into NetBox source currency or truth.

## Initial subject families

Future qualification should deliberately define source bindings for:

- physical servers;
- virtual machines;
- sites and locations;
- interfaces and IP assignments;
- device and VM roles;
- tenant or ownership metadata where its exact claim meaning is defined;
- cluster membership and relationships;
- k3s nodes or workloads only under a deliberately defined NetBox representation;
- lifecycle declarations such as planned, active, offline, staged, or decommissioned;
- selected tags or custom fields only when an explicit source contract assigns
  them claim meaning.

Arbitrary tags and free-text comments acquire no operational semantics.

## Identity law

These identities remain distinct:

1. the NetBox object identity and exact source instance;
2. the Monitor observation identity;
3. the canonical typed operational-subject identity.

Each supported subject family needs an explicit binding rule. Operational
identity must not be derived solely from a hostname, IP address, NetBox object
ID, DNS name, Kubernetes node name, VM display name, or local filesystem path.
Those values may be locator or source evidence, but never silently become the
canonical subject identity.

Duplicate or ambiguous inventory matches must remain explicit qualification
outcomes. A route, cache key, API URL, database row locator, or transport
connection is mechanism data rather than identity or authority.

## Independent temporal axes

NetBox declarations and Monitor observations have independent histories.
Preserve separately:

- declaration/source revision time;
- source acquisition time;
- receiver custody time;
- NQ qualification time;
- Nightshift evaluation and currentness time.

A newer observation does not silently supersede a conflicting declaration. A
newer declaration does not retroactively change the fact that an earlier
observation was honestly made. Stale declaration, stale observation, unavailable
declaration source, unavailable observation source, and mutually contradictory
current evidence remain separate.

## Required future qualification cases

At minimum:

1. NetBox declares a physical host active while Monitor cannot reach it.
2. Monitor observes a host with no matching NetBox identity.
3. The sources bind one intended subject to different addresses.
4. A healthy observed VM is declared decommissioned.
5. A declared VM cannot be observed by the hypervisor or Monitor.
6. Declared and observed hardware identity disagree.
7. An observed k3s node has a different declared cluster or role.
8. Inventory changes while older observations remain otherwise valid.
9. Observation changes while inventory remains stale.
10. Both sources are individually valid and mutually contradictory.
11. NetBox is unavailable while Monitor remains available.
12. Monitor is unavailable while NetBox remains readable.
13. Both sources are unavailable.
14. Duplicate or ambiguous inventory records match one observed subject.
15. A transport or cache serves stale NetBox data with otherwise valid provenance.

These cases must not collapse into a generic `drift`, `inventory_matches`,
`unhealthy`, or aggregate-health classification. Acquisition failure, missing
source, identity ambiguity, contradiction, and temporal staleness are
independent dimensions.

## Future Casework presentation

SHIFT-ATLAS or an explicit successor may later render:

- the canonical typed operational subject;
- the current applicable NetBox declaration;
- the current applicable Monitor observation;
- exact source provenance and custody for each;
- NQ-supported claims and cannot-testify findings;
- contradictions and identity ambiguity;
- missing or unavailable sources;
- independent temporal lineage;
- remaining trigger; and
- next lawful action.

No single aggregate inventory-match or system-health verdict is created. Unknown
fields and arbitrary source metadata remain raw-only unless a later closed
contract assigns semantics.

## Read-only write boundary

CENSUS-MIRROR is read-only with respect to NetBox. It authorizes no object
creation, synchronization, automatic correction, reconciliation write, IP
reassignment, tag mutation, Kubernetes change, hypervisor change, or
remediation.

Any future reconciliation/write campaign must be physically and semantically
separate and pass the existing governed target-effect boundaries.

## Roadmap relationships

- **FIELD-CLOCK** supplies the typed operational-subject, Monitor acquisition,
  NQ claim-support, and Nightshift temporal handoff foundation.
- **DISTANT-BELL** may carry exact declared-state testimony, but delivery custody
  never becomes declaration truth or NQ qualification.
- **SHIFT-ATLAS** supplies the future operational-condition read surface.
- **SILICON-ORCHARD** supplies a non-agent operational specimen with exact
  environment and artifact identities.
- **FALLING-PIANO** may later exercise source outage, stale cache, partition,
  and contradictory-evidence cases under its own separately authorized
  occurrences.

CENSUS-MIRROR should follow qualification of the basic operational evidence
model so NetBox tests that model as an independent declared-state source instead
of forcing NetBox-specific semantics into the foundation.

## Activation gate

CENSUS-MIRROR remains **PLANNED / NOT STARTED**, with classification **none**.
It has no active V2 packet edge. Future activation requires an exact source
contract, fresh campaign authority, independently identified NetBox and subject
bindings, read-only credentials under separate custody, and its own
qualification and closeout.
