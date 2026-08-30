# CLOCKWORK-MOTH qualification

Campaign: CLOCKWORK-MOTH

Canonical slug: nightshift-durable-foreman-state-machine-v1

Track: nightshift-orchestration-runtime

Authorized integration base:
888dd1e140200394cfadb81dc0c9377887585b05 (MORTISE-OWL)

Sealed V2 packet:
sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc

Remote packet checkpoint:
dc3ef4f0ff0eda007fd5b46adab0fa347208d006

Contract freeze:
6ff870892f9c534fdce1b41d671fc99b70fed6f5

Qualified subject:
c4b313f29eba3e5b11b121ad69079022df8ed4dd

Successor-base policy: exact CLOCKWORK-MOTH result-head ancestry.

## Scope precedence

The direct operator instruction for the active V2 run explicitly superseded
the completed Casework campaign's local deferral of a durable foreman and
provider-capacity work. That older statement is historical scope evidence, not
a durable prohibition. This campaign did not relax canonical Nightshift,
approval, retry, target-effect, production, or default-branch boundaries.

The root-owned CENSUS-MIRROR roadmap-only commit
7390b8d5f0f0114abf37c30a43bb7836ece5cca2 is present in ancestry. It changes
no packet dependency, contract, runtime, or CLOCKWORK qualification meaning.

## Qualified implementation

The separate nightshift-foreman operator-tool crate provides:

- closed, domain-separated admission and execution-profile records;
- a closed generic worker start/event/terminal/not-started protocol;
- a SQLite WAL journal retaining exact packet, admission, profile, event,
  receipt, and final-snapshot bytes;
- append-only triggers for source, event, receipt, and snapshot evidence;
- deterministic replay into a closed scheduler-state projection;
- exact dependency terminality that permits entry evaluation only;
- no comparison or interpretation of result-classification strings;
- atomic opaque resource-lock acquisition and release;
- a fresh attempt identity and no V1 transition for a second attempt;
- same-attempt restart/resume;
- exact adapter event identity and duplicate refusal;
- lane-local human-question persistence;
- identity-bound terminal and not-started receipt acceptance;
- exact deterministic nightshift.run-receipts/v1 closeout only after complete
  explicit terminality;
- read-only status, replay, events, live export, and final export;
- deterministic admission/profile sealing;
- no subprocess, approval-response, target-actuator, provider-specific packet,
  or aggregate-result surface.

Canonical nightshiftd remains unchanged with autobins disabled, exactly two
declared production binaries, exactly two corresponding source files, and no
src/main.rs.

## Deterministic qualification

The following ran against the qualified subject:

    cargo fmt --all -- --check
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

The locked workspace ran 338 cases: 325 passed, 13 documented
environment-dependent cases remained ignored, and none failed. The focused
foreman package ran six integration cases; all passed. Formatting and
all-feature warnings-denied Clippy passed.

Foreman cases cover:

- admission/profile/digest/currentness substitution and duplicate run ID;
- WAL mode and append-only update refusal;
- dependency state without classification parsing;
- atomic overlap serialization and disjoint bounded concurrency;
- restart after claim and before dispatch;
- same-attempt resume and terminal-attempt resume refusal;
- provider completion before terminal receipt;
- wrong attempt identity and duplicate adapter event;
- human question on one lane while an independent lane continues;
- malformed, missing-field, and oversized terminal receipts;
- retained unknown extensions with no scheduler meaning;
- close refusal until all exact receipts are present;
- deterministic byte-identical final export;
- exact final bytes accepted by the qualified Casework loader;
- CLI admission sealing and absence of an approval command.

Executable Draft 2020-12 schema and renderer qualification:

    python3 -B -m unittest +      tests.test_render_nightshift_reports +      tests.test_casework_schema +      tests.test_base_admission_schema +      tests.test_foreman_schema

All 11 cases passed. Unknown fields, authority widening, approval-response
fields, and aggregate-result fields were refused where their closed contracts
apply.

Structural boundaries:

    bash scripts/check_no_actuation_surface.sh
    bash scripts/check_no_actuation_surface.sh --self-test-inject
    bash scripts/check_casework_read_only_surface.sh
    bash scripts/check_casework_read_only_surface.sh --self-test-inject
    bash scripts/check_casework_ui_read_only_surface.sh
    bash scripts/check_casework_ui_read_only_surface.sh --self-test-inject
    bash scripts/check_foreman_read_only_boundary.sh
    bash scripts/check_foreman_read_only_boundary.sh --self-test-inject

All gates and deterministic negative controls passed.

## Restart and recovery result

An occurrence stopped after atomic lock claim and attempt creation replays with
the same exact attempt identity and held claims. Resume records reconciliation
for that same attempt; it creates no new attempt. A provider completion
observation persists as waiting-provider evidence and does not become a
terminal result. A terminal attempt cannot resume or create an implicit second
attempt.

## Limitations and exact next graph

CLOCKWORK-MOTH qualifies the generic core and deterministic reference path. It
does not qualify a provider adapter, real Codex session, provider-capacity
policy, or browser surface.

The next lawful dependent work is:

1. TUNNEL-FINCH implements the external Switchyard adapter against the frozen
   start/event/receipt protocol and independently qualifies real isolated
   provider custody.
2. LEDGER-FOX consumes the frozen read-only live projection without adding
   scheduler mutation.
3. FUEL-NEEDLE supplies separately qualified capacity observations and policy
   digests; it does not change this core contract.
4. MIDNIGHT-RAIL begins only after its four exact predecessor results are
   independently accepted.

## Classification

NIGHTSHIFT-DURABLE-FOREMAN-STATE-MACHINE-V1-QUALIFIED

This independent classification qualifies the exact subject above. It creates
no aggregate result and grants no target-effect, approval, execution,
production, publication, or default-branch authority.
