# Deployment Maturity Pattern

> Shared across the observatory-family constellation
> (Night Shift / NQ / Continuity). Draft; may migrate to a shared repo.

This document describes a deployment maturity curve common to the
state/evidence projects in the constellation. It is not specific to
Night Shift, but is hosted here until a shared location exists.

## The shared curve

```text
v1 local:       prove semantics
v2 shared:      coordinate safely
v3 service:     scale without changing authority assumptions
```

Each project travels the same three stops. Each stop has the same
shape:

### v1 — Local instrument

"Prove the substrate." Build the real semantics on a surface that
cannot be mistaken for enterprise vapor.

- SQLite
- Single operator
- Local daemon / CLI
- Minimal UI
- Receipts/events stored locally
- No multi-tenant assumptions
- Dogfood first
- Fail closed

| Project | v1 shape |
|---------|----------|
| NQ | Local observatory / finding store |
| Night Shift | Local agenda runner / packet producer |
| Continuity | Local/project memory + reliance ledger |

### v2 — Shared self-hosted substrate

The sweet spot. Enough moving parts that SQLite-plus-vibes becomes
annoying, but no vendor relationships required.

- Postgres
- Multiple runners / workers
- Approval queues
- Stable API contracts
- Operator UI
- Cross-project visibility
- Stronger leases / locks
- Richer audit history
- Optional org/team identity

| Project | v2 shape |
|---------|----------|
| NQ | Shared finding / control-plane backend |
| Night Shift | Shared agenda + run ledger + review queue |
| Continuity | Shared scoped memory / reliance system |

### v3 — Managed / federated / service mode

The "be careful, the sales goblins are near" tier.

- Multi-tenant
- Remote runners
- Hosted ingestion
- Org boundaries
- Policy delegation
- Audit / export
- Service-level guarantees
- Billing, therefore sadness

| Project | v3 shape |
|---------|----------|
| NQ | Evidence / finding service |
| Night Shift | Managed / federated agenda control plane |
| Continuity | Managed / federated reliance / context service |

## The invariant that makes the curve safe

> The constellation scales by moving coordination outward while
> keeping authority explicit.

Equivalently:

> Scaling coordination must not silently scale authority.

This applies to every project on the curve. Backend choice, deployment
shape, hosting model — none of these may implicitly expand what the
system is allowed to do. Authority stays local or becomes explicitly
delegated; it does not become SaaS landlord theology.

## Constellation discipline — what each project must not become

Each project has one thing it must resist, no matter how tempting the
product roadmap becomes:

```text
NQ              must not become authority
Night Shift     must not become autonomous command
Continuity      must not become truth
Governor        must not become cron
MCP             must not become permission
```

These are role invariants. A project that crosses its line becomes a
different (worse) system wearing its old name.

## Governor is the oddball

Governor does not share this maturity curve.

Governor is the authority plane. Backend choice for an authority plane
has different risk semantics — hosting someone else's authority boundary
makes you part of their trust chain. That is a constitutional decision,
not a deployment decision.

Governor's roadmap is therefore scoped differently: the authority layer
remains locally governed unless explicitly and receiptably delegated.
Coordination can be hosted; authorization should not ride along.

## Practical consequences

For any given project on the curve:

- **v1 → v2 migration** is a deployment upgrade and a backend
  transition. Semantics must not change.
- **v2 → v3 transition** is a trust decision, not just a scaling one.
  Each project must confirm which invariants hold across tenants
  before it crosses that boundary.
- **Cross-project integration** at v2+ depends on stable contracts,
  not shared internals. (See Night Shift's `GAP-governor-contract.md`
  and `GAP-nq-nightshift-contract.md` as examples.)

## Open questions

- Where should this document eventually live? (Observatory-family
  shared repo if/when one exists. Until then, mirrored across project
  docs or linked from each.)
- Does Continuity's v2/v3 shape look enough like NQ's that they can
  share a hosted deployment pattern? (Possibly; the reliance-class
  primitives and evidence primitives have overlap.)
- Is there a v0 before v1? (Probably not — v1 *is* the dogfood
  substrate. Anything earlier is a prototype, not a deployment.)
