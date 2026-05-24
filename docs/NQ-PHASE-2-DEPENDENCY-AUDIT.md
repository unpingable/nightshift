# Nightshift ↔ NQ Phase 2: dependency-shape audit

A narrow audit, written against the proposed NQ `docs/architecture/PATH_TO_1_0.md`. Not a Night Shift v1 plan. Not a request to start building. Not a redesign.

The single question:

> Given NQ's proposed path to 1.0, what Night Shift work is genuinely blocked on NQ Phase 2 (receipt discipline / Slice 1), what can proceed independently, what should wait until after NQ 1.0, and what should be explicitly rejected or deferred?

## Frame

NQ's PATH_TO_1_0 calls Phase 2 ("Receipt discipline") the only *invention* phase on the 1.0 mainline, and names Phase 3 ("Nightshift consumption") as gated on it. The NQ memo's prose implies a tight coupling: finish Phase 2 → unblock Night Shift. The actual coupling is looser, and the looseness is load-bearing for time allocation. This document names where the coupling is real and where the NQ memo overstated it.

The memo's other job: name the consumption-semantics traps that *would* tighten the coupling if Night Shift drifts into them, so they can be refused on contact.

## NQ Phase 2 in one paragraph

Phase 2 / Slice 1 invents receipt durability for `nq.receipt.v1`: witness ref content hashes (1a), receipt canonicalization + hash + evaluator-version stamping (1b), an explicit `freshness_horizon` field (1c), `nq receipt check` for structural verification (1d), and `nq receipt replay` for semantic re-evaluation (1e). The wire this lands on is the **CI receipt wire** — the output of `nq verify` consumed by GitHub Actions and `nq receipt check/replay` consumers.

## What Night Shift actually consumes from NQ today

Night Shift does **not** consume `nq.receipt.v1`. Night Shift consumes two NQ wire surfaces, both at the ops-evidence altitude:

| NQ wire | What it carries | Night Shift's relationship |
|---|---|---|
| `nq.witness.v1` | Witness packets (raw observations) | Not consumed |
| `nq.receipt.v1` | CI receipts (preflight verdicts) | Not consumed |
| `nq.finding_snapshot.v1` | Operational findings (warning-state rows) | **Consumed** via `CliNqSource` (`crates/nightshiftd/src/nq.rs`) |
| `nq.liveness_snapshot.v1` | Witness liveness DTO ("is the witness still witnessing?") | **Consumed** via `liveness.rs` as a hard run-gate before findings are read |

Night Shift does not consume `nq.receipt.v1` today, and no committed v1 work requires it. Phase 2's invention is on the receipt wire. Night Shift's two seams are the finding-snapshot wire and the liveness wire. The receipt wire and the finding/liveness wires are non-identical, and conflating them is the first trap.

This does not mean Night Shift is unaffected by Phase 2 — but it shifts the dependency from "blocked on" to "may opportunistically consume new fields, after they exist." That difference governs the rest of this memo.

### Wire stability is a separate dependency from Phase 2

Night Shift's two NQ seams *do* depend on NQ's wire stability — `schema = "nq.finding_snapshot.v1"`, `contract_version = 1`, the `finding_key` canonical encoding (`nq.rs::nq_canonical_key`), the `admissibility.state` taxonomy. These are already shipped and stable. Phase 2 adds fields; it does not break them. A breaking NQ wire change (a hypothetical `v2`) would be a separate event — independent of Phase 2 — and would require an NS-side migration regardless of when it happens.

Important: a wire-stability dependency is not a Phase-2 dependency. The temptation to merge "we depend on NQ wire being stable" with "we depend on NQ Slice 1 shipping" inflates the coupling. Keep them separate.

## What Night Shift has already done that mirrors Phase 2 discipline

Three pieces of Phase-2-shaped discipline already landed on the NS side, applied to findings instead of receipts:

1. **Schema + contract_version gating at parse** (`crates/nightshiftd/src/nq.rs:204-215`). NS hard-refuses any export that isn't `schema = "nq.finding_snapshot.v1"`, `contract_version = 1`. This is the equivalent of `nq receipt check`'s shape-verification, applied to the wire NS actually reads.
2. **Admissibility gating at parse** (`nq.rs:216-222`). NS refuses any finding with `admissibility.state != "observable"`. NQ's `FINDING_EXPORT_V1` ships the `admissibility` block as a contract claim; NS treats non-observable findings as inadmissible at the wire boundary, not as something to be later filtered.
3. **Producer-clock-aware freshness assessment** (`GAP-imported-basis-freshness.md`, status: **landed**, Slice B.1 commit `8821efd`, Slice B.2 commit `f510a44`, closeout `821933d`, all between 2026-05-18 and 2026-05-20). NS distinguishes producer basis time from custody/lifecycle time and refuses to launder one into the other. Eight acceptance tests pin the contract; the "laundering killshot" (recent custody, stale producer extraction) yields `EvidenceState::Stale` with reason `imported_producer_basis_stale`. This is the same discipline NQ's `freshness_horizon` field (1c) would land at the receipt altitude.

