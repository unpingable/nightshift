# GAP: NQ <-> Night Shift Activation Semantics

> Status: identified, partially specified

## Question

Push or pull? What if NQ pushed out to Night Shift?

Answer: **both, but with different authority semantics.**

## Pull (safer default)

Night Shift pulls from NQ on its own schedule.

```text
Night Shift → NQ
```

Good for: reconciliation, scheduled review, bounded context capture,
keeping NQ as evidence substrate (not dispatcher).

Fits the clean ontology:
> NQ observes. Night Shift decides whether observation deserves agenda
> activation. Governor gates force.

## Push (useful, but dangerous)

NQ pushes events to Night Shift.

```text
NQ → Night Shift
```

Good for: high-severity findings, persistence threshold crossed, finding
recovered, regime transition detected, "this is no longer routine, wake
something up."

But then NQ is no longer just passive evidence. It becomes an
**activation source**. Not authority, but trigger. That distinction
matters.

## Three event paths

```text
poll      Night Shift periodically queries NQ
notify    NQ emits finding events to Night Shift
page      Night Shift escalates to human/operator
```

NQ can **notify**. Night Shift can **activate agenda**. Governor can
**authorize action**. Human can **override / approve / break glass**.

No component gets to smuggle itself upward.

## Core rule

> A push event may wake Night Shift, but it may not authorize Night Shift.

Push is never enough standing. Night Shift must reconcile pushed state
before acting. Always.

## Activation is not evidence

Explicit rule:

> Notify events are activation hints. Reconciliation must re-pull current
> NQ state before any proposal or promotion.

A push event gives Night Shift permission to *care*, not permission to
*act on what it was told*. The bundle must be built from reconciled
state, not from the push payload.

This prevents push from smuggling stale state into the bundle.

## Event identity / dedupe

Every push event needs a stable identity so Night Shift can say "I
already activated an agenda for this transition."

Proposed shape:

```text
source=nq
event_type=finding_persisted | finding_recovered | finding_flapped | regime_shift
finding_id=...
generation=...
host=...
detector=...
subject=...
severity=...
event_hash=H(source, event_type, finding_id, generation, transition)
```

Night Shift tracks recently-processed `event_hash` values and refuses
duplicate activation within an agenda's cooldown window.

## Storm control

Pushed events can storm. Agenda activation must not.

Controls:

- **per-agenda cooldown** — no re-activation within window
- **per-host cap** — max concurrent agendas per host
- **per-detector cap** — max concurrent agendas per detector
- **severity override** — critical may bypass caps, still subject to cooldown
- **grouping window** — collapse related events into a single activation
- **recovery cancellation** — `finding_recovered` may cancel or downgrade
  pending agenda

Event-to-activation mapping:

```text
finding_persisted     may activate agenda
finding_recovered     may cancel or downgrade pending agenda
finding_flapped       may suppress activation and emit advisory packet
regime_shift          may re-prioritize pending agenda
```

This is where NQ's persistence/recovery work becomes load-bearing.

## Open questions

- Does NQ push raw findings, state transitions, or summarized incidents?
  (Probably transitions + summarized, never raw.)
- Does Night Shift subscribe by detector, host, severity, persistence,
  regime hint? (Probably all, as filter expression.)
- Are pushed events authoritative inputs or activation hints?
  (Hints. Resolved above.)
- What receipt is emitted when a pushed finding starts an agenda?
  (Run-ledger event + authority receipt if Governor present.)
- What prevents alert storms from spawning agenda storms?
  (Cooldown/caps/grouping. Specified above.)
- How does recovery cancel, pause, or downgrade a pending agenda?
  (Recovery cancellation. Specified above, details TBD.)

## Transport

TBD. Candidates:

- HTTP webhook from NQ (fits NQ's existing notification model)
- Unix socket / local RPC (colocated deployments)
- Queue/broker (multi-host deployments)

Webhook is almost certainly first, since NQ already emits webhook
notifications on severity escalation.

## Receipt shape for activation

When a push event activates an agenda:

```text
event: agenda.activated_by_notify
agenda_id: ...
event_hash: ...
source_event: { ...nq_event... }
activation_reason: finding_persisted
reconciliation_required: true
authority_level: observe | advise | ...
ts: ...
```

The `reconciliation_required: true` is invariant for push activations.
