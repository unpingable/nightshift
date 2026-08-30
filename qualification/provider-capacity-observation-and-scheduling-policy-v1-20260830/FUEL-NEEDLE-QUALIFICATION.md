# FUEL-NEEDLE qualification

Campaign: FUEL-NEEDLE
Canonical slug: provider-capacity-observation-and-scheduling-policy-v1
Track: provider-capacity-observation
Packet: sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc
Branch: campaign/fuel-needle-provider-capacity-observation-and-scheduling-policy-v1-20260829
Qualified subject commit: 7578ad504071e95b8cb3bd5e0e7cb1935bc790c6
Starting seed: 20e56f983923d5fdb198b0fa4a43d86707c5c49b
Authorized integration base: 888dd1e140200394cfadb81dc0c9377887585b05

Classification:
PROVIDER-CAPACITY-OBSERVATION-AND-SCHEDULING-POLICY-V1-QUALIFIED

This is an independent FUEL-NEEDLE classification. It is not an aggregate
campaign or run classification and grants no target-effect authority.

## Scope and precedence

The direct operator authorization of 2026-08-30 explicitly superseded the
completed Casework campaign's provider-gas-gauge deferral. That older statement
was campaign-local, not a durable prohibition. Repository constitutional
boundaries remained in force throughout this work.

The canonical nightshiftd crate and its exact two production binaries are
unchanged. Provider-specific code exists only in the separate
nightshift-provider-capacity operator crate. The foreman-facing boundary is the
provider-neutral normalized observation, policy, and decision contract.

## Qualified subject

The subject defines three closed contracts:

- nightshift.provider-capacity-observation/v1;
- nightshift.provider-capacity-policy/v1;
- nightshift.provider-capacity-decision/v1.

Their digests are RFC 8785/JCS projections with distinct NUL-terminated domains.
Every decision retains the exact observation and policy digests. Source class
and confidence remain separate. ABUNDANT, NORMAL, CONSERVE, CRITICAL, and
UNKNOWN are closed scheduling-policy states, not campaign classifications.

The only live probe uses a fresh bounded foreground connection to the installed
Codex App Server and sends initialize, initialized, and
account/rateLimits/read. It performs no PTY interaction, browser access,
configuration/session/credential-file read, account mutation, model turn,
listener, or provider retry. It retains only a domain-separated digest of the
exact response line. The process is killed and reaped on every result path.

## Deterministic qualification

Commands run against exact qualified subject
7578ad504071e95b8cb3bd5e0e7cb1935bc790c6:

- cargo fmt --all -- --check
- cargo test --locked --workspace --all-targets
- cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
- python3 -B -m unittest for the provider-capacity, renderer, Casework, and
  base-admission schema suites
- scripts/check_no_actuation_surface.sh and --self-test-inject
- scripts/check_casework_read_only_surface.sh and --self-test-inject
- scripts/check_casework_ui_read_only_surface.sh and --self-test-inject
- scripts/check_provider_capacity_boundary.sh and --self-test-inject

Results:

- Rust workspace: 319 passed, 0 failed, 13 ignored.
- The ignored set is the existing two opt-in real AG adapter cases, ten opt-in
  adjacent AG/Docket integration cases, and one opt-in Monitor/NQ/Pulse case.
- Provider-capacity crate: 11 passed, 0 failed.
- Python schema/report suites: 11 passed, 0 failed.
- Formatting and all-feature warnings-denied Clippy: passed.
- Canonical Nightshift no-actuation, Casework backend/UI read-only, and provider
  capacity boundary gates: passed.
- Every deterministic negative control made its corresponding gate refuse.

Qualification cases cover authoritative, observed, inferred, and unknown source
classes; distinct confidence; all five states; impossible percentage; malformed
and layout-mutated response; contradictory windows; provider refusal; timeout;
no output; oversized output disposition; staleness; reset rollover; exact
digest reproduction; content mutation; minimum-window policy; context/quota
separation; and safe custody of already active work under CRITICAL or UNKNOWN.

## Live supported-probe evidence

Installed client: codex-cli 0.147.0
Observed at: 2026-08-30T20:14:26.864158168Z
Disposition: USABLE
Source/confidence: OBSERVED / HIGH
Reported usable windows: one weekly window
Remaining fraction: 0.94
Reset: 2026-09-05T21:21:03Z
Raw-response digest:
sha256:3ad8332275fb815b4b20d6ff21d89a2e42b82e3510377933dc52de7ef9e6d58f
Observation digest:
sha256:611be49a04413b5e84321a2c19950bfde79ebf9a480d58678135a1ab64eea599
Policy digest:
sha256:01d4b03d71901a04ff79e856bc389bcd9cfb1a9752fae4f8dad19c3f733724bb
Decision digest:
sha256:b0da49591b2ef22b9ce85c1106c1a2d969054e3cf17ff813afd48519576d4572
Decision: ABUNDANT / ORDINARY_BOUNDED

The supported response supplied no separate short-window value. No short-window
capacity was inferred. The live record expired after its bounded fifteen-minute
observation lifetime; it remains historical qualification evidence and is not
current scheduling testimony.

## Negative boundaries

The structural gate refuses provider mutation/login/reset methods, thread or
turn creation, PTY/web/browser/configuration/session/credential surfaces,
independent network clients or listeners, filesystem mutation, aggregate
classification or health synthesis, and installed provider-capacity timers or
services. Its deterministic substitution fixture proves those refusals.

CRITICAL and UNKNOWN allow admitted active work to reach receipt custody but
admit no new work under the default policy. Reset rollover and expiry require a
new observation. Neither event invents restored capacity. No automatic retry
exists.

## Custody and limitations

No raw provider response bytes, account identifier, secret, credential,
provider session, browser profile, timer, service, listener, or spool were
retained. The safe display locator local-codex-profile is mechanism metadata,
not account identity.

The Codex bootstrap parser is pinned to the qualified 0.147.0 App Server response
shape. Layout movement becomes UNKNOWN. Other providers have no live adapter in
this campaign. The durable foreman consumes these contracts in its own campaign;
FUEL-NEEDLE does not modify foreman scheduler state.

Successor base policy: exact result ancestry. The final campaign result head is
the remote-verified branch head reported at publication; the independently
qualified implementation subject remains the exact commit above.

## Next lawful action

CLOCKWORK-MOTH may consume the closed provider-neutral contracts and retain the
exact observation/policy digests for each scheduling decision. It must preserve
UNKNOWN as conservative admission, keep context separate from quota, and must
not import Codex App Server response fields into foreman ontology.
