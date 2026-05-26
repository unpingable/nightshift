# Operator docs (placeholder)

**Status:** owed; not yet written.

Night Shift does not yet have a stable operator surface. This directory is reserved for operator-facing documentation that will land when Night Shift reaches MVP exit:

- `nightshift watchbill run wal-bloat-review` running against real NQ ops data
- Governor wired for the action-authorized path (already partial: pipe-through receipts shipped 2026-05-21; `check_policy` / `authorize_transition` still open — see [`../working/gaps/GAP-governor-contract.md`](../working/gaps/GAP-governor-contract.md))
- Operator workflows around capture / reconcile / packet review observed end-to-end

When the first operator doc lands here, the [top-level README](../README.md) routing block will be updated accordingly.

Owed (not exhaustive):

- Quickstart — install, configure NQ source + Governor socket, run one watchbill agenda end-to-end
- Reading a review packet — what `Attention.evidence_state`, `RelianceClass`, `ProposedAction`, and the freshness/horizon receipts mean operationally
- `nightshift runs show` — the timeline view, what each `RunLedgerEventKind` represents
- `nightshift liveness peek` — the NQ liveness gate, wrinkle visibility
- `--horizon-policy` / `--governor-socket` — the Tier-2 horizon path

This stub exists as the reminder that the work is owed. It is not authority to skip it.
