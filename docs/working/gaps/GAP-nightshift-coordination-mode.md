# GAP: Night Shift Coordination Mode

Status: proposed
Owner: TBD
Created: 2026-04-24
Revised: 2026-04-24 (post-Island-Discipline topology correction)
Applies to: Night Shift, Continuity, cross-agent project workflows
Related: deferred-run split, continuity reliance classes, agent handoff discipline
Depends on (Continuity-side doctrine): `ISLAND_DISCIPLINE.md`, `CROSS_SCOPE_REFERENCE_GAP`

## Summary

Night Shift needs a coordination mode that can sweep Continuity and project-local state for stale, unresolved, or deferred work, then produce bounded status prompts for the relevant operator or agent.

This is not project management authority. It is anti-amnesia infrastructure.

Coordination mode may notice, summarize, and ask.
It may not decide, close, authorize, or silently promote state.

## Problem

The constellation now spans multiple repos, agents, and continuity surfaces. Work can be validly deferred, blocked, partially landed, or waiting for another project. Without a recurring coordination pass, unresolved items decay into oral tradition.

Existing mechanisms capture pieces:

- Continuity preserves observed / committed / relied-on context.
- Night Shift coordinates work across time.
- Governor gates admissible transitions.
- Project repos contain GAP docs, TODOs, receipts, and local state.
- Agents can report project-specific status when prompted.

But there is no bounded mechanism for asking:

> What open work exists, what has gone stale, who should be asked, and what must not be inferred?

The result is distributed context decay.

## Motivation

Night Shift exists to prevent handled-in-the-moment work from becoming forgotten-in-the-system work.

Coordination mode extends that role from scheduled operational tasks to cross-project continuity hygiene.

The target failure mode is not “missed task.”
The target failure mode is stale advisory state being mistaken for closure, or unresolved work disappearing because no active session is holding it.

## Non-Goals

Coordination mode is not:

- a project manager
- a planner with authority
- a Jira replacement
- a Governor bypass
- a closure engine
- a system that decides blocked work is resolved
- a mechanism for promoting advisory memory into authoritative state
- a system that nags humans without preserving why it is nagging

If coordination mode cannot determine status, it must emit uncertainty, not closure.

## Core Principle

Coordination mode may nag.
It may not govern.

More formally:

> Coordination mode can request status about stale or unresolved work, but cannot confer authority, close obligations, or promote reliance class.

## Topology: domain-aware by construction

Per Continuity's `ISLAND_DISCIPLINE.md` (landed 2026-04-24, advisory / global):

> scope=global is global only within its declared domain. That is the anti-empire bolt.

Coordination mode is a **consumer** of Island Discipline, not an alternate topology system. Consequences:

- A sweep must take a declared continuity domain. `--scope global` alone is underspecified: `global` inside `observatory-family` is a different set of memories than `global` inside `book`.
- Night Shift may coordinate **across** declared domains but MUST NOT merge, promote, import, or infer equivalence. A stale item in domain A and a similarly-titled item in domain B are two items, not one, until Continuity's bridge machinery says otherwise.
- Cross-domain references ride the `CROSS_SCOPE_REFERENCE_GAP` hash-pinning primitives that Continuity provides. Night Shift does not invent its own `source_hash` scheme, and it does not construct bridge imports. At most, it emits a coordination item of kind `candidate_bridge_review` and hands it to an operator or to Continuity.
- Firewall-class domains (`domain_purpose: firewall`) are visible to coordination mode only through declared exports. A sweep that touches a firewall domain may emit bridge candidates only as `denied` or `requires_operator_review`; it never imports.

These are topology invariants, not preferences. They constrain the CLI, the digest schema, and the acceptance tests below.

## Scope

Initial scope:

- read Continuity entries within declared scopes
- identify stale advisory / observed / deferred items
- read project-local handoff or GAP metadata where available
- produce a coordination digest
- generate suggested prompts for relevant agents/operators
- optionally record the digest back into Continuity as observed coordination state

Out of scope for MVP:

- automatic issue creation
- automatic closure
- direct repo mutation
- direct task reassignment
- cross-agent command execution
- autonomous status interpretation beyond declared evidence

## Terminology

### Coordination Item

A remembered unit of unresolved or deferred work that may require review.

Examples:

- open GAP doc
- deferred decision
- blocked implementation
- stale advisory memory
- unpromoted observation
- pending supersession
- project_state entry older than threshold
- known “next natural work” not touched since prerequisite landed

### Status Prompt

A bounded question emitted for an operator or agent.

A status prompt asks for current state.
It does not assert current state unless evidence supports it.

### Coordination Digest

A summary of discovered coordination items, grouped by project/scope/status, with evidence and suggested next prompts.

### Staleness

