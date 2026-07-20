# Audit: Night Shift ↔ Admissibility Calculus runtime conformance

**Filed:** 2026-07-20
**Auditors:** three parallel Claude agents (refusal-spine, stored-decision,
authority/break-glass), consolidated by session lead; operator-directed.
**Audited against:** `~/git/lean` v14 `Admissibility.Calculus` stable root
(Core / Spine / Crossing / Comparison + BreakGlass terminal instance,
released 2026-07-18) plus the five formerly-annex modules now
PUBLIC-SHIPPED/PUBLIC-EVIDENCE.
**Frame:** runtime conformance is the goal (operator, 2026-07-20). The lean
repo's "no runtime conformance claim" is a fence for third-party consumers,
not a ceiling on this program. Findings below are conformance debt, not
resonance notes.

Fix-ladder status is tracked at the bottom of this file; the shipped-state
ledger remains `FEATURE-HISTORY.md`.

## Blockers (guarantee-typed seams)

One uncovered seam falsifies the guarantee; these are conjunctive.

1. **`RefusalKind::BasisInvalidated` is dead code.** Found independently by
   two auditors. Both live basis-invalidation refusal paths emit
   `refusal: None`: reconciler `InputStatus::Invalidated` →
   `build_summary` pushes a bare input id into `blocked[]`
   (`reconciler.rs:458`) and `build_success_packet` hardcodes
   `refusal: None` (`pipeline.rs:861`); `HorizonAction::EscalateBasisInvalidated`
   carries full hash data into the ledger but no `refusal_for`-style
   mapping exists for horizon actions (`pipeline.rs:1071` covers liveness
   only). Law: Spine `funnel_never_opaque` — a refusing claim never
   funnels to an empty/opaque result. The Slice-5
   "`Invalidated → emits packet`" packet is the opaque one.

2. **Horizon `Defer` decided without consulting reconciliation
   admissibility.** `process_horizon` iterates `phase.results` with no
   status filter; `action_for` never sees `result.status`
   (`reconcile_horizon.rs:113-144`, `horizon.rs:249-315`). A finding just
   ruled `Stale`/`Invalidated` can yield `Defer`, persisting a
   `ToleranceRecord` and minting an `action.authorized` Governor receipt
   whose `evidence_hash` falls back to the captured (stale) hash.
   Mitigations (typed `Freshness` non-discharge claim; WLP3 warranty
   refusal) act downstream; the grant itself is standing-unchecked at
   mint. Law: Core `witness_requires_standing`; NS's own "don't propose
   execution on stale evidence."

3. **Tolerance custody chain broken at two points.**
   `save_tolerance` runs before `record_receipt`
   (`reconcile_horizon.rs:240-300`): a failed RPC leaves a live,
   un-archived grant the next run consumes as `PriorTolerance`. And
   `ToleranceRecord` has no `receipt_id`, so a consuming run cannot
   verify the grant was archived (custody survives only indirectly via
   `granted_in_run_id`). Law: Core `witness_preserves_custody`.