The pattern: NS has independently arrived at canonicalization, hash-binding, freshness-vs-custody, and admissibility-as-wire-claim — applied to the finding surface. Phase 2 would land the same discipline at the receipt surface. The disciplines don't need to be the same code path; they need to be the same posture.

## Four-bucket classification

### 1. Currently blocked on NQ Slice 1

None of Night Shift's committed v1/MVP work is blocked on NQ Slice 1. Night Shift's MVP exit (`nightshift watchbill run wal-bloat-review`, described in DESIGN.md §"MVP: Ops observe/advise only") does not require any field that Phase 2 invents. The MVP consumes `nq.finding_snapshot.v1` + `nq.liveness_snapshot.v1`, which both ship today at contract_version 1 with the fields NS needs (`admissibility`, `origin.producer_extraction_time`, `lifecycle.*`, the liveness witness identity / age fields).

The honest answer to "what's blocked on Slice 1" is: nothing NS has committed to building. The looser version, "what would be sharper if Slice 1 existed," appears in bucket 3 below. Wire-stability concerns (finding_key encoding, schema versioning, admissibility taxonomy) are blocked on NQ keeping its wire stable, not on Phase 2 — see the wire-stability note above.

### 2. Can proceed before NQ Slice 1

Most of Night Shift's named work falls here.

- **Slice C — silence-aware posture** (`GAP-silence-aware-posture.md`, `GAP-reack-doctrine.md` ratified 2026-05-20, C.1 implementation landed). Operates entirely on NS-side `PostureClass` derivation from `nq.finding_snapshot.v1.silence` + `lifecycle.condition_state`. No Phase 2 dependency. The `silence` envelope is `#[serde(default)]` in `nq.rs:NqExportDto` — forward-compat, absence-tolerant.
- **Governor adapter `check_policy` / `authorize_transition` methods** (`GAP-governor-contract.md`). `record_receipt` shipped 2026-05-21 (`61a5789`); the remaining two methods don't need anything from NQ. Their firing sites depend on NS gaining a real action-propose surface, which is internal sequencing.
- **The other named GAPs that don't touch NQ at all**: `GAP-attention-state`, `GAP-escalation`, `GAP-rumination-boundary`, `GAP-incident-modes`, `GAP-parallel-ops`, `GAP-narrowing-posture-transition`, `GAP-architectural-promotion-boundary`, `GAP-autonomous-execution-boundary`, `GAP-lesson-distillation-boundary`, `GAP-mcp-authority`, `GAP-workflow-routing-boundary`, `GAP-backup-restore`, `GAP-storage`, `GAP-slice-cycle`, `GAP-solution-family-exhaustion`, `GAP-deferred-run-split`, `GAP-nightshift-coordination-mode`, `GAP-governor-contract` (remaining methods).
- **The single AUDIT-BACKLOG entry** ("verdict observable, not constructible," 2026-05-07). This audit-owed item is *structurally* tied to NQ's `TESTIMONY_OBSERVABLE_NOT_CONSTRUCTIBLE_GAP`, but NQ explicitly punted the consumer-side question to Night Shift. NQ Slice 1 does not resolve it. Night Shift owes itself this audit independent of NQ Phase 2.

### 3. Should wait until after NQ Slice 1 (opportunistic consumption)

Four Phase-2-adjacent wire fields *could* arrive on `nq.finding_snapshot.v1` alongside `nq.receipt.v1`, and Night Shift would benefit from optionally consuming them. None are required for NS v1; all are forward-compat enrichments.

