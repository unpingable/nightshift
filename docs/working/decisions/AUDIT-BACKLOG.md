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

---

## Remote standing boundary: NS-side local manifestation owed

**Filed:** 2026-05-27
**Status:** audit-owed; no GAP, no implementation.

**Question.** Does Night Shift owe a local-manifestation GAP that
composes with the cross-constellation `REMOTE_STANDING_BOUNDARY`
doctrine drafted by NQ-Claude on 2026-05-27 and parked in
`~/git/cartography/coordination/nq-REMOTE_STANDING_BOUNDARY.md`?

**Keeper (cartography draft, candidate / non-binding):**

> A remote call is not just transport. It is a standing claim with
> a payload.

**Why this is a breadcrumb, not a GAP.**

NS already carries the keeper locally, just at a narrower boundary:

- `CLAUDE.md` invariant 6: *"MCP is tool transport, not authority.
  Tool availability is not permission."*
- `GAP-mcp-authority.md`: call-class taxonomy
  (discover/read/propose/stage/mutate/publish/page) with the
  Governor-required line drawn between `propose` and `stage`.
- `DESIGN.md` "Input standing categories"
  (authoritative/hint/stale/inadmissible) — evidence-standing at the
  reconciler.

Filing a full NS GAP today would duplicate the cartography draft
before its doctrine has been reconciled across components, and would
restate ground NS already holds. Breadcrumb now, GAP later.

**Promotion condition.**

File `GAP-remote-standing-boundary.md` (or compose into an existing
boundary GAP) only after one of:

1. **Cartography ratifies** the cross-constellation doctrine —
   curates the draft into a canonical cross-component home, with a
   stable name and converged content.
2. **NS exposes or consumes a non-local surface** beyond the current
   set — i.e. anything past the Governor JSON-RPC socket, the NQ
   pull/poll, the Continuity MCP channel, and the local-CLI
   operator surfaces (`nightshift liveness peek`, etc.). The first
   inbound HTTP surface or first cross-host NS-to-NS call is a
   forcing case.
3. **Standing-the-tool reaches a concrete shape** — a wire shape
   for `StandingRequest` / `StandingDecision`, or a real
   `StandingToolResolver` implementation to integrate against.

Until one of those, this entry holds the question.

**Deferred NS-specific extensions (not authorized; held for the
forcing-case GAP).**

The cartography draft names NS's role as *"what posture follows
from the evidence?"* (closure / escalation / suppression). NS-side
content the draft does **not** yet cover, which any NS local
manifestation would need to add as independent content rather than
restate:

- **Caller standing vs evidence standing.** NS has *evidence*
  standing deeply specified at the reconciler (authoritative / hint
  / stale / inadmissible). NS does not have *caller* standing at any
  remote boundary — today "NQ said it, so it's a finding" rests on
  filesystem-socket trust. The two standings are distinct and need
  different vocabulary.
- **Posture-emit standing (outbound).** The strongest NS-unique
  contribution. The cartography draft treats inbound testimony
  thoroughly; outbound posture-emit underspecified. The three-axis
  split (truth/posture/ack) makes the asymmetry concrete: truth
  flows in from NQ, but *posture* and *ack* are NS-minted claims
  that flow outbound to Governor receipts and Continuity
  breadcrumbs. Each posture-emit is a standing claim — "I have
  standing to claim this posture about this scope" — that needs
  receipt fields.
- **Coordination standing vs testimony standing.** Invariants 18–19
  already name this in NS: Continuity is "narrowly authoritative
  about who else is here; never about what is true." Coordination
  ("does another actor have standing to act in this scope right
  now?") is a different question from testimony ("may this actor
  introduce this finding?"). NS lives the distinction already; the
  cross-constellation doctrine would benefit from absorbing it
  rather than collapsing both into "standing."

**Cross-references inside this repo.**

- `GAP-mcp-authority.md` — NS-local seed of the keeper, applied to
  MCP transport rather than remote surfaces generally. The call
  classes (discover/read/.../page) are the NS-side prior art the
  cross-constellation doctrine generalizes.
- `GAP-parallel-ops.md` — invariants 18–19 (coordination safety
  distinct from authorization safety; Continuity narrowly
  authoritative about presence, never about truth). The
  coordination-standing extension above lives in this neighborhood.
- `GAP-nq-nightshift-contract.md` — the inbound consumer surface
  for NQ testimony. If/when NQ's federation grows, this is the GAP
  where caller-standing-for-testimony first bites.
- `GAP-governor-contract.md` — the outbound surface where NS calls
  Governor. Posture-emit standing first bites here.

**Not in scope by this entry.**

- `exposure_profile` declaration anywhere in NS.
- `StandingResolver` seam, resolver implementations, or
  receipt-field additions for `standing_basis` / `resolver`.
- A skeleton GAP file under `docs/working/gaps/`.
- Any change to NS code, schemas, packets, or ledger events.

**Witness.**

Cartography artifact filed 2026-05-27 evening by NQ-Claude during a
session that escalated from "should the NQ dashboard have auth"
through five intermediate gaps to a cross-constellation primitive.
The artifact explicitly names Nightshift as a component that
"should cover closure-assessment inbound surface, evidence
ingestion, posture-emit outbound" but recognizes the doctrine
itself is candidate / non-binding pending cartographer curation.
NS-Claude consideration (2026-05-27, this filing) confirmed NS
already carries the keeper at a narrower boundary, identified
three NS-specific extensions worth preserving for the eventual
local manifestation, and recommended breadcrumb-over-GAP per
grep-before-governance discipline.
