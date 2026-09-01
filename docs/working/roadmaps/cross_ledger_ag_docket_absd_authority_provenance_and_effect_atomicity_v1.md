# CROSS-LEDGER — AG/Docket/ABSD authority provenance and effect atomicity V1

> **Track:** `governed-effect-integration`
> **Codename:** `CROSS-LEDGER`
> **Canonical slug:** `ag-docket-absd-authority-provenance-and-effect-atomicity-v1`
> **Status:** **PLANNED / NOT STARTED**
> **Result classification:** none
> **Filed:** 2026-08-31
> **Codename collision search:** clear in Nightshift content and reachable local branch names at filing time.
> **Authority:** documentation only; no AG, Docket, ABSD, civild, target, route, or production mutation.

## Purpose and entry gate

CROSS-LEDGER composes exact provenance across AG, Docket, and ABSD. The strongest current property is **one-spend / one-attempt bounded effect custody**: exactly one AG consumption in one authoritative history; at-most-one Docket standing/attempt/marker per issuance and exact dispatch; ABSD same-attempt idempotent admission/journal; terminal replay with no mechanics; outcome-unknown same-attempt reconciliation. Scope is one AG history, one Docket database, and one stable ABSD attempt-store/object domain. This is not global duplicate prevention, distributed atomic commit, guaranteed success, effect causation, or literal physical exactly-once.

CROSS remains **PLANNED / NOT STARTED** until VIOLET-SLUICE and TALLY-FOX each have terminal result heads and independent classifications. A terminal NOT-QUALIFIED is a valid result artifact and satisfies result availability, not semantic qualification. CROSS ingests exact result/classification/qualified subject if any/successor-base policy and must not restate a predecessor as qualified.

Before mutation, Docket and ABSD divergence must be closed by independently qualified non-rewriting owner bases: Docket native `7aee690d724ff3b91c03d2b55efa266bcaad0e1d`, C1 `191f51a83ce4f1708cb443ba2185d846d87fe2f1`, C2 `c49ad8d0f26fb2a13b9dbafdde84d7abfe1f867b`, base `c9f4a8a328434700b30757a7d668013d0ac63de0` (28 native-only/five C2-only); ABSD native `9b219246733e53dd52a02b9b7cc449436b9be24c`, C1 `cf57f1f62a6103c71ea1d925073eac35a90d73b6`, base `0fa64a9ca20f81ef3d80637cf65bc14f65f936f6` (53 native-only/one C1-only). AG C1 `278867fb0e5f106a6fe39fd52e14c63d6e3ca9c4` descends native `c612fadf8e86304dd54115ce0be54737a428c315`. Content equivalence cannot replace ancestry or inherited qualification.

## Ownership and evidence

AG `/data/git/ag_ng` and `crates/ag-app/src/governed_loop.rs` own issuance/consumption. Canonical Docket `/data/git/docket/runtime` and `crates/gwr-local/src/{authz_intake.rs,governed_loop.rs}` plus migrations `0003`/`0006` own standing/attempt/dispatch; `/data/git/docket` is evidence only. ABSD `/data/git/absd` and `src/bin/civil-mf-executor.rs`, `src/dispatch_custody.rs`, `src/writer.rs`, `qualification/DGE002-PROTOCOL.md`, `qualification/dge/receipt.toml` own enactment custody/reconciliation. Transport sources are `docs/governed-runtime/{executor-transport-v1.md,executor-transport-c1-qualification.md,governed-loop-c2-layering.md}` and `conformance/executor-transport-v1/`. civild evidence is `research/native-governed-mechanism-closure/{summary.md,milestone-index.toml,receipt.toml}` and `research/coupled-freebsd-convergence/{u002/summary.md,handoffs/A-010.md}` at subject `630c5c3e4ad71db96e35fcb08800614825b425a9`, result `7d5b01b779a62b19b53d0d3ea94a8d4c405230f6`; civild mutation is excluded. No Copper Semaphore was found; Copper Lantern is distinct.

## Four independent properties

1. **Exact-once authority consumption:** one occurrence consumed once in one AG history; says nothing alone about target mechanics.
2. **At-most-once attempt/dispatch admission:** one Docket DB admits no more than one standing, attempt, marker, and dispatch for the issuance; not proof of enactment.
3. **Idempotent/effectively-once enactment:** ABSD binds restart/duplicate delivery to one stable attempt and journal; target-specific, not literal physical exactly-once.
4. **Outcome-unknown reconciliation:** ambiguous loss remains unknown and inspects the same attempt/domain without a fresh spend, attempt, or dispatch.

Never promote absence of observed duplicates into a physical exactly-once claim.

Authorization consumption and enactment are separate local invariants connected
by exact issuance and attempt identities, with outcome-unknown reconciliation.
The strongest justified composed claim remains **one-spend / one-attempt bounded
effect custody** until a stronger property is independently qualified.

