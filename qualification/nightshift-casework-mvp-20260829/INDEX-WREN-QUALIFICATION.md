# INDEX-WREN campaign record

- Codename: `INDEX-WREN`
- Canonical slug: `nightshift-run-receipt-casework-read-model-and-api-v1`
- Work item: `index-wren-backend`
- Branch: `campaign/index-wren-nightshift-run-receipt-casework-read-model-and-api-v1-20260829`
- Qualified implementation commit: `f50e5c4ee64ae16d29fa5b39dedf166ad9dca4f9`
- Campaign closeout commit: `83ad130468c940848d3d0fb89ed9f48925baabd8`
- Projection digest: `sha256:aa2e823cf8d8f323af1ed2e6a1cfc27dc84e8193f3915de75a03a348654651e8`

The normal repository collision search found no prior codename or canonical-slug
use outside this campaign's sealed packet and records.

## Independent classifications

| Dimension | Classification | Basis |
| --- | --- | --- |
| Receipt V1 compatibility | `QUALIFIED` | Renderer-loose scalar, joinable, question-link, custody-repository, unknown-extension, and timestamp fixtures pass without semantic promotion. |
| Projection determinism and schema | `QUALIFIED` | Exact domain-separated vectors reproduce; checked-in golden validates under Draft 2020-12; an aggregate-result injection is refused. |
| Loopback read API | `QUALIFIED` | The exact five GET routes, raw byte equality, ETags, restrictive headers, loopback refusal, and 405 write methods pass bounded wire tests. |
| Filesystem custody | `QUALIFIED` | Directory-handle-relative no-follow opens and already-open descriptor reads pass pathname and run-directory replacement cases. |
| Read-only structural boundary | `QUALIFIED` | Canonical and casework gates pass, including deterministic alternate-bind, filesystem-mutation, canonical-store, CORS, subprocess, client-dependency, and generic-listener injections. |
| VELVET fixture | `QUALIFIED` | Exact source bytes project 14 work items, six questions, independent custody sections, historical currentness, and no aggregate result. |
| Publication custody at implementation qualification | `LOCAL-COMMITTED-NOT-PUSHED` | The qualified implementation checkpoint was initially retained locally under the foreman then-current instruction. |
| Publication custody at campaign closeout | `REMOTE-VERIFIED-EXACT` | The later campaign closeout commit was pushed to the established Nightshift remote and independently resolved at the exact SHA. |

These classifications remain independent. This record creates no aggregate
campaign verdict.

## Qualification commands

```bash
cargo fmt --all -- --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 -B -m unittest tests.test_render_nightshift_reports tests.test_casework_schema
bash scripts/check_no_actuation_surface.sh
bash scripts/check_no_actuation_surface.sh --self-test-inject
bash scripts/check_casework_read_only_surface.sh
bash scripts/check_casework_read_only_surface.sh --self-test-inject
```

All commands passed on 2026-08-29. The workspace test suite retained its
documented environment-dependent ignored tests; no executed test failed.

## Closeout

Canonical `nightshiftd` retains exactly two production binaries. No service
was installed. No packet V1 schema, digest law, or `validate_at` behavior
changed. The only packet addition is integrity-only validation used to classify
historical evidence separately from currentness. No casework listener remains.
The campaign branch was published and verified at
`83ad130468c940848d3d0fb89ed9f48925baabd8` before the enclosing run-closeout
receipt was authored on the dependent MAP-CABINET branch. The earlier local-only
statement and later remote-verified statement describe different custody instants;
neither is discarded by the convergence merge.
