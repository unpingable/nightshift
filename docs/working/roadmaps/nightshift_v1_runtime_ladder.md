# Nightshift v1 runtime ladder — non-actuating reconciliation loop

> **Status:** working draft. Not ratified architecture.
> **Filed:** 2026-05-27
> **Framing:** *Build the runtime surfaces that let the pre-positioned doctrine gates fire honestly; do not build the gates as features.*

## What this is

A build ladder for getting Night Shift to a **deployed, non-actuating, reconciling daemon** running against real NQ ops data, governed by Governor on the action-authorized path, with operator disposition durable across runs.

The artifact is a sequence of slices, each with concrete acceptance shape and an honest read on which slices are already substantially shipped vs. which are the real next work.

## What this is NOT

- Not a roadmap for doctrine gates. The pre-positioned gates in [`../decisions/pre-positioned-doctrine-gates.md`](../decisions/pre-positioned-doctrine-gates.md) are recognition records, not features to build. Some slices below get NS closer to where those gates can fire honestly; that is a *consequence* of the slice, not the motivation.
- Not authorization to build actuation, the consolidation interrupt, Wicket integration, or generic cross-kernel machinery (see Non-goals).
- Not a substitute for the gap docs in `working/gaps/`. The ladder names which gaps are load-bearing for which slice; the gap docs hold the design.
- Not committed scope past Slice 3. Slices 4–5 are scoped but not authorized; ratification waits on Slice 3 contact with real ops data.

## The build target

> NQ evidence snapshot → NS posture/reconciliation → proposal packet → **operator disposition / ack** → **watch / defer / hold lifecycle** → receipt / log replay.

The MVP-exit anchor is named in `docs/README.md`: `nightshift watchbill run wal-bloat-review` against real NQ ops data, governed by Governor for the action-authorized path. The ladder finishes when that target is repeatedly survivable as a *scheduled, operator-acked* loop — not just an interactive run.

---

## Current implemented surface inventory

Substantially complete. The runtime is real; what remains is the lifecycle around it.

### Pipeline (shipped)

- **Capture / reconcile split** — `watchbill capture` + `watchbill reconcile <run_id>` deferred path; `watchbill run` convenience over the pair. See `GAP-deferred-run-split.md` (shipped).
- **Liveness gate** — `--nq-liveness` consumes NQ's `liveness export`; Stale/Skewed halts with a Stale-shape packet and revalidate-only `ProposedAction` (`LIVENESS_CONSUMER V1`, FEATURE-HISTORY).
- **Imported basis freshness** — `FreshnessBasis` enum + `assess_freshness`; producer-extraction time refuses to launder as observation time (`SLICE_B V1`).
- **Silence-aware posture** — `PostureClass::{IncidentShape, SilenceShape, Unknown}`; surface-only, does not change reconciliation (`SLICE_C_1 V1`).
- **Three-axis split** — truth (NQ-owned) / notification posture (NS) / ack obligation (NS); `Stale → advise(revalidate-only)`, `Invalidated → emits packet` (`SLICE_5_CONTRACT V1`).
- **Horizon CLI + pipe-through** — paired `--horizon-policy` + `--governor-socket`; Governor receipt IDs reach `packet.receipt_references.governor_receipts` and the `RunHorizonOutcome` ledger event (`HORIZON_CLI_PIPE_THROUGH V1`).
- **`--no-governor` ceiling-lowering** — promotion ceiling drops to advise when Governor is absent; mutation, publication, paging, and staged actions disabled.

### Persistence (shipped)

- **SQLite store** with run lifecycle (open → finalized), run-ledger event stream, atomic transitions per CLAUDE.md invariant 13.
- **Query surfaces** — `runs list` (filter by agenda / finding / held-only / limit) and `runs show <run_id>` (metadata + ceiling + hold reason + event timeline).
- **Inspection surfaces** — `nq peek` (translation-only NQ findings) and `liveness peek` (side-by-side NQ-fresh vs. NS-verdict, the clock-skew wrinkle).

