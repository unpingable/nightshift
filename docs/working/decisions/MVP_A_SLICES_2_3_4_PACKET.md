# MVP-A Slices 2/3/4 Packet — NS Cook + Wicket + WLP wiring

**Filed:** 2026-05-28 by chat-context Cartographer (cross-repo coordination scope).
**Status:** repo-local spec drop. **No implementation authorized yet.** Build target for whoever takes the slice (component-agent or bounded-exception Cartographer session).
**Origin:** [MVP-A Plan rev1](file:///home/jbeck/git/cartography/audit/2026-05-28-mvp-plan-rev1.md), Slices 2, 3, 4.

## What this packet is

A self-contained build scope for NS's role in the MVP-A demo loop:

```
substrate → NQ → NS (this slice) → Wicket → WLP → Continuity persistence
```

NS takes a sushi-k `disk_pressure` finding, classifies it (already proven by e0b51e0 dogfood), cooks a Wicket Intent, invokes Wicket, wraps the Outcome as a WLP AuthorizationReceipt, and writes the HandlingReceipt to a local sink for Continuity to pick up.

Estimated work: ~400 LOC new Rust + integration tests.

## What already exists (Cartographer re-audit 2026-05-28, post-NS-tools-down)

**Core internals:**
- `crates/nightshiftd/src/nq.rs:28-74` — `NqSource` trait + `CliNqSource` production impl (shells out to `nq findings export`; proven working in commit `e0b51e0`).
- `crates/nightshiftd/src/reconciler.rs:97-207` — `reconcile_nq_input()` classification.
- `crates/nightshiftd/src/packet.rs:222-251` — `Packet` struct with `finding_summary`, `diagnosis`, `proposed_action`, `authority_result`, `attention`.
- `crates/nightshiftd/src/posture_class.rs` — `PostureClass` enum (already derived per Slice C.1).
- `crates/nightshiftd/src/closure.rs:1-100` — Slice 4 closure predicate (refusal-only; conservative).

**Recent commits (NS-Claude tools-down 2026-05-28, late-day):**
- `c654521` — AUDIT-BACKLOG remote-standing breadcrumb (NS-side)
- `f33b75a` — NS-side channel-split half + forbidden-cycle audit owed
- `e0b51e0` (2026-05-28 12:56) — first live-data dogfood; proved NQ → NS pipe end-to-end on live `nq:disk_pressure:sushi-k:`. Used `/tmp/nightshift-probe.sqlite` and `~/nq/nq.db` local instance.
- `002fde9` — `deploy/agendas/sushi-k-disk-pressure.yaml` — first named local pilot agenda
- `11248d6` — `deploy/systemd/user/` units for sushi-k local pilot

**Pilot status:** wired but **not enabled.** Per NS-Claude tools-down note: durable store at `~/nightshift/local.sqlite`, agenda at `deploy/agendas/sushi-k-disk-pressure.yaml`, user-level systemd units at `deploy/systemd/user/`. Operator's call to enable (`systemctl --user enable --now`) — gated on building a release first.

## What does NOT exist yet

- Zero Intent-cook code (no Wicket schema knowledge in NS).
- Zero posture-packet outbound to file sink (current emit is stdout YAML via `render_show()`; pilot durable store at `~/nightshift/local.sqlite` is internal, not a Wicket-Intent sink).
- Zero `wicket` Cargo dependency.
- Zero `wlp` Cargo dependency.

## Input shape (pick one for build target)

The build can source disk_pressure findings from either:

**Option A — fixture-based (recommended for development).** Use a captured FindingSnapshot from a known sushi-k `disk_pressure` finding. Deterministic; no pilot prerequisite; matches integration-test discipline.

**Option B — live pilot.** If the pilot is enabled (`systemctl --user enable --now` per `deploy/systemd/user/`), the durable store at `~/nightshift/local.sqlite` carries real reconciler output and the agenda at `deploy/agendas/sushi-k-disk-pressure.yaml` schedules NQ snapshots. This exercises the slice under realistic conditions but adds operational dependencies.

**Option C — /tmp/ probe path** (per e0b51e0 dogfood). Also valid; transient; matches what was already exercised.

Slice acceptance (this packet) is the same regardless of input source. Build against Option A first; verify against B/C if useful.

### Build acceptance ≠ demo closeout eligibility

The three options above govern **build acceptance** for Slices 2/3/4. They do *not* govern **demo closeout eligibility**, which is a separate scope owned by Slice 6 (cartographer-drafted runbook).

- **Build acceptance** (this packet): fixture / pilot / `/tmp` are all valid. Synthetic runs may prove wire compatibility — that the pipes fit.
- **Demo closeout eligibility** (Slice 6 in [`~/git/cartography/audit/2026-05-28-mvp-plan-rev1.md`](file:///home/jbeck/git/cartography/audit/2026-05-28-mvp-plan-rev1.md)): the post-MVP-A closeout may only cite a run whose root NQ receipt is a **live observation of sushi-k host disk/filesystem/resource state**, not a synthesized specimen. A `/tmp` path is acceptable only as capture-location for a live-root receipt, not as the source of a synthetic probe.

Wire-E2E success (any of A/B/C) closes the build slices. Substrate-rooted E2E (live-root NQ receipt → hash-walkable chain → Continuity readback) authorizes the closeout note. These are independent statuses.

The framing exists because absence of fakery is not proof of substrate contact — the same discipline as the subject-boundary interpretation (`MVP_A_SLICE_1_PACKET.md` and rev1 §"Subject-boundary interpretation"). Build-flexibility doesn't drift into the closeout claim.

## Build target

### Slice 2 — Intent-cook + posture-packet outbound (~200 LOC)

**Intent-cook layer:**

- Input: nq-core `Receipt` + `FindingSnapshot` for a `disk_pressure` finding (already produced by existing `NqSource`).
- Output: Wicket Intent JSON conforming to `~/git/wicket/schemas/intent.schema.json`.
- Required Intent fields:
  - `actor` = NS component identity (e.g., `"ns:nightshiftd:<instance_id>"`).
  - `action` = the operation NS proposes — narrow, falsifiable. Suggested: `"classify_finding"` or `"propose_closure"`. Caller-asserted per Wicket v0.1 conventions; `STANDING_CALLER_ASSERTED_UNVERIFIED` will be emitted in the receipt, which is doctrinally correct for MVP-A.
  - `basis` = NQ receipt reference (`receipt_hash` + `receipt_id` from nq-core `Receipt`).
  - `precedence` = NS reconciler version (so Wicket can pin the policy).
  - `evidence` = array including:
    - NQ receipt content hash (kind: `prior_receipt`)
    - NS internal classification policy_ref (kind: `policy_ref`, e.g., `CLAUDE.md` or `SLICE_4_CONTRACT` hash)
  - `scope` = the finding's host + claim_kind context.
- Preserve NQ evidence reference into `Intent.evidence`: this is the chain Wicket walks.

**Posture-packet outbound:**

- Write the existing `Packet` struct to a known local file path on each `reconcile_phase` completion.
- Recommended interface: `--posture-sink <path>` CLI flag; default `/tmp/ns-posture-<run_id>.json`.
- Canonical JSON serialization (deterministic field order).
- Existing SQLite packet storage is unaffected; this is additive.

### Slice 3 — Wicket invocation (~50 LOC + Cargo dep)

- Add `wicket = { path = "../wicket" }` (or appropriate dependency) to `crates/nightshiftd/Cargo.toml`.
- Call `wicket::check(&intent) -> Outcome` from NS code after Intent-cook.
- Capture Outcome.
- Write Outcome to local sink (e.g., `/tmp/ns-wicket-outcome-<run_id>.json`).
- **Zero changes to `~/git/wicket/`** — read-only. No new fixtures, no schema edits, no `cases/` additions.

### Slice 4 — WLP AuthorizationReceipt wrapping (~100 LOC + Cargo dep)

- Add `wlp = { path = "../wlp" }` to `crates/nightshiftd/Cargo.toml`.
- Adapter module: Wicket `Outcome` → WLP `Artifact` (kind = `AuthorizationReceipt`).
  - `Outcome.verdict`, `dimensions`, `reason_codes` → `Artifact.payload`.
  - Wicket receipt hash → `Artifact.custody.causal_parents[0]`.
- Call `wlp::handle(&artifact, &[], &opts)` with empty context (no revocation in MVP-A).
- Capture `HandlingReceipt`.
- Write to local sink (e.g., `/tmp/ns-wlp-handling-<run_id>.json`).
- **Zero changes to `~/git/wlp/`** — read-only.

## Subject-boundary

The `disk_pressure` finding's subject MUST remain interpretable as:

> **sushi-k host filesystem/resource state — NOT NQ, NOT NS, NOT the observation loop.**

If the input finding's subject ever stops matching that interpretation (e.g., a refactor in NQ makes it about NS-internal state), **stop and report** before constructing Wicket Intent. The interpretation is anchored to artifacts in `~/git/notquery/` (see [NQ Slice 1 packet](file:///home/jbeck/git/notquery/docs/working/decisions/MVP_A_SLICE_1_PACKET.md) for anchors).

Don't rely on empty subject field; rely on the explicit interpretation.

## Acceptance

Slices 2/3/4 close when, given a sushi-k `disk_pressure` finding fixture:

1. NS produces Wicket Intent JSON that validates against `intent.schema.json`.
2. Posture-packet appears at the known local sink path with deterministic canonical content.
3. Wicket Outcome JSON captured from `wicket::check()`.
4. WLP HandlingReceipt JSON captured from `wlp::handle(&artifact, &[], &opts)`.
5. **Hash chain is walkable** (the load-bearing acceptance):
   - `WLP HandlingReceipt.custody.causal_parents[0] == Wicket receipt_hash`
   - `Wicket Intent.evidence[<index for prior_receipt>] == NQ receipt content hash`
   - NS posture-packet references NQ `receipt_id`
6. Integration test fixture demonstrates the round-trip deterministically.

## Must NOT

- Treat NS posture as truth (the forbidden NS-posture-into-NQ-truth cycle remains structurally absent per [NQ-NS-CHANNEL-SPLIT.md](file:///home/jbeck/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md); no code path can launder).
- Add subscription, notification, or publish surfaces (transport is deferred to a separate gap).
- Add remote emit beyond the existing Governor RPC posture-fact path (if it exists; don't invent new remote surfaces).
- Add revocation handling. AuthorizationReceipt path only; no RevocationReceipt in MVP-A.
- Extend NS posture vocabulary or add new closure verdicts.
- Touch `~/git/wicket/`, `~/git/wlp/`, or `~/git/notquery/` code.
- Call Linode (no Labelwatch, no Driftwatch — Path B is later, separate).
- Call lil-nas-x (Path A.5 is later, separate).
- Introduce schema migrations for new tables. Slice 2 output is JSON to filesystem, not new SQLite columns. If SQLite changes seem needed to express the slice, **stop and report.**

## Stop conditions

- Subject-boundary interpretation can no longer be re-derived from NQ artifacts → stop, report, do not improvise.
- Intent construction requires Wicket schema changes → stop and report (Wicket is in v0.1.0 freeze for MVP-A).
- WLP `handle()` requires v0.2 graph-aware context beyond empty `&[]` → stop and report.
- Posture-packet sink requires a transport / notification primitive (subscribe, replay, deliver) → stop; that's a separate gap.
- Any slice surfaces a need to modify Wicket or WLP source → stop and report.

## Composes with

- [MVP-A Plan rev1](file:///home/jbeck/git/cartography/audit/2026-05-28-mvp-plan-rev1.md) — full Slices 2/3/4 context, path ladder, subject-boundary section
- [Coordination registration](file:///home/jbeck/git/cartography/coordination/MVP-PATH-A-PLAN.md)
- [NQ-NS-CHANNEL-SPLIT](file:///home/jbeck/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md) — channel discipline (the load-bearing cycle prohibition)
- [SELF-SUBJECT-COLLAPSE](file:///home/jbeck/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md) — MVP-A avoids self-subject path; chosen substrate is external
- This repo's existing work: `NqSource`, `reconciler`, `Packet`, `PostureClass`, closure predicate (do not duplicate; do not re-derive)
- e0b51e0 milestone (2026-05-28): demonstrates the input shape

## Wicket / WLP integration anchors (read-only)

- Wicket entry point: `wicket::check(intent: &Intent) -> Outcome` (public API in `wicket/src/lib.rs`)
- Wicket Intent schema: `~/git/wicket/schemas/intent.schema.json`
- Wicket Outcome schema: `~/git/wicket/schemas/verdict.schema.json`
- Wicket fixtures for reference: `~/git/wicket/cases/` (study; do not modify)
- WLP entry point: `wlp::handle(parent: &Artifact, context: &[&Artifact], opts: &HandleOpts) -> Artifact`
- WLP Artifact model: `~/git/wlp/src/model.rs` (`Kind::AuthorizationReceipt`)
- WLP tests for reference: `~/git/wlp/tests/boundary_admissibility.rs`, `revocation_edge.rs`, `canonical_equivalence.rs`

## Provenance

Filed by Cartographer per operator instruction 2026-05-28 (post §H confirmation, packet placement phase). Cartographer's authority for cross-repo writes is bounded to coordination/docs scope per operator directional 2026-05-28; this packet is a docs-only spec drop, not implementation.
