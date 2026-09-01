# TALLY-FOX — AG NG one-use authorization mutation conformance V1

> **Track:** `authorization-state-conformance`
> **Codename:** `TALLY-FOX`
> **Canonical slug:** `ag-ng-one-use-authorization-mutation-conformance-v1`
> **Status:** **PLANNED / NOT STARTED**
> **Result classification:** none
> **Filed:** 2026-08-31
> **Codename collision search:** clear in Nightshift content and reachable local branch names at filing time.
> **Authority:** documentation only; no AG mutation, issuance, consumption, approval response, Docket attempt, or target effect.

## Purpose and prerequisites

TALLY-FOX is a future AG-owner campaign to qualify the exact mutation consuming one authorization once in one authoritative AG history. Its contribution is **one-spend / one-attempt bounded effect custody**: AG owns the spend, Docket attempt/dispatch admission, and ABSD same-attempt enactment custody.

Canonical AG is `/data/git/ag_ng`; owner source is `crates/ag-app/src/governed_loop.rs`. Exact native head is `c612fadf8e86304dd54115ce0be54737a428c315`; C1 is `278867fb0e5f106a6fe39fd52e14c63d6e3ca9c4` and descends native, so no analogous AG split was found. Activation still re-verifies remote identity, branch policy, clean ancestry/base, and exact qualification. Repository/wire identities are catalogued in `/data/git/docket/constellation-canonicalization/{stage0-repository-identities.md,stage3-wire-contract-catalog.md}`. TALLY is independent of VIOLET and must not mutate Docket, ABSD, civild, or a target; civild is excluded absent a new seam.

## Exact consumption law and qualification

Exact-once authority consumption means exactly one accepted transition for one authorization occurrence in one bounded authoritative AG history. Bind authorization/issuance, subject, audience, action, scope, currentness, one-use/spend identity, predecessor/history identity, consumer/attempt, mutation and custody time, refusals, and receipt. It is not a global distributed transaction or proof of attempt creation/effect enactment.

Qualify: valid mutation yields one accepted row/receipt; identical replay converges; fresh reuse refuses; concurrent writers converge; restart preserves consumed state/predecessor; authorization/subject/audience/action/scope/currentness/consumer/attempt/history substitutions refuse; missing, malformed, stale, revoked, consumed, and outcome-unknown inputs remain distinct; interruption cannot leave consumable state paired with an accepted receipt; query-only replay is non-mutating; no production/default registration, approval response, dispatch, effect, or aggregate result. Exit zero, response text, or absent duplicates is insufficient without relational reopen and cross-binding.

Keep distinct: (1) exact-once authority consumption in AG; (2) at-most-once Docket attempt/dispatch admission; (3) ABSD idempotent/effectively-once same-attempt enactment; (4) outcome-unknown same-attempt reconciliation. None implies another; no literal physical exactly-once claim follows from absent duplicates.

## Historical reliance basis

TALLY must preserve three different answers without collapsing them into one
`admissible`, `authorized`, or `valid` status:

1. **Original decision** — the exact AG decision actually recorded at the
   historical cut.
2. **Historical warrant** — whether the exact evidence, policy, authority,
   subject/version, scope, and temporal basis recorded for that cut supported
   that decision.
3. **Current support** — what the complete record supports now after later
   evidence, revocation, correction, policy change, or observation.

The accepted native AG design already retains the original decision and spend
as distinct durable state. A bounded read-only inspection at exact AG subject
`c612fadf8e86304dd54115ce0be54737a428c315` found that the stored runtime
profile pins the exact-work catalog by path and byte digest, while the campaign
store retains the runtime-profile bytes, decision `policy_basis`, complete
resolver result, resolver identity, subject, scope, freshness windows,
standing, and spend. It does not retain the exact catalog bytes themselves.
Typed observation results may retain only an application-owned opaque basis
identity, with no general owner-addressable source-evidence locator. Therefore
AG custody alone does not yet guarantee independent reopening of every
historical warrant after configured source artifacts disappear.

TALLY's first owner qualification must separately test reopening of the exact
policy catalog content, observation/evidence record, resolver identity,
subject/version, authority, scope, and temporal/freshness basis. Current policy
or evidence may never replace the recorded historical basis. If authoritative
existing custody reopens every item, this remains a read-model clarification
and no schema changes. Otherwise the narrow owner correction is a versioned AG
successor that adds stable, owner-addressable references to the exact policy
catalog and evidence records used at the decision cut. It must not duplicate
evidence into Docket or ABSD, create a universal receipt, or make either
downstream component an owner of AG policy semantics.

Later owner records may affect current support without rewriting the original
decision. Where the current AG owner already has a lawful record location, an
append-only relation to the earlier decision must classify its effect as
`PROSPECTIVE_REVOCATION`, `RETROSPECTIVE_CORRECTION`,
`EPISTEMIC_CONTRADICTION`, or `POLICY_AMENDMENT`. No such record type is
authorized merely by this roadmap.

Qualification must cover policy change after enactment, evidence staleness,
later contradiction, prospective revocation, retrospective correction, wrong
subject-version evidence, missing historical policy, missing historical
observation, substitution of current evidence for historical evidence, and an
unrecorded AG-to-Docket provenance edge. The historical decision remains in
custody when historical warrant or current support changes.

Casework/Phosphor may render raw authorization, issuance, spend/predecessor, mutation receipt, consumer/attempt, currentness, and restart lineage. No approve, answer, consume, retry, dispatch, execute, reconcile, merge, promote, aggregate authority, or success control/verdict.

TALLY remains **PLANNED / NOT STARTED**, classification **none**. Future activation requires fresh authority, clean exact AG base, isolated worktree, closed fixtures, query-only audit, and independent review. No default merge, production/live route, history rewrite, credential access, or downstream effect. Its exact terminal result, including NOT-QUALIFIED, becomes a CROSS-LEDGER prerequisite artifact.
