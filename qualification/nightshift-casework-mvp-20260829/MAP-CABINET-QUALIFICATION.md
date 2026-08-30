# MAP-CABINET qualification

Campaign: MAP-CABINET

Canonical slug: `nightshift-read-only-casework-console-mvp-v1`

Dependency contract: INDEX-WREN
`f50e5c4ee64ae16d29fa5b39dedf166ad9dca4f9`

Independent INDEX-WREN closeout:
`83ad130468c940848d3d0fb89ed9f48925baabd8`

Frontend implementation checkpoints:
`8ab22ad59624d0862031aeeb9f87f25e954935e7`,
`01666c40b31d27a27cefaf344918b17548e23f6c`, and
`020b405`.

## Qualified surface

- React, TypeScript, Vite, and plain CSS operator surface under `ui/casework`.
- Stable run, work-item, question, custody, and raw-artifact client routes.
- Exact state, classification, track, question, packet-intent, receipt-outcome,
  repository-custody, and source-byte presentation without an aggregate result.
- Nullable renderer-compatible question and final-custody identities remain
  raw-only when unrecognized; navigation uses separate closed projection IDs.
- Same-origin GET-only API adapter and no answer, disposition, agent-control,
  or case-mutation control.
- Explicit `--ui-dir` integration that preloads the Vite-manifest-closed asset
  set through directory-handle-relative no-follow opens and serves only
  in-memory bytes on declared routes.

## Commands and observed results

```text
cd ui/casework && npm ci
```

117 packages installed from the committed lock file; npm reported zero known
vulnerabilities.

```text
cd ui/casework && npm test
cd ui/casework && npm run build
```

Vitest: 4 files, 16 tests passed. The production build completed and emitted
one hashed JavaScript asset, one hashed CSS asset, `index.html`, and the Vite
manifest. Golden journeys cover 14 work items, RIVER-CLERK, GLASSHOPPER, all
six human questions, exact filtering, every stable route, separate custody,
raw bytes, unknown strings, raw-only values, nullable linkage identities, and
the absence of mutation controls.

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 -B -m unittest tests/test_casework_schema.py
```

The full Rust workspace passed. The casework package contributed 28 passing
tests: 5 library/wire tests, 4 API tests, 17 projection tests, and 2 static UI
tests. Strict Clippy passed. Both executable Draft 2020-12 schema qualification
cases passed.

```text
scripts/check_no_actuation_surface.sh
scripts/check_no_actuation_surface.sh --self-test-inject
scripts/check_casework_read_only_surface.sh
scripts/check_casework_read_only_surface.sh --self-test-inject
scripts/check_casework_ui_read_only_surface.sh
scripts/check_casework_ui_read_only_surface.sh --self-test-inject
```

All three structural gates and all three deterministic negative controls
passed. MAP-CABINET's narrow static integration required the casework gate to
count the run-directory and compiled-UI `openat` sites independently; it did
not change INDEX-WREN source semantics or its independent classification.

Actual loopback HTTP and installed-browser observations are recorded in
`MAP-CABINET-BROWSER-TRACE.md`. The browser trace includes declared deep-link
delivery, unlisted-asset refusal, UI write-method refusal, exact RIVER and
GLASSHOPPER strings, 14-item and six-question counts, local hashed asset names,
and control absence. The supplementary screenshot is
`map-cabinet-run-case.png`.

## Custody and limitations

The campaign branch is locally committed and was not pushed, as directed by
the frontend worker handoff. Compiled `dist` assets and dependency installation
directories remain ignored reproducible outputs. Browser qualification used
headless Chrome rather than a screen-reader session; semantic HTML, keyboard
navigation, visible focus, color-independent exact labels, and responsive
layout are covered structurally and by component tests.

No service was installed. No casework listener, browser process, temporary
profile, or teardown obligation remains.

## Classification

`NIGHTSHIFT-READ-ONLY-CASEWORK-CONSOLE-MVP-V1-QUALIFIED`

This classification is independent of INDEX-WREN. It qualifies the read-only
operator information architecture and local delivery path; it grants no
authority and introduces no write/control plane.
