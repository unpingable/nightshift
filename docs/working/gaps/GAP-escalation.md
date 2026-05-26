# GAP: Escalation Policy

> Status: identified, partially specified. Drive-to-resolution gating
> must be designed in, not bolted on later.

## Core rule

Night Shift pursues resolution only while the next step remains within:

1. **Evidence** — required inputs are current, admissible, and consistent.
2. **Authority** — the next action is within the agenda's promotion ceiling.
3. **Scope** — blast radius remains within declared bounds.
4. **Budget** — time, tokens, tool calls, mutation caps.

Once the next useful step crosses any of those, the run stops pursuing
autonomously and produces an escalation artifact.

> Drive to resolution ends where standing begins.

## Secondary rule

> If the next diagnostic step changes the system, stop.

Read-only disambiguation is fine. Mutation as disambiguation is not.
Otherwise diagnosis quietly becomes action.

## Escalation triggers

### 1. Authority ceiling reached

Next useful step exceeds agenda's `promotion_ceiling`:

```text
proposal: restart service
ceiling: advise
→ request_approval (or escalate, depending on severity)
```

No "but it's probably fine." Ceiling is ceiling unless Governor /
operator changes it.

### 2. Confidence cannot improve without mutation

Possible causes cannot be discriminated using read-only checks. The
next discriminating step would change system state.

→ escalate.

### 3. Evidence conflict

Sources disagree materially (NQ finding active, live probe says absent,
recent repair receipt, incomplete generation). Reconciliation cannot
produce a coherent admissible set.

- Low severity → downgrade to observe + packet note
- High severity → escalate with conflict packet

### 4. Staleness / invalidation

Required inputs are stale or invalidated and cannot be refreshed:
- host unreachable
- repo HEAD changed (for code mode)
- policy hash changed
- finding absent for N generations
- source generation incomplete

→ stop relying on captured premise. Escalate or downgrade by severity.

This is where "3am agent must not act on 11pm vibes" becomes
operational, not decorative.

### 5. Recurrence after repair

Same finding returns within the recurrence window after an approved
remediation.

→ escalate. Our model of the problem is probably wrong.

**Exception**: if NQ reports status `resolving`, the finding is still
part of the story but trajectory has turned. That is not recurrence —
that is convalescence. Do not trigger recurrence escalation on
`resolving`. See `stall` trigger below.

### 5b. Resolving stalled

NQ status has been `resolving` for more than N generations without
further improvement toward `clear`, and severity remains non-trivial.

→ escalate with type `evidence` (trajectory stopped turning).

This avoids the "greenwashing" failure mode: treating `resolving` as
clear. It also avoids the "panic automation" failure mode: treating
`resolving` as a new incident.

### 6. Repeated low-confidence loop

Self-check / conference keeps producing the same blocked assumption,
confidence stays below threshold, no new evidence source available.

→ escalate. Otherwise: agent dithering, which is basically systemd with
a philosophy minor.

### 7. Blast radius threshold

Proposed action touches more than a declared limit:
- more than N hosts
- production-tagged service
- shared datastore
- identity / auth / secrets
- public publication
- irreversible deletion
- network-wide config
- anything with "global" in the name

→ escalate / request approval.

### 8. Budget exhaustion

Time, tokens, tool calls, or mutation budget reached before the packet
meets confidence threshold.

→ emit partial packet + escalate or downgrade. Budget exhaustion must
not silently convert into lower-quality action.

## Escalation types

Escalation is typed. "Page me now" and "ask me in the morning" are
different creatures.

```text
authority_escalation      ceiling reached
context_escalation        needs human-in-the-loop knowledge
risk_escalation           blast radius / reversibility threshold
evidence_escalation       conflicting or invalidated premises
recurrence_escalation     same finding returns after remediation
budget_escalation         time/token/tool caps hit
incident_escalation       live probe suggests active incident
```

## Destinations (severity × urgency)

