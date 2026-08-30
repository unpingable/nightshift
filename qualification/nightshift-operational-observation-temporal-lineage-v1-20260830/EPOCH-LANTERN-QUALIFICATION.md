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

Focused Nightshift owner qualification: 8 passed, 0 failed. Both operational-lineage gate modes, both canonical no-actuation gate modes, warnings-denied focused Clippy, formatting, schema parsing, exact two-binary census, and diff hygiene passed.

## Custody and scope

No FIELD artifact is changed. No listener, subprocess, service, browser, provider session, credential, secret, or external office is used. This campaign does not activate a runtime path or alter a default branch.

The exact checkpoint SHA is the commit containing this receipt; independent acceptance and closeout remain pending. No aggregate result is stated.
