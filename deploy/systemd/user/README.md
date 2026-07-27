# Night Shift — historical user-level dogfood specimen

> **Quarantined: do not install these files verbatim.**

This directory preserves the first local `sushi-k` dogfood unit as historical
evidence. The service pins one developer's absolute checkout, binary,
database, agenda, and state paths. Those paths are not a supported install
layout, are not portable to another account or clone, and must not be treated
as repository discovery or defaults.

The files remain useful only for reconstructing that pilot:

| File | Historical role |
|---|---|
| `nightshift-watchbill.service` | one non-actuating, user-level oneshot for the original host |
| `nightshift-watchbill.timer` | five-minute jittered cadence used by that oneshot |

For current use:

- follow the [operator quick start](../../../docs/operator/README.md) for the
  source-build and explicit `nq-monitor` injection contract;
- use the system-level reference scaffold in
  [`../`](../) when a timer-driven deployment is required; and
- create site-owned units and environment files with explicit absolute paths.

Do not copy the `/home/jbeck`, `scheduler`, or `notquery` values from the
historical service. Night Shift does not infer its executable, NQ executable,
database, or agenda from a workstation checkout.

This quarantine does not promote the shelf into a production deployment
story. It remains single-finding, non-actuating, unauthenticated local
dogfood with no remote operator support.
