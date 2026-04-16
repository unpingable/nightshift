# SCHEMA: Review Packet (v0)

> Status: draft. The human-facing output artifact.

A **packet** is the reviewable output of a Night Shift run. It is what
an operator reads to decide whether to approve, dismiss, or escalate.

The packet is an artifact. It does not carry authority on its own;
authority receipts live separately (Governor). A packet may reference
them.

## Shape

```yaml
packet_version: 0
packet_id: pkt_2026041603000000
agenda_id: wal-bloat-review
run_id: run_2026041603000000
produced_at: 2026-04-17T03:04:12Z

finding_summary:
  source: nq
  detector: wal_bloat
  host: labelwatch-host
  subject: /var/lib/labelwatch.sqlite
  severity: warning
  domain: Δg
  persistence_generations: 6
  first_seen_at: 2026-04-10T14:32:15Z
  action_bias: investigate_business_hours

reconciliation_summary:
  captured_at: 2026-04-16T22:00:00Z
  reconciled_at: 2026-04-17T03:00:00Z
  admissible_inputs: 4
  hints: 1
  invalidated: 0
  notes:
    - nq finding persisted and intensified since capture
    - repo HEAD advanced, not invalidating for ops-mode scope

diagnosis:
  regime: accumulation / pinned reader
  evidence:
    - wal size grew from 412 MB to 518 MB across last 4 generations
    - checkpoint interval unchanged; writer cadence unchanged
    - no recovery observed
  confidence: medium
  alternatives_considered:
    - regime: long_transaction
      ruled_out_by: no long-lived txn visible in sqlite_stat
    - regime: disk_pressure
      ruled_out_by: free space > 30%

proposed_action:
  kind: advisory
  steps:
    - inspect active readers via lsof / fuser on db path
    - verify no pinned PID holding wal snapshot
    - if no pinned reader, consider PRAGMA wal_checkpoint(TRUNCATE)
      during quiet window
  risk_notes:
    - do not restart labelwatch service unless pinned PID confirmed
    - restart during active write may lose most recent batch
  reversible: true
  blast_radius: single_host
  requested_authority_level: advise

authority_result:
  requested: advise
  governor_present: true
  governor_verdict: pass
  authority_receipts:
    - rcpt_...
  # for staged/applied runs, include:
  # staged_command: null
  # apply_receipt: null
  # verify_receipt: null

blocked_assumptions:
  - assumption: no pinned reader
    evidence_needed: fuser output
    checked: false
  - assumption: quiet write window available
    evidence_needed: writer cadence metric
    checked: true

required_approvals:
  - operator_review      # any advise+ packet requires operator eyes
  # for apply, add: operator_approval

diagnosis_review:
  mode: self_check                    # singleton | self_check | conference
  objections:
    unsafe_assumptions: []
    stale_context_risks: []
    promotion_overreach: []
    missing_verification:
      - fuser output on db path not checked
    recommended_downgrade: null
  # for conference, add:
  # agreement: [...]
  # disagreement: [...]
  # operator_question: [...]

escalation:                           # present only when run escalated
  escalated: false
  type: null                          # authority | context | risk | evidence |
                                       # recurrence | budget | incident
  destination: null                   # packet_note | hold_for_review |
                                       # create_ticket | notify |
                                       # request_approval | page |
                                       # block_and_record
  trigger_reason: null
  next_step_blocked_by: null          # evidence | authority | scope | budget
  resume_requires: null               # what operator action would unblock

receipt_references:
  run_ledger: ledger://nightshift/runs/run_2026041603000000
  governor_receipts: [rcpt_...]
  evidence_bundle: bundle://run_2026041603000000
```

## Invariants

- A packet must reference the bundle it was produced from.
- A packet's `proposed_action.requested_authority_level` must not exceed
  the agenda's `promotion_ceiling`.
- A packet's `authority_result` must record Governor verdict if
  `governor_present = true`.
- `blocked_assumptions` must be populated if any proposal step depends on
  unchecked evidence.
- `confidence` must be one of: low | medium | high.
- Packets are append-only. Revisions produce new packets referencing the
  prior `packet_id`.
- `diagnosis_review.mode` must be declared. `self_check`/`conference`
  objections may downgrade `requested_authority_level` or trigger
  escalation; they may not raise authority.
- If `escalation.escalated = true`, the packet is a **terminal artifact**
  for the run. No apply/publish may follow without a new operator
  action.

## Intended reader

A human operator reviewing overnight output. The packet should answer,
in order:

1. What did we find?
2. Has it changed since we captured context?
3. What do we think is going on?
4. What do we propose doing?
5. What's the risk?
6. What hasn't been verified?
7. What's needed to approve?
8. Where are the receipts?

If the packet cannot answer those cleanly, it should not have been
emitted.

## Open questions

- Rendering: markdown + machine-readable sidecar? (Probably yes.)
- Review UX: email? web? tui? (TBD; probably file + web later.)
- Approval surface: where/how does the operator say "apply"? (Governor's
  job to own, Night Shift renders the request.)
