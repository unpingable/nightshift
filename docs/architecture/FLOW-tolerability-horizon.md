# Tolerability Horizon — Flow and Seam Architecture

Scratch note for the subsystem landed across `8bdc54d`, `70904b0`,
`2363a1b`, `d8013c5` (2026-04-23). Written while the scar tissue is
fresh. Point is shape, not literature.

## Problem

Real adverse conditions are not necessarily immediate-action
conditions. "Sometimes bad is tolerable until T" must be
representable without either (a) hiding it from the operator or
(b) collapsing it into urgency.

The collapse stack this invariant defends against:

> badness → urgency → intervention → escalation

Each arrow is an inference that must be earned. Horizon is the
axis that prevents the collapse from firing at the consumer.

## Final split

### Night Shift

- **Declares** horizon locally. Source is producer-local policy
  (agenda rules, operator declaration, fixture manifest in tests).
  NOT fetched from Governor.
- **Reconciles** the four-way A5 distinction on each run:
  - `tolerated_active` — future expiry, matching basis → `Defer`
  - `expired_tolerance` — past expiry, matching basis + prior
    record → `EscalateExpired` with lineage
  - `basis_invalidated` — basis hash diverged from prior record →
    `EscalateBasisInvalidated` (wins over expiry)
  - `fresh_arrival` — no prior record, missing declaration →
    `ActOnVerdict(Missing)` (fail-closed)
- **Persists** tolerance state locally (`tolerance_state` table,
  keyed by `FindingKey`). The A5 write obligation: on `Defer`,
  write the grant so the next run can distinguish expired
  tolerance from fresh arrival. Without the write, the collapse
  stack fires at the consumer on the next run.

Code:
- `crates/nightshiftd/src/horizon.rs` — pure
  `action_for(block, now, prior)`
- `crates/nightshiftd/src/horizon_policy.rs` — local declaration
  surface (`HorizonPolicySource` trait, `FixtureHorizonPolicySource`)
- `crates/nightshiftd/src/reconcile_horizon.rs` — acquisition
  (`process_horizon`) + apply (`apply_horizon_outcomes`)
- `crates/nightshiftd/src/store.rs` — `ToleranceRecord` CRUD

### Governor

- **Records** horizon-bearing receipts. On NS `Defer`, NS emits
  `nightshift.record_receipt(event_kind=action.authorized,
  horizon={...})`. Governor archives it on the receipt chain; the
  horizon block is content-bound to its basis.
- **Not the lookup source** for live per-finding horizon at
  reconcile time.
  - `check_policy` returns verdict-only (no horizon field).
  - `receipts.detail` is debug/inspection.
  - `receipts.horizon_expiring_soon` is sweep/polling.
  - None front reconcile-time decisions.

Code:
- `crates/nightshiftd/src/governor_client.rs` — `GovernorClient`
  trait, `FixtureGovernorClient`, `JsonRpcGovernorClient`

### Continuity

Expiry is real wall clock. Tolerated-for-now does not become
durable legitimacy. Past expiry re-escalates to `now` until the
horizon is re-emitted — the grant does not fossilize into standing
permission.

## Seam correction (important)

The first Phase A draft labeled the NS-local declaration source
`GovernorSource::fetch_gate_receipt(finding_key)` — implying
Governor offered a horizon read path. A 10-minute Governor-side
preflight (`test_daemon.py`, `CheckPolicyResponse`) refuted that:
horizon is not returned from any RPC; it is attached by NS on
`record_receipt`.

Rename in `70904b0`:
- `GovernorSource` → `HorizonPolicySource` (NS-local declaration)
- `GateReceipt` → `HorizonDeclaration` (dropped Governor-receipt
  framing: `verdict`, `receipt_id`, `extras`)
- Separate `GovernorClient` introduced in `2363a1b` for the actual
  RPC surface

Lesson worth keeping: **label the seam for what it is, not what
you imagined the neighbor would offer.** Preflight against the
neighbor's shipped contract before writing the abstraction.

## Orthogonality (do not collapse)

- **Freshness TTL ≠ horizon expiry.** Freshness = evidence
  staleness. Horizon = tolerance window. A finding can carry
  fresh evidence of a condition that is tolerable until T.
- **Verdict ≠ horizon.** A `deny` verdict with `horizon=hours`
  still denies. Horizon declares consumer routing; verdict
  declares gate decision.
- **Local tolerance state ≠ archived receipt chain.** NS's
  `tolerance_state` answers "what does the next-run reconciler
  see?"; Governor's receipt chain answers "what did NS declare
  and when?" Both are necessary; neither subsumes the other.

## Wire notes

- `HorizonBlock.class` (Rust) serializes on-wire as `"kind"` to
  match Governor's field name (`#[serde(rename = "kind")]`).
- Governor enforces `basis_hash` format `sha256:<64 hex>`.
  Fixture tests accept informal hashes; the live test
  (`NIGHTSHIFT_GOVERNOR_SOCKET`) must use valid hashes.
- `JsonRpcGovernorClient` uses Content-Length-framed JSON-RPC 2.0
  over `std::os::unix::net::UnixStream`. Fresh connection per
  call. Matches `agent_gov/src/governor/daemon.py:read_message`.

## Current status

Horizon-audit-trail loop is closed end-to-end:

1. NS pulls declaration from `HorizonPolicySource` at reconcile.
2. Dispatches via `action_for(block, now, prior)` — pure.
3. On `Defer`: writes `ToleranceRecord` + emits `record_receipt`
   via `JsonRpcGovernorClient`.
4. Next run reads prior tolerance from its own store →
   four-way distinction.

158 tests across unit + integration, including env-gated live
test against a real Governor daemon.

## Still ahead (not blocking the core claim)

- Wire `check_policy` at action-propose points and
  `authorize_transition` at promotion points when NS has natural
  firing sites. (NS is advise-only today; no home for these yet.)
- `AgendaHorizonPolicySource` reading declarations from agenda
  YAML once the schema carries them. Fixture-only is fine until.
- Operator-facing packet rendering of the horizon outcome —
  currently the effect is in the store + receipt chain but not
  on the packet surface.

## Pointers

**Commits (2026-04-23):**
- `8bdc54d` — Phase A: horizon consumer + cross-run lineage
- `70904b0` — Seam repair: rename after preflight
- `2363a1b` — Phase B.1: `GovernorClient` trait + `record_receipt`
  wiring
- `d8013c5` — Phase B.2: `JsonRpcGovernorClient` over Unix socket

**Specs:**
- `~/git/agent_gov/specs/gaps/GOV_GAP_TOLERABILITY_HORIZON_001.md`
  (A5 persistence obligation)

**Load-bearing tests:**
- `crates/nightshiftd/tests/horizon_cross_run.rs` — four-way
  distinction end-to-end
- `crates/nightshiftd/src/governor_client.rs::tests` — JSON-RPC
  transport (mock-server round-trip, error surface, missing
  socket)
- `crates/nightshiftd/tests/governor_rpc_live.rs` — env-gated
  live daemon integration
