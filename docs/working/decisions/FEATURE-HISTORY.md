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

## SLICE_4_CLOSURE_CANDIDATE V1 (partial Gate 1)

**Status:** `shipped` 2026-05-27. Slice 4 close-out per [`working/roadmaps/nightshift_v1_runtime_ladder.md`](../roadmaps/nightshift_v1_runtime_ladder.md). **Refusal-only — no closure authority.** Implements the conservative form of Gate 1 from [`pre-positioned-doctrine-gates.md`](pre-positioned-doctrine-gates.md): the predicate refuses closure under known blocker conditions and explicitly marks the missing-channel-classification case as `Unassessable`, *not* eligible-by-default.

**Shipped artifacts:**
- `crates/nightshiftd/src/closure.rs` — `ClosureCandidate::{NotEligible{reason}, UnassessableMissingChannelClassification, EligibleForClosureReview}`, `NotEligibleReason` six-variant enum, pure `assess()` function, `render_label()` for operator-facing strings.
- `crates/nightshiftd/src/packet.rs` — new `Packet.closure_candidate` field with `#[serde(default = ...)]` so pre-Slice-4 stored packets deserialize cleanly (default is `UnassessableMissingChannelClassification`, the conservative fallback).
- `crates/nightshiftd/src/pipeline.rs` — three call sites set the field:
  - `build_success_packet` — `Reconciled { nq_input_status }`. Recomputed inside `reconcile_phase_with_horizon` after the Slice 3 attention projection mutates `attention_state` (operator ack/silence promotes the verdict from Unassessable to `NotEligible(OperatorAttentionActive)`).
  - `hold_for_preflight` — `RunOutcome::PreflightHeld`.
  - `liveness_gate_failed` — `RunOutcome::LivenessGateFailed`.
- `crates/nightshiftd/src/posture.rs::render_show` — single new `closure:` line, label-rendered. Synthetic WatchUntil test packet updated to include the field; the test now also confirms WatchUntil does **not** count as operator attention for closure purposes.

**Evidence:**
- 12 unit tests in `crates/nightshiftd/src/closure.rs::tests`, including a combinatorial sweep across `{PostureClass × AttentionState × EvidenceState × RunOutcome}` that asserts `EligibleForClosureReview` is unreachable in v1.
- 9 integration tests in `crates/nightshiftd/tests/closure_candidate.rs`:
  - `silence_shape_finding_refuses_with_proxy_quiet`
  - `imported_basis_stale_refuses_with_stale_basis` (Slice B import path)
  - `liveness_gate_failure_refuses_with_liveness_gate_failed`
  - `finding_invalidated_at_reconcile_refuses_with_invalidated_basis` (Slice 5 contract Invalidated)
  - `active_ack_refuses_with_operator_attention_active`
  - `active_silence_refuses_with_operator_attention_active_over_proxy_quiet` (operator intent wins over silence-shape posture)
  - `incident_shape_with_no_blockers_yields_unassessable` (the conservative-default sentinel)
  - `preflight_held_run_refuses_with_preflight_held`
  - `no_pipeline_path_emits_eligible_for_closure_review`
- 264/264 tests green (243 prior + 12 unit + 9 integration = 264), 1 ignored. Clippy clean.

