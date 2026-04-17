# SCHEMA: Context Bundle (v0)

> Status: draft. The architectural heart of Night Shift.

A **context bundle** is a declared evidence object. Every input has
standing, freshness, and an invalidation rule. Two phases: capture
(when the agenda is set) and reconciliation (when the run begins).

This is what prevents the 3am agent from acting on 11pm vibes.

## Core invariants

- **Recheck is the gate, not metadata.** Every input passes through
  the Reconciler by virtue of the pipe it enters on. There is no
  per-input `requires_recheck` flag that can be forgotten.
- **Inputs enter as `observed`**, not `authoritative`. Authority is
  granted by the Reconciler (and, for promoted action, by Governor).
- **`committed` is scoped, not permanent.** A committed input is
  accepted for *this run, under this scope, after reconciliation*. It
  does not become a fossilized fact oracle.
- **Concurrent-activity check is structural.** Every bundle includes a
  `concurrent_scope_activity` input queried from Continuity by default;
  the reconciler classifies scope overlap before any proposal. Missing
  Continuity is a soft dependency (lowers ceiling; never raises
  authority). See `GAP-parallel-ops.md`.

## Shape

```yaml
bundle_version: 0
agenda_id: wal-bloat-review
run_id: run_2026041603000000

capture:
  captured_at: 2026-04-16T22:00:00Z
  captured_by: scheduler                    # operator | scheduler | nq_notify
  capture_reason: scheduled_review
  inputs:
    - input_id: nq:finding:wal_bloat:labelwatch-host:/var/lib/db
      source: nq
      kind: nq_finding_snapshot
      status: observed                      # all inputs enter as observed
      evidence_hash: sha256:...
      freshness:
        captured_at: 2026-04-16T22:00:00Z
        expires_at: 2026-04-17T04:00:00Z
        invalidates_if:
          - finding_absent_for_n_generations: 1
          - host_unreachable
      payload_ref: ledger://...

    - input_id: git:repo:labelwatch:HEAD
      source: git
      kind: repo_state
      status: observed
      evidence_hash: sha256:abc...           # commit sha
      freshness:
        invalidates_if:
          - repo_head_changed
      payload_ref: git://labelwatch@abc...

    - input_id: operator:note:wal-bloat-review
      source: operator
      kind: operator_note
      status: observed
      evidence_hash: sha256:...
      payload_ref: file://notes/wal-bloat-review.md

    - input_id: continuity:prior_decision:wal_bloat
      source: continuity
      kind: prior_decision
      status: observed                      # native to continuity vocabulary
      evidence_hash: sha256:...
      payload_ref: continuity://mem_...
      admissible_for:
        - diagnosis
        - packet_context
      inadmissible_for:
        - authorization
        - mutation

    - input_id: governor:policy:nightshift.ops.propose_only
      source: governor
      kind: policy_binding
      status: observed
      evidence_hash: sha256:...
      freshness:
        invalidates_if:
          - policy_hash_changed

    - input_id: continuity:concurrent_activity:scope:labelwatch-host
      source: continuity
      kind: concurrent_scope_activity        # see GAP-parallel-ops.md
      status: observed
      evidence_hash: sha256:...
      freshness:
        invalidates_if:
          - concurrent_actor_transitioned_state
          - concurrent_actor_opened_new_scope_overlap
      payload_ref: continuity://query:scope=labelwatch-host,window=24h

reconciliation:
  reconciled_at: 2026-04-17T03:00:00Z
  reconciled_by: scheduler
  results:
    - input_id: nq:finding:wal_bloat:labelwatch-host:/var/lib/db
      status: committed                     # committed | changed | stale | invalidated
      reliance_class: authoritative
      scope:
        run_id: run_2026041603000000
        valid_for: [diagnosis, proposal, packet_context]
      current_evidence_hash: sha256:...
      notes: persistence increased from 4 to 6 generations

    - input_id: git:repo:labelwatch:HEAD
      status: changed
      reliance_class: authoritative
      scope:
        run_id: run_2026041603000000
        valid_for: [diagnosis, packet_context]
      previous_evidence_hash: sha256:abc...
      current_evidence_hash: sha256:def...
      notes: 2 commits since capture; not invalidating for ops-mode scope

    - input_id: operator:note:wal-bloat-review
      status: committed
      reliance_class: hint
      scope:
        run_id: run_2026041603000000
        valid_for: [packet_context]

    - input_id: continuity:prior_decision:wal_bloat
      status: stale
      reliance_class: historical
      scope:
        run_id: run_2026041603000000
        valid_for: [packet_context]
      notes: recall predates current finding by 3 weeks; downgraded

    - input_id: governor:policy:nightshift.ops.propose_only
      status: committed
      reliance_class: authoritative
      scope:
        run_id: run_2026041603000000
        valid_for: [authorization]

    - input_id: continuity:concurrent_activity:scope:labelwatch-host
      status: committed
      reliance_class: authoritative_for_coordination   # narrow — see reliance classes
      scope:
        run_id: run_2026041603000000
        valid_for: [coordination_gating, diagnosis, packet_context]
      concurrent_activity:
        overlap_class: shared_write         # disjoint | shared_read | shared_write | contested
        decision: hold_for_context          # proceed | downgrade | hold_for_context | escalate
        actors:
          - actor_id: labelwatch-claude/mem_c1a452f0
            session: case:volume-migration-2026-04-17
            touched_at: 2026-04-17T13:42:00Z
            scope_overlap: shared_write
            last_breadcrumb: "nq-publish turned down; pending nq-claude"

  summary:
    admissible_for_authorization: [governor:policy:nightshift.ops.propose_only]
    admissible_for_proposal: [nq:finding:..., governor:policy:...]
    admissible_for_diagnosis: [nq:finding:..., git:repo:..., operator:note:...]
    hints_only: [operator:note:..., continuity:prior_decision:...]
    blocked: []
    downgraded: [continuity:prior_decision:...]
  ok_to_proceed: true
```

