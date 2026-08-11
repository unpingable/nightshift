# Nightshift canonical runtime C1

**Status:** canonical production runtime; legacy cutover closed; development
evidence green; not yet operationally qualified.

This is the active developer-facing runtime correspondence note for the
canonical Nightshift path. It does not issue standing, authority, execution
permission, or a qualification claim.

## Responsibility boundary

Nightshift owns exact recurrence-slot and observation-cycle identity,
scheduling posture, application posture over complete current diagnostic
evidence, attention, and typed non-authorizing intent. Present-evidence
currentness is supplied by its owning authority. NQ supplies the complete
diagnostic basis. AG exclusively owns exact-work occurrence governance.
Docket and the executor remain behind AG.

The sole production binary is `nightshift`. Its only
consequence-adjacent port is `ag-loopctl`, restricted to `status`, `init`,
`continue`, and `record-proposal`. It has no standing, authorization,
dispatch, retry, reconciliation, Docket, executor, or human-disposition
command.

## Runtime flow

```text
exact recurrence slot
  -> exact observation cycle
  -> authority-owned present-support result
  -> complete NQ diagnostic posture
  -> display/attention and optional typed intent
  -> immutable exact-work proposal
  -> new AG occurrence
  -> read-only AG status/settlement reference
  -> observation required, reconciliation display, halt display, or close
```

Recurrence permits observation only. A diagnostic headline, severity,
attention record, prior NQ generation, AG status rendering, receipt, or
persisted support record cannot originate work. The in-process live-cycle
lease is deliberately non-serializable. Restart preserves historical facts
but erases the live witness used to prepare an AG request.

Pulse-style support expiry is current only while `expiry > evaluated_at` on
the evidence authority's receiver clock. Recurrence latest-admissible time is
Nightshift-owned and inclusive at equality. A temporal hold is active only
while `now < expiry`. These are distinct semantic types.

## Store and recovery

`canonical_recurrence_slots`, `canonical_observation_cycles`, and
`canonical_cycle_events` form the authoritative SQLite state. IMMEDIATE
transactions, predecessor-digest CAS, a one-successor event constraint, and
an exact `(campaign_id, occurrence_id)` claim prevent duplicate slot work,
stale completion, and occurrence reuse.

After restart:

- local observing or posture-recorded work becomes `RecoveryRequired`;
- a prepared AG occurrence is recovered only by an exact AG status query and
  is never resubmitted by Nightshift;
- reconciliation, settlement, halt, and completion remain AG facts;
- prior support and posture remain historical evidence, never reconstructed
  currentness.

An AG settlement records only attempt-native facts and moves the Nightshift
cycle to `ObservationRequired`. Subject posture can change only after a new
qualified observation and NQ evaluation.

## Current command surface

```sh
cargo build --locked --release --bin nightshift
./target/release/nightshift cycle --help
```

`cycle run` accepts one exact sealed cycle request and an executable named
`pulse-support-resolver`. AG options are required only when the request
contains an exact precompiled proposal. `cycle sync-ag` and `cycle recover`
read AG state through `ag-loopctl`; neither can resubmit a prepared request.

## Production exclusivity

Cargo automatic binary discovery is disabled and the manifest declares one
binary target: `nightshift` at `src/bin/nightshift.rs`. Wicket/WLP path
dependencies, MVP-A, classic Governor, the authority ladder, prose action,
same-generation skip, and production drills are absent from production source.
The structural gate enforces this closed graph and mutation-tests representative
resurrections.

Historical Watchbill test sources that retain useful archaeology are
quarantined outside Cargo discovery and are explicitly noncanonical. The old
user-level unit files were deleted. Neither can supply a production runtime or
authority path.
