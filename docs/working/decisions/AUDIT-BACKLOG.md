# Audit Backlog

Audits Night Shift owes itself but has not yet performed.

Entries here are *audit-owed* breadcrumbs — a checked-in retrieval
hook so candidate audits don't decay into operator memory. This
file is not a GAP. Filing a GAP from an entry below requires running
the audit and identifying actual findings; this file holds the
question until then.

Each entry names: the audit question, the witness or motivation,
the promotion condition that would justify filing a GAP, and the
cross-references to related doctrine inside or outside this repo.

---

## Verdict boundary: observable, not constructible

**Filed:** 2026-05-07
**Status:** audit-owed; no GAP, no implementation.

**Question.** Can Night Shift's wire-format surfaces — packets,
bundles, and ledger events emitted to Governor on the minter side;
NQ findings and liveness artifacts ingested on the consumer side —
be shape-constructed by non-conforming processes without earned
provenance?

**Keeper:**

> **Verdict observable, not constructible.**

Night Shift's translation of the cross-system construction
discipline already filed as siblings in adjacent codebases:

- `agent_gov/specs/gaps/GOV_GAP_SEALED_OUTCOME_BOUNDARY_001.md` —
  *authority observable, not constructible* (filed 2026-05-06).
  Names the discipline at the AG layer and identifies that
  `AuthorizationVerdict` is defined with no production minter.
- `notquery/docs/gaps/TESTIMONY_OBSERVABLE_NOT_CONSTRUCTIBLE_GAP.md`
  — *testimony observable, not constructible* (filed 2026-05-07).
  Names the discipline at the NQ wire boundary; in-process `Finding`
  is type-sealed (no `Deserialize`); `LivenessArtifact` and
  `nq findings export` JSONL are bidirectional/shape-only.

NQ's gap explicitly punts the consumer-side answer to Night Shift:

> "Whether Night Shift's downstream actions (admission, scheduling)
> are sufficiently bounded to absorb a laundered finding without
> consequence is a Night Shift question; from NQ's side, the wire
> format does not constrain the answer."

**Cross-system primitive** (lifted verbatim from chatty's framing
during the AG/NQ derivation):

> The thing that matters must be emitted by the process that earns
> it, not constructed by whichever consumer finds the enum.

**Two-sided exposure (not yet audited).**

- *NS-as-consumer.* Can NS safely absorb laundered NQ-shaped
  findings, liveness artifacts, or admissibility records? Vector B
  in NQ's gap names this exposure structurally; NS has not audited
  downstream-action containment. The liveness wrinkle contract
  defends one *content* field (`freshness.fresh`); it does not
  defend *provenance* generally.
- *NS-as-minter.* Can packets, bundles, or ledger events be
  shape-constructed by a non-NS process and honored by Governor?
  NS has not audited the in-process construction discipline of its
  own emission types (Serialize/Deserialize derives, public minters,
  factory locations) nor the wire-format provenance of YAML/JSON
  flowing over the Governor RPC socket.

**Promotion condition.**

File a Night Shift sibling GAP only after an NS-side grep/audit
identifies one of:

1. A non-test, non-source-of-truth production minter of a
   wire-format type that consumers would treat as authoritative
   (the AG `AuthorizationVerdict`-with-no-minter shape, applied to
   NS verdict types).
2. A laundering vector at an NS wire boundary — a path where shape
   conformance is sufficient for downstream consumers to honor the
   bytes as NS-emitted.
3. A live case — an actual incident where laundered input
   produced a non-trivially-bounded NS action.

Until one of those, this entry holds the question.

**Cross-references inside this repo.**

- `GAP-rumination-boundary.md` — adjacent doctrine. Synthesis ≠
  standing is the memory-domain analogue of the same goblin door
  (consumer-finds-the-enum). The verdict-boundary discipline
  operates at the wire-format altitude rather than the synthesizer
  altitude. Different surfaces, same family.
- `GAP-autonomous-execution-boundary.md` — *trigger ≠ execution
  authority* is the action-domain analogue. The verdict-boundary
  discipline is the wire-format-domain analogue.
- `GAP-nq-nightshift-contract.md` — the three-axis split assumes
  NQ-shaped JSON is real NQ testimony. The audit-owed question
  bears directly on whether that assumption is value-typed or
  shape-conformance only.

**Not in scope by this entry.**

- Implementation, signing schemes, path-binding mechanisms.
- Schema migrations or constructor changes.
- A skeleton GAP file.
- Refactoring Governor RPC, the liveness consumer, the NQ consumer,
  or the packet emission paths.

**Witness.**

Two independent derivations converged on the construction-discipline
primitive from different substrates (Lean kernel + Ada probe via AG;
Rust wire-format audit via NQ). Night Shift's sibling exposure is
structurally plausible — and named explicitly in NQ's Vector B —
but unverified by NS-side audit. The audit-owed flag waits for the
audit to run before promotion.
