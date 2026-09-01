# VIOLET-SLUICE — ABSD/Docket auth-exercise trusted-dispatch conformance V1

> **Track:** `governed-effect-conformance`
> **Codename:** `VIOLET-SLUICE`
> **Canonical slug:** `absd-docket-auth-exercise-trusted-dispatch-conformance-v1`
> **Status:** **PLANNED / NOT STARTED**
> **Result classification:** none
> **Filed:** 2026-08-31
> **Codename collision search:** clear in Nightshift content and reachable local branch names at filing time.
> **Authority:** documentation only; no Docket, ABSD, AG, civild, service, route, provider, or target mutation.

## Purpose and bounded claim

VIOLET-SLUICE is a future conformance campaign for the exact transition from a Docket-custodied authorization exercise into one trusted ABSD dispatch attempt. Its contribution to the strongest current end-to-end property is **one-spend / one-attempt bounded effect custody**. It preserves separately authority issuance/consumption, Docket standing/attempt/dispatch admission, ABSD same-attempt journal, target-effect observation, and outcome-unknown reconciliation. Absence of duplicates is not proof that a physical effect happened exactly once.

## Exact identities, ownership, and source map

Authoritative identity archaeology: `/data/git/docket/constellation-canonicalization/stage0-repository-identities.md` and `stage3-wire-contract-catalog.md`. `/data/git/docket/runtime` is the canonical Docket product; `/data/git/docket` is campaign/evidence only. `/data/git/absd` owns trusted dispatch admission, same-attempt journal/effect custody, and reconciliation. `/data/git/ag_ng` owns authorization issuance/consumption. `/data/git/civild` owns historical native evidence and is excluded from mutation absent a new authorized seam.

Exact heads: civild subject `630c5c3e4ad71db96e35fcb08800614825b425a9`, result `7d5b01b779a62b19b53d0d3ea94a8d4c405230f6`; AG native `c612fadf8e86304dd54115ce0be54737a428c315`; Docket native `7aee690d724ff3b91c03d2b55efa266bcaad0e1d`, C1 `191f51a83ce4f1708cb443ba2185d846d87fe2f1`, C2 `c49ad8d0f26fb2a13b9dbafdde84d7abfe1f867b`; ABSD native `9b219246733e53dd52a02b9b7cc449436b9be24c`, C1 `cf57f1f62a6103c71ea1d925073eac35a90d73b6`.

Docket native/C2 diverge at `c9f4a8a328434700b30757a7d668013d0ac63de0` (28 native-only, five C2-only). ABSD native/C1 diverge at `0fa64a9ca20f81ef3d80637cf65bc14f65f936f6` (53 native-only, one C1-only). No exact current owner head inherits both relevant lines. VIOLET remains not started until each owner has a non-rewriting successor base preserving exact relevant heads, evidence, independently qualified combined semantics, exact result head, and successor-base policy. Content equivalence cannot silently establish inherited qualification; material semantic divergence stops entry.

Exact source surfaces: Docket `crates/gwr-local/src/{authz_intake.rs,governed_loop.rs}` and migrations `0003`/`0006`; ABSD `src/bin/civil-mf-executor.rs`, `src/dispatch_custody.rs`, `src/writer.rs`, `qualification/DGE002-PROTOCOL.md`, `qualification/dge/receipt.toml`; transport `docs/governed-runtime/{executor-transport-v1.md,executor-transport-c1-qualification.md,governed-loop-c2-layering.md}` and `conformance/executor-transport-v1/`; civild `research/native-governed-mechanism-closure/{summary.md,milestone-index.toml,receipt.toml}` and `research/coupled-freebsd-convergence/{u002/summary.md,handoffs/A-010.md}`. No `Copper Semaphore` record was found; `Copper Lantern` is distinct.

## Future qualification

Prove one exact AG issuance/consumption reference becomes one Docket standing and canonical attempt; Docket admits at most one exact dispatch; dispatch bytes/audience/plan/ABSD endpoint/attempt bind exactly; ABSD admits only that attempt; duplicate delivery converges; terminal replay performs no mechanics; ambiguous loss becomes outcome unknown with same-attempt reconciliation only; refusal before attempt, refusal after custody, and outcome unknown remain distinct; owner classifications remain independent.

Casework/Phosphor may render exact raw provenance, identities, times, contradictions, missing evidence, and outcome unknown. No approve, answer, dispatch, retry, execute, reconcile, merge, promote, remediation, aggregate authority/health/success, or exactly-once indicator. Activation requires fresh authority, exact terminal convergence results, isolated worktrees, closed non-production fixtures, and independent review. No default merge, production/service/route activation, credential access, history rewrite, or consumed replay. Status remains **PLANNED / NOT STARTED**, classification **none**; QUALIFIED or NOT-QUALIFIED terminal results must be preserved exactly.
