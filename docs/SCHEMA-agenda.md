# SCHEMA: Agenda (v0)

> Status: draft. Not frozen. Contract will tighten before first run.

An **agenda** is a declared deferred intention. It specifies what will
run, when, under what authority, with what context sources, and what
artifact it is supposed to produce.

## Shape

```yaml
agenda_version: 0
agenda_id: wal-bloat-review            # stable slug, human-readable
mode: ops                              # ops | code | publication
owner: operator@local                  # identity for audit
cadence:
  kind: scheduled | event | manual
  expr: "0 3 * * *"                    # cron, for scheduled
  triggers:                            # for event-driven
    - source: nq
      filter: { detector: wal_bloat, min_persistence: 3 }
scope:
  hosts: [labelwatch-host]
  repos: []
  services: []
  paths: []                            # for code mode
artifact_target: repair_proposal       # packet | diff | report | publication_update
promotion_ceiling: advise              # max authority level this agenda may reach
reconciler:
  required: true
  invalidates_if:
    - finding_absent_for_n_generations: 1
    - host_unreachable
    - policy_hash_changed
allowed_evidence_sources:
  - nq
  - git
  - fs
  - continuity                         # optional, never authoritative
allowed_tool_classes:
  - discover
  - read
  - propose
budget:
  max_wall_seconds: 600
  max_tokens: 100000
  max_mcp_calls: 50
governor_binding:
  required_above: observe              # governor required if promotion_ceiling > this
  policy_id: nightshift.ops.propose_only

diagnosis:
  mode: self_check                     # singleton | self_check | conference
  conference_triggers:                 # when to upgrade from declared mode
    severity_at_least: critical
    promotion_at_least: stage
    confidence_below: 0.65
    recurrence_after_repair: true
    evidence_conflict: true

criticality:
  class: standard                       # standard | business_critical | safety
  re_alert_after: "4h"                  # ack expiry — finding re-surfaces after
  ack_due_by: "24h"                     # unowned findings must be acked within
  handoff_required: false               # true → leaving attention state requires named transfer
  business_hours_okay: true             # false → urgency ignores quiet hours
  silence_max_duration: "72h"           # hard cap on any single silence window

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
    critical: page                      # severity override
  recurrence_window_minutes: 180
  blast_radius_limits:
    hosts: 1
    irreversible: block_and_record
    public_publication: request_approval
```

## Field rules

- **agenda_id** — stable slug. Used to dedupe runs and key the run ledger.
- **mode** — ops | code | publication. Determines which workflow family
  runs and which Governor policy applies.
- **cadence.kind = scheduled** — requires `expr` (cron).
- **cadence.kind = event** — requires one or more `triggers`.
- **cadence.kind = manual** — no automatic activation; run only on
  operator invocation.
- **promotion_ceiling** — hard cap. A run may not exceed this authority
  level even if policy would otherwise permit it.
- **reconciler.required = true** — mandatory for any promotion above
  `observe`. Cannot be disabled for advise+ agendas.
- **allowed_evidence_sources** — enumerated sources. Anything else in the
  captured bundle is inadmissible.
- **allowed_tool_classes** — MCP call classes this agenda may attempt.
  `stage | mutate | publish | page` require `governor_binding.required_above`
  to be set at or below the attempted class.

## Diagnosis mode rules

- **singleton**: one workflow produces the packet
- **self_check**: same model produces a packet, then runs a constrained
  second pass that emits structured objections. May downgrade or block.
  May not raise authority.
- **conference**: multiple independent workflows review. Output is
  disagreement extraction, not majority vote.

Conference triggers override the declared `mode` upward (a `singleton`
run may be upgraded to `conference` if triggers fire). Triggers cannot
downgrade a declared conference agenda.

## Criticality rules

- **class** — `standard | business_critical | safety`. Shapes default
  urgency weightings. Not authority; urgency only.
- **re_alert_after** — required if promotion_ceiling > observe. Ack
  cannot be open-ended.
- **silence_max_duration** — required. No agenda may declare an
  unbounded silence window.
- Criticality policy is captured into the bundle at run time;
  post-capture edits do not affect the in-flight run.
- Criticality never raises `promotion_ceiling`. A `safety`-class agenda
  is not automatically allowed to mutate; it just surfaces more
  urgently.

## Escalation policy rules

- Each `on_*` clause maps a trigger to a destination from
  `GAP-escalation.md`. Missing clauses inherit sane defaults.
- `quiet_hours` applies only to destinations (packet_note, hold,
  notify, page). Severity overrides are explicit.
- `blast_radius_limits` is enforced before `classify_next_step` —
  exceeding a limit is an escalation trigger, not a warning.

## Validation

At capture time:

- All fields present / typed correctly
- `promotion_ceiling` is not above `governor_binding.required_above`
  without Governor available
- `cadence.expr` parses (if scheduled)
- `scope` is not empty for ops/publication modes

At run time:

- Reconciler produces a bundle whose inputs are all drawn from
  `allowed_evidence_sources`
- Proposed MCP calls are all in `allowed_tool_classes`
- Proposed authority level does not exceed `promotion_ceiling`
- Budget not exceeded

## Open questions

- How are agendas stored? (Filesystem YAML vs. SQLite vs. both.)
- Are agendas versioned? (Probably yes; `agenda_version` + content hash.)
- How does an operator edit an agenda mid-flight without invalidating
  pending runs? (Likely: edit produces a new version; pending runs bind
  to the version they captured.)
