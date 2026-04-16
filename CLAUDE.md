# CLAUDE.md — Instructions for Claude Code

## What This Is

Night Shift: deferred agent work with receipts, reconciliation, and governed promotion.

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
   A run moves through lifecycle phases but cannot exceed its authority ceiling.
6. MCP is tool transport, not authority. Tool availability is not permission.
7. Continuity is optional context, never authority. Inputs enter as `observed`; the Reconciler may grant `committed` status for a declared scope. `committed` means "accepted for this run under this scope," not "true forever."
8. Missing intelligence dependencies (Continuity, MCP, LLM) must never increase authority. Missing safety dependencies (Governor, evidence adapter, run ledger) lower the promotion ceiling or fail closed.
9. Diagnostic review (self-check / conference) may reduce confidence, downgrade promotion, or require escalation. It may not raise the promotion ceiling or authorize force.
10. Drive to resolution ends where standing begins. Night Shift pursues resolution only while the next step remains within evidence, authority, scope, and budget. Once any boundary is crossed, the run escalates.
11. If the next diagnostic step changes the system, stop. Read-only disambiguation is fine; mutation as disambiguation is not.

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
