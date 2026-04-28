# AGENTS.md — Working in this repo

This file is a **travel guide**, not a law.
If anything here conflicts with the user's explicit instructions, the user wins.

> Instruction files shape behavior; the user determines direction.

---

## Role

Night Shift manages admissibility across time. Its job is not to decide whether action is authorized, but to prevent old observations, stale plans, and deferred work from silently becoming current authority.

Pair with NQ (premise-movement detection) and Governor (authorization decisions): *NQ detects that the premise moved. Governor decides whether the authorization fell off. Night Shift makes that movement legible across the deferral.*

---

## Quick start

```bash
# TBD — project is in framing stage
# Rust: cargo build && cargo test
# Python: pip install -e '.[dev]' && pytest
```

## Tests

```bash
cargo test          # Rust
pytest              # Python
```

Always run tests before proposing commits. Never claim tests pass without running them.

---

## Safety and irreversibility

### Do not do these without explicit user confirmation
- Push to remote, create/close PRs or issues
- Delete or rewrite git history
- Modify dependency files in ways that change the lock file
- Any action that would promote an agenda beyond `propose` stage
- Anything that touches Governor policy or NQ configuration

### Preferred workflow
- Make changes in small, reviewable steps
- Run tests locally before proposing commits
- For any operation that affects external state, require explicit user confirmation

---

## Repository layout

```
crates/                     Rust workspace
  nightshift-core/          Agenda, bundle, reconciler, promotion primitives
  nightshift-daemon/        Scheduler daemon, execution leases
  nightshift-nq/            NQ client / findings integration
  nightshift-ledger/        Run ledger, receipt emission
src/nightshift/             Python package
  workflows/                LLM/interferometry orchestration
  analysis/                 Diagnosis plugins, repair generation
  reports/                  Packet rendering, output formatting
tests/                      Test suites
docs/                       Design documents, architecture decisions
```

---

## Coding conventions

- Rust: stable toolchain, clippy clean, no unsafe without justification
- Python: 3.10+, type hints, pytest >=8.0
- Receipts: content-addressed, append-only, deterministic
- Governor integration: required for any mode above `observe`

---

## Invariants

1. No mutation without Governor authorization. Night Shift proposes; Governor permits.
2. Context bundles must be reconciled before execution. Stale context is not evidence.
3. Every run produces receipts. If it happened without a receipt, it didn't happen correctly.
4. Promotion is explicit and sequential. No step in `observe → reconcile → propose → authorize → execute → verify → publish` may be skipped.
5. MCP is tool transport, not authority. Tool availability is not permission.
6. Staleness escalates to revalidation, not action. Night Shift may schedule a recheck on stale evidence; it may not propose mutation against it.
7. Witness positions are NQ's grammar, not Night Shift's. NS consumes the finding shape NQ surfaces; it does not branch behavior on substrate / application_internal / application_external / platform_internal / platform_external. If position disagreement should change scheduling, NQ encodes it into the finding shape and NS responds to the shape.

---

## What this is not

- Not cron. Cron executes blindly; Night Shift reconciles before acting.
- Not Governor. Governor is the authority layer. Night Shift is the scheduling/promotion layer.
- Not an autonomous operator. It reduces toil without laundering accountability.

---

## When you're unsure

Ask for clarification rather than guessing, especially around:
- Whether a change affects the Governor integration boundary
- Anything involving the promotion path or escalation ladder
- Receipt schema changes (these are append-only contracts)
- Anything that changes a documented invariant

---

## Agent-specific instruction files

| Agent | File | Role |
|-------|------|------|
| Claude Code | `CLAUDE.md` | Full operational context, build details, conventions |
| Codex | `AGENTS.md` (this file) | Operating context + defaults |
| Any future agent | `AGENTS.md` (this file) | Start here |
