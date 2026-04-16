# Night Shift — Design Document

> Deferred agent work with receipts, reconciliation, and governed promotion.

## Thesis

Night Shift schedules and resumes *intent*, not commands. It is an agenda
runner, not a cron daemon. The distinction:

- A cron job says: "run this command at this time."
- Night Shift says: "resume this intention under this policy with this
  context and produce this kind of artifact."

The core claim:

> Night Shift handles the boring night-work around observation,
> reconciliation, hypothesis, and packet assembly. Governor decides
> whether anything gets force. The operator remains the source of
> authority.

Or shorter: it reduces toil without laundering accountability.

## Separation of concerns

```text
Night Shift = intention queue / deferred work / recurring execution / continuity handoff
Governor    = authority / policy / receipts / permission boundary
NQ          = observatory / failure-domain classification / evidence
MCP         = capability discovery / tool transport (not authority)
Agent       = interpretation / proposal generation (not force)
```

Night Shift asks: "What should run, when, with what context, under what
mode, and what constitutes success?"

Governor answers: "Is this allowed, what tools may it touch, what must be
recorded, and where does operator review happen?"

NQ answers: "What is observed, how is it classified, and how long has it
persisted?"

**This separation is load-bearing.** Night Shift must not become the
authority layer. Governor must not become cron with opinions.

## Core primitives

### Agenda

A declared deferred intention.

```text
agenda_id: wal-bloat-review
mode: ops.propose
cadence: scheduled | event | manual
owner: operator identity
scope:
  hosts: [...]
  repos: [...]
  services: [...]
artifact_target: repair_proposal | diff | report | static_site_update
promotion_ceiling: observe | advise | stage | apply | publish
```

### Bundle (Context Bundle)

Captured context with admissibility. Not "whatever seems useful" — a
declared evidence object where every input has standing.

```text
inputs:
  - nq_finding_snapshot     (authoritative)
  - prior_receipts          (authoritative)
  - repo_state              (authoritative, but may be stale)
  - operator_notes          (hint)
  - policy_binding          (authoritative)
freshness:
  captured_at: ...
  expires_at: ...
  invalidates_if:
    - repo_head_changed
    - finding_absent_for_n_generations
    - host_unreachable
    - policy_hash_changed
```

Input standing categories:
- **Authoritative**: verified by Governor receipt or NQ finding
- **Hint**: operator-provided guidance, not independently verified
- **Stale**: was authoritative, freshness expired
- **Inadmissible**: explicitly excluded from action decisions

### Reconciler

The freshness/invalidation pass that runs before execution begins. This
is what keeps the 3am agent from acting on 11pm vibes.

Two phases:

**1. Capture bundle** — created when the agenda is set:
> Here is what we knew when this was scheduled.
> Here is why this run exists.
> Here is the intended output.

**2. Reconciliation bundle** — created when the run actually starts:
> Here is what changed.
> Here is what is still valid.
> Here is what must be downgraded from fact to historical note.

Without reconciliation, the context bundle becomes an alibi machine:
"The agent acted correctly according to stale context." No. That is
bureaucratic negligence with better indentation.

### Promotion path

The escalation/promotion ladder:

```text
observe     no action, record state
advise      produce diagnosis / recommendation
stage       prepare reversible command or patch
request     ask operator for approval
apply       execute approved action
verify      confirm effect
escalate    page human because standing/confidence/scope failed
```

Per mode:

**Ops mode:**
```text
capture → reconcile → propose → authorize → execute → verify → receipt
```

**Code mode:**
```text
capture → reconcile → draft → diff → review → promote
```

**Publication mode:**
```text
capture → scan → diff → claim-check → stage → publish
```

Same skeleton, different verbs.

### Packet

The reviewable output artifact. The important object is not "agent did
work." It is:

> "Agent produced a reviewable artifact after deferred execution under
> declared constraints."

### Watchbill

Ops-mode roster. Scheduled operational agendas that consume NQ findings.

## Three modes

### 1. Ops mode (Watchbill)

Scheduler wakes on time/event, pulls current findings, runs diagnosis
workflow, proposes repair, Governor gates any action, receipt chain
records finding → hypothesis → proposed action → authorized action →
result.

Key rule: **auto-repair starts as propose-only.** Actual mutation comes
later, probably per-host/per-check allowlists. Otherwise you've built
`cron.d/skynet`.

### 2. Code mode

User gives plan/spec before sleep. Scheduler runs bounded agent sessions
later. Multiple-model/interferometry path optional. Output is
branch/diff/report, not automatic merge. Governor hooks constrain file
edits, commands, budgets, and promotion.

### 3. Publication mode (Atlas Runner)

Scheduled scan, source fetch/check, diff against known state, generate
candidate update, publish only if permitted, receipts for public claims.
The easiest public-facing demo because it's mundane, useful, and
non-spooky.

## Governor binding

Night Shift without Governor is degraded / unsafe / demo-only. The
coupling is conceptual, not accidental:

- Night Shift creates deferred intent.
- Deferred intent is dangerous because authority drifts over time.
- Governor exists to prevent language/intent/tool-use from becoming
  unauthorized force.
- Therefore Night Shift's safety claim is incomplete without Governor.

Loosely coupled at code boundary, tightly coupled at protocol boundary.

Governor requirement by promotion level:

