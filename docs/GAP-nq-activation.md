# GAP: NQ <-> Night Shift Activation Semantics

> Status: identified, not specified

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

## Open questions

- Does NQ push raw findings, state transitions, or summarized incidents?
- Does Night Shift subscribe by detector, host, severity, persistence,
  regime hint?
- Are pushed events authoritative inputs or activation hints?
- What receipt is emitted when a pushed finding starts an agenda?
- What prevents alert storms from spawning agenda storms?
- How does recovery cancel, pause, or downgrade a pending agenda?