A coordination item is stale when its last observed or committed touch exceeds a configured threshold, or when a known prerequisite has changed and no follow-up state exists.

Staleness is not failure.
Staleness is a reason to ask.

## Inputs

MVP inputs:

- Continuity records
  - scope
  - kind
  - reliance_class
  - basis
  - created_at / committed_at / superseded_at
  - linked project/repo if available
  - note/body/title
- project-local state
  - GAP docs
  - project_state files
  - receipts/run ledgers where available
  - known handoff docs
- optional configuration
  - stale-after threshold
  - included scopes
  - excluded scopes
  - project mapping
  - agent routing hints

## Outputs

### Human-readable digest

Example:

```text
Coordination digest: 2026-04-24

Governor:
- deferred-run split appears eligible after liveness reader landed.
  Evidence: GAP-deferred-run-split.md, last project_state touch 2026-04-20.
  Suggested prompt: "What is current state of deferred-run split?"

NQ:
- activity-bias work remains flagged but unadvanced.
  Evidence: project_state 2026-04-21.
  Suggested prompt: "Is activity-bias still the highest-value next detector work?"

Constellation:
- thesis snapshot discussed; verify whether committed/promoted in Continuity.
  Suggested prompt: "Has constellation thesis snapshot been committed advisory/global?"
```

### Machine-readable JSON

Shape TBD, but must include the declared continuity domain at both digest and item level. Same-domain items and cross-domain candidates have distinct shapes: same-domain items carry `continuity_domain_id` + scope + memory pointers; cross-domain candidates carry both sides and an explicit forbidden-actions list that blocks merge / promote / infer-equivalence.

**Same-domain coordination item:**

```json
{
  "schema_version": "nightshift.coordination_digest.v0",
  "generated_at": "...",
  "mode": "coordination",
  "continuity_domain_id": "observatory-family",
  "items": [
    {
      "continuity_domain_id": "observatory-family",
      "domain_purpose": "bridgeable",
      "source_scope": "global",
      "memory_id": "mem_...",
      "project": "agent_governor",
      "title": "Deferred-run split status unknown",
      "status": "needs_status",
      "evidence_refs": [],
      "source_reliance_class": "advisory",
      "last_seen_at": "...",
      "stale_reason": "prerequisite_landed_no_followup",
      "suggested_prompt": "What is current state of deferred-run split?",
      "allowed_actions": ["ask_status", "record_observation"],
      "forbidden_actions": ["close", "authorize", "promote"]
    }
  ]
}
```

**Cross-domain candidate (bridge-review only):**

```json
{
  "source_domain_id": "book",
  "source_scope": "global",
  "source_memory_id": "mem_...",
  "source_content_hash": "sha256:...",
  "target_domain_id": "observatory-family",
  "status": "candidate_bridge_review",
  "suggested_prompt": "Operator: is a Continuity bridge export from `book:global` into `observatory-family` warranted here?",
  "allowed_actions": ["ask_status", "recommend_bridge_review"],
  "forbidden_actions": ["merge", "promote", "import", "infer_equivalence"]
}
```

If the source-side content hash is not available (firewall domain, missing CROSS_SCOPE_REFERENCE_GAP record), the candidate carries an explicit `evidence_absent: "source_content_hash"` marker instead of fabricating one.

## State Semantics

Coordination mode may emit the following statuses:

* `needs_status`
* `stale_observation`
* `blocked_known`
* `waiting_on_operator`
* `waiting_on_agent`
* `candidate_supersession`
* `candidate_cleanup`
* `no_action_recommended`

It must not emit:

* `resolved`
* `authorized`
* `approved`
* `complete`

unless those states are explicitly supported by authoritative project evidence.

## Reliance Semantics

Coordination digests are advisory unless explicitly committed otherwise.

Default writeback behavior:

* raw scan result: `observed`
* digest summary: `observed` or `advisory`
* follow-up prompt: `observed`
* agent response: `observed`
* operator-confirmed status: candidate for `advisory` or stronger, depending on project rules

Coordination mode must not promote reliance class on its own.

## Authority Boundaries

Coordination mode must not:

* authorize repo changes
* modify project files
* close GAPs
* mark decisions ratified
* infer project completion from absence of evidence
* treat agent silence as agreement
* treat “no recent changes” as “done”
* convert an observed stale item into a committed fact

Governor remains responsible for admissibility.
Continuity remains responsible for reliance state.
Project repos remain responsible for local doctrine and closure rules.

## CLI Sketch

Every sweep takes an explicit continuity domain. `--scope` names the scope *within* that domain. Omitting `--continuity-domain` is an error (or a warning in MVP) — never silently defaulted.

