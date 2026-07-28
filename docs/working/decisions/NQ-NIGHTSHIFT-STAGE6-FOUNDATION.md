# NQ–Nightshift Stage 6 foundation

Status: implemented bounded foundation; not a deployment or authority switch.

This unit adds a parallel NQ-NG operational-posture surface. It does not
reinterpret or mutate the historical Watchbill runtime.

## Exact producer-contract pin

The closed consumer implementation and copied conformance vectors correspond to the
local NQ-NG contract commit:

- commit: `81489644fef4cf56b9fb61e943e18d2313728931`
- tree: `921b45d082eea93f338f80300b9c8216d45718d5`
- contract:
  `crates/nq-core/src/diagnostic_execution.rs`
- vector source:
  `audit/nq-nightshift-stage6-foundation/vectors/`

That NQ-NG checkout has no configured remote. This pin records the exact
local producer source used for conformance; it does not claim remote
availability or publication.

Copied vector byte hashes:

| Vector | SHA-256 |
|---|---|
| `positive.json` | `89c7f685aa4aa717484dabe99b06e6ace9b8554f6e7be8e8b63fd9bf45df1545` |
| `refused.json` | `07680b2bada81225358a1203df2e67154ec3eccfc8a3c7225704d201ccdaaed3` |
| `provider_no_response.json` | `0f423c64a49b108971f2c13c92553457bf2fb7c6ec050270a4587d7620bbe628` |
| `hostile_projection_collision_match.json` | `e70ebbf3d4745e063d33c664bf8063b2c56fc893f28a00b11215055264a4e609` |
| `hostile_projection_collision_mismatch.json` | `74fda36f49652838a715e5c57dcecf27778963f27dbc3d2892a4a319118bb3cd` |

The machine journey is:

```text
exact nq.diagnostic_execution.v1 artifacts
  + closed Nightshift inventory
  + exact recurrence records
  + delivery standing
    -> immutable nightshift.operational_posture.v1
```

Invoke it with:

```text
nightshift diagnostics posture \
  --policy POLICY.json \
  --inputs INPUTS.json \
  --recurrence RECURRENCE.json \
  --evaluated-at 2026-07-27T20:00:10Z \
  --format json
```

The operation is pure and read-only. It does not open the SQLite store,
persist a snapshot, invoke a provider, schedule a process, prepare an action,
authorize anything, or execute anything. The explicit evaluation instant is
mandatory; wall-clock `now` is never substituted by the machine interface.

## Contracts

- NQ input: a closed consumer implementation of
  `nq.diagnostic_execution.v1`, pinned to the exact producer source and
  canonical vectors above, including
  complete input accounting, per-input acquisition intervals and clocks,
  per-claim dependency and state frontiers, producer/build/cohort identity,
  evaluator/policy/projection identities and projection omissions,
  raw/normalized/projected transform identities, limitations, and nonclaims.
  Nightshift validates the producer invariants required by this consumer
  before posture evaluation. Copied positive, refusal, no-response, and
  hostile projection-collision vectors qualify that boundary; they do not
  prove validator equivalence with NQ. This is structural validation, not
  producer authentication, evidence admission, or consumer reliance.
- Inventory: `nightshift.diagnostic_posture_policy.v1` declares a closed set
  of mandatory, optional, and excluded bounded questions with exact semantic
  bindings, required claim/state binding, and source-age policy. The policy
  carries explicit subject and role identities. Its self-identified
  `policy_id` commits the complete canonical policy preimage.
- Receiver input: `nightshift.diagnostic_inputs.v1` distinguishes delivered
  NQ artifacts from Nightshift `no_response`, `acquisition_failed`, and
  `not_configured`. Missing and duplicate declared inputs are derived without
  using latest-wins behavior. Undeclared inputs remain visible but mint no
  inventory obligation. Its self-identified `inputs_id` commits the exact
  receiver-side input set, including delivered canonical NQ artifacts.
- Recurrence: `nightshift.recurrence_evidence.v1` binds deterministic stable
  jitter, immutable run-slot identity, attempts/requests, execution budgets,
  exact NQ request/run/artifact/attempt references, standing windows, and
  delivery standing. Historical records may remain in the ledger; evaluation
  selects the exact deterministic due slot, and duplicate means more than one
  record for that exact slot rather than more than one historical record for
  the diagnostic key. Its self-identified `recurrence_id` commits the exact
  obligations, retained slot records, invocation attempts, artifact
  references, and delivery standing.
- Output: `nightshift.operational_posture.v1` exposes separate completeness,
  condition, coverage, recurrence, and delivery axes. Its `posture_id` is the
  SHA-256 identity of the RFC 8785 JCS output preimage with `posture_id`
  omitted. The committed preimage embeds the complete validated posture
  policy, exact receiver input evidence, exact recurrence evidence, derived
  assessments, ordered closed inventory and schedule obligations, and the
  fixed semantic identity
  `nightshift.operational_posture_evaluator@1`.
  `OperatorProjection` is an immutable internal rendering projection bound to
  the source posture identity and generation. It retains every assessment in
  source order with explicit shown/omitted visibility, recurrence display,
  delivery standing, and a derived headline under its own JCS identity. It is
  not a published standalone machine contract in this unit; JSON output is
  the canonical posture artifact. A future standalone projection contract
  requires its own schema, read/export surface, and drift gate.

Positive freshness is evaluated independently for every received input on
which the primary NQ claim depends, under that input's own acquisition
interval, clock identity, and uncertainty. Nightshift does not synthesize a
cross-clock interval. A refused or unsupported NQ artifact has no primary
claim; its NQ-owned attempt interval is used only to age that attributable
attempt/refusal. NQ completion, receipt, transport, or Nightshift evaluation
time never refreshes source testimony. Source acquisition intervals may
predate the Nightshift invocation and may use different clocks; they are not
required to fit inside the invocation attempt. Recurrence instead binds the
exact NQ request, run, artifact, and NQ attempt interval.

## Operator truthfulness

The generated internal operator projection retains every closed-inventory
assessment, exact artifact/request/run/claim reference, NQ outcome axes,
primary-claim state bindings, and recurrence standing in source order. It
references the unchanged source posture rather than cloning and resealing a
modified posture. A visual projection marks omitted rows rather than deleting
them, and any omission forces an incomplete headline. A clean headline
requires complete current mandatory testimony, jointly established NQ
coherence, complete NQ coverage, current recurrence, and required delivery
qualification. Coverage is derived independently from condition, derivation,
and coherence: current complete coverage remains visible even when another
axis blocks completeness, while partial current testimony yields narrowed
coverage. Contradictory, partial, stale, future-dated, state-mismatched,
refused, no-response, missing, duplicate, and unconfigured states remain
distinct.

## Explicit nonclaims

This foundation does not establish:

- a complete Host Operational Portrait;
- cross-diagnostic joint-state semantics;
- notification transport or delivery qualification;
- persistent Nightshift schedule workers;
- an exact Nightshift binary build-provenance identity (the fixed evaluator
  semantic identity is not a substitute; repository-native build provenance
  remains a follow-on requirement);
- recovery/supersession episode tracking;
- a published standalone operator-projection schema or read/export contract;
- consumer reliance for an operational purpose;
- proposal, authorization, execution, deployment, or cutover;
- equivalence between the Rust runtime and the Lean model.

The Lean relationship is a pinned correspondence and hostile-test guide, not
a proof that the Rust implementation refines the model. See
`NQ-NIGHTSHIFT-LEAN-CORRESPONDENCE.md`.
