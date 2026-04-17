# Night Shift — Design Document

> Deferred agent work with receipts, reconciliation, and governed promotion.

## Thesis

Night Shift schedules and resumes *intent*, not commands. It is an agenda
runner, not a cron daemon.

- A cron job says: "run this command at this time."
- Night Shift says: "resume this intention under this policy with this
  context and produce this kind of artifact."

The core claim:

> Night Shift handles the boring night-work around observation,
> reconciliation, hypothesis, and packet assembly. Governor decides
> whether anything gets force. The operator remains the source of
> authority.

Or shorter: it reduces toil without laundering accountability.

Night Shift is allowed to be useful before it is trusted with force.

## Separation of concerns

```text
Night Shift = intention queue / deferred work / recurring execution / continuity handoff
Governor    = authority / policy / authority receipts / permission boundary
NQ          = observatory / failure-domain classification / evidence
MCP         = capability discovery / tool transport (not authority)
Continuity  = optional cross-run context (not authority, not required for safety)
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

## Three ladders — distinct, not drifting

The project has three related-but-separate progressions. Mixing them
produces subtle bugs in the drywall. Keep them distinct.

### Lifecycle phases

Where a run currently is:

```text
capture → reconcile → plan → review → run → verify → record
```

- **capture**: agenda is declared; initial bundle assembled
- **reconcile**: bundle inputs re-checked against current state
- **plan**: workflow produces proposed actions
- **review**: proposed actions checked against authority/policy
- **run**: authorized actions execute
- **verify**: observed effect compared to expected effect
- **record**: run ledger events written, receipts linked

### Authority levels

What a run is permitted to do:

```text
observe → advise → stage → request → apply → publish → escalate
```

- **observe**: no action, record state
- **advise**: produce diagnosis / recommendation
- **stage**: prepare reversible command or patch
- **request**: ask operator for approval
- **apply**: execute approved action
- **publish**: expose artifact to external audience
- **escalate**: terminal / interrupt posture — standing failed

**Note on `escalate`.** Escalate is a **terminal run posture**, not
a peer action of `apply` or `publish`. A run "reaches escalate" when
drive-to-resolution fails within evidence, authority, scope, or
budget. It is the exit condition, not another verb in the ladder.

Destinations that realize an escalation (`packet_note`,
`hold_for_review`, `create_ticket`, `notify`, `request_approval`,
`page`, `block_and_record`) are **implementations**, not authority
levels. An MCP call class like `page` is the *transport* for a page
destination; it is separate from the authority posture that caused
the page. See `GAP-escalation.md` (posture + destinations) and
`GAP-mcp-authority.md` (transport).

### Artifact kinds

What a run produces:

```text
receipt | packet | diff | report | page | publication_update
```

### The load-bearing rule

> A run moves through lifecycle phases, but it cannot exceed its
> configured authority level. Artifacts are recorded through both.

`verify` is a phase. `apply` is an authority level. `receipt` is an
artifact. They are not interchangeable.

## Core primitives

### Agenda

A declared deferred intention. See `SCHEMA-agenda.md`.

```text
agenda_id: wal-bloat-review
workflow_family: ops                 # ops | code | publication — see SCHEMA-agenda.md
cadence: scheduled | event | manual
owner: operator identity
scope:
  hosts: [...]
  services: [...]
  paths: [...]                       # filesystem / publication paths
  repos: [...]