```bash
nightshift coordination sweep \
  --continuity-domain observatory-family \
  --scope global \
  --stale-after 7d \
  --format text

nightshift coordination sweep \
  --continuity-domain observatory-family \
  --scope global \
  --project governor \
  --format json

nightshift coordination prompt \
  --digest <digest_id> \
  --project governor

nightshift coordination record \
  --digest <digest_id> \
  --continuity-domain observatory-family \
  --continuity-scope global \
  --reliance-class observed
```

Possibly later:

```bash
nightshift coordination ask-agent \
  --project governor \
  --agent claude \
  --prompt-id <id>
```

Not MVP unless agent routing is already explicit and safe.

Cross-domain candidate review is a distinct verb, not a flag on `sweep`:

```bash
# Hypothetical — surfaces bridge-review candidates only; does not
# import, merge, or promote. Requires an operator-side action via
# Continuity's bridge machinery to actually establish a bridge.
nightshift coordination bridge-candidates \
  --continuity-domain observatory-family \
  --since 7d
```

## Configuration Sketch

```yaml
coordination:
  stale_after: "7d"

  scopes:
    include:
      - global
      - project:agent_governor
      - project:notquery
      - project:continuity
      - project:nightshift
    exclude: []

  projects:
    agent_governor:
      repo: "~/git/agent_governor"
      handoff_files:
        - "PROJECT_STATE.md"
        - "docs/GAP_BUILD_ORDER.md"
        - "docs/gaps/*.md"
      agent_hint: "governor_claude"

    notquery:
      repo: "~/git/notquery"
      handoff_files:
        - "PROJECT_STATE.md"
        - "docs/gaps/*.md"
      agent_hint: "nq_claude"
```

## Invariants

1. **Ask, don’t decide.**
   Coordination mode can request status but cannot settle status.

2. **Evidence before prompt.**
   Every suggested prompt must cite why it exists.

3. **No silent closure.**
   Missing evidence is never interpreted as resolved work.

4. **No sovereignty.**
   Night Shift coordination mode does not become the meta-authority over the constellation.

5. **No status laundering.**
   Agent replies remain observed until committed by the appropriate project/continuity flow.

6. **No escalation by fluency.**
   A polished digest has no more authority than its evidence permits.

7. **Stale means ask.**
   Staleness is a coordination signal, not a verdict.

8. **Domain-aware by construction.**
   A sweep is always scoped to a declared continuity domain. `scope=global` is meaningful only inside a declared domain. Undeclared domain is a topology fault, not a convenience default.

9. **No cross-domain authority.**
   Night Shift may surface cross-domain items as bridge-review candidates, but MUST NOT merge, promote, import, or infer equivalence between memories in different domains. Bridge construction is Continuity's responsibility via `CROSS_SCOPE_REFERENCE_GAP`.

10. **No parallel hash-pinning.**
    If coordination mode needs to cite a source-side artifact across domains, it uses the content hash Continuity already provides. It does not invent its own.

## MVP Behavior

Given a Continuity scope and optional repo config, coordination mode should:

1. load relevant Continuity records
2. identify unresolved/deferred/stale items
3. inspect configured project-local handoff files if present
4. group items by project/scope
5. attach evidence references
6. classify item status
7. emit suggested status prompts
8. produce text and JSON digest
9. optionally record digest as observed Continuity state

## MVP Exclusions

MVP should not:

* contact agents automatically
* mutate repos
* create tickets
* infer closure
* perform Governor checks
* execute project commands
* write authoritative Continuity commits

## Acceptance Tests

### Detect stale advisory item

Given an advisory Continuity entry older than `stale_after` with no supersession, coordination mode emits `stale_observation` or `needs_status`.

### Do not close stale item

Given a stale item with no recent evidence, coordination mode must not emit `resolved`.

### Preserve evidence

Every emitted coordination item includes at least one evidence reference or an explicit `evidence_absent` marker.

### Respect supersession

Given a superseded Continuity entry, coordination mode does not nag on the old item unless the superseding entry is itself stale or unresolved.

### Respect exclusion config

Given an excluded scope/project, coordination mode does not scan or emit items for it.

### No authority escalation

Given an observed agent response claiming work is done, coordination mode records it as observed status only unless authoritative project evidence is present.

### JSON stability

Given identical inputs, coordination mode emits deterministic JSON ordering and stable item identifiers.

### Domain-aware sweep rejects undeclared domain

Given a `sweep` invocation without `--continuity-domain`, coordination mode errors (MVP may warn) rather than silently defaulting. Declared domain is a topology requirement, not a UX affordance.

### Domain isolation: A-global is not visible in B

Given a memory at `(domain=A, scope=global)` and a sweep of `(domain=B, scope=global)`, coordination mode does NOT include the A-memory in B's digest unless a Continuity-side bridge / `CROSS_SCOPE_REFERENCE_GAP` record exists for it. Similar titles across domains are not equivalence.

