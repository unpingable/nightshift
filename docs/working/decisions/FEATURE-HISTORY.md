# Feature History

The shipped-state ledger for Night Shift. Per-feature entries record what landed, when, with explicit evidence pointers (commits, paths, evidence summary, what's unblocked).

This file exists because gap docs are *design records*, not shipped-state ledgers. See [`../../README.md`](../../README.md) §"Shipped state vs. design records" for the doctrine; the cross-project pattern (agent-governor → NQ → Night Shift, third instance) was the trigger.

## Conventions

Each entry is one section, named for the gap or feature it closes (e.g. `## SLICE_C_1 V1`). Sections carry:

- **Status** — one of `shipped` / `partial` / `superseded`. `partial` lists what landed and what's outstanding.
- **Shipped commits** — the commits that delivered the work. Hashes plus a one-line description.
- **Evidence** — concrete pointers a future reader can spot-check: production paths, test names, schema migrations, acceptance criteria covered. Not prose claims; specific artifacts.
- **Unblocks** — gap docs whose `Blocks:` field is now lifted by this entry, if any.
- **Field notes** — *optional*. Discoveries during shipping that future-you would want to know but that don't belong in the gap doc's design record. Keep brief; if it grows large, the fact probably belongs in a memory tripwire or back in the gap doc.

Entries are written *after* shipping, not as plans. The gap doc is where plans live; this file is where they get cashed out.

The chronological order below is newest-first.

---

## SLICE_2_OPERATOR_VISIBILITY V1

**Status:** `shipped` 2026-05-27. Slice 2 close-out per [`working/roadmaps/nightshift_v1_runtime_ladder.md`](../roadmaps/nightshift_v1_runtime_ladder.md). The packet already carried the load (Slice C.1 posture, Slice B freshness, horizon WatchUntil tolerance_basis_id + re_alert_after, governor_receipts pipe-through). This slice surfaces those fields in the operator-facing `runs show` and corrects the held-status defect for liveness-gate failures.

**Shipped artifacts:**
- `crates/nightshiftd/src/posture.rs`:
  - `RunPosture::is_held` extended to include `RunLivenessGateFailed` — a liveness-gate failure that completed via `liveness_gate_failed` no longer mislabels as `ok`.
  - New `RunPosture::hold_gate() -> Option<&'static str>` returns `"liveness"` or `"preflight"`. Naming the gate is the operator's first answer to "why did the daemon stop." Horizon escalation is deliberately not a `hold_gate` — it produces a reconciled packet, surfaced via the attention block.
  - `hold_reason` rewritten to use the gate label as a prefix (`"liveness: <verdict>"` / `"preflight <outcome>: <reasons>"`) so the gate name is visible whether the operator reads `runs show` or just `runs list`.
  - `render_show` extended with an attention block: `attention`, `posture`, `proposed` always render; `owner`, `next check` (re_alert_after), `watch basis` (tolerance_basis_id), `ack expires`, `follow up`, `gov receipts` render only when populated.

**Evidence:**
- New unit tests in `crates/nightshiftd/src/posture.rs`:
  - `liveness_failed_run_is_held_with_liveness_gate` — event-only fallback path produces `Some("liveness")` gate and `liveness:` reason prefix.
  - `preflight_hold_gate_is_named_in_render` — `render_show` emits `hold gate:  preflight`.
  - `render_show_surfaces_next_check_and_watch_basis` — synthetic WatchUntil packet renders `next check`, `watch basis`, `attention:  WatchUntil`, and `gov receipts` lines.
- New integration tests in `crates/nightshiftd/tests/posture_surface.rs`:
  - `protected_class_hold_names_preflight_gate` — pipeline-driven; existing held run now reports `hold_gate() == Some("preflight")`.
  - `liveness_failed_run_is_held_with_liveness_gate` — pipeline-driven with stale liveness DTO; `status_label() == "HELD"`, gate is `"liveness"`, render output names the gate.
  - `render_show_surfaces_attention_block_for_reconciled_run` — happy-path packet shows `attention:` / `posture:` / `proposed:` and *omits* `watch basis:` / `next check:` (proves the optional-field discipline).
- All existing posture tests still green (the `"protected-class service in scope"` assertion is preserved — the rename only changes the prefix).
- 225/225 tests green (222 prior + 3 new unit + 3 new integration), 1 ignored. Clippy clean under `--all-targets -- -D warnings`.

**Field notes:**
- Hold-cause taxonomy is *gate-named*, not event-named. Operators ask "did liveness fail or did preflight refuse to coordinate" — they should not have to read `RunLedgerEventKind` variants to find out. `hold_gate()` is the answer; `hold_reason()` carries the detail under the gate prefix.
- The horizon case is deliberately not a `hold_gate`. A `Defer` (WatchUntil) or `EscalateExpired` packet reconciles successfully and surfaces via the attention block's `next check` / `watch basis` / `gov receipts` lines — different operator question.
- The synthetic WatchUntil packet in the unit test is the only place a `Packet` is constructed by hand in the test suite. It guards against rendering regressions without coupling to the horizon path's specific construction sequence.

**Unblocks:**
- Slice 3 (operator disposition lifecycle) — operators now have a stable surface to inspect what NS thinks before invoking attention commands against it.

---

## SLICE_1_SCHEDULED_LOOP V1

**Status:** `shipped` 2026-05-27. Slice 1 close-out per [`working/roadmaps/nightshift_v1_runtime_ladder.md`](../roadmaps/nightshift_v1_runtime_ladder.md). The single-invocation pipeline already shipped; this slice adds (a) the `--trigger scheduled` flag, (b) idempotency that skips re-reconciliation when NQ has not advanced its `snapshot_generation`, and (c) a systemd deployment surface.

**Shipped artifacts:**
- `crates/nightshiftd/src/scheduled.rs` — `check_scheduled_idempotency` + `ScheduledOutcome::{Skipped(SkipReport), Proceed}`. Skip applies only when the most recent *completed* run for `(agenda_id, finding_key)` captured the same NQ generation as the current snapshot. Open runs do not block fresh invocations (intentional fail-open; reasoning in module docs).
- `crates/nightshiftd/src/main.rs` — `--trigger {manual|scheduled|event}` global CLI flag (default `manual`); `TriggerArg` clap-side enum mapped to `RunTrigger`; `run_watchbill_cmd` gates on `check_scheduled_idempotency` only when `--trigger scheduled`.
- `deploy/systemd/` — non-templated service + timer + `EnvironmentFile` example + install README. Hardened sandbox (`ProtectSystem=strict`, `MemoryDenyWriteExecute=true`, `SystemCallFilter=@system-service`, `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`). One timer = one `(agenda, finding)`; multi-finding orchestration is out of scope.

**Evidence:**
- Six acceptance tests in `crates/nightshiftd/tests/scheduled_idempotency.rs`:
  - `scheduled_skip_when_same_generation_already_reconciled` — green-path skip; SkipReport contains prior run_id + generation + completed_at; message starts with `scheduled-skip:` and includes the prior run_id.
  - `scheduled_runs_when_generation_advances` — new generation opens a new run row; second `list_runs` returns two distinct run_ids.
  - `idempotency_with_no_prior_runs_proceeds` — fresh store → Proceed.
  - `idempotency_proceeds_when_finding_absent_from_nq` — `AbsentNqSource` returns None → Proceed (pipeline will emit its canonical absent-target error).
  - `trigger_kind_is_persisted_on_run_row` — `RunTrigger::Scheduled` survives the persist round-trip via `list_runs`.
  - `idempotency_scopes_to_agenda_not_just_finding` — a different `agenda_id` targeting the same finding-key Proceeds (one agenda's run cannot silence another's).
- 222/222 tests green (216 prior + 6 new), 1 ignored (`governor_rpc_live`).
- Clippy clean under `--all-targets -- -D warnings`.

**Field notes:**
- Idempotency is the daemon's responsibility, not the timer's. The roadmap acceptance criterion was explicit on this: "Re-running within the same NQ generation against the same `finding_key` either (a) finds the existing run and reports it, or (b) opens a new run with an explicit reason — never silently double-counts." Per-(agenda, finding) keying handles the "different agenda, same finding" case correctly without conflating workflow contexts.
- Open-prior-run case (a capture is open but reconcile pending) is *not* a skip — the new invocation proceeds. Rationale: a stuck deferred-split run should not freeze the timer; the operator sees both runs in `runs list` with status `running` on the stuck one. If this turns out to be a real surface, a third outcome (`OpenPriorRun(...)`) is the natural extension; the type is already shaped for it.
- Idempotency applies only to `watchbill run`, not to `watchbill capture` or `reconcile` individually. The deferred-split commands are deliberate operator workflows; idempotency-skip in them would surprise.

**Unblocks:**
- Slice 2 (operator visibility / next-check rendering).
- Tier-2 wal-bloat-review pilot deployment against the Linode NQ host (per memory `reference_vm_access`).

---

## HORIZON_CLI_PIPE_THROUGH V1

**Status:** `shipped` 2026-05-21. Tier-2 horizon path made CLI-reachable and verdict-observable. Closes the "Pipe-through debt" section in `GAP-governor-contract.md`. `check_policy` / `authorize_transition` (the remaining two `nightshift.*` Governor methods) are NOT in this slice — they remain open in the gap doc.

**Shipped commits:**
- `9ba17fe` — wire horizon CLI flags (`--horizon-policy` + `--governor-socket`, paired or both error), AGENTS/README/3 GAP-banner doc-truth refresh
- `61a5789` — make horizon Governor receipts observable: `HorizonReceipt` struct captures `RecordReceiptResponse`; `packet.receipt_references.governor_receipts` populated; new `RunLedgerEventKind::RunHorizonOutcome` ledger event
- `0bc570e` — CLAUDE.md repo-status drift refresh
- `cdf4007` — closeout audit: flip live-daemon trip-wire assertion (`governor_rpc_live.rs` now asserts receipt_id non-empty + matches packet); update GAP-governor-contract pipe-through section to "landed"

**Evidence:**
- CLI flags: `crates/nightshiftd/src/main.rs::build_horizon_deps` — paired-flag validation; both required, either alone bails.
- Pipe-through: `crates/nightshiftd/src/reconcile_horizon.rs::HorizonReceipt` + `apply_horizon_outcomes` returning `Vec<HorizonReceipt>`; `crates/nightshiftd/src/pipeline.rs::build_success_packet` populates `packet.receipt_references.governor_receipts` from receipts; `RunHorizonOutcome` ledger event emitted per outcome with `action / finding_key / basis_id / basis_hash / expires_at` plus `receipt_id` + `receipt_hash` when a Governor receipt was emitted.
- Acceptance: `crates/nightshiftd/tests/horizon_packet_state.rs::defer_makes_governor_receipt_observable_in_packet_and_ledger` + `escalate_expired_emits_event_without_receipt_fields` + `horizon_disabled_preserves_pre_horizon_behavior` (extended assertions).
- Live-daemon trip-wire: `crates/nightshiftd/tests/governor_rpc_live.rs::live_horizon_defer_pipeline_round_trips` — asserts receipt_id non-empty AND ledger receipt_id matches packet receipt_id.
- All 216 tests green, 1 ignored, clippy clean.

**Verdict-observable taxonomy (worth keeping):** The audit produced a structural refinement of the verdict-observable shape, distinguishing two orthogonal disciplines:
- **Observable, not constructible** — wire-format minting discipline ("this tool must not be able to mint authoritative shapes without earned provenance"). Audit-owed for NS as a downstream-mint check; tracked in `working/decisions/AUDIT-BACKLOG.md`.
- **Observable AT (pipe-through addressability)** — once an upstream authority agrees, its identity must reach this tool's output. NS instance: receipt_id from Governor reaching `packet.receipt_references.governor_receipts` and the run ledger. This slice closes the NS instance.

**Trip-wire doctrine paid off:** `GAP-governor-contract.md` named the trip-wire condition mechanically when the work was deferred. The live-daemon test asserted `governor_receipts.is_empty()` *as documentation of the unfixed state*. When the seam closed, the closeout audit found the assertion and flipped it — exemplar of trip-wire-as-self-falsifying-probe.

**Unblocks:**
- Tier-2 deployment against real Governor daemon (post-MVP work).
- Future operator-facing horizon visibility (`nightshift runs show` already renders the new event via existing event-row formatter).

---

## SLICE_C_1 V1 (silence-aware posture, surface-only)

**Status:** `shipped` 2026-05-20. Surface-only as designed; **no disposition-enum extension** in v1. Slice C.2 (class-aware disposition extension on the frozen six-value enum) remains deferred under three named unlock triggers; design-space exploration filed in [`class-aware-disposition-design-space.md`](class-aware-disposition-design-space.md) 2026-05-26.

**Shipped commits:**
- `ca181a1` — Slice C ratification + Open Q1 hybrid-path resolution (re-ack doctrine filed as prerequisite)
- `b15fe54` — scaffolding + red tests
- `ccd21d5` — implementation

**Evidence:**
- Enum + derivation: `crates/nightshiftd/src/posture_class.rs` — `PostureClass::{IncidentShape, SilenceShape, Unknown}` with envelope-presence derivation rule (absence of `silence` envelope → `Unknown`, not `IncidentShape`).
- Surface plumbing: `posture_class` field on `ReconciliationResult` (`bundle.rs`), `FindingSummary` / `Attention` (`packet.rs`).
- Regime prefix + notification language: `silence_shape_rewrite` in `pipeline.rs` actively drops laundering phrases (`"resolved"`, `"recovered"`, `"safe to ignore"`, etc.) and prepends silence-shape posture line.
- Acceptance: 25 tests in `crates/nightshiftd/tests/silence_aware_posture.rs`, all green.
- Boolean-laundering refusal trio pinned: `silence_present ≠ incident_absent`, `acked_silence ≠ acked_incident`, `no_new_evidence ≠ resolved`.

**Unblocks:** Nothing actively. Slice C.2 remains deferred. Three unlock triggers (any one suffices): operator review surfaces a confused-disposition incident / NQ migrates ≥ 1 legacy silence detector to the unified envelope / a second posture class lands.

**Field notes:** Open Q1 (re-ack doctrine prerequisite) resolved via hybrid path: file re-ack doctrine first as `GAP-reack-doctrine.md` (now `architecture/GAP-reack-doctrine.md`), then run Slice C.1 surface-only under that doctrine. Q2 ("`AttentionState::Silenced` naming collision") resolved with option (a) — accept the collision, document in both struct docs; revisit if operator review surfaces confusion.

---

## REACK_DOCTRINE PROMOTION

**Status:** `shipped` 2026-05-20. Doctrine promoted from memory leaf to repo text as a prerequisite for Slice C.1's surface-only implementation.

**Shipped commits:**
- `419ed39` — `GAP-reack-doctrine.md` filed (now at `architecture/GAP-reack-doctrine.md`)

**Evidence:**
- Six-value disposition enum (frozen): `advanced` / `unchanged-waiting-on-X` / `blocked-on-Y` / `handed-off` / `escalated` / `resolved/no-longer-valid`.
- Nine invariants pinning ack-lineage scope, re-ack-on-context-change, silence-ack ≠ active-ack, stale-ack ≠ invalidated-ack, ack ≠ resolution / safety / freshness / truth.
- Six-axis table of what ack must not collapse.
- Memory pointer `project_reack_doctrine.md` retained as pre-promotion provenance only.

**Unblocks:** Slice C.1 (no-disposition-enum-extension contract); future Slice C.2 explorations (see `class-aware-disposition-design-space.md`).

---

## SLICE_B V1 (imported basis freshness)

**Status:** `shipped` 2026-05-18 → 2026-05-20. Closeout audit pinned "ok_to_proceed is NOT an authorization summary" across struct doc, SCHEMA-bundle, and gap doc. Slice B.1 observe-only receipt + Slice B.2 stale-imported-basis binding to Slice 5 advise(revalidate-only).

**Shipped commits:**
- `dc91058` — spec
- `82f2807` — red tests
- `7b920cf` — `FreshnessBasis` primitive + `assess_freshness`
- `8821efd` — B.1 observe-only receipt
- `f510a44` — B.2 binding to Slice 5 advise(revalidate-only)
- `821933d` — closeout sentinel

**Evidence:**
- Primitive: `crates/nightshiftd/src/freshness.rs` — `FreshnessBasis::{NativeLifecycle, ProducerExtraction, MissingProducerExtraction, IncoherentProducerExtraction}` + `assess_freshness` returning verdict + reason.
- Five reconciliation cases pinned in spec; eight acceptance tests in `crates/nightshiftd/tests/imported_basis_freshness.rs`.
- Closeout sentinel: `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization` in `nq_integration.rs` — guards that `ok_to_proceed = true` is preserved on `Stale + Historical + Stale + revalidate-only` while no input was invalidated. Updating this test deliberately is the signal that v1 Slice 5 doctrine moved.
- Doctrinal pin: `imported_producer_basis_stale` is the laundering-killshot reason on Stale; Slice B's `EvidenceState::Stale` ≠ Slice C's `SilenceShape` posture (two separate axes, must not merge).

**Unblocks:** Slice C (silence-aware posture) — separated cleanly from Slice B clock-staleness. The design-record narrative in `GAP-imported-basis-freshness.md` §"Closeout" stays in the gap doc.

---

## SLICE_A V1 (V1 substrate visibility)

**Status:** `shipped` 2026-05-12. NS consumes NQ's `DURABLE_ARTIFACT_SUBSTRATE` V1 `origin` / `silence` envelopes for operator visibility (peek + packet). Surface-only; does not change reconciliation. Sibling slice to NQ's V1 ship same day.

**Shipped commits:**
- `250fc5d` (plus earlier scaffolding per the older `project_slice_a_v1_substrate.md` memory).

**Evidence:**
- Wire shape: `NqExportDto` adds `origin` + `silence` as `#[serde(default)]` optional envelopes; forward-compat against NQ legacy detectors that haven't migrated yet.
- Surfaces: `nightshift liveness peek` shows envelopes side-by-side with wrinkle data; review packet carries them in finding context.
- Invariant test: `origin_and_silence_are_visible_but_do_not_change_reconciliation` — pins that envelope presence does not alter `EvidenceState` / `RelianceClass` / `ProposedAction`.
- 12/12 tests green at the dry-run point.

**Field notes:** Build-gate language doctrine retuned during Slice A: keeper "Exploration does not require prior evidence. Ratification does." The three-gate distinction (implementation permission / ratification of operational claims / doctrine-GAP promotion) was filed in the test module doc, not promoted to CLAUDE.md. See memory `project_slice_a_v1_substrate.md` for provenance.

**Unblocks:** Slice B (clock freshness) and Slice C (silence-aware posture) — both consume the same envelopes Slice A made visible.

---

## LIVENESS_CONSUMER V1

**Status:** `shipped` ~2026-05-06. Peek surface landed same week. Gate fires on Stale/Skewed liveness; wrinkle contract binding (do not trust upstream `fresh`).

**Shipped commits:**
- `474a1b1` — liveness consumer + gate
- (peek work, 2026-05-06) — `nightshift liveness peek` operator surface

**Evidence:**
- Gate behavior: liveness fail → run held as `Stale` packet, revalidate-only `ProposedAction`, no remediation. Per Slice-5 contract (`docs/working/gaps/GAP-nq-nightshift-contract.md`).
- Operator surface: `crates/nightshiftd/src/liveness_peek.rs` — side-by-side wrinkle visibility.
- Wrinkle invariant: NS does not trust NQ's upstream `fresh` field as authoritative; binds its own freshness via the liveness snapshot consumer.

---

## SLICE_5_CONTRACT V1 (three-axis split)

**Status:** `shipped` ~2026-04 (commit `e9f9a27`). Pre-dates this ledger; entry recorded retrospectively as the foundation for Slice A/B/C.

**Shipped commit:**
- `e9f9a27` — three-axis split: truth (NQ-owned) / notification posture (NS) / ack obligation (NS); `Stale → advise(revalidate-only)`; `Invalidated → emits packet`.

**Evidence:**
- Load-bearing test names pinned in `project_slice5_constraints.md` memory.
- `EvidenceState`, `RelianceClass`, `InputStatus`, `ProposedAction` semantics — all flowing from this split.

**Unblocks:** Every subsequent slice (A/B/C of substrate consumption, liveness consumer, etc.). The three-axis split is the load-bearing primitive the consumption chain composes onto.

---

## Pre-ledger work

Earlier work (capture/reconcile/packet primitives, NQ snapshot DTO, agenda YAML parser, store layer, etc.) is not enumerated here. Reconstruct from git history if needed:

```bash
git log --oneline --reverse | head -50
```

Future entries cash out specific gaps in `working/gaps/` against this ledger; the bare-foundations work pre-dates the gap-doc discipline.
