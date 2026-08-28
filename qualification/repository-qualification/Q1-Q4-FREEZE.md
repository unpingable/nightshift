# Repository-qualification evidence path Q1–Q4 freeze

Date: 2026-08-27

## Exact predecessors

- NQ: `b941e61c6235dfb7a22f1a45bd3878e6e23414c4`, tree
  `789d2867ab8cce8f5e4faa6ef10b881249cab7b9`.
- Nightshift: `c94e870102214141ecc2196d10ce7364ebce822b`, tree
  `2dab7b3b4eb37f3e3fcca9301480d0c4aa32aeb7`.
- AG: `278867fb0e5f106a6fe39fd52e14c63d6e3ca9c4`, tree
  `aa35f478923a49cde994fc58c03803ef6c11289e`.
- Porter remained unchanged at `7f05c03caeb5ca326d0eef5b2edc2060a515cc5b`.
- Docket remained unchanged at `e21b9b8163b5d8ba28442ec119585438080e1fed`.

All five worktrees were clean when custody was established.

## Frozen implementation chain

- Q1 NQ contract: `35aa7cedfe4fc75302ee1b6a10938a90b687bece`.
- Q2 NQ evaluator/transport: `4f1e6a1018ff3f8133c707bd117ab56c537980b1`,
  tree `d90dd1e57c4721c0e848dc84865667178f708122`.
- Q3 Nightshift ingress/applicability: `3635a0c82594e58887af617059a8388eb5e0dd53`.
- Q4 Nightshift real-NQ specimen: `229d61d405ac7f1943f84b37f4cfbdf91ee353e6`,
  tree `28f1accc4c65c6a8ea3e9c452c8eab2d0b0ed2c4`.
- Q4 AG test-only typed-basis qualification: implementation commit
  `cfc6fd070fd916aaeba57cfc4793b2e9cb1b3e37`, narrowed by
  `6bcd117749f105926b0bf7059c5c693f40934e7f`; final tree
  `c7e5b42f1ff662e1bfc66d80c939414fd0751449`.

AG production source and program-counter law are byte-unchanged. The net AG
diff is one bounded test-file addition (225 lines) over its predecessor.

## Contract and authority result

Q1 freezes:

- `nq.campaign-stage-qualification-profile/v1`;
- `nq.campaign-stage-qualification-evidence/v1` factual evidence, with no
  caller verdict;
- `nq.campaign-stage-qualification/v1`, with only `QUALIFIED`, `FAILED`, and
  `INDETERMINATE`.

Every receipt explicitly disclaims standing, authorization, successor choice,
continuation, effect authority, freshness, and present applicability. An NQ
receipt remains immutable historical evidence. Nightshift alone evaluates
fresh applicability to the exact current `SettledObservationRequired` AG
predecessor and exact attempt/settlement. AG sees only the existing v3 opaque
type-and-identity envelope. Porter does not produce `QUALIFIED`; Nightshift
does not evaluate gates; AG does not parse NQ status or qualification detail.

No new authority office, policy language, continuation judgment, or program
counter was introduced.

## Qualification and hostile matrix

- Q1/Q2: exact schemas and nonclaims; no raw verdict; complete positive;
  complete negative gate/artifact/custody results; missing/reordered gates;
  profile, packet, predecessor, result, context, producer, artifact set,
  workspace predicate, worktree, and replay substitution.
- Q3: exact NQ executable/evaluator/profile/schema replay; immutable SQLite
  retention; `FAILED` and `INDETERMINATE` retained-only; exact settled
  predecessor/attempt/settlement; observation and subject binding; stale
  applicability without receipt invalidation; read-only subprocess
  exclusivity.
- Q4: exact type, applicability-profile identity, occurrence, observation,
  subject, work, resolver, currentness, and catalog binding; all substitutions
  refuse with zero AG spends. Exact green authorization spends once.

Executed gates:

- NQ core: 228 passed; monitor library: 254 passed, one unrelated ignored.
- Nightshift: formatting clean; Clippy warnings denied; full workspace green;
  182 library tests passed with two environment-gated ignores; exclusivity
  shell and compiled sentinels green.
- AG: Clippy warnings denied; governed-loop suite 40 passed, one explicit
  corpus-writer ignored. The real external Q4 vector passed.

The live cross-office specimen executed:

```
factual raw evidence
  -> real nq-monitor deterministic QUALIFIED receipt
  -> real nq-monitor exact replay
  -> Nightshift durable ingress and current applicability
  -> exact AG observation-resolution/v3 bytes
  -> existing AG ExactWorkCatalogV2 exact match
  -> one AuthorizationConsumed transition / one AG spend
```

## Explicit inherited qualification limitations

- NQ's repository-wide formatting and warnings-denied Clippy gates are already
  red on unrelated predecessor files; focused Q1/Q2 code, complete NQ core,
  and monitor-library tests are green. No unrelated cleanup was admitted.
- AG's full `ag-app` run has six existing host worker-sandbox failures in this
  execution environment; 125 other library tests passed before that group,
  and the complete touched governed-loop suite is green. AG's predecessor test
  file also has inherited formatter drift, so the Q4 diff was deliberately
  kept byte-bounded rather than rewriting it.

These limitations do not weaken the repository-qualification evidence path,
but prevent representing the entire unrelated repository gate surface as
green.

## Artifact identities

- NQ contract/evaluator source:
  `sha256:4dcc04408188c6079fecf84233bddf9a8cd08ffcf3fe63fce30a35536064e9e9`.
- Nightshift ingress/applicability source:
  `sha256:97360584daacffcd2357608ecdc5da3a73d30b654982dc0fbd8ea08ed9ca7ef4`.
- Nightshift live cross-office test:
  `sha256:426f8d388f0de8a30a3255af6faa850710502a7034d75f4ae5d201969c8b37f7`.
- AG governed-loop test after bounded cleanup:
  `sha256:e0678f8f6f7b308f6505bc445a122398d2de7e3ce9d3de91c666674616ef4f31`.
- Persisted 924-byte Q4 v3 resolution:
  `sha256:0bcaaa9f24f1c1ad7924f2b392b756545e18783af7795635f049e7c6bd035106`.

Classification: `QUALIFIED-WITH-EXPLICIT-LIMITATION`.
