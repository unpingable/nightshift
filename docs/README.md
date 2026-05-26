# Night Shift Documentation

Where to start, by what you're trying to do.

## I want to use Night Shift

Night Shift does not yet have a stable operator surface. Operator-facing docs
will land under [`operator/`](operator/) when MVP exit is reached
(`nightshift watchbill run wal-bloat-review` against real NQ ops data,
governed by Governor for the action-authorized path). Until then,
[`operator/README.md`](operator/README.md) is a stub placeholder.

## I want to understand Night Shift

Start at `architecture/` for current design, then `theory/` for why-it's-this-shape.

- [`architecture/DESIGN.md`](architecture/DESIGN.md) — canonical architecture, three ladders, MVP scope
- [`architecture/FLOW-tolerability-horizon.md`](architecture/FLOW-tolerability-horizon.md) — A5 verdict flow, NS↔Governor pipe-through
- [`architecture/SCHEMA-agenda.md`](architecture/SCHEMA-agenda.md) — agenda declaration schema (v0 draft)
- [`architecture/SCHEMA-bundle.md`](architecture/SCHEMA-bundle.md) — context bundle schema (v0 draft)
- [`architecture/SCHEMA-packet.md`](architecture/SCHEMA-packet.md) — review packet schema (v0 draft)
- [`architecture/GAP-reack-doctrine.md`](architecture/GAP-reack-doctrine.md) — re-ack as mini re-triage; six-value disposition enum; nine invariants

Then theory:

- [`theory/DEPLOYMENT-MATURITY.md`](theory/DEPLOYMENT-MATURITY.md) — v1 local / v2 shared / v3 service ladder; constellation pattern

## I'm contributing

Read the architecture set above first. Then:

- [`working/gaps/`](working/gaps/) — open design questions and spec-shaped gap records (14 entries)
- [`working/decisions/`](working/decisions/) — candidate doctrine, working notes, cross-cutting design records, shipped-state ledger
- [`working/decisions/FEATURE-HISTORY.md`](working/decisions/FEATURE-HISTORY.md) — what landed, when, with evidence pointers
- [`working/decisions/AUDIT-BACKLOG.md`](working/decisions/AUDIT-BACKLOG.md) — audit-owed breadcrumbs

## Naming convention

Where a doc lives tells you what it's for. New docs land by lifecycle:

| Lifecycle | Location | Audience | Mutability |
|---|---|---|---|
| Operator-facing reference | `operator/` | users of NS | kept current, breaking-change-aware |
| Current design | `architecture/` | contributors | kept current, ratified |
| Positioning / why-this-shape | `theory/` | recruiters of the right audience | written-once-ish |
| Decision substrate / candidate / non-binding | `working/decisions/` | future-you, contributors | mutable until ratified, then either promoted or retired |
| Open design questions / gap specs | `working/gaps/` | future-you, anyone scoping work | mutable; may be retired |

Two discipline rules (imported from NQ, which imported from agent-governor — this is the third instance of the pattern):

1. **Promote into `operator/`, `architecture/`, or `theory/` only when ratified.** Until then, working notes sit under `working/`. Candidate doctrine that turned out load-bearing gets promoted; candidate doctrine that didn't pay rent gets retired.
2. **Don't promote a doc by duplication.** If a `working/decisions/` doc becomes architecture, move it (don't clone). If a gap is solved, mark it solved with a pointer to FEATURE-HISTORY; don't leave the gap doc as a parallel canon to the architecture.

The `GAP-` prefix is preserved across directories — it identifies "constitutional doc, not feature ticket," and the prefix is uniform across `architecture/`, `working/gaps/`, and `working/decisions/`. Location encodes lifecycle; prefix encodes intent.

## Shipped state vs. design records

**Gap docs are design records, not shipped-state ledgers.** Shipped state lives in [`working/decisions/FEATURE-HISTORY.md`](working/decisions/FEATURE-HISTORY.md). The split exists because front-matter status fields rot when forced to track ongoing reality — code lands incrementally, consumers change unevenly, and the gap doc has no mechanism to refresh itself.

A landed gap looks like:

```markdown
**Status:** shipped; see [FEATURE-HISTORY § SLICE_C_1](../decisions/FEATURE-HISTORY.md#slice_c_1)
```

Or for partial:

```markdown
**Status:** partial — pipe-through receipts landed, check_policy/authorize_transition open
**Current ledger:** [FEATURE-HISTORY § HORIZON_CLI_PIPE_THROUGH](../decisions/FEATURE-HISTORY.md#horizon_cli_pipe_through)
```

The detailed Shipped State narrative subsections that already exist in some gap docs stay — those are design-record content about what was deferred, what was discovered, where the boundary fell. The thing being moved out of the gap doc is the *ledger burden* in the front-matter status.

## Subdirectory readmes

- [`operator/README.md`](operator/README.md) — stub; reminder that operator-facing docs are still owed.

## Status vocabulary (gap docs)

Gap specs in `working/gaps/` carry one of:

- **`proposed`** — drafted, not yet being built
- **`specified, ready to build`** — spec hardened, implementation not yet started
- **`partial`** — some slice shipped, other slices pending (see FEATURE-HISTORY)
- **`built, shipped`** — fully implemented per acceptance criteria (see FEATURE-HISTORY)
- **`candidate`** — candidate doctrine sketch in `working/decisions/`, not in `working/gaps/`
- **`stub`** — placeholder pinning a boundary

Candidate-doctrine GAPs in `working/decisions/` carry their own status (e.g., `candidate, no internal witness` / `candidate, witness pair`).