artifact_target: repair_proposal | diff | report | static_site_update
promotion_ceiling: observe | advise | stage | request | apply | publish
reconciler: required
allowed_evidence_sources: [nq, git, fs, continuity]
allowed_tool_classes: [discover, read, propose]
```

The `scope` tuple `(hosts, services, paths, repos)` is what the
reconciler canonicalizes into a scope key for concurrent-activity
checks (see `GAP-parallel-ops.md`). All four axes matter — scope
overlap is derived from the full tuple, not from hosts alone.

### Bundle (Context Bundle)

Captured context with admissibility. See `SCHEMA-bundle.md`.

Not "whatever seems useful" — a declared evidence object where every
input has standing.

Input standing categories:
- **authoritative**: verified by Governor receipt or NQ finding
- **hint**: operator-provided guidance or Continuity recall, not
  independently verified
- **stale**: was authoritative, freshness expired
- **inadmissible**: explicitly excluded from action decisions

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
"The agent acted correctly according to stale context." That is
bureaucratic negligence with better indentation.

### Packet

The reviewable output artifact. See `SCHEMA-packet.md`.

The important object is not "agent did work." It is:

> "Agent produced a reviewable artifact after deferred execution under
> declared constraints."

### Watchbill

Ops-mode roster. Scheduled operational agendas that consume NQ findings.

## Receipts: distinguish the kinds

"Receipts" becomes a magic word with several hats if left unqualified.
Three distinct kinds:

- **Run ledger events** (Night Shift): record scheduler/run lifecycle
  facts (agenda captured, bundle reconciled, workflow invoked, run
  completed). These are not authority records.
- **Authority receipts** (Governor): record permission decisions,
  policy checks, tool authorization, promotion gates. These are the
  ones that carry force.
- **Evidence receipts** (WLP-style, optional): record observed facts
  and admissibility/chain integrity, when used.

> Night Shift records run events; Governor emits authority receipts.
> A run may contain many receipts, but Night Shift does not manufacture
> authority by logging itself.

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

## Dependency classes

Night Shift distinguishes **safety dependencies** from **intelligence
dependencies**. Missing hard dependency reduces authority. Missing soft
dependency reduces quality.

### Safety dependencies (hard)

Constrain authority. When unavailable, Night Shift fails closed or
lowers the promotion ceiling.

- **Governor**: required for `stage | request | apply | publish`.
- **Evidence adapter** (NQ for ops mode; git/fs for code mode): required
  for reconciliation in that mode.
- **Run ledger**: required for all runs.

### Intelligence dependencies (soft)

Improve context quality and diagnosis. When unavailable, Night Shift
may continue with degraded output, but must mark missing inputs in the
packet.

- **Continuity**: prior decisions, project memory, operator preferences.
- **MCP**: broader tool discovery/transport.
- **LLM / interferometry**: hypothesis generation and proposal drafting.

**Invariant: missing intelligence dependencies must never increase
authority.** That last clause is load-bearing. A smarter model does not
grant it permission; a missing model does not unlock a shortcut.

### Capability degradation ladder

```text
No Governor:
  allowed: capture, reconcile, observe, advise
  blocked: stage, request, apply, publish
  receipts: local run-ledger only, explicitly non-authoritative

No Continuity:
  ordinary runs:
    allowed: all authorization-safety-preserving modes
    degraded: poorer context, fewer prior decisions, less cross-run memory
    required: use current evidence + local run ledger only
  risky classes (shared infra, topology/config/publisher/source change,
                 mode transition, protected-class scope):
    preflight cannot clear
    default: hold_for_context
    may proceed only with named, receipt-generating operator_override
    (see GAP-parallel-ops.md)

No NQ (ops mode):
  blocked unless another evidence adapter is configured
  code/publication may still run if their evidence sources reconcile

No LLM:
  allowed: deterministic packet generation from rules/templates
  degraded: no hypothesis generation, no interferometry

No MCP:
  allowed: built-in adapters only
  degraded: narrower tool surface
