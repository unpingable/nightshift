# Night Shift

Deferred agent work with receipts, reconciliation, and governed promotion.

> Let agents work late without giving them the keys.

Night Shift schedules and resumes *intent*, not commands. A cron job says
"run this command at this time." Night Shift says "resume this intention
under this policy with this context and produce this kind of artifact."

It is allowed to be useful before it is trusted with force.

## Starting point: governed ops review packets from real NQ findings

The MVP is narrow and real:

```bash
nightshift watchbill run wal-bloat-review
```

1. Read current NQ findings
2. Assemble context bundle from captured agenda
3. Reconcile: compare captured state vs. current state
4. Run bounded diagnosis workflow
5. Emit a repair proposal packet
6. Record run events; Governor emits authority receipts
7. No mutation. No sudo. No cowboy shit.

Ops mode (Watchbill) is first because it pressure-tests the authority
boundary on low-blast-radius work before anything heavier gets built on
top of it.

## Future modes

- **Code mode**: Deferred coding sessions that produce reviewable diffs,
  branches, and reports — not automatic merges.
- **Publication mode (Atlas Runner)**: Recurring scans and candidate
  updates for public observatories (Grid Dependency Atlas, feeds, static
  sites) with claim-checked receipts.

Build order: ops → code → publication. Most Governor-demanding to most
audience-legible.

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
| **Agenda** | Declared deferred intention: task, mode, cadence, owner, scope, promotion ceiling |
| **Bundle** | Captured context with admissibility: inputs, freshness, standing |
| **Reconciler** | Freshness/invalidation pass before execution begins |
| **Watchbill** | Ops-mode roster of recurring operational agendas |
| **Packet** | Reviewable output artifact (diff, report, proposal) |
| **Run ledger** | Append-only record of scheduler lifecycle events |

## The three ladders

These are distinct. Keep them distinct.

**Lifecycle phases** — where a run is:

```text
capture → reconcile → plan → review → run → verify → record
```

**Authority levels** — what a run is allowed to do:

```text
observe → advise → stage → request → apply → publish → escalate
```

**Artifact kinds** — what a run produces:

```text
receipt | packet | diff | report | page | publication_update
```

A run moves through lifecycle phases, but it cannot exceed its configured
authority level. Artifacts are recorded through both.

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

**Night Shift** schedules and resumes intent. **Governor** authorizes
force. **NQ** provides evidence. **The agent** produces proposals under
constraint.

## Separation of concerns

- **Night Shift owns**: when, why, what context, what constitutes success;
  the run ledger records lifecycle events
- **Governor owns**: whether, under what authority, what must be recorded;
  authority receipts record permission decisions
- **NQ owns**: what is observed, failure classification, persistence
  tracking
- **Agent owns**: interpretation, proposal generation (never direct
  mutation)

Night Shift records run events; Governor emits authority receipts. A run
may contain many receipts, but Night Shift does not manufacture authority
by logging itself.

## Authority model

Governor requirement by authority level:

```text
observe     may run without Governor
advise      may run without Governor, emits unsigned/local receipts
stage       requires Governor
request     requires Governor
apply       requires Governor
publish     requires Governor
escalate    configurable, receipts through Governor when available
```

Night Shift without Governor is degraded / unsafe / demo-only. The
coupling is conceptual, not accidental: deferred intent is dangerous
because authority drifts over time. Governor exists to prevent intent
from becoming unauthorized force.

**Don't trust the agent. Trust the boundary.**

## MCP role

MCP is capability discovery and tool transport. It tells Night Shift what
tools exist and provides a normalized way to call them. **Tool
availability is not permission.**

MCP call classes:

```text
discover   list tools/resources           local policy may allow
read       fetch state                    local policy may allow
propose    produce candidate action       local policy may allow
stage      prepare mutation               requires Governor
mutate     change state                   requires Governor
publish    expose public artifact         requires Governor
page       wake human                     requires Governor (receipt)
```

Every non-local-policy call passes through an authority checkpoint:

```text
Night Shift agenda
  → reconcile context
  → choose proposed MCP tool call
  → Governor policy check
  → MCP invocation
  → result capture
  → Governor receipt
```

## Continuity role

Continuity is an optional context provider, not an authority source.

Continuity inputs enter the bundle as **observed context**. They become
**relied-upon** only after the Reconciler evaluates them against current
evidence, scope, and freshness.

> Continuity can explain why an agenda exists. It cannot prove that
> an action is allowed.

Recheck is the gate, not metadata. Every Continuity input passes through
the Reconciler by virtue of the pipe it enters on — there is no
per-input "requires_recheck" flag that can be forgotten.

Safety must not depend on Continuity. If Continuity is unavailable,
Night Shift should still be able to reconcile current evidence, respect
promotion ceilings, emit packets, and route all promoted actions through
Governor.

> Optional context, never authority.

## Dependency classes

Night Shift distinguishes **safety dependencies** from **intelligence
dependencies**.

**Safety dependencies** (hard) — constrain authority; when unavailable,
Night Shift fails closed or lowers the promotion ceiling:
- Governor (required for `stage | request | apply | publish`)
- Evidence adapter (NQ for ops; git/fs for code)
- Run ledger

**Intelligence dependencies** (soft) — improve quality; when
unavailable, Night Shift continues with degraded output:
- Continuity (prior decisions, cross-run memory)
- MCP (tool discovery/transport)
- LLM / interferometry (hypothesis generation)

> Missing intelligence dependencies must never increase authority.

## Build tiers

1. **Core** — agenda + bundle + reconciler + run ledger + packet.
   Useful, safe, boring. Cannot mutate.
2. **Governed** — Core + Governor adapter + authority receipts +
   promotion gates. The real product.
3. **Constellation** — Governed + NQ + Continuity + MCP + observatory
   adapters. Ecosystem mode.

## Language split

- **Rust**: scheduler daemon, agenda state machine, run ledger, receipt
  emission, policy binding, execution leases, concurrency, NQ integration
- **Python**: LLM/interferometry orchestration, analysis plugins,
  candidate repair generation, report writing, prompt experiments

Rust binary invokes Python workflows as controlled subprocesses.

**Python workflow boundary (invariant):** Python workflows do not
receive production credentials, mutable tool handles, or unrestricted
shell access. They read context JSON and emit proposal JSON.
Night Shift and Governor decide whether any proposed operation is
staged, applied, or published.

Python can be weird without being sovereign.

## License

Licensed under Apache-2.0.