Escalation is a **run posture**, not a peer action of `apply` or
`publish`. A run reaches escalate when standing fails. Destinations
below are **implementations** that realize an escalation outcome;
they are not authority levels. `page` as an MCP call class
(`GAP-mcp-authority.md`) is the *transport* for a page destination —
distinct from the escalation posture itself.

HITL escalation does not always mean "wake human." It means **human
standing required**. Destinations vary:

```text
packet_note          no notification; operator reads at leisure
hold_for_review      queued for scheduled review
create_ticket        external tracker item
notify               advisory message, no page
request_approval     UI/email/slack prompt, non-paging
page                 real pager-grade interrupt
block_and_record     refuse to proceed; record reason
```

Severity × urgency mapping (default; agenda-overridable):

```text
low severity + blocked context         → packet_note
warning + authority needed             → request_approval / hold_for_review
critical + evidence current + blocked  → page
critical + evidence conflict           → page with conflict packet
recovered before run                   → downgrade / close packet
```

## Agenda escalation policy (schema)

```yaml
escalation:
  on_authority_ceiling: request_approval
  on_confidence_threshold:
    below: 0.5
    action: hold_for_review
  on_evidence_conflict: hold_for_review
  on_staleness: downgrade_to_observe
  on_recurrence_after_repair: page
  on_blast_radius_exceeded: block_and_record
  on_budget_exhaustion: emit_partial_escalate
  on_critical_blocked_action: page

  quiet_hours:
    window: "22:00-06:00 America/Los_Angeles"
    low: hold_for_review
    warning: notify
    critical: page

  recurrence_window_minutes: 180
  blast_radius_limits:
    hosts: 1
    irreversible: block_and_record
    public_publication: request_approval
```

This keeps "drive to resolution" from being hardcoded into the agent.
The agenda declares what kind of human interruption is acceptable.

## State machine: classify_next_step

Every loop iteration asks:

> What is the next discriminating action, and is it allowed?

```text
capture
  ↓
reconcile
  ↓
diagnose
  ↓
self_check / conference
  ↓
classify_next_step
  ├─ safe_read_available        → continue
  ├─ enough_for_packet          → emit advise packet
  ├─ needs_authority            → request_approval / escalate
  ├─ needs_human_context        → context_escalation
  ├─ evidence_invalid           → observe / downgrade
  ├─ blast_radius_exceeded      → risk_escalation
  ├─ budget_exhausted           → budget_escalation + partial packet
  └─ unsafe_or_unknown          → escalate
```

The `classify_next_step` gate prevents drive-to-resolution from quietly
becoming drive-to-mutation.

## Escalation artifact

An escalation emits:

- A packet marked with `escalation_type` and `trigger_reason`
- Current reconciled evidence summary
- What was tried
- What stopped progress
- What would be needed to resume (authority / context / evidence)
- Run-ledger event of kind `run.escalated`
- Governor receipt (if present) with role `escalation`

If destination is `page`, a page is issued *after* the receipt lands,
not instead of it. No silent pages.

## Invariants

- Escalation is a **terminal or interrupt state**, not a stage the run
  passes through to reach apply.
- An escalated run does not self-resume. Resumption requires a new
  operator action (approval, edited agenda, or manual re-run).
- The agenda's `escalation` policy is captured into the bundle; changes
  to the policy after capture do not affect the in-flight run.

## Open questions

- Does `page` integrate with existing paging systems (PagerDuty,
  Opsgenie)? (Via MCP call class `page`, yes — see `GAP-mcp-authority.md`.)
- How are quiet hours reconciled with critical-severity findings?
  (Severity override in agenda; critical bypasses quiet hours by
  default.)
- Can an operator lift a `block_and_record` without re-running? (Probably
  yes — via an operator decision captured as a new authority receipt
  via Governor.)
- How does the `classify_next_step` gate consume self-check / conference
  output? (Structured objections with enum tags map directly onto
  triggers; exact wire format TBD.)