```

## Build / profile tiers

Night Shift ships as three tiers of completeness:

### 1. Core (standalone)

```text
Agenda + local bundle + reconciler + run ledger + packet output
```

Useful, safe, boring. Can produce review packets. Cannot mutate.

### 2. Governed

```text
Core + Governor adapter + authority receipts + promotion gates
```

This is the real product. Can `stage | request | apply | publish`
according to policy.

### 3. Constellation

```text
Governed + NQ + Continuity + MCP + observatory adapters
```

This is the ecosystem mode. Not because Core was incomplete — because
the ecosystem gives it better evidence, memory, tooling, and governance.

## Governor binding

Night Shift can **run** without Governor, but cannot **promote force**
without Governor. That makes "no Governor" a legitimate
demo/local/propose mode, not a fully-featured unsafe fork.

```text
nightshift --no-governor
```

prints, bluntly:

```text
Governor unavailable.
Promotion ceiling lowered to advise.
Mutation, publication, paging, and staged actions disabled.
```

The coupling is conceptual, not accidental:

- Night Shift creates deferred intent.
- Deferred intent is dangerous because authority drifts over time.
- Governor exists to prevent language/intent/tool-use from becoming
  unauthorized force.
- Therefore Night Shift's safety claim is incomplete without Governor.

Loosely coupled at code boundary, tightly coupled at protocol boundary.

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

The protocol boundary is specified in `GAP-governor-contract.md`.

## MCP role

MCP is capability discovery and tool transport. It tells Night Shift
what tools exist and provides a normalized way to call them.

MCP does not decide whether a tool call is allowed, whether stale
context is admissible, whether an agenda can promote, or whether an
operator must be paged. That is Governor territory.

> Night Shift may use MCP for tool access, but MCP is not an authority
> layer. Tool availability is not permission. All promoted actions pass
> through Governor policy and receipt boundaries.

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

Call-class details and examples live in `GAP-mcp-authority.md`.

The enforcement flow:

```text
Night Shift agenda
  → reconcile context
  → choose proposed MCP tool call
  → classify call
  → Governor policy check (if above local-policy threshold)
  → MCP invocation
  → result capture
  → Governor receipt
