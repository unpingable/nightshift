# GAP: Night Shift Coordination Mode

Status: proposed
Owner: TBD
Created: 2026-04-24
Applies to: Night Shift, Continuity, cross-agent project workflows
Related: deferred-run split, continuity reliance classes, agent handoff discipline

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

Shape TBD, but must include:

```json
{
  "schema_version": "nightshift.coordination_digest.v0",
  "generated_at": "...",
  "mode": "coordination",
  "items": [
    {
      "project": "agent_governor",
      "scope": "global",
      "title": "Deferred-run split status unknown",
      "status": "needs_status",
      "evidence_refs": [],
      "last_seen_at": "...",
      "stale_reason": "prerequisite_landed_no_followup",
      "suggested_prompt": "What is current state of deferred-run split?",
      "allowed_actions": ["ask_status", "record_observation"],
      "forbidden_actions": ["close", "authorize", "promote"]
    }
  ]
}
```

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

```bash
nightshift coordination sweep \
  --scope global \
  --stale-after 7d \
  --format text

nightshift coordination sweep \
  --scope global \
  --project governor \
  --format json

nightshift coordination prompt \
  --digest <digest_id> \
  --project governor

nightshift coordination record \
  --digest <digest_id> \
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
nightshift coordination sweep --scope global --stale-after 7d
```

Reads Continuity only.
Emits a text digest and JSON digest.
No repo scanning.
No agent contact.
No writeback by default.

That proves the core semantic rule:

> Night Shift can notice stale remembered work without becoming the authority that resolves it.

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
