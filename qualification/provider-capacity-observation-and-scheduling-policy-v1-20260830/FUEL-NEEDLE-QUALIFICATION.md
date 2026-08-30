# FUEL-NEEDLE qualification

Campaign: FUEL-NEEDLE
Canonical slug: provider-capacity-observation-and-scheduling-policy-v1
Track: provider-capacity-observation
Packet: sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc
Branch: campaign/fuel-needle-provider-capacity-observation-and-scheduling-policy-v1-20260829
Qualified subject commit: c29c1c8f7c0b9845499fd25d81a4e93ead839fcd
Rejected predecessor result head: 896098a87daa7d51a8e8a87b95180b677eb4f788
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

An independent acceptance audit refused the predecessor result head because it
could treat a missing quota window as complete testimony, did not close
decision state/admission invariants at both Rust and JSON Schema boundaries,
checked response size only after line allocation, and did not bind the executed
Codex binary and reported protocol version. Commit c29c1c8 repairs all four
findings without rewriting the rejected predecessor.

## Qualified subject

The subject defines three closed contracts:

- nightshift.provider-capacity-observation/v1;
- nightshift.provider-capacity-policy/v1;
- nightshift.provider-capacity-decision/v1.

Their digests are RFC 8785/JCS projections with distinct NUL-terminated domains.
Every decision retains the exact observation and policy digests. Source class
and confidence remain separate. ABUNDANT, NORMAL, CONSERVE, CRITICAL, and
UNKNOWN are closed scheduling-policy states, not campaign classifications.

The default policy explicitly requires FIVE_HOUR and WEEKLY window types.
Absence of either yields UNKNOWN and NO_NEW_WORK; the remaining fraction is
evaluated only after the required set is complete. Rust validation and JSON
Schema conditionals both refuse digest-consistent state/admission/flag
substitution.

The live probe requires an operator-supplied canonical native-executable path,
raw executable SHA-256 digest, and expected protocol version. It opens and
verifies the executable before descriptor-pinned invocation; initialize must
report the exact expected `codex_cli_rs/VERSION` before
account/rateLimits/read can become usable. Its collector checks the byte limit
incrementally before extending its message buffer. It performs no PTY
interaction, browser access, direct configuration/session/credential-file
read, account mutation, model turn, listener, or provider retry. It retains
only domain-separated digests, and kills, waits, and joins on every post-spawn
result path.

## Deterministic qualification

Commands run against exact qualified subject
c29c1c8f7c0b9845499fd25d81a4e93ead839fcd:

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

- Rust workspace: 338 passed, 0 failed, 13 ignored.
- The ignored set is the existing two opt-in real AG adapter cases, ten opt-in
  adjacent AG/Docket integration cases, and one opt-in Monitor/NQ/Pulse case.
- Provider-capacity crate: 19 passed, 0 failed.
- Python schema/report suites: 13 passed, 0 failed.
- Formatting and all-feature warnings-denied Clippy: passed.
- Canonical Nightshift no-actuation, Casework backend/UI read-only, and provider
  capacity boundary gates: passed.
- Every deterministic negative control made its corresponding gate refuse.

Qualification cases cover authoritative, observed, inferred, and unknown source
classes; distinct confidence; all five states; impossible percentage; malformed
and layout-mutated response; contradictory windows; provider refusal; timeout;
no output; oversized output disposition; staleness; reset rollover; exact
digest reproduction; content mutation; explicit required-window absence;
digest-consistent decision substitution; minimum-window policy; context/quota
separation; fixed-chunk oversized/no-output/timeout collection; executable
symlink, wrapper, and digest substitution; initialize version substitution; and
safe custody of already active work under CRITICAL or UNKNOWN.

## Historical unbound probe evidence

Installed client: codex-cli 0.147.0
Observed at: 2026-08-30T20:14:26.864158168Z
Remediation live probe launched: no
Disposition: UNKNOWN
Source/confidence: UNKNOWN / UNKNOWN
Reported usable windows: one weekly window
Remaining fraction: 0.94
Reset: 2026-09-05T21:21:03Z
Raw-response digest:
sha256:3ad8332275fb815b4b20d6ff21d89a2e42b82e3510377933dc52de7ef9e6d58f
Executable path/digest: unavailable in the historical capture
Observation digest:
sha256:ae0da1931218b4eee91052c316856c1737d32f7d039ee8dca37c7a5fa4bf9d86
Policy digest:
sha256:7567588f6fe319430e5d83e5955c7c20ded2dcb8e5554676db9a1a6482305cb0
Decision digest:
sha256:9d17f39b44fa3c9ad64856b5c17117cccc43af6df52198d1fb2e64d62ec40da6
Decision: UNKNOWN / NO_NEW_WORK
Reasons: EXECUTABLE_IDENTITY_UNBOUND, REQUIRED_WINDOW_MISSING_FIVE_HOUR

The earlier supported response supplied no separate short-window value and did
not capture the exact native executable identity. No missing capacity or
executable identity was inferred. Its exact raw-response digest remains
available as historical custody, but the repaired contract projects it to
UNKNOWN and forbids new work. No replacement live probe was needed or launched.

## Negative boundaries

The structural gate refuses provider mutation/login/reset methods, thread or
turn creation, PTY/web/browser/configuration/session/credential surfaces,
unbound PATH/pathname execution, independent network clients or listeners,
filesystem mutation, aggregate classification or health synthesis, and
installed provider-capacity timers or services. Its deterministic substitution
fixture proves those refusals.

CRITICAL and UNKNOWN allow admitted active work to reach receipt custody but
admit no new work under the default policy. Reset rollover and expiry require a
new observation. Neither event invents restored capacity. No automatic retry
exists.

## Custody and limitations

No raw provider response bytes, account identifier, secret, credential,
provider session, browser profile, timer, service, listener, or spool were
retained. No provider process was launched during remediation. The safe display
locator local-codex-profile is mechanism metadata, not account identity.

The Codex bootstrap parser is pinned to the expected 0.147.0 App Server response
shape and requires executable plus initialize-version verification for usable
testimony. Layout or identity movement becomes UNKNOWN. Descriptor-pinned
execution is currently Linux-only; other platforms become UNKNOWN. Other
providers have no live adapter in this campaign. The durable foreman consumes
these contracts in its own campaign; FUEL-NEEDLE does not modify foreman
scheduler state.

Successor base policy: exact result ancestry. The final campaign result head is
the remote-verified branch head reported at publication; the independently
qualified implementation subject remains the exact commit above.

## Next lawful action

CLOCKWORK-MOTH may consume the closed provider-neutral contracts and retain the
exact observation/policy digests for each scheduling decision. It must preserve
UNKNOWN as conservative admission, keep context separate from quota, and must
not import Codex App Server response fields into foreman ontology.
