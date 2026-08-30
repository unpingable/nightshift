# BOLT-LOOM qualification

Campaign: BOLT-LOOM

Canonical slug: `nightshift-durable-foreman-identity-binding-correction-v1`

Track: `nightshift-orchestration-runtime`

Exact predecessor/result parent:
`30373353d4472720bf62f60d378056658d068e88` (CLOCKWORK-MOTH).

Sealed V2 packet:
`sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`.

Contract freeze:
`f044843031be238d81327baa891a467a684d44a8`.

Qualified subject:
`60547014f6c7474d15e196412f430250a851749f`.

Successor-base policy: exact remote-verified BOLT-LOOM result-head ancestry.

## Reason for successor correction

The exact accepted CLOCKWORK-MOTH head was not rewritten. Review found that
execution profile V1 did not bind `adapter_version`; adapter events and
terminal receipts checked adapter ID but not version; later accepted events
could substitute provider/model/session/thread/turn/queue identities; and
closeout could predate retained terminal evidence.

These are acceptance-significant custody gaps under the V2 packet's
identity-bound terminal-receipt law. Exact head `3037335...` is therefore
historical evidence and explicitly superseded/not accepted as a successor
integration base.

## Corrected subject

The qualified subject provides:

- closed `nightshift.foreman-execution-profile/v2`, including exact
  `adapter_version` and a separate V2 profile-digest domain;
- event and terminal-receipt binding to the profile's adapter ID and version;
- incremental, write-once provider/model/session/thread/turn/queue identity
  custody, permitting an absent field to appear later while refusing every
  contradictory substitution;
- terminal receipt agreement with every identity already frozen by retained
  accepted events;
- optional identity syntax validation equivalent to required identities;
- closeout refusal when the requested snapshot time precedes any retained
  terminal receipt end time or not-started receipt time;
- the original non-authorizing scheduler, no-retry, classification-separation,
  exact-byte receipt, and two-binary canonical Nightshift boundaries.

The historical execution-profile V1 schema remains unchanged at its exact
predecessor commit. The corrected V2 loader does not silently admit, migrate,
or project a V1 profile database. Historical V1 state would require the exact
immutable predecessor implementation for inspection. CLOCKWORK-MOTH activated
no service and retained no operator run, so no live/durable V1 state required
migration.

## Qualification

The following ran against the exact qualified subject:

    cargo fmt --all -- --check
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    python3 -B -m unittest tests.test_render_nightshift_reports tests.test_casework_schema tests.test_base_admission_schema tests.test_foreman_schema

The locked workspace ran 338 cases: 325 passed, 13 documented
environment-dependent cases remained ignored, and none failed. The focused
foreman package ran six integration cases; all passed. The 11 renderer/schema
cases, formatting, and all-feature warnings-denied Clippy passed.

Corrected negative qualification cases explicitly cover:

- missing adapter version in the closed V2 execution profile;
- event adapter-version substitution;
- incremental provider identity establishment;
- contradictory provider, model, session, thread, turn, and queue event values;
- terminal adapter-version and all frozen provider-custody substitutions;
- close snapshot time earlier than retained terminal evidence;
- restart/replay preserving the frozen identities.

The canonical, Casework backend, Casework UI, and foreman structural gates and
their deterministic negative controls all passed. Canonical `nightshiftd`
still declares exactly two production binaries and has exactly two production
binary source files.

## Custody and closeout

The BOLT-LOOM codename and canonical slug had no pre-existing repository,
reference, or documentation collision before this branch was created. The
qualified subject has exact parent `3037335...`; no predecessor history was
rewritten and no default branch was changed.

No Nightshift/Casework/foreman campaign process, listener, service, provider
session, browser profile, credential, secret, or teardown obligation remains.
System listeners observed at closeout were pre-existing and unrelated to this
campaign. No production or target-effect mutation occurred.

## Classification

`NIGHTSHIFT-DURABLE-FOREMAN-IDENTITY-BINDING-CORRECTION-V1-QUALIFIED`

This classification belongs only to BOLT-LOOM. It creates no aggregate result
and grants no target-effect, approval, execution, production, service,
publication, or default-branch authority.
