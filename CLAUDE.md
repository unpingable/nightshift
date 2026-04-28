# CLAUDE.md — Instructions for Claude Code

## What This Is

Night Shift: deferred agent work with receipts, reconciliation, and governed promotion.

Night Shift manages admissibility across time. Its job is not to decide whether action is authorized, but to prevent old observations, stale plans, and deferred work from silently becoming current authority.

Schedules and resumes *intent* — not commands — under declared policy, with
context bundles that revalidate their own premises before execution.

## What This Is Not

- Not cron with opinions. Cron executes blindly; Night Shift reconciles before acting.
- Not Governor. Governor is the constitutional layer (authority/policy/receipts). Night Shift is the executive calendar (intention/scheduling/promotion).
- Not an autonomous agent. The agent produces proposals under constraint. It does not get to touch production because it used confident adverbs.

## Invariants

1. No mutation without Governor authorization. Night Shift proposes; Governor permits.
2. Context bundles must be reconciled before execution. Stale context is not evidence.
3. Recheck is the gate, not metadata. Every input passes through the Reconciler by virtue of the pipe it enters on. No per-input `requires_recheck` flag.
4. Every run produces ledger events; Governor emits authority receipts. Night Shift does not manufacture authority by logging itself.
5. Lifecycle, authority, and artifact are three distinct ladders. Do not conflate them.
   - Lifecycle: `capture → reconcile → plan → review → run → verify → record`
   - Authority: `observe → advise → stage → request → apply → publish → escalate`
   - Artifact: `receipt | packet | diff | report | page | publication_update`
   A run moves through lifecycle phases but cannot exceed its authority ceiling. `escalate` is a **terminal run posture**, not a peer action of `apply`/`publish`; destinations that realize an escalation (`page`, `notify`, `request_approval`, etc.) are implementations, not authority levels. `page` as an MCP call class is transport, not posture. See `GAP-escalation.md`.
6. MCP is tool transport, not authority. Tool availability is not permission.
7. Continuity is optional context, never authority. Inputs enter as `observed`; the Reconciler may grant `committed` status for a declared scope. `committed` means "accepted for this run under this scope," not "true forever."
8. Missing intelligence dependencies (Continuity, MCP, LLM) must never increase authority. Missing safety dependencies (Governor, evidence adapter, run ledger) lower the promotion ceiling or fail closed.
9. Diagnostic review (self-check / conference) may reduce confidence, downgrade promotion, or require escalation. It may not raise the promotion ceiling or authorize force.
10. Drive to resolution ends where standing begins. Night Shift pursues resolution only while the next step remains within evidence, authority, scope, and budget. Once any boundary is crossed, the run escalates.
11. If the next diagnostic step changes the system, stop. Read-only disambiguation is fine; mutation as disambiguation is not.
12. Backend choice must not change authority semantics. Scaling the store must not scale the trust assumptions. SQLite is v1 default; Postgres is v2 production; the storage contract is the boundary (see `GAP-storage.md`).
13. A run transition must be atomic and exclusive. If the store cannot prove exclusive ownership of a run, Night Shift fails closed.
14. Operator intent has a half-life. Attention state (acknowledged, investigating, silenced) is distinct from evidence state and must carry a TTL or an explicit reason. Attention state never raises authority. See `GAP-attention-state.md`.
    - Silence is not handling.
    - Ack is not closure.
    - Suppression needs an expiry or a reason.
15. Continuity availability is not Continuity use. The reconciler queries shared substrate for concurrent activity in declared scope by default; the run ledger writes observational breadcrumbs to Continuity at surprise / partial / escalation / completion events, not only at run end. Hooked in ≠ used. See `GAP-parallel-ops.md`.
16. Incident modes (incident / remediation / architecture) are distinct and do not share a success condition. A run declares mode, objective, allowed actions, and exit criteria; crossing mode bounds is an invariant breach. Incident state, remediation state, and architectural-followup state are tracked separately. Stabilized ≠ remediated. Deployed ≠ verified. See `GAP-incident-modes.md`.
17. Protected services (observation-critical, control-plane-critical) resist casual turn-down regardless of promotion ceiling or policy verdict. A proposed action that disables a `protected` service requires explicit operator confirmation in all modes. See `GAP-incident-modes.md`.
18. Coordination safety is distinct from authorization safety. Continuity is optional for authorization safety (missing Continuity never raises authority) but required for coordination safety in named risky classes — shared-infrastructure ops, topology/config/publisher/source changes, mode transitions, or protected-class scopes. For these classes, a Continuity preflight is a guardrail: the run cannot leave capture phase without preflight clearance or a named, receipt-generating operator override. The failure mode is not forgetting; it is failing to ritualize recall. See `DESIGN.md` (Continuity role) and `GAP-parallel-ops.md`.
19. The coordination channel is narrowly authoritative. Overlap *existence* and *classification* may gate coordination; breadcrumb *contents* remain `observed` / `hint`. Continuity is authoritative about who else is here; Continuity is never authoritative about what is true.
20. Backup and restore for continuity-bearing workloads are first-class operational truths, not operator folklore. Backup scope must be explicitly declared (never "the whole box"); off-host destinations are required for continuity protection; restore drills are required and non-live by default; SQLite captures must use a safe method. Continuity (cross-run state / coordination / memory) and backup (disaster survival of declared protected state) are related concerns but MUST NOT be collapsed. See `GAP-backup-restore.md`.

## Quick Start

```bash
# TBD — project is in framing stage
```

