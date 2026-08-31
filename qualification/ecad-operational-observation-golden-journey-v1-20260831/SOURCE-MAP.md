# SILICON-ORCHARD source map

Campaign: SILICON-ORCHARD / `ecad-operational-observation-golden-journey-v1`
Track: `ecad-operations`
Packet: `sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`

## Exact predecessor and owner custody

- SHIFT-ATLAS remote result: `1493d4a419c9bac734284c791ebb98c28f9b7a20`.
- FIELD Monitor result: `b2d52fe34f146774cbf5601819982c267c7fb082`.
- FIELD NQ result: `39b9f84f2f70955dd12e5cbfe798c740f9e52854`.
- DISTANT-BELL remote result: `8a1adaae27a5da70398b445c152cd4e7548b0289`.
- SILICON Monitor fixture subject: `d75a062d42ccf339db200ffcee6559d3ddb18000`.
- SILICON NQ profile subject: `96a4ee826584fb0e27d7bec4fe168c05aa777cc8`.
- Research note: research main `29c14a8d4b65efcba4b9c33c387461581f99f2f6`,
  `docs/architecture/2026-08-17-proof-carrying-systems-eda.md`.

Monitor owns signed acquisition and exact ECAD payload testimony. DISTANT owns
transport custody. NQ owns claim support, cannot-testify, refusal, and
contradiction. Nightshift owns immutable temporal lineage and changing
currentness. Casework owns only the read projection.

## Open corpus and identity law

The corpus is a tiny one-module Verilog design plus deterministic fake-open
tool, PDK, repository, and input-artifact manifests. No proprietary program,
PDK, scheduler, license server, or worker is needed. Content-family identities
are domain-separated digests of the exact checked-in bytes. Scheduler job,
worker, license entitlement, and stage occurrence identities use their explicit
registry/occurrence domains. Hostnames, IP addresses, DNS names, display names,
paths, and process exits are not identities.

The exact environment retains design, scheduler job, worker, toolchain, PDK,
license entitlement, repository revision, input artifact set, output artifact
set, and stage occurrence identities separately.

## Independent qualification cases

The closed corpus contains 13 signed acquisitions:

1. nominal;
2. exit-zero with missing output;
3. output digest mismatch;
4. wrong repository revision;
5. wrong toolchain;
6. wrong PDK;
7. license no-response;
8. worker loss;
9. scheduler contradiction A;
10. scheduler contradiction B;
11. stale artifact;
12. delayed duplicate delivery;
13. agent-authored contradictory testimony.

Exit code and output presence are independent NQ claims. The missing-output case
is not smoothed into a result. License no-response produces no payload testimony
and becomes cannot-testify plus Nightshift acquisition-failure currentness.
Scheduler and agent contradictions retain both inputs without source-class
precedence. Staleness derives from the exact acquisition time and profile-owned
maximum age, not filesystem time or free text.

The delayed-duplicate fixture traverses the exact DISTANT sender spool and
receiver inbox. A fresh delivery attempt returns the retained first-custody
receipt; the attempt journal preserves the retry occurrence without minting a
new observation.

## Casework

Thirteen fixed condition directories retain exact Monitor, NQ, lineage, profile,
and evaluation bytes. Casework recomputes the EPOCH derivation before projecting
them. The operational API keeps GET/HEAD read semantics, fixed raw routes, and
405 for writes. The UI exposes exact supported claims, cannot-testify,
contradictions, all time axes, trigger, next lawful action, and raw artifacts.

No process exit, delivery receipt, NQ claim, Nightshift disposition, or Casework
view is authorization or an aggregate ECAD/job/operational result.