- **Witness ref digest on findings.** If `nq.finding_snapshot.v1` gains a `witness_refs[].digest` field (the same digest Phase 2 lands on receipts), NS gains a way to confirm "the finding I'm seeing is bound to a specific witness packet." Today NS sees `admissibility.state` but not witness provenance. The right consumption posture: parse with `#[serde(default)]`, surface in the reconciliation receipt as an audit field, **do not** branch reconciler verdicts on it without a separate Slice spec.
- **Evaluator-version stamping on findings.** Same shape. Today NS enforces `contract_version = 1` (wire shape) but has no visibility into which evaluator version produced the finding. Phase 2 lands evaluator-version binding on receipts; if the same stamp arrives on findings, NS can carry it for audit. Same posture: read, surface, don't branch reconciler verdicts.
- **NQ-side `freshness_horizon` field.** If NQ ships an explicit horizon field on findings, NS's `imported_basis_freshness_window_seconds` becomes a *fallback* rather than the only horizon source. Important: this does **not** mean NS adopts NQ's horizon as authoritative. Per `GAP-imported-basis-freshness.md` §"Freshness window," NQ's detector-policy horizon and NS's reconciliation-policy horizon are separate operator-policy questions that must not be deduplicated. The right posture: read NQ's horizon as an *input signal* to NS policy, not as a replacement for it.
- **`source_db_hash` (snapshot-integrity field).** Already on NS's wishlist in `GAP-nq-nightshift-contract.md` §"Open questions" ("Does NQ expose `source_db_hash` today, or does it need to? Needs to, for reconciliation integrity."). Phase-2 canonicalization work may provide a nearby design precedent, but `source_db_hash` is a finding/liveness snapshot integrity question, not automatically part of the receipt-durability slice. NS should not assume the field arrives with Phase 2, and NQ should not adopt it just because NS named it here. Same forward-compat posture if it does land: read, surface, do not branch verdicts on it without a follow-on slice.

The unifying rule for this bucket: **Phase 2 produces optional admissibility-input enrichments for Night Shift, never authorization inputs.** A field arriving on the wire from NQ does not move NS's promotion ceiling or change reconciler verdicts without a separate NS slice that ratifies the consumption.

### 4. Should be explicitly rejected or deferred

Six tempting paths to refuse on contact, listed because they would surface naturally as the constellation work proceeds:

- **Treating `nq receipt check` / `nq receipt replay` as a Night Shift consumption surface.** NS does not consume receipts. If a future slice proposes "NS verifies NQ receipts before reconciliation," the answer is no — NS reconciles against the *current finding snapshot*, not against an old verdict. Replay is for CI receipt consumers, not for ops findings.
- **Collapsing NQ findings and NQ receipts into a unified NS "evidence" category.** They're at different altitudes; the receipt vocabulary distinction in DESIGN.md §"Receipts: distinguish the kinds" already separates run-ledger / authority / evidence receipts. Adding "NQ receipts" as an authority-adjacent input would re-violate that separation.
- **Promoting NQ findings to authority via shape-conformance.** This is the AUDIT-BACKLOG verdict-boundary item. The discipline (audit shape-construction surfaces before treating NQ-shaped JSON as authoritative) is named and unaudited. The temptation: skip the audit because "NQ findings are evidence anyway." That argument loses force at v2+ when shared storage makes wire-format tampering structurally easier.
- **Using NQ receipt status as a decision input for `defer/revalidate/accounting` actions.** The careful framing: NS may treat *durable NQ receipts* as admissibility inputs for the revalidate-only path that already exists for Slice B Stale findings — but only after Phase 2 ships, and only as a *third* admissibility class alongside the existing native and imported paths, not as a replacement for them. Until then, NS integration around NQ receipts stays advisory / planning-only.
- **Letting Phase 2 wire changes drive NS Slice ordering.** Phase 2's CI-receipt focus does not force any NS slice to wait. If NQ ships Phase 2 first (the NQ memo's recommendation), NS gains opportunistic enrichment points; if NQ defers Phase 2, NS proceeds independently on its own Slice C / Governor adapter / attention-state work.
- **Building "NS-as-receipt-emitter" pretending it's symmetric to NQ-receipts.** Night Shift already emits run-ledger events; Governor emits authority receipts. The temptation to add a "Night Shift receipt" as a third kind, hash-equivalent to NQ's, is real *because* the receipt vocabulary is converging across the constellation. The DESIGN.md separation is the keeper: NS records lifecycle facts, Governor records authority decisions, NQ records evidence. Symmetry of pattern is not symmetry of role.

## What Night Shift gates on NQ (the inverse direction)

Light. Slice C.1 (silence-aware posture) and Slice B (imported basis freshness) both consume wire fields that already shipped in `nq.finding_snapshot.v1` contract_version 1. The remaining NS work surfaces (Governor adapter completion, attention-state, escalation, MCP authority, etc.) are intra-NS or intra-Governor concerns and do not touch NQ at all.

**Night Shift does not gate NQ 1.0.** This was already stated in the NQ memo; this audit confirms it from the NS side. The cross-project ordering — NQ Phase 2 first, then optional NS consumption enrichments — survives.

### Follow-up: narrow the NQ memo's claim

After this audit lands, the NQ-side `docs/architecture/PATH_TO_1_0.md` should be revised separately to narrow its claim about Phase 2's effect on Night Shift. Today the NQ memo reads as though "Phase 2 unblocks Nightshift" full stop. The accurate narrowing — derivable from this audit — is:

