# Night Shift Documentation

Where to start, by what you're trying to do.

The current runtime correspondence record is
[`CANONICAL_RUNTIME_C1.md`](CANONICAL_RUNTIME_C1.md). It defines the sole
non-authorizing observation-cycle path and records the closed legacy cutover.
Documents that describe Watchbill, Wicket/WLP, MVP-A, classic Governor,
authority levels, or prose actions are historical.

## I want to use Night Shift

Start with the [operator quick start](operator/README.md) for the canonical
`nightshift cycle` build, observation, inspection, and restart surface.
Nightshift is not yet a packaged or operationally qualified service.

## I want to understand Night Shift

Start with the canonical runtime record, then use `architecture/` and
`working/` as explicitly historical design context.

- [`CANONICAL_RUNTIME_C1.md`](CANONICAL_RUNTIME_C1.md) — current runtime contract and production topology
- [`PRESENT_EVIDENCE_SUPPORT_SOURCE_GATE.md`](PRESENT_EVIDENCE_SUPPORT_SOURCE_GATE.md) — exact external support boundary and the unresolved production-source prerequisite
- [`GENERIC_PROJECT_PREDICATE_ATTENTION_V1.md`](GENERIC_PROJECT_PREDICATE_ATTENTION_V1.md) — operator-owned generic proposition/assurance attention, distinct-evidence recurrence, Pulse replay, and currentness boundary
- [`CONTINUITY_AUTHORITY_CARRIER_V1.md`](CONTINUITY_AUTHORITY_CARRIER_V1.md) — signed Standing warrant, pre-provider NQ commitment, and Nightshift applicability boundary
- [`architecture/DESIGN.md`](architecture/DESIGN.md) — historical pre-C1 architecture and MVP scope
- [`architecture/FLOW-tolerability-horizon.md`](architecture/FLOW-tolerability-horizon.md) — historical Governor-shaped horizon flow; temporal hold policy survives in the canonical store
- [`architecture/SCHEMA-agenda.md`](architecture/SCHEMA-agenda.md) — historical agenda draft
- [`architecture/SCHEMA-bundle.md`](architecture/SCHEMA-bundle.md) — historical context-bundle draft
- [`architecture/SCHEMA-packet.md`](architecture/SCHEMA-packet.md) — historical review-packet draft
- [`architecture/GAP-reack-doctrine.md`](architecture/GAP-reack-doctrine.md) — historical re-ack doctrine

Then theory:

- [`theory/DEPLOYMENT-MATURITY.md`](theory/DEPLOYMENT-MATURITY.md) — v1 local / v2 shared / v3 service ladder; constellation pattern

## I'm contributing

Read the canonical runtime record first. Then:

- [`working/gaps/`](working/gaps/) — open design questions and spec-shaped gap records (14 entries)
- [`working/decisions/`](working/decisions/) — candidate doctrine, working notes, cross-cutting design records, shipped-state ledger
- [`working/roadmaps/`](working/roadmaps/) — build ladders sequencing runtime surfaces toward deployable targets; mutable, retire if a slice never lands
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
| Build ladders sequencing runtime surfaces | `working/roadmaps/` | future-you, anyone scoping a slice | mutable; revise as slices land; retire if a slice never materializes |

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

- [`operator/README.md`](operator/README.md) — current canonical cycle and inspection quick start.

## Status vocabulary (gap docs)

Gap specs in `working/gaps/` carry one of:

- **`proposed`** — drafted, not yet being built
- **`specified, ready to build`** — spec hardened, implementation not yet started
- **`partial`** — some slice shipped, other slices pending (see FEATURE-HISTORY)
- **`built, shipped`** — fully implemented per acceptance criteria (see FEATURE-HISTORY)
- **`candidate`** — candidate doctrine sketch in `working/decisions/`, not in `working/gaps/`
- **`stub`** — placeholder pinning a boundary

Candidate-doctrine GAPs in `working/decisions/` carry their own status (e.g., `candidate, no internal witness` / `candidate, witness pair`).