```

## Continuity role

Continuity is an optional context provider, not an authority source.

Night Shift may use Continuity to retrieve prior decisions, project
notes, operator preferences, and cross-run summaries. Continuity inputs
enter the bundle as **observed context**. They become **relied-upon
context** only after the Reconciler evaluates them against current
evidence, scope, and freshness.

> Continuity can explain why an agenda exists. It cannot prove that
> an action is allowed.

Safety must not depend on Continuity. If Continuity is unavailable,
Night Shift should still be able to reconcile current evidence, respect
promotion ceilings, emit packets, and route all promoted actions through
Governor.

> Optional context, never authority.

This preserves the constellation metaphor without making the first MVP
depend on every other piece of the empire. The empire has enough moving
parts; it does not need a ceremonial dependency kraken.

### Coordination safety ≠ authorization safety

"Safety must not depend on Continuity" refers specifically to
**authorization safety**: Governor gates force whether or not
Continuity is present. A separate concept, **coordination safety**,
*does* depend on Continuity for named risky classes — shared
infrastructure, topology / config / publisher / source change, mode
transition, or protected-class scope.

For these classes, a Continuity preflight is a gating requirement
for leaving capture phase. This does not grant authority; it prevents
silent coordination failure.

The distinction is load-bearing:

- Continuity **never grants authority.** Missing Continuity never lets
  a run do something it otherwise could not.
- Continuity **does gate coordination** for risky classes. Missing
  Continuity in a risky class holds or downgrades the run, per
  `GAP-parallel-ops.md`.

> Optional for authorization safety. Required for coordination safety
> in named classes.

### Vocabulary alignment

Continuity's native lifecycle (`observed → committed`) and Night Shift's
bundle lifecycle map 1:1. No new vocabulary needed on either side:

```text
Continuity input enters bundle as    status: observed   (reliance_class: none)
Reconciler evaluates against reality
If admissible for this run/scope      status: committed  (reliance_class: scoped)
Packet quotes / workflow relies       relied-upon under that scope
```

Important: `committed` here does **not** mean "true forever." It means
*accepted for this run, under this scope, after reconciliation*.
Continuity does not become a fossilized fact oracle.

### Recheck is the gate, not metadata

Every Continuity input passes through the Reconciler by virtue of the
pipe it enters on. There is no per-input `requires_recheck: true` flag
that can be forgotten or omitted.

> Recheck is not metadata. Recheck is the gate.

Annotating recheck at the input level is how, six months from now, one
stale record "just happens" not to require recheck. Very enterprise.
Very cursed. We do not do this.

## Three-verb alignment across the constellation

The observatory family shares a core three-verb progression:

```text
Continuity:     observe   →  commit    →  rely
Night Shift:    capture   →  reconcile →  propose-or-act
Governor:       request   →  authorize →  execute
```

These are not the same verbs, but they align:

- **observe / capture / request**: something has entered the system
  without yet being trusted.
- **commit / reconcile / authorize**: the system has checked premises,
  scope, and policy, and accepts the thing for a bounded purpose.
- **rely / propose-or-act / execute**: action is taken under that
  acceptance.

`commit` in this constellation means "a premise has survived
reconciliation for a declared scope." It does **not** mean code commit,
permanent truth, or authorization. Define it carefully, or a future
reader (possibly you, 1:13 AM edition) will confuse it with git.

## Diagnostic review modes

Diagnosis quality does not need a symposium for every run. Three modes,
agenda-declared:

```text
singleton    one model/workflow produces the packet
self_check   same model, second pass constrained to critique (not rewrite)
conference   multiple independent reviews → disagreement extraction
```

**Default for Watchbill MVP: `self_check` with promotion ceiling
`advise`.** Cheap, fast, useful. Second pass is the sweet spot.
Singleton is for low-severity routine summaries; conference is for high
severity, mutating proposals, recurrent findings, or evidence conflict.

Self-check produces structured objections, not a rewrite:

```yaml
unsafe_assumptions: []
stale_context_risks: []
promotion_overreach: []
missing_verification: []
recommended_downgrade: null
```

Conference is not majority-rule voting. It is **disagreement
extraction** — the output is an agreement/disagreement/operator-question
decomposition, not a tallied truth.

### Invariant

> Diagnostic review can reduce confidence, downgrade promotion, or
> require escalation. It cannot raise the promotion ceiling or
> authorize force.

The robots may harmonize beautifully. They still do not get keys.

Conference triggers (default; overridable per agenda):

```text
severity_at_least: critical
promotion_at_least: stage
confidence_below: 0.65
recurrence_after_repair: true
evidence_conflict: true
```

## Drive to resolution: escalation gating

Drive-to-resolution is where helpful agent becomes tiny incident
commander with boundary issues. Needs gating early. Full specification
in `GAP-escalation.md`.

### Core rule

Night Shift pursues resolution only while the next step remains within
**evidence, authority, scope, and budget**. Once the next useful step
crosses any of those boundaries, it stops being automation and becomes
escalation.

> Drive to resolution ends where standing begins.

### Secondary rule

> If the next diagnostic step changes the system, stop.

Read-only disambiguation is fine. Mutation as disambiguation is not.
The moment the cheapest next useful check requires touching production,
the run exits toward the operator.

### Escalation is not a single blob

Escalation is typed — *why* matters:

```text
authority_escalation      ceiling reached
context_escalation        need human-in-the-loop knowledge
risk_escalation           blast radius / reversibility threshold
evidence_escalation       conflicting or invalidated premises
recurrence_escalation     same finding returns after remediation
budget_escalation         time/token/tool caps hit
incident_escalation       live probe suggests active incident
```

"Page me now" and "ask me in the morning" are not the same creature.
Severity × urgency drives destination (packet note, queued review,
ticket, notification, page).

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

### Python workflow boundary (invariant)

Python workflows do not receive production credentials, mutable tool
handles, or unrestricted shell access. They:

- Read context JSON
- Call models / advisory tools (no-authority)
- Emit proposal JSON
- Never directly mutate production
- Never originate Governor authority receipts

Night Shift and Governor decide whether any proposed operation is
staged, applied, or published.

Python can be weird without being sovereign. "Never directly mutates"
must be an enforced invariant, not a gentleman's agreement with a
subprocess. Gentlemen's agreements with subprocesses are how daemons
learn knife tricks.

## Architecture

```text
nightshiftd (Rust)
  - owns agenda state
  - owns run ledger (lifecycle events)
  - owns budgets
  - owns execution leases
  - calls Governor adapter for authority decisions
  - invokes python workflow runner
  - integrates with NQ (pull + optional notify)

