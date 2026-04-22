# GAP: Deferred capture → reconcile split

> Status: specified, in flight. Surfaced 2026-04-20 during the first
> e2e watchbill run against a real NQ VM snapshot (`wal-bloat-review`
> agenda, `nq:wal_bloat:labelwatch-host:...`). Mechanics work; the
> deferral they exist to serve does not yet — this slice makes the
> deferral real.

## Core problem

The system's value prop is stated in `reconciler.rs`:

> The 3am agent must not act on 11pm vibes.

But `run_watchbill` currently captures the bundle and reconciles it
against the live NQ source in the **same invocation** — so capture and
reconcile land within the same generation, and the reconciler always
reports `Committed` with matching `previous_evidence_hash` ==
`current_evidence_hash`. The `Changed` / `Stale` / `Invalidated` paths
are only exercisable via unit tests with synthesized bundles.

That means the deferral problem the reconciler was built for — capture
at t0, act at t0+Δ, state may have moved — is not exercised
end-to-end. The seam mechanics are proven; the *deferral* is not.

## What gets split

Two CLI verbs where today there is one:

```bash
# t0: capture and persist the bundle. No reconcile. No packet. Run stays open.
nightshift watchbill capture <agenda> --finding <key>
  # → emits run_id, persists bundle (baseline snapshot), returns

# t0+Δ: revalidate the captured bundle, emit packet, close the run.
nightshift watchbill reconcile <run_id>
  # → loads bundle, re-runs preflight, performs ONE explicit live
  #   acquisition of current NQ state, persists that acquisition to
  #   the bundle, runs pure adjudicate() over baseline + current +
  #   policy fingerprint, emits packet, finalizes run.
```

With that split, the gap between capture and reconcile is real wall
time, and the four reconciliation statuses become first-class
operational outcomes rather than test-only branches.

`watchbill run` remains as a thin convenience wrapper that composes
`capture` + `reconcile` in a single invocation (same external
behavior as today). It is not the canonical path; it is the
same-generation shortcut.

## Load-bearing contract

The load-bearing design question is: **what is frozen at capture
time, and what is allowed to vary at reconcile time?**

Resolution:

**Frozen at capture, immutable thereafter:**

- Baseline finding snapshot (the "observation basis")
- Captured input metadata (freshness, invalidation rules)
- Capture-time preflight outcome
- Capture-time liveness verdict (if configured)
- Agenda reference (agenda_id)
- Authority ceiling declaration

**Allowed to vary at reconcile time, but only under explicit rules:**

- The current NQ snapshot, via **one** explicit live acquisition step
  per run, which is **persisted** as part of the run record. After
  persistence, adjudication is deterministic over the stored bundle.
- The coordination state (preflight is re-checked at reconcile;
  coordination state can move while the run is paused).
- Current policy/config **only if surfaced explicitly and versioned**
  in the packet output. v1 stubs the policy fingerprint (no
  externally-versioned policy yet); the shape is reserved.

**Never varied between capture and reconcile:**

- The baseline observation itself. It is frozen evidence.

## Invariants

1. **A captured run can be completed later without repeating
   capture-time acquisition.** Capture happens exactly once.
2. **Any reconcile-time live acquisition must be explicit,
   single-shot, and persisted as part of the run record.** The live
   read is a named step with a ledger entry
   (`RunCurrentSnapshotAcquired`), not an implicit side-effect of
   adjudication.
3. **Once reconcile-time acquisition is persisted, adjudication is
   deterministic with no further live NQ dependency.** Re-running
   adjudication over the stored bundle produces the same verdict.
4. **Reconcile is one-shot.** `reconcile <run_id>` on an already
   completed run is an error. If you want a new verdict, start a new
   capture.

The invariant we actually want is not *"first reconcile never calls
NQ"* — that would force `Changed` and absence-driven `Invalidated`
out of this slice, which is a semantic retreat from slice-5, not a
phase split. The invariant is: **no hidden or repeated live
dependency once reconcile-time observation has been persisted.**

## Pipeline shape

`run_watchbill` today does:

```
authority_ceiling → capture → preflight → reconcile → build_packet → complete
```

After the split:

**`watchbill capture`:** authority_ceiling → capture → preflight (initial) → persist (run stays open)

**`watchbill reconcile <run_id>`:** load_bundle → preflight (re-check) → acquire_current → adjudicate → build_packet → complete

Ledger events already distinguish `run_captured` /
`run_preflight_cleared` / `run_reconciled` / `run_completed`. New
event `run_current_snapshot_acquired` marks the reconcile-time live
acquisition as a named step with its own receipt payload.

Internally, reconcile is two-phase:

1. **Acquisition** — one live NQ call per captured input, result
   persisted to the bundle alongside the baseline snapshot.
2. **Adjudication** — pure function over (baseline, persisted
   current, policy fingerprint). Produces the ReconciliationPhase.

That structure kills the Tuesday goblin: the live read is no longer
hidden inside judgment. Re-adjudication over a persisted reconciled
run is pure and idempotent.

## Resolved open questions

- **Preflight placement.** Preflight runs at capture AND at reconcile.
  Coordination state can change while the run is paused; re-checking
  is safer than caching.
- **Re-reconcile.** One-shot. A run can be reconciled exactly once;
  the packet is then frozen. Re-reconcile-on-demand is not in v1.
  (Adjudication itself is pure over persisted bundle, so replay for
  debugging/analysis is separately trivial.)

## Deferred to followup GAPs

These are real questions; they are not load-bearing for the split
itself and are out of scope for this slice.

- **Evidence TTL defaults + enforcement.** `Freshness.expires_at` is
  in the data model but not yet defaulted or enforced at CLI level.
  Semantics: past expires_at, adjudication emits `Stale` regardless
  of current state. Defaults should probably come from the agenda,
  not the code.
- **Abandonment / GC of never-reconciled runs.** Today there is no
  lifecycle state for "captured but never reconciled." After this
  slice, that state will exist (open run, no packet). A run left open
  indefinitely needs either a time-based abandonment policy or an
  explicit `watchbill abandon <run_id>` verb. Not in v1.
- **Policy fingerprint.** Packet output reserves a policy-version
  field; v1 stubs it. Real policy versioning arrives with the
  Governor adapter (policy lives in Governor, not in Night Shift).

## Why this isn't an NQ blocker

NQ's consumer seam (`nq findings export` + `nq liveness export` DTOs)
is what Night Shift depends on. The deferred-run split is internal
orchestration: Night Shift's own slicing, not NQ's. NQ can keep
shipping other consumer-surface work (liveness reader, peek parity,
etc.) while this is open here.

## Trigger condition

This GAP was originally "pick up after the NQ liveness reader lands."
That landed in `474a1b1` (Night Shift-side consumer + pipeline gate).
Trigger condition satisfied. Starting this slice now.