4. **`governor_binding` validation is an empty `if`.**
   `agenda.rs:284-288` — the declared gate ("Governor required above
   level X") rejects nothing and appears nowhere else in enforcement.
   Masked today by hardcoded `requested: Advise`. Declared-but-unenforced
   guarantee-typed seam.

5. **Preflight clearance minted from an unwitnessed boolean.**
   `--continuity-configured` is a bare CLI flag; `preflight()` clears
   risky/protected-class runs on it with no Continuity query
   (`coordination.rs:104-115`), then mints `RunPreflightCleared`
   (`pipeline.rs:248-252`) — a clearance receipt attesting a check that
   never ran. Documented v1 placeholder; under invariant 18 this seam is
   a guardrail. Availability-asserted ≠ use-witnessed.

## Lossy encodings and basis drift

6. **`PreflightHeld` is a unit variant** collapsing three coordination
   outcomes (`HoldForContext` / `Coordinate` / `BlockForResolution`)
   that the ledger distinguishes (`pipeline.rs:882-891, 1046`). Law:
   Spine `distinct_refusals_encode_distinct` (injectivity). The liveness
   variants are exactly lossless; this one is a convenient summary.

7. **`InvalidAgenda(String)` is a cross-strata catch-all**: NQ export
   schema and contract-version refusals arrive typed as agenda errors
   (`nq.rs:215-220`; also `pipeline.rs:650` bundle-missing). Law: Spine
   no-vocabulary-laundering — a family's refusals arrive in their own
   domain stratum.

8. **Posture-class basis divergence.** With the finding absent at
   reconcile, the stored reconciliation says `Unknown`
   (`reconciler.rs:408`) but the packet re-derives posture from the
   *captured* snapshot (`pipeline.rs:700-705`) and
   `silence_shape_rewrite` (`pipeline.rs:1331`) branches on it. Two
   derivations of one judgment from different bases, unmarked, driving
   behavior. Law: Crossing derive-from-stored-decision.

9. **No reconcile-time liveness gate.** Liveness runs only in
   `capture_phase`; the reconcile-phase fresh NQ acquisition
   (`pipeline.rs:396`) is trusted on the capture-time verdict. Not
   re-deciding — a missing conjunct on the new claim the reconcile
   acquisition constitutes. `GAP-deferred-run-split.md` freezes
   capture-time liveness but names no reconcile-time obligation.

10. Smaller: WLP3 warranty gate keys on `packet.unsettled`, self-documented
    "display surface ONLY" (`mvp_a.rs:844-871`, `packet.rs:292-298`);
    `governor_present: !opts.no_governor` testifies presence from
    flag-absence (`pipeline.rs:753`); `ZERO_EVIDENCE_HASH` fills a typed
    absence with shaped data (`reconcile_horizon.rs:188-193`) — the move
    the NS-1 skew amendment refused; `AuthorityCeilingExceeded` demotes
    typed levels to `String` (`errors.rs:14-18`); ledger `RunReconciled`
    payload is a bool + bare id list (`pipeline.rs:489-490`);
    `FreshnessReceipt` wire projection is open-string typed
    (`freshness.rs:113-127`, tracked incompleteness by its own comment).

11. **Break-glass advertised, unimplemented.** The preflight-hold packet
    advises "invoke operator_override with a named reason"
    (`pipeline.rs:958`); no such verb, type, or ledger event exists.
    Fail-closed today, so nothing launders. When built, the rung-7
    BreakGlass instance is the spec: origin-bound (who), history-bound
    (ledger entry settlement never cleans), refused as ordinary authority.

12. **`Packet.refusal` is single-slot** (`Option`, `packet.rs:325`).
    Currently unreachable loss: gates short-circuit, so a second
    simultaneous refusal is never computed (unlike a lossy encoding).
    Owed: a doc line on the field stating this.

## Conforming surfaces (verified, protect these)

- Decide-once core: `acquire_current` single boundary, `adjudicate` pure
  with clock frozen at `acquired_at`, purity/determinism test-locked.
- One-shot reconcile (`RunAlreadyCompleted`) — the contraction shape.
- Horizon: `action_for` called once; `HorizonReceipt.unsettled` retained
  explicitly to avoid re-derivation (stored-decision rule stated in code).
- Tier-2 flag pairing hard-bails fail-closed (`main.rs:783-797`);
  Tier-1 `effective_ceiling` caps at Advise.
- Ledger write-only in decision paths (`list_events`: zero production
  callers); scheduled idempotency reads lower activity, never raise
  authority.
- WLP3/A.5 refusal branches retain the successful segment's witnesses
  (Crossing `rightRefused` shape); receipt ids reach
  `packet.receipt_references.governor_receipts` and `RunHorizonOutcome`.
- NS-1 `RefusalKind` registry: closed, no catch-all, each variant carries
  exactly the evidence that participated in its verdict (`LivenessSkewed`
  omits the threshold that took no part). The liveness path end-to-end is
  the model the other gates are held to.
- Witness-position passthrough is render-only: absent from
  `semantic_diff`, `posture_class`, `closure`. Nothing counts
  witnesses/acks toward authority (`authority_has_no_multiplicity`).

## Correspondence map (toward a conformance claim)

> **Promoted.** The correspondence map below has been lifted into a
> first-class candidate manifest, `CALCULUS-CONFORMANCE.md`, which also
> names the constitutional transition (formal-reference → runtime
> governing spec) whose absence was the root-cause loophole. This
> section is retained as the audit-time snapshot; the manifest is the
> living surface.

Per lean-side discipline, a runtime-conformance claim needs an explicit
correspondence map plus evidence/refinement. Current fit:

- **`decide`** — mostly exists: `verdict_for`, `action_for`, `adjudicate`
  are total and evidence-returning.
- **`Refusal`** — per-family (`RefusalKind` / `NonDischargeKind` /
  `NightShiftError`), which the Calculus supports; blockers are findings
  1, 6, 7.
- **`Obligation`** — nearly free: unsettled claims / `NonDischargeKind`
  already form an obligation book.
- **`Witness`** — weak: clean paths carry evidence diffusely (ledger +
  receipts), not as a typed replayable artifact; `exclusive` and the
  witness-side laws have nothing to attach to yet.
- **`Standing`** — findings 2, 4, 5 are exactly the seams where the map
  is unwritable today.
- Every `refusal: None` on a refused claim is a point where `decide`'s
  totality fails to inhabit.

## Fix ladder

- [x] 1. Wire `BasisInvalidated` at both sites; horizon gets a distinct
      `ToleranceBasisInvalidated` carrying its basis hashes. *(commit
      a9eb5a8; deviation noted — split rather than one variant.)*
- [x] 2a. `receipt_id` on `ToleranceRecord`; reorder save-after-receipt.
      *(commit cde4a64.)*
- [ ] 2b. Status-filter `process_horizon` / pass `result.status` into
      the horizon decision. **Design-scoped** (F2: missing conjunct vs
      documented freeze) — owed a decision record.
- [x] 3a. Payload `PreflightHeld` (outcome + reasons). *(commit
      42d1e00.)*
- [ ] 3b. Split NQ contract refusals out of `InvalidAgenda`.
      *(Mechanical; not yet applied.)*
- [ ] 4. Resolve posture-class basis divergence (use stored decision or
      mark basis on packet).
- [ ] 5. Enforce `governor_binding`; decide reconcile-time liveness
      (missing conjunct vs documented freeze). **Design-scoped.**
- [x] 6a. Flip `AUDIT-BACKLOG` lean entry to conformance-target register
      (promotion condition 3 fired). *(commit a99ff91.)*
- [ ] 6b. `Packet.refusal` single-slot doc line.

Items 2b, 5 (preflight witness + `governor_binding`), the reconcile-time
liveness question, and break-glass implementation (F6/§3.4 of the
manifest) are design-scoped, not mechanical; each gets its own decision
record before code. They are now tracked as **BLOCKER** rows in
`CALCULUS-CONFORMANCE.md §3`, not footnotes.