> Phase 2 unblocks future receipt-discipline consumption and optional Nightshift enrichments. Current Nightshift MVP work consumes the finding and liveness wires and is not blocked on NQ receipt discipline.

That edit is owed on the NQ repo, not here. It is named in this memo only so the two docs don't disagree after this one lands and so a future agent picking up either side sees the same coupling shape.

## Consumption-semantics traps to watch (the user's caveat made explicit)

The four progressive framings, ordered worst → best:

1. **Worst**: *"Night Shift should use NQ receipt status to decide action X."* This makes NQ receipts an authorization input. Refuse on contact.
2. **Bad**: *"Night Shift's bundle should treat NQ receipts as authoritative evidence."* The receipt vocabulary distinction breaks; NQ becomes authority. Refuse.
3. **Better**: *"Night Shift may treat durable NQ receipts as admissibility inputs for defer/revalidate/accounting paths, subject to freshness and scope."* Acceptable post-Phase-2, after a dedicated NS slice ratifies it. Not before.
4. **Best**: *"Until NQ Phase 2 lands, Night Shift integration around NQ receipts remains advisory/planning-only."* This is the current state. Preserve.

The slope toward (1) is gentle. Each step rationalizes the next. The DESIGN.md keeper ("Night Shift records run events; Governor emits authority receipts. A run may contain many receipts, but Night Shift does not manufacture authority by logging itself") is the canonical refusal.

## Does Night Shift need its own calibration doc alongside NQ's PATH_TO_1_0.md?

Recommendation: **yes, but smaller than NQ's, and as a deployment-maturity calibration, not a path-to-1.0.**

The reasoning:

- NQ wrote `PATH_TO_1_0.md` because its `SPINE_AND_ROADMAP.md` defined phases but not a 1.0 cut-line, and "what's left for 1.0 vs. what's post-1.0" needed pulling out of prose.
- Night Shift's analog is *already distributed* across `DESIGN.md` (v1 MVP field budget, §"v1 MVP field budget" explicit), `DEPLOYMENT-MATURITY.md` (v1 local / v2 shared / v3 service ladder), `AUDIT-BACKLOG.md`, and the 25 GAP-* docs (each a named shortfall). NS has more articulated state than NQ did pre-memo.
- What NS lacks: a single doc that says "v1 ships when X, Y, Z are true" — equivalent to NQ's "Minimum 1.0 cut" section. The DESIGN.md MVP exit is close but informal.

The smaller doc NS needs would be a `PATH_TO_V1.md` (note: v1, not 1.0 — NS's maturity vocabulary in DEPLOYMENT-MATURITY.md is v1/v2/v3, not 0.x/1.0). It would:

- Pin the v1 exit condition (MVP working end-to-end against real NQ ops data, governed by Governor for the action-authorized path that already ships, with the audit-backlog verdict-boundary item resolved).
- Inventory which of the 25 GAPs are v1-required vs. v2+ deferred. (Probably ~5 v1-required, ~20 v2+ or "ratified doctrine, no implementation needed yet.")
- Make explicit that NS v1 ships **independent of NQ Phase 2**, and that post-v1 work on opportunistic Phase 2 consumption (bucket 3 above) is a separate cycle.

This is a smaller and sharper job than NQ's memo — maybe 6-10KB vs. NQ's 21KB. It is not required to ratify *this* memo; this memo names the dependency shape, and the v1 calibration is a separate next step.

## Scope of this memo

This is a strategy / calibration record. Approving it does not authorize:

- Starting any NS slice.
- Implementing any cross-project integration.
- Writing the NS PATH_TO_V1.md (that's a separate decision).
- Changing NQ's PATH_TO_1_0.md.

Approving it ratifies the four-bucket classification, names the consumption-semantics traps, and confirms the NS-doesn't-gate-NQ direction. The only action that follows from approval is committing this memo into the repo so it sits alongside DEPLOYMENT-MATURITY.md and the GAP-* series as the cross-project calibration anchor.

## Caveat on the audit itself

The four-bucket decomposition is satisfying enough that approving it creates implicit forward momentum toward acting on bucket 3 ("opportunistic consumption") before Phase 2 has actually landed in NQ. Don't. The point of bucket 3 is that the work is *conditional* on Phase 2 shipping in NQ, and that even after it ships, each consumption point is a separate NS slice with its own design surface. The audit identifies opportunity, not commitment.

The second satisfying thing: the consumption-semantics-traps list reads as a complete enumeration of failure modes. It is not — it is the enumeration *visible from the current state*. New traps will surface as the constellation moves. The list is a starting set, not a closed taxonomy.
