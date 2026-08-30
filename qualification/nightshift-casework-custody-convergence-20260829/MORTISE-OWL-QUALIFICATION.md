# MORTISE-OWL qualification

Campaign: MORTISE-OWL

Canonical slug: `nightshift-casework-qualified-head-custody-convergence-v1`

Track: `nightshift-repository-custody`

Qualified subject:
`c13da63d434f5450a976cd8ac6d2c5a4301d3751`

Successor-base policy: exact MORTISE-OWL result-head ancestry.

## Custody convergence

The integration commit
`397ee9fe7f405ae3c0b44b3942fa003c47861c17` has the exact parents:

1. MAP-CABINET result head
   `ad36932be2d4aceeff71b848bad534aae1c3c938`;
2. INDEX-WREN result head
   `83ad130468c940848d3d0fb89ed9f48925baabd8`.

Both exact predecessor heads are ancestors. Neither predecessor was rebased,
squashed, rewritten, or replaced with another cherry-pick. The merge resolved
three add/add conflicts: two INDEX-WREN custody records retain both the earlier
local checkpoint and the later remote-verified closeout, and the Casework gate
retains its loader-scoped `openat` law alongside MAP-CABINET's separately
scoped compiled-UI law. No runtime, projection, API, schema, or UI semantic
choice was required.

The pre-merge topology and stable patch identities are recorded in
`PRE-MERGE-TOPOLOGY.md`. The closed
`nightshift.base-admission-receipt/v1` receipt records exact-result ancestry
and keeps verified content equivalence as implementation provenance only.

## Complete Casework qualification

All commands below ran against the qualified subject.

```text
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo test --locked -p nightshift-casework --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
```

Formatting and Clippy passed. The full workspace ran 332 cases: 319 passed,
13 documented environment-dependent cases remained ignored, and none failed.
The focused Casework package ran 28 cases; all passed.

```text
python3 -B -m unittest \
  tests.test_render_nightshift_reports \
  tests.test_casework_schema \
  tests.test_base_admission_schema
```

All eight renderer, schema, and base-admission cases passed, including the
closed-schema aggregate-result negative case and the base-receipt digest
vectors.

```text
bash scripts/check_no_actuation_surface.sh
bash scripts/check_no_actuation_surface.sh --self-test-inject
bash scripts/check_casework_read_only_surface.sh
bash scripts/check_casework_read_only_surface.sh --self-test-inject
bash scripts/check_casework_ui_read_only_surface.sh
bash scripts/check_casework_ui_read_only_surface.sh --self-test-inject
```

All three structural gates and their deterministic negative controls passed.
Canonical `nightshiftd` retains `autobins = false`, exactly the two declared
production binaries, exactly the two corresponding source files, and no
`src/main.rs`.

```text
cd ui/casework
npm ci
npm test
npm run build
```

The lock-file installation completed with 117 packages and no reported known
vulnerabilities. Vitest ran five files and 18 cases; all passed. The production
TypeScript/Vite build completed from local source and the committed lock file.

## Loopback and installed-browser replay

A fresh foreground Casework process loaded the exact VELVET-ORRERY and
Casework self-description run directories and the newly built UI. A fresh
temporary profile used installed Google Chrome 143.0.7499.109. The replay
observed:

- VELVET-ORRERY: 14 work items and six human questions;
- exact RIVER-CLERK identity-successor disposition;
- exact GLASSHOPPER closeout-complete/not-qualified state and classification;
- Casework self-description: two work items and zero questions with its two
  independent exact state strings;
- all four raw packet and receipt HTTP bodies byte-equal to their source files;
- POST returned 405, an unlisted asset returned 404, and declared deep routes
  returned 200;
- direct fresh-profile deep-link loads rendered their exact cases;
- keyboard activation focused `#main` and `#human-questions` without changing
  the run pathname;
- no aggregate result key or verdict and no button, textarea, content-editable,
  approval, dispatch, retry, execute, merge, or promote control.

The server, Chrome process, temporary profiles, and transient captures were
removed after replay. A process and listening-socket census found neither the
qualification listener nor the Chrome debugging listener. No service was
installed and no teardown obligation remains.

## Classification

`NIGHTSHIFT-CASEWORK-QUALIFIED-HEAD-CUSTODY-CONVERGENCE-V1-QUALIFIED`

This classification is independent of INDEX-WREN and MAP-CABINET. It qualifies
the converged repository custody base and the replayed Casework behavior. It
creates no aggregate result and grants no scheduling, execution, publication,
or other authority.
