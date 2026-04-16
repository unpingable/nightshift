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
3. Every run produces receipts. If it happened without a receipt, it didn't happen correctly.
4. Promotion is explicit: observe → reconcile → propose → authorize → execute → verify → publish. No step may be skipped.

## Quick Start

```bash
# TBD — project is in framing stage
```

## Project Structure

- `crates/` — Rust workspace: daemon, agenda state machine, ledger, NQ integration
- `src/nightshift/` — Python package: workflows, LLM orchestration, analysis plugins
- `tests/` — Test suites (Rust: cargo test, Python: pytest)

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
