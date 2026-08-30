# STILL-CIPHER qualification

Campaign: STILL-CIPHER

Canonical slug: `nightshift-foreman-query-only-store-correction-v1`

Track: `nightshift-orchestration-runtime`

Exact predecessor/result parent: `212bf10ebb4dcd2c1b142291afd9b84a2df89833` (BOLT-LOOM). Exact original CLOCKWORK-MOTH result `30373353d4472720bf62f60d378056658d068e88` remains in ancestry. Neither predecessor was rewritten.

Sealed V2 packet: `sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`.

Contract freeze and qualified subject: `5989bc010a341c831b7f51bb1cb21cb5fbc6690a`.

Successor-base policy: exact remote-verified STILL-CIPHER result-head ancestry.

## Reason for successor correction

The BOLT-LOOM head remains immutable evidence, but its CLI read/export verbs constructed `ForemanStore` through the writer path. That path initialized missing schema and selected WAL mode. Reading an absent path could create a database, and observing an existing store could create or change WAL/SHM state. This violated the V2 rule that export must not refresh or mutate state, so `212bf10...` is superseded as a successor integration base. BOLT-LOOM activated no service and retained no operator run.

Focused qualification also established that SQLite `mode=ro` alone can create an empty WAL file and SHM file when neither exists. The final contract therefore uses descriptor-bound `mode=ro&immutable=1` only when both sidecars are absent; with an existing WAL/SHM pair it retains and validates both sidecar inodes across `mode=ro`; partial or changing sidecar custody is refused.

## Corrected subject

All CLI read/export verbs use `ForemanStore::open_read_only`. It opens an existing regular main database with `O_NOFOLLOW`, retains that file descriptor, and gives SQLite a `/proc/self/fd/{fd}` URI, so later pathname replacement cannot redirect a read. It refuses absent, symlink, non-regular, incomplete-schema, partial-sidecar, unlinked-descriptor, and changing-sidecar cases. It performs no schema initialization, journal-mode assignment, writer pragma, or writer timeout. Live projection and event export use one deferred read transaction; final export is one exact retained-byte query.

Deterministic cases compare exact directory entries, main database bytes, complete table/index/trigger schema, WAL bytes, and SHM bytes before and after live projection, event export, and exact final export. They also cover an alternate-thread pathname replacement after admission: the reader remains bound to the originally admitted inode, while a newly opened reader sees the replacement and refuses the absent run.

No foreman result state, scheduler law, adapter identity, receipt schema, packet schema, approval surface, retry law, canonical `nightshiftd` binary, target-effect boundary, or aggregate classification changed.

## Qualification

The following ran against exact qualified subject `5989bc0...`:

    cargo fmt --all -- --check
    cargo test --locked --workspace --all-targets
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
    python3 -B -m unittest tests.test_render_nightshift_reports tests.test_casework_schema tests.test_base_admission_schema tests.test_foreman_schema
    npm ci
    npm test
    npm run build
    bash scripts/check_no_actuation_surface.sh [and --self-test-inject]
    bash scripts/check_casework_read_only_surface.sh [and --self-test-inject]
    bash scripts/check_casework_ui_read_only_surface.sh [and --self-test-inject]
    bash scripts/check_foreman_read_only_boundary.sh [and --self-test-inject]

The locked Rust workspace ran 343 cases: 330 passed, 13 documented environment-dependent cases remained ignored, and none failed. The foreman package contributed nine integration and two CLI cases. All 11 renderer/schema cases passed. All-feature warnings-denied Clippy and formatting passed. Casework ran 18 frontend tests across five files; all passed. The production frontend build succeeded from the exact lockfile dependency graph.

Every canonical, Casework backend, Casework UI, and foreman structural gate and deterministic negative control passed. Canonical `nightshiftd` still declares exactly two production binaries and has exactly two production binary source files.

## Custody and closeout

The STILL-CIPHER codename and canonical slug had no pre-existing repository, reference, or documentation collision before branch creation. The campaign branch is `campaign/still-cipher-nightshift-foreman-query-only-store-correction-v1-20260830`. Publication and exact remote SHA verification occur only after this qualification receipt is committed. No default branch changed.

No campaign process, listener, service, provider session, browser profile, credential, secret, or teardown obligation remains. Lockfile-exact frontend dependencies and build outputs are ignored worktree artifacts, not retained runtime state or tracked campaign evidence. No production or target-effect mutation occurred.

## Classification

`NIGHTSHIFT-FOREMAN-QUERY-ONLY-STORE-CORRECTION-V1-QUALIFIED`

This classification belongs only to STILL-CIPHER. It creates no aggregate result and grants no target-effect, approval, execution, production, service, publication, or default-branch authority.