```text
observe     may run without Governor
advise      may run without Governor, emits unsigned/local receipts
stage       requires Governor
request     requires Governor
apply       requires Governor
publish     requires Governor
escalate    configurable, receipts through Governor when available
```

## MCP role

MCP is capability discovery and tool transport. It tells Night Shift
what tools exist and provides a normalized way to call them.

MCP does not decide whether a tool call is allowed, whether stale
context is admissible, whether an agenda can promote, or whether an
operator must be paged. That is Governor territory.

> Night Shift may use MCP for tool access, but MCP is not an authority
> layer. Tool availability is not permission. All promoted actions pass
> through Governor policy and receipt boundaries.

Every MCP call passes through an authority checkpoint:

```text
Night Shift agenda
  → reconcile context
  → choose proposed MCP tool call
  → Governor policy check
  → MCP invocation
  → result capture
  → Governor receipt
```

## Trust model

The market mostly asks people to trust the model, the vendor, the
prompt, the demo, the eval, the vibes, or the claim that "humans remain
in the loop, somewhere, spiritually."

Night Shift + Governor says: don't trust the agent. Trust the receipts,
the gates, the reconciliation step, the promotion path, and the fact
that availability is not permission.

**People trust the boundary, not the agent.**

Once people trust the boundary, the agent is replaceable. Claude today,
GPT tomorrow, local model next week. The authority plane stays put.

> Model-agnostic trust through governed execution.

Ops is the perfect trust-building domain because operators trust exactly
as far as the system proves it doesn't mutate without authority, tells
you what it saw, re-checks stale assumptions, preserves evidence,
escalates instead of improvising, fails closed, produces useful review
packets, and doesn't turn every warning into a 3am religious experience.

Trust accumulates through boring survivorship, not charisma.

## Language split

**Rust** where failure semantics matter:
- Scheduler daemon
- Run ledger
- Agenda state machine
- NQ integration
- Receipt emission
- Policy binding
- Execution leases / lockfiles / concurrency
- "Do not run this twice and set the datacenter on fire" logic

**Python** where flexibility matters:
- LLM/interferometry orchestration
- Prompt/render experiments
- Analysis plugins
- Candidate repair generation
- Report writing
- Quick adapters for weird APIs

Rust binary invokes Python workflows as controlled subprocesses.
Python can be weird without being sovereign.

## Architecture

```text
nightshiftd (Rust)
  - owns agenda state
  - owns receipts
  - owns budgets
  - owns execution leases
  - calls Governor boundary
  - invokes workflow runner

python workflow
  - reads context JSON
  - calls models/tools
  - emits proposal JSON
  - never directly mutates production
```

Integration flow:

```text
NQ / Driftwatch / Labelwatch
        |
   Night Shift daemon
        |
  load agenda + assemble context bundle
        |
  reconcile (revalidate premises)
        |
  invoke python workflow runner
        |
  agent produces proposal / diff / repair packet
        |
  Governor policy check + authorization
        |
  execute (if authorized) or hold for review
        |
  verify effect
        |
  emit receipts, store run result
```

## Build order

1. **Ops / Watchbill** — strongest Governor dogfood, simplest inference
   burden, real NQ findings from day one. This is the on-ramp.
2. **Code / Night Shift proper** — deferred diff production, higher
   complexity, needs battle-tested Governor hooks first.
3. **Publication / Atlas Runner** — best demo surface, least demanding
   governance workout. The demo, not the workout.

Ops first means by the time code mode arrives, the Governor hooks have
been stressed by something simpler and less terrifying.

## MVP: Ops observe/propose only

Minimum useful object:

```bash
nightshift watchbill run wal-bloat-review
```

It:
1. Reads NQ findings
2. Detects persistent finding pattern
3. Assembles context bundle
4. Reconciles: re-checks NQ finding state, compares current vs. captured
5. Marks inputs as current, changed, stale, or invalidated
6. Runs bounded diagnosis workflow
7. Produces repair proposal packet
8. Emits receipts
9. Refuses mutation

Example output:

```text
Finding: wal_bloat on labelwatch-host
Persistence: 4 generations
Likely regime: accumulation / pinned reader
Proposed action: inspect active readers, verify fuser state,
                 avoid restart unless pinned PID confirmed
Confidence: medium
Governor verdict: observe/propose only
Receipt: ...
```

No mutation. No sudo. No cowboy shit.

## Neighboring projects

| Project | Role | Language | Location |
|---------|------|----------|----------|
| Governor | Authority / policy / receipts | Python | `~/git/agent_gov` |
| NQ | Observatory / failure-domain classifier | Rust | `~/git/nq` |
| Grid Dependency Atlas | Publication-mode target | YAML/HTML | `~/git/grid-dependency-atlas` |
| Continuity | Cross-project context (MCP) | Python | workspace: observatory-family |
| Labelwatch | Live ops target (NQ consumer) | — | VM-hosted, in continuity |

## Key phrases

- Night Shift schedules and resumes intent. Governor authorizes force.
- The agent is the intern with astonishing confidence and no legal personhood.
- Tool availability is not permission.
- The 3am agent must not act on 11pm vibes.
- Python can be weird without being sovereign.
- Trust the boundary, not the agent.
- It reduces toil without laundering accountability.
- A good automation system must know when to become a pager, not a priest.