python workflow
  - reads context JSON
  - calls models/tools via classified MCP calls
  - emits proposal JSON
  - never directly mutates production
```

Integration flow:

```text
NQ / Driftwatch / Labelwatch
        |
   Night Shift daemon
        |
  load agenda + assemble context bundle   [capture]
        |
  reconcile (revalidate premises)         [reconcile]
        |
  invoke python workflow runner           [plan]
        |
  agent produces proposal / diff / packet
        |
  Governor policy check + authorization    [review]
        |
  execute (if authorized) or hold for op   [run]
        |
  verify effect                           [verify]
        |
  emit run ledger + authority receipts    [record]
```

## Storage backend

SQLite default for v1 (local / single-operator). Postgres for v2
(shared control plane). Full stance, contract, and `Store` trait in
`GAP-storage.md`.

Two invariants:

> Backend choice must not change authority semantics.
> Scaling the store must not scale the trust assumptions.

A run transition must be atomic and exclusive. If the backend cannot
prove exclusive ownership of a run, Night Shift fails closed.

## Build order

1. **Ops / Watchbill** — strongest Governor dogfood, simplest inference
   burden, real NQ findings from day one. This is the on-ramp.
2. **Code mode** — deferred diff production, higher complexity, needs
   battle-tested Governor hooks first.
3. **Publication / Atlas Runner** — best demo surface, least demanding
   governance workout. The demo, not the workout.

Ops first means by the time code mode arrives, the Governor hooks have
been stressed by something simpler and less terrifying.

## MVP: Ops observe/advise only

Minimum useful object:

```bash
nightshift watchbill run wal-bloat-review
```

It:
1. Reads NQ findings
2. Detects persistent finding pattern
3. Assembles context bundle (capture)
4. Reconciles: re-checks NQ finding state, compares current vs.
   captured; marks inputs current/changed/stale/invalidated
5. Runs bounded diagnosis workflow (plan)
6. Produces repair proposal packet
7. Emits run-ledger events and (if Governor present) authority receipts
8. Refuses mutation

Example output:

```text
Finding: wal_bloat on labelwatch-host
Persistence: 4 generations
Likely regime: accumulation / pinned reader
Proposed action: inspect active readers, verify fuser state,
                 avoid restart unless pinned PID confirmed
Confidence: medium
Authority level: advise (no stage requested)
Governor verdict: n/a (advise-only)
Run ledger: ledger://nightshift/runs/...
```

No mutation. No sudo. No cowboy shit.

## Neighboring projects

| Project | Role | Language | Location |
|---------|------|----------|----------|
| Governor | Authority / policy / authority receipts | Python | `~/git/agent_gov` |
| NQ | Observatory / failure-domain classifier | Rust | `~/git/nq` |
| Grid Dependency Atlas | Publication-mode target | YAML/HTML | `~/git/grid-dependency-atlas` |
| Continuity | Optional cross-project context (MCP) | Python | workspace: observatory-family |
| Labelwatch | Live ops target (NQ consumer) | — | VM-hosted, in Continuity |

## Key phrases

- Night Shift schedules and resumes intent. Governor authorizes force.
- The agent is the intern with astonishing confidence and no legal personhood.
- Tool availability is not permission.
- The 3am agent must not act on 11pm vibes.
- Python can be weird without being sovereign.
- Trust the boundary, not the agent.
- It reduces toil without laundering accountability.
- A good automation system must know when to become a pager, not a priest.
- Night Shift is allowed to be useful before it is trusted with force.
- Optional context, never authority.
