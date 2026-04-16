# Night Shift

Deferred agent work with receipts, reconciliation, and governed promotion.

> Let agents work late without giving them the keys.

## What it does

Night Shift schedules and resumes *intent*, not commands. A cron job says
"run this command at this time." Night Shift says "resume this intention
under this policy with this context and produce this kind of artifact."

- **Ops mode (Watchbill)**: Scheduled operational agendas that consume NQ
  findings, produce diagnosis and repair proposals, and gate all mutation
  through Governor.
- **Code mode**: Deferred coding sessions that produce reviewable diffs,
  branches, and reports — not automatic merges.
- **Publication mode**: Recurring scans and candidate updates for public
  observatories (Grid Dependency Atlas, feeds, static sites) with
  claim-checked receipts.

## What this is not

- Not cron. Cron executes; Night Shift intends, and the intention must
  survive a gauntlet before it touches anything real.
- Not Governor. Governor owns authority and permission boundaries.
  Night Shift owns scheduling, context, and promotion.
- Not an autonomous agent framework. The agent is the intern with
  astonishing confidence and no legal personhood.

## Core primitives

| Primitive | What it is |
|-----------|-----------|
| **Agenda** | Declared deferred intention: task, mode, cadence, owner, scope |
| **Bundle** | Captured context with admissibility: inputs, freshness, standing |
| **Reconciler** | Freshness/invalidation pass before execution begins |
| **Watchbill** | Ops-mode roster of recurring operational agendas |
| **Packet** | Reviewable output artifact (diff, report, proposal) |
| **Promotion** | `capture → reconcile → propose → authorize → execute → verify → publish` |

## Architecture

```text
NQ / Driftwatch / Labelwatch
        |
   Night Shift
        |
  agenda + context bundle
        |
  reconcile (revalidate premises)
        |
  agent/interferometry workflow
        |
  proposal / diff / repair packet
        |
  Governor hook boundary
        |
  receipted action or review artifact
```

**Night Shift** schedules and resumes intent. **Governor** authorizes force.
**NQ** provides evidence. **The agent** produces proposals under constraint.

## Separation of concerns

- **Scheduler owns**: when, why, what context, what constitutes success
- **Governor owns**: whether, under what authority, what must be recorded
- **NQ owns**: what is observed, failure classification, persistence tracking
- **Agent owns**: interpretation, proposal generation (never direct mutation)

## Authority model

Night Shift may use MCP for tool access, but MCP is not an authority
layer. Tool availability is not permission. All promoted actions pass
through Governor policy and receipt boundaries.

Night Shift without Governor is degraded / unsafe / demo-only. The
coupling is conceptual, not accidental: deferred intent is dangerous
because authority drifts over time. Governor exists to prevent
intent from becoming unauthorized force.

Don't trust the agent. Trust the boundary.

## Escalation ladder

```text
observe     no action, record state
advise      produce diagnosis / recommendation
stage       prepare reversible command or patch
request     ask operator for approval
apply       execute approved action
verify      confirm effect
escalate    page human because standing/confidence/scope failed
```

Night Shift is not an autonomous operator. It is a deferred operational
assistant that prepares, constrains, records, and escalates work under
human-governed authority.

It reduces toil without laundering accountability.

## Language split

- **Rust**: scheduler daemon, agenda state machine, run ledger, receipt
  emission, policy binding, execution leases, concurrency, NQ integration
- **Python**: LLM/interferometry orchestration, analysis plugins, candidate
  repair generation, report writing, prompt experiments

Rust binary invokes Python workflows as controlled subprocesses.
Python can be weird without being sovereign.

## Build order

1. **Ops / Watchbill** — strongest Governor dogfood, simplest inference
   burden, real NQ findings from day one
2. **Code / Night Shift proper** — deferred diff production, higher complexity,
   needs battle-tested Governor hooks
3. **Publication / Atlas Runner** — best demo surface, least demanding
   governance workout

## MVP: Ops observe/propose only

```bash
nightshift watchbill run wal-bloat-review
```

1. Read NQ findings
2. Detect persistent finding pattern
3. Assemble context bundle
4. Reconcile: check what changed since capture
5. Run bounded diagnosis workflow
6. Emit repair proposal packet
7. Write receipts
8. No mutation. No sudo. No cowboy shit.

## License

Licensed under Apache-2.0.