**Field notes:**
- **closure candidate ≠ closure authorization.** There is no close verb in v1 and this predicate does not introduce one. Its purpose is to make refusal categories explicit, testable, and operator-visible *now*, so that when a close verb eventually exists it can consult this surface and refuse closure on the same grounds. The variant `EligibleForClosureReview` exists but is *unreachable* under v1 conditions — the combinatorial sweep proves it.
- **Missing channel classification is a visible refusal, not a deferred feature.** The early framing of this slice was "defer the missing-consequence-channel case until NQ moves." The user-tuned framing (per ChatGPT consultation 2026-05-27) sharpened it: silently dropping the case lets a healthy-looking IncidentShape finding round up to eligibility by absence. Slice 4 instead emits `UnassessableMissingChannelClassification` — the predicate *names* what it cannot assess. "Review has a way of becoming approved in a trench coat."
- **Operator attention vs. silence-shape priority.** Active operator ack/silence is checked before posture-class. Rationale: operator intent is a stronger refusal signal than the finding's wire shape; if both apply, the operator-intent reason is more useful to surface (the operator made an explicit choice). One acceptance test pins this.
- **WatchUntil is not operator attention.** Horizon-driven `AttentionState::WatchUntil` is system-set, not operator-set, and does not refuse closure on operator-attention grounds. It falls through to whatever other refusal applies (typically `Unassessable` for IncidentShape).
- **Stale via Slice B routes through `EvidenceState::Stale`.** The Slice B `imported_producer_basis_stale` path produces `InputStatus::Stale` AND `EvidenceState::Stale`. Either signal causes `NotEligible(StaleBasis)`; the predicate doesn't need to discriminate between the two stale sources.
- **Held packets get refusals too.** Preflight and liveness-gate failures now carry `closure_candidate: NotEligible(PreflightHeld | LivenessGateFailed)` rather than silently inheriting the schema default. The same incident state the run halted on is the same incident state that refuses closure.

**Deferred — NQ-side prerequisite for the full Gate 1:**

To make `EligibleForClosureReview` reachable, NQ must expose a wire-shape distinction between **proxy-channel** findings (dashboard-only normalization signals, "the alert is quiet") and **consequence-channel** findings (substrate witnesses, "customer impact / downstream effect"). The closure predicate would then add a final check: "all blockers absent AND finding is consequence-channel" → eligible; "all blockers absent AND finding is proxy-channel OR channel-unknown" → unassessable (conservative refusal).

Until that NQ-side classification lands, every IncidentShape finding is `Unassessable`. The NS side is *complete* as a refusal surface; the missing piece is upstream evidence shape, not NS implementation. Cross-project advisory should be filed at the NQ side when this slice ships.

**Unblocks:**
- Nothing in NS itself — Slice 4 is the terminal Gate-1 work NS can do without NQ moving.
- A future operator-facing closure verb can now consult `packet.closure_candidate` and refuse closure under the existing six refusal classes.

---

## SLICE_3_ATTENTION_LIFECYCLE V1 (ack + silence)

**Status:** `shipped` 2026-05-27. Slice 3 close-out per [`working/roadmaps/nightshift_v1_runtime_ladder.md`](../roadmaps/nightshift_v1_runtime_ladder.md) — **incremental scope**: ack + silence verbs land in v1. `investigate`, `handoff`, and `request-revalidation` are deferred to follow-ons with explicit unlock triggers (see below).

**Shipped artifacts:**
- `crates/nightshiftd/src/attention.rs` — `PersistedAttentionState::{Acknowledged, Silenced}`; `ReAckDisposition` (the frozen six-value enum from `architecture/GAP-reack-doctrine.md`); `AttentionRow::ack` / `AttentionRow::silence` constructors enforce the at-rest invariants (silence requires both `silence_until` and `silence_reason` at the type level); `project_attention` + `apply_to_packet` are the read-time projection.
- `crates/nightshiftd/src/store.rs` + `store/sqlite.rs` — new `attention_state` SQLite table keyed on `(agenda_id, finding_key)`; `Store::save_attention` (upsert) + `Store::get_attention`.
- `crates/nightshiftd/src/main.rs` — `nightshift attention --agenda <a> --finding <k> --operator <op> ack [--ttl 4h] [--note ...] [--disposition ...]` and `silence --until <rfc3339> --reason <text>`. Re-ack on an existing row requires `--disposition`; silence with empty `--reason` or past `--until` is rejected at parse time. `parse_duration` accepts `s|m|h|d` suffixes.
- `crates/nightshiftd/src/pipeline.rs` — read-time projection injected between `build_success_packet` and `save_packet`. Loads `(agenda, target)` attention, projects against `Utc::now()`, mutates the packet's attention block, and emits a `RunAttentionChanged` ledger event whenever something was applied (acknowledged / silenced / expired).