## Historical reliance views

Every cross-ledger presentation and qualification keeps three answers separate:

1. **Original decision** — what AG actually decided at the historical cut.
2. **Historical warrant** — whether that decision was supported by the exact
   evidence, policy/version, authority, subject/version, scope, and temporal
   basis recorded for that cut.
3. **Current support** — what the complete record supports now after later
   evidence, revocation, correction, policy change, or observation.

Changing historical warrant or current support never rewrites the original
decision. No aggregate `admissibility`, `authorized`, `executed`, `healthy`, or
success field spans these views.

## Two-plane operational architecture

The observation/attention plane remains:

```text
operational subject
  -> Monitor acquisition
  -> NQ qualification
  -> Nightshift temporal applicability
  -> read-only Casework
```

The authority/effect plane is separately owned:

```text
exact proposed work
  -> AG authorization occurrence and one-use authority-state mutation
  -> Docket authorized dispatch custody
  -> ABSD authoritative exercise and enactment custody
  -> resulting world state
  -> fresh Monitor observation
```

The planes join only through exact identities, predecessor/successor references,
raw owner evidence, and custody times. Casework visibility is not authority; AG
authorization is not execution; Docket dispatch is not proof of successful
effect; ABSD enactment evidence is not proof of the present postcondition. A
fresh Monitor observation, subsequently qualified by NQ and evaluated for
currentness by Nightshift, is required for a claim about the resulting world.

## Future qualification and read-only presentation

Cross-bind AG authorization/issuance/spend/predecessor/currentness/receipt; Docket DB/standing/attempt/marker/dispatch bytes/audience/plan/times; ABSD admission/stable attempt-store/journal/mechanics evidence/terminal/reconciliation; all independent times; refusal before consumption/attempt/enactment, terminal and outcome-unknown cases; restart/concurrent writers; identity/byte/history/audience/attempt/receipt substitutions; independent classifications. Exit status, acknowledgement, duplicate-free samples, and final target appearance cannot substitute for owner receipts; outcome unknown does not erase honest AG/Docket custody.

The read-only projection also exposes the exact policy and evidence references
used at the AG cut when they are mechanically available. It never substitutes
current policy, current evidence, reconstructed evidence, or an inferred edge
for missing historical custody. Missing required material is explicitly
`NOT_RECORDED`; unresolved or disagreeing authoritative material is explicitly
`INDETERMINATE`. Presentation stops at the first unsupported edge.

Before any owner-schema change, TALLY's durability qualification must reopen
the exact policy catalog/version/content, observation/evidence record, resolver
identity, subject/version, authority, scope, and temporal/freshness basis from
authoritative retained artifacts. If this succeeds, CROSS consumes those
references without a new receipt type. If it fails, only a versioned AG-owned
successor may add the narrow stable references needed for historical reopening;
Docket and ABSD continue to reference the chain by exact identities.

Where an authorized owner already records later changes, append-only links to
the historical AG decision distinguish `PROSPECTIVE_REVOCATION`,
`RETROSPECTIVE_CORRECTION`, `EPISTEMIC_CONTRADICTION`, and `POLICY_AMENDMENT`.
This roadmap does not itself authorize a new amendment service or record family.

Qualification includes policy change after enactment, evidence staleness,
later contradiction, prospective revocation, retrospective correction,
wrong-subject evidence, missing historical policy, missing historical
observation, current-for-historical substitution, and attempted inference of an
unrecorded AG-to-Docket-to-ABSD edge. Each case must retain the original
decision even when historical warrant or current support changes.

Read-only Casework/Phosphor show exact raw chain, digests, result heads, bindings, time axes, contradictions, missing custody, outcome unknown, remaining trigger, and next lawful action. The view walks proposal/work, AG authorization occurrence, authority-state spend, Docket dispatch and custody, ABSD exercise occurrence, effect outcome or settlement, and fresh postcondition observation. It stops exactly where evidence stops and never reconstructs an edge from matching payload text, timestamps alone, filenames, hostnames, process exit, target paths, or free-text descriptions.

The display keeps proposed, authorized, consumed/spent, dispatched, custody
accepted, exercise started, effect known successful, effect known absent, effect
outcome unknown, settled, and postcondition freshly observed semantically and
visually distinct. No approve, answer, consume, dispatch, retry, execute,
reconcile, merge, promote, remediation, aggregate authority/atomicity/health/
success, `executed=true`, `healthy=true`, or exactly-once verdict. The write
plane stays physically and semantically separate.

Activation requires fresh explicit mutation authority after terminal VIOLET and TALLY results, exact qualified owner bases, closed non-production fixtures, and independent review. No current packet edge, default merge, production route/service, credential operation, civild mutation, or target effect. Closeout reports owner/composed classifications independently, stopped/not-started lanes, outcome unknown, questions, custody limitations, and teardown; preserve NOT-QUALIFIED results and never retry consumed occurrences.
