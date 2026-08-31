# EPOCH-LANTERN qualification

Campaign: EPOCH-LANTERN

Canonical slug: nightshift-operational-observation-temporal-lineage-v1

Exact predecessor/result parent: 87e39aeaac07c74819a7e20a24cc905ad8929d63 (STILL-CIPHER).

Exact FIELD pins:

- Monitor b2d52fe34f146774cbf5601819982c267c7fb082;
- NQ 39b9f84f2f70955dd12e5cbfe798c740f9e52854.

Sealed V2 packet: sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc.

## Qualification dimensions

The checkpoint directly exercises:

- exact signed Monitor byte reopening and Ed25519 verification;
- exact Monitor and NQ raw-byte custody plus independent semantic digests;
- closed typed stable-basis identity and exact producer key binding;
- produced testimony, no-response/failure, refusal, cannot-testify, and contradiction paths;
- independent acquisition, producer, receiver, NQ, admission, and evaluation time ordering;
- profile-owned max age and half-open currentness horizon;
- exact replay convergence, fork refusal, missing predecessor refusal, and successor admission;
- NQ claim-subset preservation and claim-widening refusal;
- unknown-field/control-mechanism refusal;
- no producer-class precedence, aggregate classification, or target-effect authority;
- unchanged two-binary canonical production graph.

## Commands

The exact checkpoint qualification ran:

    cargo test --locked -p nightshiftd operational_lineage --lib
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    cargo fmt --all -- --check
    bash scripts/check_operational_lineage_boundary.sh
    bash scripts/check_operational_lineage_boundary.sh --self-test-inject
    bash scripts/check_no_actuation_surface.sh
    bash scripts/check_no_actuation_surface.sh --self-test-inject

The non-rewriting correction additionally ran:

    cargo test --locked -p nightshiftd operational_lineage --lib
    python3 -B -m unittest tests.test_operational_lineage_schema
    cargo clippy --locked -p nightshiftd --lib --all-features -- -D warnings
    cargo fmt --all -- --check
    jq -e . schemas/nightshift.operational-observation-lineage.v1.schema.json
    jq -e . schemas/nightshift.operational-reobservation-evaluation.v1.schema.json
    bash scripts/check_operational_lineage_boundary.sh
    bash scripts/check_operational_lineage_boundary.sh --self-test-inject
    bash scripts/check_no_actuation_surface.sh
    bash scripts/check_no_actuation_surface.sh --self-test-inject

Corrected focused Nightshift owner qualification: 13 passed, 0 failed. Executable Draft 2020-12 schema qualification: 4 passed, 0 failed. The full locked workspace all-targets suite passed, including 198 nightshiftd library cases with 2 documented environment-dependent ignores. Both operational-lineage gate modes, both canonical no-actuation gate modes, warnings-denied workspace all-targets all-features Clippy, formatting, schema parsing, exact two-binary census, and diff hygiene passed.

## Rejected checkpoint correction

Checkpoint 17d0743909eff79d2b95cfdd40477712e23a8a37 remains immutable but is not an accepted predecessor. Its successor verifies the Ed25519 transcript over the exact supplied raw Monitor body slice, preserves fractional RFC3339 precision, mirrors FIELD Monitor 512-byte and 32-item limits plus the closed locator enum, recomputes embedded identities, cross-binds NQ finding references and values, and executes exact family-specific JSON Schema parity tests. Independently fixed accepted FIELD Monitor and NQ bytes plus four refused Monitor vectors are retained under crates/nightshiftd/tests/fixtures/operational_lineage.

Checkpoint bf64f4d6ef4d9291f0c4ca6d471698364bbc45da also remains immutable and is not an accepted predecessor. Its successor makes the runtime and executable schemas share an explicit printable-ASCII metadata subset without widening FIELD, and mirrors the exact NQ qualify_one closure across every input: unopened refusal, reopened refusal, produced support/cannot-testify, acquisition-failure cannot-testify, and exact contradiction derivation remain mutually constrained.

## Custody and scope

No FIELD artifact is changed. No listener, subprocess, service, browser, provider session, credential, secret, or external office is used. This campaign does not activate a runtime path or alter a default branch.

The exact checkpoint SHA is the commit containing this receipt; independent acceptance and closeout remain pending. No aggregate result is stated.