## Project Structure

- `crates/` — Rust workspace: daemon, agenda state machine, ledger, NQ integration (not yet created)
- `src/nightshift/` — Python package: workflows, LLM orchestration, analysis plugins (not yet created)
- `tests/` — Test suites (Rust: cargo test, Python: pytest)
- `docs/` — Design and specification
  - `DESIGN.md` — canonical architecture document
  - `SCHEMA-agenda.md` — agenda declaration schema (v0 draft)
  - `SCHEMA-bundle.md` — context bundle schema (v0 draft)
  - `SCHEMA-packet.md` — review packet schema (v0 draft)
  - `GAP-governor-contract.md` — adapter contract with Governor (+ `--no-governor` degraded mode)
  - `GAP-mcp-authority.md` — MCP call-class authority rules
  - `GAP-nq-activation.md` — push/pull semantics for NQ findings
  - `GAP-nq-nightshift-contract.md` — NQ finding snapshot contract (first artifact to stabilize)
  - `GAP-escalation.md` — drive-to-resolution gating, escalation triggers/types/destinations
  - `GAP-attention-state.md` — evidence vs attention vs criticality axes; anti-amnesia field kit
  - `GAP-parallel-ops.md` — cross-session coordination; scope overlap; Continuity-as-substrate invariant; breadcrumb cadence
  - `GAP-incident-modes.md` — incident / remediation / architecture modes; incident state ladder; change envelope; protected role class; NOC primitives
  - `GAP-backup-restore.md` — operational backup / restore for continuity-bearing workloads; Backup Contract; capture methods; off-host destinations; verification and restore drills; integration with Governor / NQ / Continuity boundaries
  - `GAP-storage.md` — backend stance (SQLite v1, Postgres v2), contract, Store trait sketch, deployment roadmap
  - `DEPLOYMENT-MATURITY.md` — shared constellation pattern (v1 local → v2 shared → v3 service); Night Shift / NQ / Continuity share the curve, Governor does not

## Conventions

- License: Apache-2.0
- Rust: stable toolchain, clippy clean, no unsafe without justification
- Python: 3.10+, type hints, pytest
- Receipts: content-addressed, append-only, deterministic

## Neighboring projects

- **Governor** (`~/git/agent_gov`): authority/policy/receipts — Python, JSON-RPC daemon
- **NQ** (`~/git/nq`): observatory/failure-domain classifier — Rust, HTTP API
- **Grid Dependency Atlas** (`~/git/grid-dependency-atlas`): publication-mode target
- **Continuity**: cross-project context via MCP (workspace: observatory-family)

## Don't

- Don't let Night Shift become the authority layer. It schedules intent; Governor authorizes force.
- Don't auto-repair without explicit promotion. Start propose-only; mutation comes later via allowlists.
- Don't treat context bundles as junk drawers. Every input has standing: authoritative, hint, stale, or inadmissible.
- Don't skip reconciliation. The 3am agent must not act on 11pm vibes.
- Don't conflate the three ladders (lifecycle/authority/artifact). Terminology drift breeds bugs in the drywall.
- Don't give Python workflows production credentials, mutable tool handles, or unrestricted shell. Workflows read context JSON and emit proposal JSON. Nothing else.
- Don't make Continuity a hard dependency. Optional context, never authority.
- Don't add per-input `requires_recheck` flags. Recheck is the gate, not metadata.
- Don't let a smarter model unlock higher authority. Intelligence dependencies improve quality, never permission.
- Don't treat `committed` as "true forever." It means "accepted for this run under this scope, after reconciliation."
- Don't treat acknowledgment as closure. Ack needs a TTL; silence needs a reason or a timestamp. Attention state without a half-life is a graveyard.
- Don't assume Continuity is being used just because it's hooked in. The reconciler queries it for concurrent activity by default; the run ledger writes breadcrumbs by default. Availability ≠ use.
- Don't mix incident modes. Stabilization is not a license for redesign; architectural insight is not a substitute for stabilization; shipped remediation is not closed remediation. Cross-mode work requires explicit operator override.
- Don't treat `protected` services as a flag checked at the end. The reconciler resists casual turn-down throughout the run, not just at authorization.
- Don't collapse Continuity (cross-run state / coordination / memory) into backup (disaster survival of declared protected state). Related concerns, different failure modes. See `GAP-backup-restore.md`.
- Don't treat "same host, different directory" as off-host. That is staging, not continuity protection. Off-host means out of the protected host's primary failure domain.
- Don't let Nightshift become a universal archive substrate. Nightshift owns backup *orchestration and visibility*, not preservation theory. Evidentiary archive is a separate concern if it ever needs to exist.
- Don't treat Continuity as advisory for risky work. For shared-infrastructure ops, topology/config/publisher/source changes, mode transitions, or protected-class scopes, preflight is a guardrail — a run that skips it is not a faster run, it is an unsafe run.
- Don't propose execution on stale evidence. Staleness escalates to revalidation, not action. Night Shift may schedule a recheck; it may not propose mutation against evidence the reconciler flagged stale.
- Don't branch Night Shift behavior on NQ witness positions. NQ witness-position taxonomy (substrate / application_internal / application_external / platform_internal / platform_external) lives entirely inside NQ's grammar. Night Shift consumes the finding *shape* NQ surfaces; it does not interpret which witness wins. If two positions disagreeing should change scheduling, NQ encodes that into the finding shape and Night Shift responds to the shape — not to the witness metadata.
