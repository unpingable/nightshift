# Artifact change and requalification v1

**Status:** canonical Nightshift/local-Compose lifecycle contract, 2026-08-23.

> Qualification follows the exact artifact that earned it, not the lineage
> of designs descended from that artifact.

> A design change may preserve history. It does not preserve earned
> qualification by implication.

This contract applies the qualification/steady-state evidence split to a
semantic PlanDocument successor. It introduces no artifact-equivalence rule
and no new authority. An inapplicable qualification is an observable
requirement for new exact governed work; it is not permission for the
observation orchestrator to repeat an effectful test.

## Exact lifecycle

```text
C1 / compiled work W1
  -> ordinary AG-NG authorization and Docket execution
  -> effectful fault qualification Q1
  -> passive observations S1/S2

typed Plan Core successor operations
  -> checked and locked C2 / compiled work W2
  -> Q1 remains historical for C1
  -> Q1 + C2 refuses by exact PlanDocument identity
  -> ordinary governed C2 qualification occurrence
  -> fresh one-use authorization and Docket execution
  -> Q2
  -> passive S3
  -> Q2 + current S3 supports C2 routine continuation
```

The qualification source is bound to its exact PlanDocument digest,
compilation, exact work, subject, scope, workflow profile, occurrence,
proposal, issuance, attempt, settlement, evidence custody, and claim set.
Stable PlanNode identity is useful for cross-probing but is deliberately
insufficient for qualification applicability.

## V1 deployment/qualification atomicity

The closed local-Compose V1 qualification workflow couples the authorized
operational deployment/effect with qualification of that same exact artifact.
It does not authorize an artifact to become an operational passive-observation
target independently of qualification applicable to that artifact.

This gives a negative reachability property for V1: there is no currently
governed transition to a state in which successor artifact C2 is deployed and
active as the passive observation target, lacks applicable qualification, and
is nevertheless eligible for passive Nightshift acquisition. Consequently,
the retained real-world lifecycle does not claim to have exercised a pre-Q2
passive C2 observation. S3 was acquired only after the governed C2
qualification settled.

This is a boundary of the present contract, not a claim that
deploy-before-qualify is inherently wrong. A future deploy-only, canary, or
staging workflow would require its own explicit authority, lifecycle,
rollback, custody, restart, concurrency, and qualification-interaction laws.
V1 does not infer any of those semantics.

Qualification is therefore not a mutable artifact attribute such as
`qualified(C2) = true`. Q is append-only evidence of one particular governed
qualification occurrence concerning one exact artifact.

V1 has no “minor change,” path-similarity, configuration-similarity, or
operator attestation that carries qualification between artifacts. If a
future workflow wants carry-forward, it needs a separate owner contract that
proves the relevant equivalence. This runtime does not guess.

## What happens to Q1

Q1 remains append-only historical evidence that C1 survived the exact
controlled fault test that occurred. Creating or governing C2 neither deletes,
supersedes, mutates, nor refreshes Q1. Nightshift checks the target
PlanDocument digest before passive currentness so the refusal cannot be hidden
by an unrelated stale-evidence result.

A second deliberate qualification of the exact same artifact is also a new
governed occurrence and a new evidence object. Both qualifications remain
inspectable; neither silently replaces the other.

## What passive evidence can establish

Passive evidence for C2 may establish only the closed steady-state claims:

- front door reachable;
- cache A present;
- cache B present;
- ordinary cache behavior observed.

It cannot establish that C2 survived a cache fault or that restoration
succeeded after that fault. The model-level substitution witness combining
current passive S3(C2) with Q1(C1) therefore remains inadequate for a C2
resilience-dependent decision. S3 was acquired after Q2; the hostile witness
proves that passive freshness cannot make Q1 applicable, not that V1 deployed
C2 as an unqualified passive target. Observation failure likewise grants no
remediation or qualification authority.

## Who may request requalification

Missing or inapplicable qualification appears as an exact refusal. A human,
PlanDocument workflow, or other already-authorized owner may then propose
closed C2 qualification work through the ordinary Nightshift → AG-NG →
Docket path. The observation-acquisition orchestrator cannot originate this
occurrence and cannot invoke cache stop/start mechanics.