### Packet (shipped, mostly)

- `FindingSummary` with origin/silence envelopes, posture class
- `Diagnosis` with regime, evidence, confidence, alternatives considered
- `ProposedAction` (advise-only for v1) with reversibility, blast radius, requested authority
- `AuthorityResult` with governor_present, verdict, receipts
- `AttentionState` enum already named (six values, sibling to GAP-attention-state's operator-axis)
- `OperationalUrgency` enum already named
- `receipt_references` with `governor_receipts` populated by horizon path

### Tests

~216 tests; load-bearing sentinels include `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization` (Slice B's "ok_to_proceed is not authorization summary"), the silence-aware boolean-laundering refusal trio (Slice C.1), and the live-daemon trip-wire (`governor_rpc_live.rs`).

---

## Missing runtime surfaces

What stands between the current state and a deployed loop. Listed by the slice that closes each.

| Surface | Why missing matters | Closed by |
|---|---|---|
| Scheduled invocation | NS runs on-demand; can't "wake up and look around" | Slice 1 (deployment) |
| Operator disposition commands | `runs show` is read-only; no way to ack / defer / silence / handoff against a finding | Slice 3 |
| Attention TTL + re-surface | Acks have no half-life; lapsed silences don't bring the finding back | Slice 3 |
| Closure-candidate predicate | Packet says `ProposedAction`; there is no explicit "eligible for closure" check distinguishing proxy-quiet from substrate-recovered | Slice 4 |
| Proxy-shock recognition | Sudden regime-change classification cannot be distinguished from target-witness event | Slice 5 |
| Real-deployment NQ fixture | Tests use fixture NQ + manifest; the wal-bloat-review pilot path is not exercised against live data on a schedule | Slice 1 (deployment) |

---

## The slices

### Slice 1 — Scheduled, idempotent run loop *(shipped 2026-05-27)*

**Goal.** A boringly runnable NS that wakes up on a schedule, consumes one real NQ snapshot, emits a deterministic packet, writes ledger events, and repeats safely.

**Status read.** Shipped. See [FEATURE-HISTORY § SLICE_1_SCHEDULED_LOOP V1](../decisions/FEATURE-HISTORY.md#slice_1_scheduled_loop-v1). `--trigger scheduled` activates daemon-side idempotency (skip on same NQ generation); `deploy/systemd/` provides a hardened service + timer + EnvironmentFile template. The wal-bloat-review pilot is now timer-deployable.

**Acceptance shape.**

1. One deployable invocation unit (systemd timer is the v1 default; cron acceptable; the unit lives in the repo or in a deployment-fixture path).
2. Consumes one real NQ snapshot (via `--nq-db` against the Linode NQ host listed in memory `reference_vm_access`, or via fixture for unit acceptance).
3. Emits a deterministic reconciliation packet (YAML, today's shape).
4. Writes the receipt / log artifact (run ledger entries; horizon-receipt forwarding if `--horizon-policy` + `--governor-socket` set).
5. Repeat-invocation discipline: re-running within the same NQ generation against the same `finding_key` either (a) finds the existing run and reports it, or (b) opens a new run with an explicit reason — never silently double-counts.

**Existing GAPs.** `GAP-deferred-run-split.md` (shipped), `GAP-storage.md` (SQLite v1 / Postgres v2), `GAP-mcp-authority.md` (transport ≠ authority — relevant if scheduling crosses host boundaries).

**Non-goals at this slice.** No actuation. No Continuity preflight beyond `--continuity-configured` flag. No new packet fields. No agenda-level reconciliation across findings (per memory `feedback_agenda_reconciler_trap` — do not file/build without a failing case).

**Gates closer to firing after this slice.** None directly — this slice is plumbing.

---

### Slice 2 — Packet next-check surface *(shipped 2026-05-27)*

**Goal.** The packet exposes, for each finding it touched: evidence state, posture, recommended disposition, upstream refusal / stale / invalidated basis, operator-facing reason, and the next-check / watch-until target if applicable.

**Status read.** Shipped. See [FEATURE-HISTORY § SLICE_2_OPERATOR_VISIBILITY V1](../decisions/FEATURE-HISTORY.md#slice_2_operator_visibility-v1). `runs show` now renders the attention block (state, posture class, proposed action, next check, watch basis, ack expires, follow-up, governor receipts), each field appearing only when populated. `hold_gate()` distinguishes `liveness` from `preflight`; the previously-mislabeled liveness-failed-as-`ok` defect is fixed.

**Acceptance shape.**

1. Packet renders `next_check_at` directly when one applies (horizon defer; revalidate-only proposal; attention `WatchUntil`).
2. The operator-facing reason on a held / deferred run names *which* gate held it (liveness / freshness / preflight / horizon) without requiring the operator to read the ledger.
3. `runs show <run_id>` formats these in a way an operator can scan in two seconds.
4. No regression in the boolean-laundering refusal trio (`silence_present ≠ incident_absent`, `acked_silence ≠ acked_incident`, `no_new_evidence ≠ resolved`) or in the `ok_to_proceed is not authorization` sentinel.

**Existing GAPs.** `GAP-silence-aware-posture.md` (shipped, surface-only), `GAP-imported-basis-freshness.md` (shipped), Slice 5 contract in `GAP-nq-nightshift-contract.md`.

**Non-goals at this slice.** No new `AttentionState` semantics; no operator commands yet — this slice is read-side only. Do not extend the six-value re-ack disposition enum (frozen per `GAP-reack-doctrine.md`); class-aware extension is explicitly deferred per `class-aware-disposition-design-space.md`.

**Gates closer to firing after this slice.** None directly; surfaces the data Slice 3 will act on.

---

### Slice 3 — Operator disposition lifecycle *(shipped 2026-05-27, incremental: ack + silence)*

**Goal.** Night Shift becomes a lifecycle tool, not a clever report generator. The operator can act on a packet, and the action durably affects the next scheduled run.

**Status read.** Shipped incremental scope. See [FEATURE-HISTORY § SLICE_3_ATTENTION_LIFECYCLE V1](../decisions/FEATURE-HISTORY.md#slice_3_attention_lifecycle-v1-ack--silence). `nightshift attention ack/silence` persists into a new SQLite table keyed on `(agenda, finding)`; the reconciler applies a read-time projection so the next scheduled run sees the operator's intent. Attention never raises authority; expiry bumps urgency one step; silence requires both `--until` and `--reason`; re-ack on prior attention requires `--disposition` per the frozen six-value enum.

**Deferred to follow-ons** with explicit unlock criteria (see FEATURE-HISTORY): `investigate`, `handoff`, `request-revalidation`. Each is a smaller add (different `AttentionState` variant; no new TTL mechanism) and lands when a forcing case shows up.

**Acceptance shape.**

1. Operator commands against a stable attention key (NQ `finding_key`):
   - `nightshift attention ack <finding_key> --ttl <duration> [--note ...]`
   - `nightshift attention investigate <finding_key> [--note ...]`
   - `nightshift attention silence <finding_key> --until <timestamp|condition> --reason <text>`
   - `nightshift attention handoff <finding_key> --to <party> --note <text>`
   - `nightshift attention request-revalidation <finding_key>` (mark "operator wants fresh evidence next cycle")
2. Attention state is keyed on the **stable finding identity** per `GAP-attention-state.md`, not on the run or the packet. Re-emission of the same finding in a later NQ generation preserves attention.
3. **TTL machinery.** Acks have an expiry derived from agenda policy (`re_alert_after`) or from operator-supplied `--ttl`. On the next scheduled run after expiry, the finding re-surfaces with `attention=unowned` and operational urgency raised by one step (or as policy declares).
4. **Silence requires reason or expiry.** No open-ended silence; the reconciler refuses a silence command lacking both `--until` and `--reason` (the latter only acceptable when also accompanied by a `handoff_note`-class reason).
5. **Six invariants from `GAP-attention-state.md` enforced as tests.** In particular:
   - `silence is not handling` — silenced findings still surface in `runs list --silenced`, not hidden.
   - `ack is not closure` — `acknowledged` attention never elides the finding from urgency calculation; only changes how it is rendered.
   - `attention state never raises authority` — `investigating` does not bump promotion ceiling.
   - `recovered ≠ closed` — evidence `recovered` while attention `investigating` preserves the scar.
6. **Re-ack discipline.** When a finding re-surfaces because its ack expired, the next operator interaction is treated as a re-ack per `architecture/GAP-reack-doctrine.md` (mini re-triage with typed disposition from the frozen six-value enum).
7. **Ledger events for all attention transitions.** Operator intent is audit material.

**Existing GAPs.** `GAP-attention-state.md` (model already specified in detail — this slice is implementation), `architecture/GAP-reack-doctrine.md` (re-ack disposition enum, frozen).

**Open questions surfacing at implementation.**

- Per-run vs. per-finding projection — `GAP-attention-state.md` says probably a projection over the event log with a materialized view for read paths. Decide at implementation time.
- Whether `nightshift attention list` is needed in v1, or whether `runs list` annotation suffices.

**Non-goals at this slice.** No closure semantics — operator can ack, silence, handoff, but cannot *close* a finding. (Slice 4 territory.) No multi-operator coordination — single-operator semantics only. No web UI; CLI plus packet YAML is the surface. No Continuity wiring for cross-operator attention state — that is a `GAP-parallel-ops.md` concern, not a Slice 3 concern.

**Gates closer to firing after this slice.**

- *Closer to Gate 1 (closure predicate).* Gate 1 requires distinguishing proxy-quiet from substrate-recovered for closure authorization. Slice 3 puts the operator-axis machinery in place (attention vs. evidence as two distinct axes). Gate 1 then has a place to *refuse closure*.
- *Reinforces the boolean-laundering refusal trio.* Slice C.1 named the trio; Slice 3 makes it tested in lifecycle, not just at packet-emission time. An operator silencing a finding cannot accidentally launder it as recovered.

---

### Slice 4 — Closure-candidate predicate *(approaches Gate 1)*

**Goal.** A predicate that can say "this finding is eligible for closure," "not eligible: proxy-only evidence," "not eligible: missing consequence-channel witness," "not eligible: stale/invalidated basis." The predicate is enforceable, not advisory. It does **not** close findings yet.

The point of this slice is to make NS encounter the doctrine honestly: "looks quiet" is not the same as "actually recovered." The predicate's first job is to refuse premature closure.

**Acceptance shape.**

1. `closure_candidate(finding, attention, evidence)` → one of:
   - `eligible-for-review` — predicate would not refuse closure; operator still required.
   - `not-eligible: proxy-only-evidence` — silence-shape posture or proxy-channel-only normalization.
   - `not-eligible: missing-consequence-channel` — no consequence-witness has been required and observed.
   - `not-eligible: stale-basis` — imported basis freshness or liveness gate failure.
   - `not-eligible: invalidated-basis` — Slice 5 contract Invalidated path.
   - `not-eligible: attention-active` — operator state is `investigating` or `acknowledged` with valid TTL (no closure during active attention).
2. The predicate is surfaced in the packet and in `runs show <run_id>`. Operators see the verdict and the reason.
3. **The predicate refuses to close. It does not authorize closure.** Per the framing in `pre-positioned-doctrine-gates.md` Gate 1: closure-authorization is enforceable, not advisory. Slice 4 is the enforcement of refusal; the *positive* closure path is post-MVP.
4. Sibling-invariant tests with Slice C.1's refusal trio — closure on `SilenceShape` posture without consequence-channel witness must fail.

**Existing GAPs / doctrine references.**

- `pre-positioned-doctrine-gates.md` Gate 1 — provenance: `~/git/papers/working/tooltheory/dashboard-quiet-is-not-recovery.md`, Lean candidate `proxy_quiet_does_not_authorize_target_closure`.
- `GAP-silence-aware-posture.md` — Slice C.1's evidence-shape side of the same refusal family.
- `architecture/GAP-reack-doctrine.md` — `ack ≠ resolution` keeper.

**Open questions for the implementation moment.**

- What concrete "consequence-channel witness" looks like in NS's wire shape. NQ surfaces findings of a given detector class; the predicate needs a way to ask "is this finding the *consequence* class or the *proxy* class." Likely a finding-shape question, not an NS-side classifier. **Do not invent a new NQ axis without grepping NQ first.**
- Whether the predicate runs in the reconciler or as a separate phase. Probably in `reconcile_phase`; surfacing as a packet field; not blocking the run.

**Non-goals at this slice.** No actual closure logic. No incident-state semantics. No mode-aware closure (cross-mode in `GAP-incident-modes.md`). No actuation surface.

**Gates closer to firing.**

- *Gate 1 fires honestly here.* The doctrine ("proxy-quiet does not authorize closure") is now enforced by code, not just by gap-doc.

---

### Slice 5 — Proxy-shock recognition *(approaches Gate 2)*

**Goal.** When NS or NQ surfaces a sudden regime-change classification ("we saw a shock"), route it to the existing Stale-shape revalidate-only path; preserve uncertainty; refuse posture advancement; refuse closure / action.

Per `pre-positioned-doctrine-gates.md` Gate 2, this is **recognition, not new state.** A regime-change witness firing is a *cause* for entering `Stale → advise(revalidate-only)`; the routing already exists.

**Acceptance shape.**

1. NS recognizes a proxy-shock classification from NQ's wire shape (the exact shape TBD — coordinate with NQ at implementation time; do not invent an NS-side classifier).
2. On recognition: route to `EvidenceState::Stale` (or the equivalent Slice 5-contract path), set `ProposedAction` to revalidate-only, refuse posture advancement, do not raise authority.
3. The packet names the regime-change cause in the operator-facing reason. The operator sees "regime-change witness; revalidate before action," not a generic Stale message.
4. Closure-candidate predicate (Slice 4) refuses closure under proxy-shock.

**Existing GAPs / doctrine references.**

- `pre-positioned-doctrine-gates.md` Gate 2 — provenance: `~/git/papers/working/tooltheory/proxy-shock-mismatch.md`, Lean candidate `proxy_shock_does_not_authorize_target_closure`.
- `GAP-nq-nightshift-contract.md` — Slice 5 contract that owns the `Stale → advise(revalidate-only)` plumbing.

**Non-goals at this slice.** No new `EvidenceState` value. No proxy-shock detection in NS (that is NQ's job; NS recognizes the wire shape NQ surfaces). No mode-specific proxy-shock semantics (incident vs. remediation is `GAP-incident-modes.md` work, not this slice).

**Gates closer to firing.**

- *Gate 2 fires honestly here.* The regime-change witness now has operational routing; "we saw a shock" no longer launders as "we know what it means."

---

## First deployable slice

**Slice 1 closeout** — i.e., wiring the existing single-invocation pipeline into a scheduled, idempotent unit running against the real Linode NQ database (per memory `reference_vm_access`), with horizon and Governor wired through (Tier-2 path, both flags), and emitting packets the operator can inspect via `runs list` / `runs show`.

That is what "deployed non-actuating reconciliation loop" means in the first instance: NS wakes up, looks around, writes down what it thinks, and goes back to sleep — without authority to actuate, but with receipts the operator can review.

The wal-bloat-review pilot named in `docs/README.md` is the concrete shape. Slice 1 puts it on a timer; Slices 2–3 make it tolerable as an operator surface; Slices 4–5 make it honest under proxy-quiet and proxy-shock conditions.

---

## Explicit non-goals

These are not "later." These are not on this ladder.

- **No actuation.** NS proposes; Governor authorizes; NS does not actuate. The consolidation interrupt (Gate 3) is *recognition* now and *to-be-built only after NS gains actuation* — which is not v1. Do not build Schmitt-trigger controllers, four-stock dynamics, settlement-debt machinery, or mode-specific actuation safety bounds yet. Cosplay with equations is still cosplay.
- **No Wicket integration.** Gate 4 is recognition-shaped for the existing NQ / Governor edges (Lean now formally proves what NS already implements) and to-be-built *only when Wicket joins NS's dependency graph*. Current NS has no Wicket dependency. No Wicket adapter until there is an actual Wicket receipt to consume.
- **No agenda-level cross-finding reconciliation.** Per memory `feedback_agenda_reconciler_trap` — over-productizing the coping strategy is the named failure mode. Do not file or build agenda-level reconciliation without a failing case.
- **No multi-operator / cross-operator attention coordination.** Single-operator v1; cross-operator is `GAP-parallel-ops.md` and Continuity-mediated, not a runtime-ladder slice.
- **No web UI.** CLI + packet YAML is the operator surface for v1. Operator-facing docs land under `operator/` when MVP exit reaches the wal-bloat-review pilot.
- **No incident-mode transitions.** `GAP-incident-modes.md` (incident / remediation / architecture modes) lives as a gap doc. Slices 1–5 do not implement mode-declaration or cross-mode authority. Stabilized ≠ remediated; deployed ≠ verified; do not collapse them as a side-effect of building a faster reconciler.
- **No backup-orchestration features.** `GAP-backup-restore.md` is a separate concern, not on this ladder.
- **No `requires_recheck` flag per input.** Recheck is the gate, not metadata (CLAUDE.md invariant 3).

---

## Doctrine-gate distance after each slice

Per `pre-positioned-doctrine-gates.md`, the gates fire in the order NS hits them in real work, not in any doc's order. Where this ladder lands relative to each:

| Gate | Today | After Slice 1 | After Slice 3 | After Slice 4 | After Slice 5 |
|---|---|---|---|---|---|
| 1. Closure predicate | not-yet-triggered | same | trigger surface exists | **firing — predicate refuses proxy-quiet closure** | refines under proxy-shock |
| 2. Proxy-shock workflow | not-yet-triggered | same | same | same | **firing — routes to Stale/revalidate** |
| 3. Consolidation interrupt | not-yet-triggered (no actuation) | same | same | same | same — gate is post-actuation |
| 4. Cross-kernel refusal propagation (Wicket) | recognition-shaped (NQ/Governor edges); not-yet-triggered (no Wicket) | same | same | same | same — gate is Wicket-dependent |

The ladder gets two of the four gates to "firing honestly" and leaves the two that *should not* fire yet untriggered. That is the correct shape.

---

## Slice cycle pointers

This ladder is one expression of the slice-cycle pattern named in `working/decisions/GAP-slice-cycle.md` (candidate doctrine, not ratified). The reconciliation-vs-lifecycle ladder question is still open there. This roadmap does not resolve it; if Slice 3 implementation surfaces a forcing case, file against the slice-cycle gap doc.

## Revision discipline

Working-directory doc. Revise as slices land or scope shifts. When a slice closes:

1. Write the FEATURE-HISTORY entry first (commit hashes, evidence pointers, what unblocks).
2. Update this ladder's *status read* line for that slice ("largely shipped" → "shipped").
3. Promote nothing into `architecture/` by duplication. If the slice produces architecturally-load-bearing doctrine, move it; do not clone.

If a slice does *not* land — because the forcing case never materializes, or because the design refused to settle — retire the slice section with a one-line note on why. Memory hygiene per the global `Memory hygiene` rule: stale roadmap entries vote.
