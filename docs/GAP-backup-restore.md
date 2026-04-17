# Nightshift Gap Spec: Backup and Restore for Continuity-Bearing Workloads

## Status

Draft

## 1. Abstract

Nightshift schedules and supervises recurring work, but does not yet provide a first-class capability for backing up and restoring continuity-bearing workload state.

This is a blocking gap for any workload where host loss, storage corruption, operator error, or incomplete rebuild would cause material continuity loss. Examples include ATProto PDS state, SQLite-backed observatories, and Nightshift-owned scheduler state.

Nightshift MUST treat backup freshness and restoreability as operational truth, not operator folklore.

## 2. Problem Statement

The current Nightshift model can run jobs. It cannot yet answer the more annoying question:

> What survives when the box does not?

For continuity-bearing workloads, "rebuild from code" is insufficient. Code is reconstructible. Identity state, scheduler state, databases, and selected secrets are not.

Without explicit backup and restore support:

- host-your-own PDS remains toy-tier
- observatory workloads remain dependent on operator heroics
- backup posture remains implicit and non-auditable
- restore confidence decays into vibes

This is unacceptable for any workload Nightshift expects operators to trust.

## 3. Scope

This spec covers **operational backup and restore** for Nightshift-managed workloads.

This spec does **not** define:

- a general-purpose archival system
- an immutable evidence vault
- fleet-wide disaster recovery orchestration
- high-availability or hot failover
- universal snapshotting of arbitrary hosts
- "backup the whole box and pray" semantics

The design target is boring recoverability, not transcendence.

## 4. Design Position

Backup and restore belong in Nightshift because the immediate problem is orchestration:

- run capture jobs on policy cadence
- enforce declared scope
- move artifacts off-host
- verify artifacts and transfers
- track stale and failed posture
- run restore drills
- emit receipts and events

Nightshift MUST own orchestration and visibility.

Nightshift MUST NOT become a universal archive substrate merely because backup work makes architects feel poetic.

## 5. Terminology

### 5.1 Continuity-Bearing Workload
A workload whose locally managed state cannot be discarded and reconstructed without material loss of identity, function, or history.

### 5.2 Backup Contract
A Nightshift-managed declaration describing what state must survive host loss, how it is captured, where it is sent, how it is verified, and how restore confidence is maintained.

### 5.3 Capture
The act of producing a backup artifact or manifest from declared protected state.

### 5.4 Verification
The act of checking that a capture completed as intended and that the resulting artifact is complete and intact.

### 5.5 Restore Drill
A non-live restoration of a backup artifact into an isolated target to prove recoverability.

### 5.6 Off-Host
Stored somewhere that does not share the failure domain of the protected host. "Different directory on the same VM" is staging, not off-host. Nice try.

## 6. Requirements Language

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, and MAY in this document are to be interpreted as described in RFC 2119.

## 7. Goals

Nightshift backup/restore support MUST:

1. Make backup posture legible as an operator-visible condition.
2. Require explicit declaration of protected state.
3. Support off-host backups as a first-class capability.
4. Produce receipts/events for capture, verification, and restore drills.
5. Surface stale, failed, or unproven backup posture into operational visibility.
6. Remain workload-agnostic enough to cover SQLite-backed services and PDS-class identity services without dragging in Kubernetes theology.

## 8. Non-Goals

Nightshift backup/restore support MUST NOT attempt to solve, in v1:

- global deduplication
- block-device snapshot orchestration
- cross-region failover
- long-term compliance retention
- automatic live failback
- universal database quiescing across every storage engine under the sun

The system only needs to stop lying about recoverability.

## 9. Model

### 9.1 Backup Contract

A continuity-bearing workload MAY declare a Backup Contract.

A Backup Contract MUST identify, at minimum:

- `workload_id`
- `protected_state`
- `capture_method`
- `destination`
- `cadence`
- `retention_policy`
- `verification_policy`
- `restore_drill_policy`

A Backup Contract SHOULD also identify:

- sensitivity class of protected material
- scope exclusions
- expected artifact form
- operator-visible stale thresholds

### 9.2 Protected State

Protected state MUST be declared explicitly.

Protected state MAY include:

- files
- directories
- SQLite databases
- workload-defined exports
- configuration trees
- secret references

Protected state MUST NOT default to "the whole machine."

Nightshift MUST force the operator or workload author to name what matters. If they do not know what matters, that is already the problem.

### 9.3 Capture Methods

Initial supported capture methods SHOULD remain boring:

1. **File tree copy**
2. **SQLite-safe checkpoint and copy**
3. **Workload-defined export command**
4. **Manifest pack**

Nightshift MUST NOT perform naive live copying of SQLite files where corruption or partial capture is a realistic outcome.

Nightshift MAY add additional capture methods later, but v1 SHOULD avoid block-level snapshot machinery unless forced by an actual workload.

### 9.4 Destinations

Nightshift MUST support off-host destinations.

Examples include:

- remote object storage
- remote SSH/SFTP destination
- second VM or remote mount
- other target classes with distinct failure domains

A destination that shares the protected host's primary failure domain MUST NOT satisfy continuity requirements for identity-bearing workloads.

### 9.5 Verification

A successful backup run MUST be more than "the command exited zero."

Verification MUST be able to answer, at minimum:

- Was an artifact or declared manifest produced?
- Did transfer to the destination complete?
- Do hashes or equivalent integrity markers match?
- Was retention applied as expected, or did it fail?
- Is the resulting artifact plausibly complete for the declared scope?

Verification MAY be generic in v1, but MUST be explicit about what was and was not proven.

### 9.6 Restore Drills

Nightshift MUST support non-live restore drills.

A restore drill MUST, at minimum:

- restore an artifact into an isolated path, host, or disposable environment
- verify presence of expected files or structures
- record pass/fail outcome
- preserve evidence of what artifact was tested

A restore drill SHOULD perform workload-specific sanity checks where practical.

A backup without restore evidence MUST be considered **captured, not proven**.

## 10. Policy

Backup policy MUST be declarative per workload.

A policy SHOULD include:

- capture cadence
- off-host requirement
- verification requirement
- retention expectations
- restore drill interval
- stale thresholds
- escalation thresholds

Example posture:

- backup every 24 hours
- off-host REQUIRED
- verification REQUIRED
- restore drill every 30 days
- warning if latest verified backup older than 26 hours
- critical if older than 48 hours
- critical if restore proof older than 45 days

Nightshift MUST make stale policy machine-visible and operator-visible.

## 11. Receipts and Events

Nightshift MUST emit events and/or receipts for at least the following transitions:

- `backup.started`
- `backup.completed`
- `backup.failed`
- `backup.verified`
- `backup.verification_failed`
- `restore_drill.started`
- `restore_drill.passed`
- `restore_drill.failed`
- `backup.stale`
- `restore_proof.stale`

Each record MUST identify, at minimum:

- workload identity
- declared scope or manifest reference
- capture method
- destination
- artifact identity or hash
- policy instance applied
- outcome
- timestamp
- failure details, if any
- whether operator intervention occurred

Nightshift SHOULD preserve enough information that an operator can determine what happened without spelunking raw shell history like a medieval bone reader.

## 12. Integration Boundaries

### 12.1 Governor

Backup and restore actions are governable operations.

Nightshift SHOULD permit routine scheduled backup actions to run under pre-authorized policy where scope and destination are explicit.

Nightshift MUST treat restore actions as higher-risk than backup actions.

Nightshift MUST default restore drills to non-live targets.

Destructive or ambiguous restore actions SHOULD require stricter authorization posture.

### 12.2 NQ

Nightshift SHOULD surface backup posture as findings or regime inputs, not merely logs.

Examples include:

- `backup_missing`
- `backup_stale`
- `backup_failed_recently`
- `backup_verification_failed`
- `restore_unproven`
- `restore_proof_stale`
- `destination_unreachable`
- `retention_not_met`

The point is to make backup debt visible before it turns into autobiography.

### 12.3 Continuity

Continuity and backup are related but distinct.

- **Continuity** addresses cross-run state, coordination, and scheduler memory.
- **Backup** addresses disaster survival of declared protected state.

Nightshift MUST NOT collapse these into one conceptual blob just because both smell like persistence.

## 13. Security and Safety Considerations

### 13.1 Secrets Handling

Backup scope MUST account for secret material explicitly.

Nightshift SHOULD support secret references or workload-specific secret export handling rather than encouraging lazy whole-tree capture.

Nightshift MUST make it possible to exclude material that should not transit or rest in ordinary backup artifacts.

### 13.2 Partial Success

Nightshift MUST distinguish:

- capture succeeded, transfer failed
- transfer succeeded, verification failed
- verification succeeded, restore proof stale
- restore drill failed despite prior backup success

Collapsing these states into a single green checkbox is how backup systems earn contempt.

### 13.3 Destination Failure

Destination unreachability MUST be surfaced as backup debt, not quietly retried into oblivion while the operator accumulates false confidence.

### 13.4 SQLite Integrity

SQLite capture MUST use a safe method. Naive copying of active database files MUST NOT be treated as acceptable default behavior.

## 14. Failure Model

The implementation MUST be designed against the following failure modes:

- silent partial captures
- corrupted SQLite copies
- transfer reported successful when artifact never actually arrived
- restore drills never run
- retention silently not enforced
- secrets unintentionally swept into artifacts
- destination outages hidden behind retries
- backup logic overfit to a single workload type
- same-host "backup" mistaken for continuity protection

## 15. Minimum Viable Slice

The first slice that counts SHOULD include:

1. Explicit Backup Contract for at least one real workload
2. File tree capture
3. SQLite-safe checkpoint and copy
4. Off-host destination support
5. Manifest and hash generation
6. Verification result recording
7. Non-live restore drill
8. Operator-visible stale/failure posture
9. Receipts/events for capture, verification, and drill

This is enough to stop pretending.

## 16. Initial Target Workloads

Recommended proving targets:

1. Labelwatch SQLite and config
2. Nightshift-owned scheduler state, if continuity-bearing
3. ATProto PDS state as a design target

The PDS SHOULD be treated as the design pressure, not necessarily the first live implementation target. There is no prize for making your identity the integration test before the plumbing is boring.

## 17. Open Questions

1. What is the minimal protected-state set for true PDS continuity recovery?
2. How should key material be handled without encouraging tarball nihilism?
3. Should manifest production be Nightshift-native, workload-native, or hybrid?
4. What verification bar is sufficient for v1: hash, structural presence, startup test, or workload-specific probe?
5. How much retention logic belongs in Nightshift versus destination policy?
6. Should restore drills begin operator-confirmed and only later become fully scheduled?
7. Does Nightshift need a distinct backup object model, or is a workload-attached Backup Contract sufficient?

## 18. Acceptance Criteria

This gap is closed when Nightshift can, for at least one real workload:

- declare protected state explicitly
- capture that state on policy cadence
- store artifacts off-host
- verify artifact integrity/completeness
- surface stale or failed posture visibly
- perform and record a non-live restore drill
- emit receipts/events sufficient for audit and debugging

If any of those are missing, the gap is not closed. It is merely discussed.

## 19. Thesis

Nightshift is not ready for continuity-bearing identity or observatory workloads until backup freshness and restoreability are first-class operational truths.

Everything else is self-hosting fan fiction.