## Lifecycle

```text
input enters bundle           →  status: observed
reconciler evaluates          →  status: committed | changed | stale | invalidated
run uses committed/changed    →  under declared scope
```

## Reliance classes (assigned by Reconciler)

- **authoritative**: verified by receipt, Governor policy, NQ current
  state, or equivalent. May ground proposals and authorization.
- **authoritative_for_coordination**: narrow authority — may gate
  coordination (hold / proceed / coordinate / contested classification)
  but may not authorize policy decisions or ground state mutation.
  Used exclusively for `concurrent_scope_activity` inputs. See
  `GAP-parallel-ops.md`. Continuity never becomes truth through this
  class; it only becomes *a source of record about who else is in
  scope*.
- **hint**: operator input or Continuity recall accepted for this run
  but not independently verified. May inform proposals, never grounds
  them alone, never grounds authorization.
- **historical**: accepted as past context only. May appear in packet,
  not in action chain.
- **none**: input exists in capture but did not survive reconciliation;
  blocked or invalidated.

## Reconciliation result statuses

- **committed**: evidence hash matches, freshness not expired; granted a
  reliance class for this run's scope
- **changed**: evidence differs from capture but still admissible (note
  what changed); reconciler assigns appropriate reliance class
- **stale**: freshness expired; downgraded to `historical` or dropped
- **invalidated**: invalidation rule triggered; removed from admissible
  set

## Invalidation rules (recognized)

- `finding_absent_for_n_generations: N` — NQ no longer reports this
  finding for N consecutive generations
- `host_unreachable` — host has not reported since capture
- `repo_head_changed` — repo HEAD moved since capture (soft; default
  does not invalidate unless scope includes code mode)
- `policy_hash_changed` — Governor policy version changed since capture
- `expires_at: <timestamp>` — absolute deadline
- `concurrent_actor_transitioned_state` — another actor in overlapping
  scope changed state (attention transition, breadcrumb, run start/end)
  since capture. See `GAP-parallel-ops.md`.
- `concurrent_actor_opened_new_scope_overlap` — a new actor entered the
  declared scope after capture.

Agendas may declare additional project-specific rules.

## Scope semantics

A `valid_for` scope enumerates what the committed input may ground:

- `authorization` — may be used to justify a policy decision
- `proposal` — may ground a proposed action
- `diagnosis` — may inform the diagnostic explanation
- `packet_context` — may appear in the human-facing packet as context
- `coordination_gating` — may determine whether the run proceeds
  through capture → reconcile based on concurrent-actor state. This
  is narrow: coordination gating is not authorization. A
  `coordination_gating` input can block or hold a run but cannot
  authorize one. See `GAP-parallel-ops.md`.

An input committed for `packet_context` cannot be used to authorize
mutation, even if the workflow tries to do so. Scope is enforcement,
not advisory.

## Invariant

> A run may only propose actions grounded in inputs whose scope
> includes `proposal`. Only inputs whose scope includes `authorization`
> may ground a promotion to `stage` or higher. `hints_only` and
> `historical` inputs may shape proposals but not justify them alone.
> `invalidated` and `blocked` inputs must not appear in the proposal's
> evidence chain.

## Open questions

- Are bundle payloads stored inline or by reference? (Reference for
  large evidence, inline for small.)
- How is `evidence_hash` computed for each kind? (Kind-specific
  hashers.)
- Does reconciliation run once at start, or continuously during the
  run? (Once at start; long-running workflows must re-reconcile before
  any promoted action.)
- Can an input be promoted in reliance class mid-run? (Probably no —
  promotion requires a new reconciliation pass.)
