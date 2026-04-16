# GAP: Storage Backend

> Status: stance fixed, contract sketched, implementation deferred.

Night Shift uses a storage abstraction for agendas, bundles, runs,
reconciliation results, packets, and local run-ledger receipts.

Backend choice is a deployment concern, not an ontology. **SQLite is
the default embedded backend. Postgres is the intended production
backend for shared deployments.** Other SQL backends are possible only
if they satisfy the contract below.

## Design stance

> SQLite is the default because Night Shift should be easy to run.
> Postgres is the production target because Night Shift must not
> confuse "easy to run" with "safe to coordinate at scale."

And:

> Backend choice must not change authority semantics.
> Scaling the store must not scale the trust assumptions.

Those two sentences are load-bearing. They keep storage from quietly
becoming a policy layer.

## Not a compatibility pledge

"We support every database" is how a small tool becomes a compatibility
support group. MariaDB / Percona / MySQL-compatible are *possible* if
they satisfy the contract — not roadmap promises.

Generic SQL is where joy goes to receive a compliance badge.

## Deployment mode → backend

```text
local / homelab / single-operator       SQLite
team / shared ops / multi-runner        Postgres
managed / federated / service mode      Postgres (SQLite local-agent only)
```

## Storage contract

### Required capabilities

A backend must provide:

```text
- transactional writes
- durable run ledger
- unique agenda/run IDs
- compare-and-swap (or equivalent) for run state transitions
- leases/locks for runner ownership
- append-only event log semantics
- byte-stable storage of receipt hash material
- migration/version tracking
- explicit failure on unsafe concurrency (no silent partial commits)
```

### Nice to have

```text
- JSON querying
- notification/subscription (e.g., LISTEN/NOTIFY)
- row-level security
- partitioning / retention
- read replicas
```

### Capability map

```text
SQLite:
  yes   — single-runner local deployments
  limited — multi-runner (process-level discipline only)
  no    — shared production without serious constraint

Postgres:
  yes   — production / shared control plane
  yes   — multi-runner with advisory locks + row locks
  yes   — operator-facing review surfaces and audit queries

MariaDB / MySQL:
  possible if contract is satisfied
  not first-class until someone needs it enough to suffer responsibly
```

## Concurrency invariant

> A run transition must be atomic and exclusive. Two runners must not
> be able to promote, execute, or finalize the same run concurrently.

- SQLite: process-level discipline plus transactions / `IMMEDIATE`
  write locks. Acceptable for single-daemon; fragile for multi-daemon.
- Postgres: row locks + advisory locks for runner ownership. Designed
  for this shape.
- Any backend: if the store cannot prove exclusive ownership of a run,
  **Night Shift fails closed.** No "probably fine" promotion without a
  provable lease.

## Store trait (Rust sketch)

The store owns **state**, not intelligence.

```text
trait Store:
    create_agenda(agenda)               -> agenda_id
    get_agenda(agenda_id)               -> agenda | None
    create_run(agenda_id, trigger)      -> run_id
    acquire_run_lease(run_id, runner)   -> lease | LeaseFailed
    transition_run_state(run_id, from, to, lease) -> Ok | Conflict
    append_run_event(run_id, event, lease)
    save_bundle(run_id, bundle)
    save_reconciliation(run_id, results)
    save_packet(run_id, packet)
    record_receipt_ref(run_id, receipt_ref)
    list_runs(filter)
    list_events(run_id)
```

Implementations:

```text
SqliteStore        v1 default
PgStore            v2 / shared
MemStore           test only; not a backend, a fixture
```

## Core tables / collections (MVP)

```text
agendas
runs
run_events
bundles
bundle_inputs
reconciliation_results
packets
receipts                  # references to Governor receipts + local run-ledger events
leases
backend_migrations
```

Later (not MVP):

```text
operator_approvals
escalations
artifact_refs
tool_invocations
```

## Do not let "supports Postgres" become "the MVP needs Postgres"

MVP rule:

> SQLite only, but with a storage trait/interface shaped so Postgres is
> natural.

When scale shows up, implement `PgStore`. Do not rewrite the project
because `rusqlite` leaked into every noun.

## Version / deployment roadmap

### Night Shift v1 — single-operator / local control plane

- SQLite default
- one daemon / one runner
- local agendas
- NQ pull only
- observe / advise only
- Governor integration for receipts + promotion ceiling
- packets, not mutation

This is dogfood. It proves the shape.

### Night Shift v2 — shared control plane

- Postgres default, SQLite optional for local agents
- multiple runners (real leases via advisory locks)
- approval queue
- operator UI / review surface
- persisted packets
- escalation destinations
- richer Governor integration
- optional Continuity
- self-hosted / owner-operated

Still not SaaS. A serious self-hosted ops substrate for "I got tired
of grep being the incident UI."

### Night Shift v3 — managed / federated / service mode

- multi-tenant control plane
- org/operator identities
- remote runners
- hosted NQ ingestion (if NQ ever exposes a hosted shape)
- policy packs
- shared dashboards
- compliance/audit export

This is where the danger music starts. The invariant stays:

> Hosted Night Shift may coordinate work, but authority should remain
> locally governed unless explicitly delegated.

Coordination can be hosted. Authorization remains governed by explicit
policy and receipt boundaries. Governor does not automatically
centralize because Night Shift does.

## Cross-constellation note

The SQLite-default → Postgres-production pattern likely applies to
**Night Shift, NQ, and Continuity** as a shared stance. They are all
state/evidence stores with the same embedded-vs-shared deployment
split.

Governor is the oddball. Governor is the authority plane. Backend
choice for an authority plane has different risk semantics — hosting
someone else's authority boundary makes you part of their trust chain.
That is not a deployment decision; that is a constitutional one.

Cross-project pickup: this pattern is worth noting as a shared
observatory-family stance, not just a Night Shift decision.

## Open questions

- Exact migration strategy between SQLite and Postgres (dump/restore
  vs. parallel-write vs. read-through). Probably dump/restore for v1 →
  v2, since deployment shape changes anyway.
- Where do bundle payloads live — inline in the store, or as references
  to a content-addressed blob store? (Probably references for large
  evidence; inline for small, configurable.)
- Should the run ledger and the application tables share a transaction,
  or be separated for append-only purity? (Probably shared
  transactionally, separated logically.)
