# GAP: Attention State

> Status: identified. Separate from `GAP-escalation.md`.
> Escalation covers the *boundary* — where drive-to-resolution ends.
> This covers *within-bounds operator-intent durability* — how
> "handled" doesn't quietly become "forgotten."

## Core problem

A detector (NQ, ledger check, whatever) tells Night Shift there is a
finding. An operator touches it — acknowledges, marks investigating,
silences for a window. The finding itself has not changed. The
detector will happily keep firing, the operator will happily keep
seeing `ack=true`, and a week later the host is still stale at
localhost because nobody re-asked the question.

That is not escalation. That is operator intent decaying without a
half-life.

> Silence is not handling. Ack is not closure. Suppression needs an
> expiry or a reason.

## Three axes

Night Shift separates:

### 1. Evidence state (from detector / NQ)

What the world looks like according to the detector:

```text
active       finding present and current
worsening    finding present and intensifying
resolving    finding present, trajectory turned, not yet clear
recovered    finding was present, now absent (scar preserved)
stale        source has not reported — evidence is not current
```

Owned by NQ / the detector. Night Shift does not edit this.

### 2. Attention state (operator layer)

What the *operator* has decided about the finding, separate from what
the detector thinks:

```text
unowned         no operator has touched it
acknowledged    operator saw it (does not mean handled)
investigating   operator is actively working it
handed_off      explicit transfer to named party, with note
watch_until     silenced until timestamp or condition
silenced        suppressed with reason and expiry
```

Owned by the operator. Carries a TTL or a reason. No open-ended
suppression.

### 3. Criticality policy (from agenda)

What the system should do as attention state ages:

```text
re_alert_after         duration after which ack expires and finding re-surfaces
ack_due_by             duration within which an unowned finding must be acked
handoff_required       explicit transfer required to leave attention state
business_hours_okay    can wait; do not page outside hours
```

Declared by the agenda, not inferred.

## Operational urgency

Urgency is derived, not declared:

```text
operational_urgency = f(
    severity,              # from detector
    criticality_class,     # from agenda
    age_of_finding,        # from detector
    ack_freshness          # from attention state
)
```

A stupid-but-critical stale source beats a noisy-but-noncritical one.
A finding with a silence-until-Tuesday that expired this morning
re-surfaces. An `investigating` state untouched for hours past
`re_alert_after` decays back toward `unowned` and the finding
re-surfaces at higher urgency.

## Attention is keyed to stable finding identity

Attention state lives on the **stable finding identity** supplied by
the evidence adapter, not on a run-local packet object or dashboard
row. For NQ-sourced findings, this is the `finding_key` defined in
`GAP-nq-nightshift-contract.md` — the durable identity that survives
regeneration, snapshot refresh, and status transitions
(`active → resolving → recovered`).

```text
attention_key = finding_key  (for NQ findings)
              | <source>:<stable_id>  (for other evidence adapters)
```

If attention were keyed to a run or packet, acks would become
haunted — "why did this ack disappear?" — the moment NQ emitted the
same finding in a new generation or a new snapshot. Binding to
finding identity prevents that.

Consequences:

- An `acknowledged` attention state carries across regenerations of
  the same finding. The ack is on the finding, not on the run that
  saw it.
- `recovered` followed by recurrence within the recurrence window
  (see `GAP-escalation.md`) does **not** create a new attention
  state — it re-surfaces the existing one with the scar preserved.
- Deleting / rotating the finding_key is a deliberate operator
  action with a receipt; attention state is archived, not lost.

## Anti-amnesia field kit

Minimum fields carried per-finding (in run ledger + packet), keyed
on stable finding identity:

```text
attention_key          stable finding identity (e.g. NQ finding_key)
owner                  who owns attention right now
last_touched_by        who moved the attention state last
last_touched_at        when
acknowledged_at        timestamp of most recent ack
ack_expires_at         when the ack loses force
follow_up_by           declared next check-in
handoff_note           explicit transfer context
re_alert_after         policy-derived expiry timestamp
silence_reason         required if attention state is silenced
```

These are the minimum fields that keep "handled" from turning into
"forgotten."

## Invariants

- **Silence is not handling.** A silenced finding is not an absent
  finding. Evidence state is preserved and visible; only attention
  state is changed.
- **Ack is not closure.** An acknowledgment has a TTL. The TTL comes
  from agenda policy, not from operator vibes.
- **Suppression needs an expiry or a reason.** No open-ended silence.
  Either a timestamp, a condition, or a `handoff_note`.
- **Attention state never raises authority.** An `investigating`
  marker does not grant the investigator additional ceiling.
  Attention state is operator memory; it is not policy.
- **Recovered ≠ closed.** If evidence transitions to `recovered` while
  attention is `investigating`, preserve the scar (recurrence window
  per `GAP-escalation.md`).

## Rendering rules

Any packet UI or watchbill surface must render both axes plus the
derived urgency:

```text
evidence:    active / worsening / resolving / recovered / stale
attention:   unowned / acked / investigating / handed_off / silenced
urgency:     (derived)
```

Do not render attention state alone. A green `acked` next to an
`active` evidence state without the derived urgency is the precise
failure mode this doc exists to prevent.

## Interaction with escalation

| Situation | Lives in |
|-----------|----------|
| Next step requires authority we don't have | Escalation |
| Finding is stale and can't refresh | Escalation |
| Ack expired and finding still active | Attention → re-alert → possibly escalation |
| Operator silenced without reason | Attention (invariant violation → reject) |
| Same finding returns after repair | Escalation (recurrence) |
| Resolving for too long | Escalation (trigger 5b) |

Escalation is terminal / interrupt. Attention state is ongoing
durability. Attention-state expiry can *trigger* escalation, but they
are different concepts.

## Interaction with NQ `resolving`

`resolving` is an evidence-state signal. It does not clear attention
state. An operator may `watch_until` a `resolving` finding; when the
watch window expires, attention state returns to `unowned` and the
finding re-surfaces — even if evidence state is still `resolving`.

## Open questions

- Do attention-state transitions produce run-ledger events? (Probably
  yes; operator intent is audit material.)
- Does attention state live per-run or per-finding? (Per-finding;
  multiple runs may touch the same finding.)
- How does handoff interact with Governor? (Handoff is not
  authorization. It changes who is on the hook; it does not change
  what they're allowed to do. Governor still gates force.)
- Who authors the default TTLs — agenda only, or operator profile +
  agenda? (Agenda only at MVP; operator profiles are v2.)
- Where does attention state live across runs — in the store, or
  computed from event log? (Probably a projection over the event log
  with a materialized view for read paths.)
