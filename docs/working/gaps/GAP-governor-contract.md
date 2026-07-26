# GAP: Governor Contract

> Status: identified, not specified
>
> *Note, 2026-07-26: the swap thesis below ("the authority implementation can
> evolve or be replaced without Night Shift rewriting") has since been
> confirmed in practice — the canonical authority office is now **AG ng**
> (`~/git/ag_ng`), and the four-office pilot ran its authorization path
> through AG ng → Docket with no Night Shift rewrite. `~/git/agent_gov`
> below is the legacy implementation; its RPC wire remains optional and
> disabled by default.*

Night Shift is loosely coupled to Governor at the code boundary, tightly
coupled at the protocol boundary. "Tightly coupled at protocol boundary"
needs an actual protocol boundary.

This document specifies the adapter Night Shift expects. Governor's
concrete implementation lives in `~/git/agent_gov`; Governor's Claude
session picks up spec changes noted here.

## Why an adapter (not direct imports)

- Governor is Python; Night Shift's daemon is Rust. A stable wire
  protocol is cleaner than cross-language bindings.
- Governor's internals are rich (180+ modules, 14k+ tests). Night Shift
  should depend on a narrow surface, not the whole surface.
- A swappable adapter means the authority implementation can evolve or
  be replaced without Night Shift rewriting.
- If the replacement implements the same authority contract, the
  separation survives.

## Core methods

Three methods cover the load-bearing cases.

### `check_policy(request) → verdict`

Night Shift asks Governor whether a proposed action is allowed.

Request:

```yaml
agenda_id: wal-bloat-review
run_id: run_...
actor: nightshift
requested_action:
  kind: mcp_call | tool_exec | state_mutation | publish
  tool_class: read | propose | stage | mutate | publish | page
  tool_id: ...
  arguments_hash: sha256:...
  blast_radius: single_host | multi_host | public
  reversible: true | false
bundle_ref: bundle://...
authority_level: observe | advise | stage | request | apply | publish
```

Response:

```yaml
verdict: allow | deny | require_approval | downgrade
reason: human-readable
obligations:                           # what night shift must do if allow
  - record_receipt
  - log_high_priority
  - require_fresh_approval
downgrade_to: advise                   # if verdict = downgrade
receipt_id: rcpt_...                   # produced even for deny
```

### `record_receipt(event) → receipt_id`

Night Shift asks Governor to emit an authority receipt for a lifecycle
event that requires audit. This is separate from Night Shift's own run
ledger.

Request:

```yaml
event:
  kind: agenda.promoted | action.authorized | action.applied |
        action.denied | action.verified | escalation.paged
  run_id: ...
  agenda_id: ...
  from_level: observe
  to_level: advise
  subject_hash: sha256:...
  evidence_hash: sha256:...
  policy_hash: sha256:...
```

Response:

```yaml
receipt_id: rcpt_...
receipt_hash: ...
```

Receipts are content-addressed (Governor's existing scheme). Same inputs
produce same `receipt_id`.

### `authorize_transition(run_id, from, to) → verdict`

Night Shift asks Governor whether a run may move up the authority ladder.

Request:

```yaml
run_id: ...
agenda_id: ...
from: advise
to: stage
evidence_summary:
  bundle_ref: ...
  admissible_inputs: [...]
  blocked_assumptions: [...]
```

Response:

```yaml
verdict: allow | deny | require_approval
reason: ...
required_approvals:
  - operator_approval                  # for apply+
  - fresh_reconciliation               # if bundle aged
receipt_id: rcpt_...
```

## Transport

TBD. Candidates:

- **JSON-RPC over stdio** — matches Governor's existing daemon shape.
- **JSON-RPC over Unix socket** — colocated deployments, slightly more
  robust than stdio.
- **HTTP** — if Governor ever exposes one; not today.

Content-Length framing (same as MCP). Matches Governor's existing daemon
conventions.

## Failure modes

- **Governor unreachable**: Night Shift may run observe/advise agendas
  with a loud warning (unsigned receipts). Any agenda whose
  `promotion_ceiling > advise` fails closed.
- **Governor denies**: Run records the denial via `record_receipt` and
  emits a packet indicating blocked state. No action taken.
- **Governor times out**: Treated as deny until a fresh response arrives.
- **Schema version mismatch**: Treated as deny; operator must reconcile
  versions.

## Degraded mode: `--no-governor`

Night Shift can run without Governor, but cannot promote force without
it. The `--no-governor` flag is a legitimate demo/local/propose mode,
not a fully-featured unsafe fork.

Startup behavior:

```text
$ nightshift --no-governor watchbill run wal-bloat-review

Governor unavailable.
Promotion ceiling lowered to advise.
Mutation, publication, paging, and staged actions disabled.
Receipts will be written to local run ledger only and marked non-authoritative.

Continuing...
```

Invariants in degraded mode:

- Every emitted packet is marked `governor_present: false` and
  `authority_result: null`
- Authority level cannot exceed `advise`
- Run-ledger entries include explicit `authority_plane: absent`
- Any agenda declaring `promotion_ceiling > advise` refuses to run in
  degraded mode (operator must either provide Governor or lower the
  ceiling)
- There is no "quiet Governor-absence" — the state is loud, visible,
  and surfaced in every artifact

## Fail-closed invariants

- Night Shift never assumes permission when Governor is silent.
- Night Shift never promotes above `advise` without an affirmative
  `check_policy` or `authorize_transition` response.
- Night Shift never re-uses a stale verdict across authority levels; each
  promotion step requires a new check.

## Capabilities Night Shift needs declared in Governor

Governor's policy engine already has a capability vocabulary. Night
Shift expects at minimum:

- `READ_FS`, `WRITE_FS` (scoped)
- `EXEC` (scoped)
- `NETWORK_EGRESS`
- `REPO_WRITE` (code mode)
- `CONFIG_WRITE`
- Custom: `MCP_CALL` (by class)
- Custom: `NIGHTSHIFT_PROMOTE` (by from/to level pair)
- Custom: `PAGE_HUMAN`

Obligation vocabulary Night Shift will request:

- `REQUIRE_HUMAN_APPROVAL` (apply+)
- `REQUIRE_EVIDENCE`
- `REQUIRE_FRESH_APPROVAL` (staged command aged)
- `LOG_HIGH_PRIORITY`
- `COOLDOWN` (agenda-level rate limiting)

## Pipe-through: `receipt_id` from `record_receipt`

> **Status: landed.** Captured as pipe-through debt 2026-04-28
> during the dogfood pass against agent_gov Commit B (23dba99);
> closed 2026-05-21 in commit 61a5789 after the verdict-observable
> audit found Defer was authorized but the receipt_id was
> unaddressable from NS output. The trip-wire in the live test
> fired exactly as designed.

When Night Shift defers under a horizon, it forwards an
`action.authorized` lifecycle event to Governor via
`nightshift.record_receipt`. Governor returns
`{receipt_id, receipt_hash}` (`nightshift_adapter.py:RecordReceiptResponse`).
Night Shift now captures both and surfaces them on two surfaces:

1. **Packet** — `Packet.receipt_references.governor_receipts:
   Vec<String>` carries the `receipt_id` for every Defer outcome on
   the target finding. Populated in `pipeline.rs::build_success_packet`
   from the `Vec<HorizonReceipt>` returned by `apply_horizon_outcomes`.
2. **Run ledger** — one `RunLedgerEventKind::RunHorizonOutcome` per
   outcome, payload carries `action`, `finding_key`, `basis_id`,
   `basis_hash`, `expires_at`, plus `receipt_id` + `receipt_hash`
   when a Governor receipt was emitted. `runs show` renders it via
   the existing event-row formatter — operator sees the receipt_id
   in the timeline.

The two non-horizon packet-construction sites (preflight hold,
liveness fail) still hardcode `governor_receipts: vec![]` —
correct, because those paths never reach `apply_horizon_outcomes`.

The dogfood test
`crates/nightshiftd/tests/governor_rpc_live.rs::live_horizon_defer_pipeline_round_trips`
now asserts the receipt_id is **non-empty** on both surfaces and
the two values agree. Trip-wire fulfilled.

### Adjacent finding: fixture/real validation drift

`FixtureGovernorClient` accepts any `basis_hash` string; the live
daemon's `HorizonBlock.__post_init__` enforces `sha256:<64 hex>` and
returns `-32602 Invalid params` on violation. Canonical fixture tests
in `horizon_packet_state.rs` and `reconcile_horizon.rs` use synthetic
basis_hashes like `"sha256:basis-abc"` that the real daemon would
reject. Not blocking — fixture tests test the apply layer's logic,
not the daemon contract — but the pipeline layer should not be the
first place this divergence shows up in production.

Possible future tightening: add a `cfg(debug_assertions)` validator
in `FixtureGovernorClient::record_receipt` that rejects malformed
hashes the same way the live daemon does. Out of scope for the
dogfood pass; named here so it doesn't get lost.

### Adjacent observation: `receipt_hash == receipt_id`

`agent_gov/src/governor/nightshift_adapter.py:670` returns
`RecordReceiptResponse(receipt_id=receipt.receipt_id,
receipt_hash=receipt.receipt_id)` — the two fields carry the same
value. NS treats both as opaque strings so this is harmless on the
wire, but consumers should not assume `receipt_hash` is independent
content-addressing of the receipt body. If it ever diverges from
`receipt_id` in a future Governor version, NS does not need to change
to accommodate.

## Open questions

- Who owns the agenda → policy binding? (Agenda declares `policy_id`;
  Governor resolves.)
- How does Night Shift learn Governor's policy vocabulary? (Capability
  listing RPC, or fixed versioned contract.)
- Do authority receipts live only in Governor, or also mirrored in the
  run ledger? (Probably mirrored as reference, with Governor canonical.)
- Can Night Shift request a policy dry-run before agenda capture to
  catch misconfiguration early? (Yes, probably `check_policy` with
  `dry_run: true`.)

## For Governor's Claude session

Pickup spec: this document declares Night Shift's expected adapter
shape. Governor can implement it as a thin module over existing
`evidence_gate.py`, `policy_engine.py`, `gate_receipt.py`, and
`daemon.py`. The three methods above map cleanly onto Governor's
existing primitives:

- `check_policy` ↔ `PolicyEvalResult` via `policy_engine.evaluate`
- `record_receipt` ↔ `gate_receipt.produce` with appropriate role
- `authorize_transition` ↔ a composite of `check_policy` +
  `record_receipt` with role=authority

Receipt roles Night Shift will emit:
- `measurement` (run-ledger events mirrored)
- `proposal` (packet produced)
- `authority` (promotion granted/denied)
- `recovery_plan` (proposed repair path)