### Firewall domain surfaces as bridge-candidate only

Given a source domain with `domain_purpose: firewall`, a cross-domain sweep emits any derived candidate as `status: candidate_bridge_review` with `allowed_actions` restricted to `ask_status` / `recommend_bridge_review`. Coordination mode never imports from a firewall domain.

### No equivalence inference across domains

Given two memories with identical or near-identical titles in different domains, coordination mode emits them as two distinct items (or as a bridge-review candidate pair), never as one merged item.

### Cross-domain evidence pins provenance or marks absence

Every cross-domain coordination item includes `source_domain_id`, `source_memory_id`, and either a source-side content hash or an explicit `evidence_absent` marker. Fabricated or inferred hashes are forbidden.

## Open Questions

1. Should coordination digests get their own receipt type, or remain ordinary Continuity observations?
2. What is the minimum project-local metadata needed to avoid brittle grep archaeology?
3. Should stale thresholds be per-project, per-kind, or global for MVP?
4. How should agent routing be declared without creating a hidden authority map?
5. Does this belong under Night Shift proper, or as a Night Shift mode backed by Continuity queries?
6. Should suggested prompts be stored as artifacts, or generated ephemerally?
7. How does coordination mode distinguish “deferred intentionally” from “forgotten” without overfitting?
8. What is the first dogfood target: Governor deferred-run split, NQ activity-bias work, or constellation thesis artifact?

## First Slice

The smallest useful version:

```bash
nightshift coordination sweep \
  --continuity-domain observatory-family \
  --scope global \
  --stale-after 7d
```

Reads Continuity only.
Emits a text digest and JSON digest.
Confined to one declared domain — cross-domain candidates are out of scope for the first slice.
No repo scanning.
No agent contact.
No writeback by default.

That proves the core semantic rule:

> Night Shift can notice stale remembered work inside a declared continuity domain without becoming the authority that resolves it, and without pretending domains don't exist.

## Later Slices

### Repo-aware sweep

Add configured repo-local handoff files and GAP docs.

### Prompt generation

Emit bounded agent/operator status prompts.

### Continuity writeback

Record digest as observed coordination state.

### Agent-mediated status request

Ask configured agents for status, preserving replies as observed evidence.

### Governor-gated actions

Only after the coordination digest proposes an action should Governor enter the path.

## Risks

### PM cosplay

Coordination mode becomes a taskmaster instead of anti-amnesia infrastructure.

Mitigation: no closure, no assignment, no authorization, no progress scoring.

### Sovereignty creep

Night Shift becomes the meta-controller of the constellation.

Mitigation: coordination mode emits prompts and observations only.

### Nag fatigue

Too many stale items generate noise.

Mitigation: grouping, suppressions, thresholds, supersession awareness.

### Status laundering

Agent-generated updates get treated as committed state.

Mitigation: preserve reliance class; require explicit commit/promotion outside coordination mode.

### Oral tradition fossilization

Coordination mode encodes project assumptions not present in evidence.

Mitigation: require evidence refs or explicit uncertainty markers.

## Design Note

Calling this “PM mode” is funny but probably wrong.

The durable name should emphasize coordination and anti-amnesia, not management authority.

Candidate names:

* coordination mode
* steward mode
* anti-amnesia mode
* continuity sweep mode

Recommendation: `coordination mode`.

It sounds boring enough to survive contact with reality.

## Revision note (2026-04-24)

This GAP was first drafted and committed earlier today, without awareness that Continuity had committed `ISLAND_DISCIPLINE.md` (advisory / global, ~50 minutes prior) naming declared-domain topology and the anti-empire bolt on `scope=global`. The first draft's CLI sketch used bare `--scope global`, which is topologically underspecified: `global` inside `observatory-family` is not the same set as `global` inside `book`.

The revision:

- Adds Island Discipline and `CROSS_SCOPE_REFERENCE_GAP` as explicit doctrinal dependencies.
- Names coordination mode as a consumer of Island Discipline, not an alternate topology system.
- Makes every sweep take `--continuity-domain`; drops the silent-global-default.
- Splits the digest item shape into same-domain vs cross-domain variants, with explicit `forbidden_actions` on the cross-domain side (no merge/promote/import/infer-equivalence).
- Adds Invariants 8–10 (domain-aware, no cross-domain authority, no parallel hash-pinning).
- Adds acceptance tests for undeclared-domain rejection, A-to-B isolation, firewall bridge-candidate surfacing, no equivalence inference, and cross-domain evidence pinning.

The meta-read is that coordination mode's motivating failure mode fired on its own GAP in the ~4 hours between Island Discipline committing and this GAP committing. That is instrumentation, not embarrassment: the gap between "doctrine landed" and "downstream work was aware of it" is exactly what coordination mode is meant to shrink.