**Evidence:**
- Unit tests in `crates/nightshiftd/src/attention.rs` (7): projection of None / Acknowledged-in-TTL / Acknowledged-expired / Silenced-in-window / Silenced-expired; `bump_urgency` step-and-cap; `ReAckDisposition` token round-trip.
- Integration tests in `crates/nightshiftd/tests/attention_lifecycle.rs` (8):
  - `ack_persists_into_next_reconcile_and_packet_shows_acknowledged` — operator ack via store, next reconcile reads it.
  - `ack_with_elapsed_ttl_resurfaces_with_urgency_bumped` — projection's Expired branch bumps urgency exactly one step.
  - `silence_applies_silenced_state_and_carries_reason` — silenced packet carries reason and `re_alert_after = silence_until`.
  - `silence_does_not_hide_finding_from_runs_list` — silence is not handling (GAP-attention-state invariant).
  - `attention_never_raises_authority_under_ack_or_silence` — `requested_authority_level` and `governor_verdict` unchanged vs baseline run, for both verbs.
  - `attention_transitions_emit_run_attention_changed_event` — ledger event with `applied: "acknowledged"` payload.
  - `ack_in_one_agenda_does_not_silence_another_agenda` — agenda-scope isolation (one agenda's ack does not project onto another's reconcile).
  - `save_attention_upsert_replaces_prior` — upsert semantics on `(agenda_id, finding_key)`.
- 243/243 tests green (228 prior + 7 attention unit + 8 attention integration = 243), 1 ignored. Clippy clean under `--all-targets -- -D warnings`.

**Field notes:**
- **Operator vs. horizon attention.** When the horizon path sets `AttentionState::WatchUntil`, an operator `ack` or `silence` overrides it. Rationale: operator intent has primacy over system-set watch windows. The horizon's `tolerance_basis_id` / `tolerance_basis_hash` fields are *not* cleared by the operator override, so the Governor receipt's lineage survives in the packet even when the operator's attention state takes the front.
- **No on-expiry GC.** `Expired` projection does not delete the row. The next operator interaction is treated as a re-ack per `architecture/GAP-reack-doctrine.md` — `--disposition` is required. Per GAP-attention-state: "attention state is archived, not lost."
- **Held packets do not consult attention.** Liveness-failed and preflight-held runs build their packets via `liveness_gate_failed` / `hold_for_preflight` and never reach `build_success_packet`. They do not apply operator attention; the next reconciled run picks it up. This matches the invariant that attention applies to a *finding*, not a *run*.
- **Authority isolation.** `apply_to_packet` only mutates the `Attention` struct. The `ProposedAction.requested_authority_level` and `AuthorityResult` fields are left alone. A test explicitly compares against a no-attention baseline to guard this invariant.
- **`as_str` / `parse_token` rather than `FromStr`.** Clippy refuses `from_str` on associated functions because the name collides with the `std::str::FromStr` trait. The methods are renamed to `parse_token` to make the intent unambiguous.

**Unblocks:**
- Slice 4 (closure-candidate predicate) — the attention machinery now exists for the predicate to refuse closure when attention is active.

**Deferred (with unlock criteria):**
- `investigate` verb — unlocks when a real ops session asks "who is on this." No new mechanism; just a new `PersistedAttentionState` variant.
- `handoff` verb — unlocks when cross-operator handoff appears in real ops (or when GAP-parallel-ops moves forward and Continuity becomes load-bearing).
- `request-revalidation` verb — unlocks when an operator wants to force-mark a finding for fresh evidence next cycle without changing its attention state.
- Attention rendering in `runs show` — the projection is applied to the packet; `runs show` already surfaces the packet's attention block via Slice 2 work. A follow-up could also surface the persisted *row* directly (not just the projection's output on a specific run), but that needs a forcing case where the operator wants to see the row without running a reconcile.

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