The effectful executor receives only the already-authorized exact
`maude.local-compose-workflow/v1` qualification plan. AG burns a fresh,
occurrence-bound authorization before Docket dispatch. Q2 is evidence
resulting from the settled test; it is not authorization for the test.

## C1 and C2 PlanNode relations

The experiment preserves all 15 stable PlanNode IDs across the successor.
The workspace, document write constraint, acceptance criteria, and every
structured-work write path change semantically. The compiler consequently
produces a distinct Compose project, compilation, handoff, and exact work.

For example, `pn_continued` is present in both revisions:

```text
pn_continued @ C1 -> W1 qualification occurrence -> Q1
pn_continued @ C2 -> W2 qualification occurrence -> Q2
```

The shared node ID expresses semantic continuity for navigation. The C1/C2
artifact context determines which qualification applies.

## Failed qualification

A failed or incomplete effectful attempt cannot mint a successful Q. The
workflow adapter projects non-success claims as `unknown`; Nightshift's
strong qualification constructor additionally requires a successful outcome
and the complete satisfied claim set. Execution and evidence history remain
available. No automatic retry or repeated fault is scheduled.

## Qualified synthetic witness

The clean retained run is
`/tmp/ag-synthetic-cache-requalification-v6`; deterministic Plan Core and
compiler artifacts are in
`/tmp/ag-synthetic-cache-requalification-plan-v3`.

The exact design generation identities are:

| object | C1 | C2 |
|---|---|---|
| PlanDocument | `sha256:1238a9ea461709d9755bc05a91db797bb02e1b521eed059ca0081248ecad5261` | `sha256:c7baabf37ce84cfefbd1a34f3ab410ae9f2fbb3911fa0f6f50e860d342fa647c` |
| qualification compilation | existing C1 receipt | `sha256:42786faf82ad65d8610262b73540c362acd0ca1f1ff003ef4cf2d57f213e3e99` |
| qualification work | `sha256:78614416d6d42f303d20f66b23493b7caa93c29d3925d87d87cea07b9dfe7dcd` | `sha256:325c4755fffb077218e42d86fb51fb8f7aa6d9f4a49781bddeb0ed1f53266b47` |
| qualification evidence | `sha256:70bcea3153e80762ca6994c5a2434ea23efcb0d65fc805fa6a7315bf16e8b57f` | `sha256:4bf5454f225ff6fb59862c85df5ea148597f2d5c4e40c2633e2f1ad36f2f1c8a` |

C2 passive observation S3 is
`sha256:913f6896b5249c0c3cc1dfd85da0b6bcabfc3627b1176490adf594453867d468`.
The Q2+S3 decision-relative composition is
`sha256:89cb17e3be108ec3411e849fedcb2fb3c23dba7b1d4534e2bc7f16b7b0dbaa28`.
The AG replay records 28 transitions, four one-use spends, four Docket
attempts, and four settlements. Both Compose projects have zero remaining
containers and networks.

The bundle separately retains exact refusal witnesses:

```text
external application evidence refused:
historical qualification does not apply to the target PlanDocument
```

The first refuses Q1 against the C2 target before C2 qualification. The second
is a model-level substitution check performed after S3 exists: it combines
current C2 passive evidence with Q1 and proves that passive freshness cannot
carry Q1 onto C2. It is not evidence that pre-Q2 C2 was deployed as a passive
observation target.

## Inspection and nonclaims

Phosphor displays the qualification's exact artifact and historical governed
execution separately from passive evidence and the target occurrence. Exact
IDs remain available under progressive disclosure. The UI does not calculate
qualification applicability or currentness, and it remains GET/HEAD-only.
`/design` may display the exact C1/C2 drift and governed links read-only; it
cannot initiate qualification.

The local run uses the test NQ admission port and integration standing issuer.
It does not qualify production service-principal isolation, physical power
loss, live backup/restore, credential rotation, or executor/world
correspondence on a designated production host.

## If C2 changes again

C3 is another exact artifact. Q2 remains evidence for C2 and does not follow
C3. Passive C3 observations can describe only their closed present-world
claims. Any C3 decision requiring effectful resilience qualification refuses
until ordinary governed C3 qualification earns Q3.
