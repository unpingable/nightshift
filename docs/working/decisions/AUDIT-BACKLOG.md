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

**Adjacent cross-constellation artifacts (2026-05-28 update).**

- `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md` — bilateral
  planning spike (NS-Claude origin); names channel categories,
  forbidden-cycle line, canonical absence taxonomy adoption,
  composition rule, subscription action-class.
- `~/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md` — shared
  cross-component gap; three forcing instances (NS, NQ-on-NQ,
  `GOV_GAP_BASIS_001` family). Adjacent to the remote-standing
  breadcrumb because the external-reconciler architectural answer
  (when it arrives) will need standing semantics to bind operators
  or peer components as reconcilers — composing with the
  Standing-tool-shaped resolvers the cross-constellation
  remote-standing doctrine anticipates.
- `docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md` — NS-side gap
  filed against the bilateral spike. Names NS's commitments on five
  asks; symmetric to NQ's `NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md`.

---

## Self-subject-collapse: NS forbidden-cycle structural-absence audit owed

**Filed:** 2026-05-28
**Status:** **partially closed 2026-05-29** by sentinel test
`crates/nightshiftd/tests/forbidden_cycle_sentinel.rs`
(`forbidden_cycle_structural_absence_sentinel_ns_does_not_write_to_nq`).
The test covers the **subprocess-invocation** and **direct-DB**
outbound surfaces (the only NQ-bound outbound surfaces NS has
today). Two surfaces named in the NS-side GAP remain uncovered
because they do not exist in NS code: **Continuity MCP** (no MCP
wiring in NS today) and **operator CLI** (output direction is
stdout, never NQ — structural absence by direction, not by file
inspection). Test scope expands when those surfaces materialize.
Local-manifestation GAP at
`docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md` remains open
(this audit covered the code-level verification, not the gap's
first-slice obligations).

**Question.** Does NS code today contain any path — packet field,
ledger event kind, attention enum variant, MCP call, Governor RPC,
Continuity breadcrumb, or operator CLI output — that *could*
forward NS posture / closure verdict / `SilenceShape` into a
NQ-readable substrate-truth path?

**Keeper:**

> The cycle-closing channel does not exist — structurally, not by
> guard.

The bilateral spike and NS-side GAP commit NS to *structural*
absence of the forbidden cycle (`NS posture / closure verdict → NQ
truth`). The NS-side GAP's "Forbidden cycle" section claims the
absence holds across all four current NS outbound surfaces
(Governor RPC, Continuity MCP, operator CLI, NS-internal SQLite).
This audit converts the narrative claim into a code-level
verification.

**Cross-references (shared / cross-component artifacts).**

- `~/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md`
  (2026-05-28, NQ-Claude origin) — shared gap; three forcing
  instances. The architectural answer for *who* the external
  reconciler is awaits operator decision (option a, b, or c).
- `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md`
  (2026-05-28, NS-Claude origin) — bilateral spike with the
  radioactive forbidden-cycle line and the keeper *"health is a
  subject, not an axis."*
- `~/git/notquery/docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md`
  (2026-05-28) — NQ's symmetric forbidden-cycle commitment from
  the substrate-truth-ingestion side.
- `~/git/notquery/docs/working/gaps/WITNESS_IDENTITY_AND_ABSENCE_GAP.md`
  §2 — canonical absence taxonomy. NS adopts the six PascalCase
  states (with `SourceRefused` MAY-split); NS does not coin local
  synonyms.

**Promotion condition.**

This entry closes when:

1. A structural-absence sentinel test ships as part of the first-
   slice work (per NS-side GAP acceptance criterion 3). The test
   should be of the Slice 5
   `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`
   shape: pinning that updating the test deliberately is the signal
   that NS forbidden-cycle doctrine has moved.
2. Or — if first-slice work is deferred past a forcing case for
   the audit (a PR that proposes adding an outbound NS→NQ emit, a
   new packet field that could carry posture-as-truth, etc.) — the
   audit runs without the sentinel test and the entry is either
   converted to a sibling GAP or held closed with explicit findings.

**Cross-references inside this repo.**

- `docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md` § *Forbidden
  cycle — NS-side enforcement posture* — names the four outbound
  surfaces and the structural absence claim this audit verifies.
- `docs/working/decisions/FEATURE-HISTORY.md` § SLICE_5_CONTRACT V1
  — the sentinel-test discipline this audit's eventual test would
  follow.
- `docs/working/gaps/GAP-mcp-authority.md` — the action-class
  taxonomy that constrains what MCP calls can carry; relevant to
  the audit's coverage of the MCP outbound surface.

**Not in scope by this entry.**

- The first-slice implementation itself (NS-side GAP territory).
- The external-reconciler architectural answer (lives in
  `SELF-SUBJECT-COLLAPSE.md` awaiting operator decision).
- Any NS code change to introduce a forbidden-cycle path even
  temporarily — the audit is read-only / sentinel-test-only.
- Refactoring the `runs show` rendering or packet composition (the
  composition rule audit is a separate question; this audit covers
  emit, not render).

**Witness.**

The forbidden-cycle line is the only back-edge in the channel
table; every other channel is a forward DAG edge. Once the back-
edge closes, *"NS asserts SilenceShape"* → *"NQ substrate-truth:
silence"* → input to NS's next posture, which is the ouroboros the
bilateral spike exists to refuse. NS-side commitment is structural
absence; this audit verifies the commitment holds at the code
level.

**Closeout 2026-05-29.** The sentinel test ships at
`crates/nightshiftd/tests/forbidden_cycle_sentinel.rs`. It pins:
(1) no forbidden NQ subcommand string appears in `nq.rs` or
`liveness.rs` (15-entry forbidden list naming write-shape verbs;
intentionally broader than verbs `nq` exposes today, so the test
rejects new verbs of that shape); (2) every `Command::new` site in
`nq.rs`/`liveness.rs` is paired with an `.arg("export")` (read-only
discipline); (3) no NS source opens an NQ DB path via
`Connection::open` (NS's only `Connection::open` site is its own
store). Same sentinel shape as
`b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`:
the test is the doctrine surface, and updating it deliberately is
the signal that doctrine has moved.

---

## Lean annex alignment: Night Shift anti-laundering family

**Filed:** 2026-06-03
**Status:** retrieval handle; no GAP, no implementation, no doctrine
promotion. Cross-confirms shape only.

**Question.** Several Night Shift anti-laundering / freshness /
authorization-boundary doctrines now have independent Lean
scratch-annex analogues in
`~/git/lean/LeanProofs/Admissibility/`. Does this alignment
warrant any change to NS code, schemas, doctrine surface, or
external-facing claims?

Default answer for this entry: **no**. The entry exists so the
alignment is recoverable as a retrieval handle when a future NS
doctrine question needs sharpening; it is not a coronation.

**Keeper:**

> Night Shift has been independently rediscovering the same
> refusal geometry across runtime doctrine and Lean scratch
> kernels. **Formal resonance, not formal backing.** Useful, not
> sovereign.

**Relevant alignments.**

- `ProjectionLaundering.lean` ↔ `silence_present ≠ incident_absent`;
  `ok_to_proceed` is NOT an authorization summary; defer-preserving
  projection discipline. The paired theorems
  (`projection_launders_deferral` /
  `loss_aware_projection_blocks_deferral_laundering`) are the
  refusal-kernel form of the Slice C.1 boolean-laundering refusal
  trio and the Slice B sentinel
  `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`.
  NS's `silence_shape_rewrite` filter is `PreservesDefer` in code;
  consumer-side ack-laundering refusal is
  `PolicyRespectsDeferSignal`.
- `RetroactiveLegitimation.lean` / `AmendmentFragment.lean` ↔ NS
  invariant 4 *"Night Shift does not manufacture authority by
  logging itself"*; invariant 6 *"MCP is tool transport, not
  authority. Tool availability is not permission"*; invariant 7
  `committed` ≠ "true forever"; the 2026-05-28 live-data dogfood
  sentence *"Acknowledged did not imply closable"*; the
  verdict-observable-not-constructible breadcrumb above. T4 + T5
  (post-validation does not imply authorization) and the A1
  axiom (self-certifying amendment refused at the type level) are
  the formal shapes.
- `StaleEvidenceMerge.lean` ↔ NS invariant 2 *"Context bundles must
  be reconciled before execution. Stale context is not evidence"*;
  Slice 5 contract (`Stale → advise(revalidate-only)`,
  `Invalidated → emits packet`); imported-basis-freshness GAP
  (`captured_at` proves when NS saw the finding, not when the
  world was observed). The "honesty of the operator" note
  (staleness emerges from the gate's interaction of inherited
  horizon with reconciliation time, never stamped at capture) is
  the same discipline the NS reconciler enforces.
- `ContractionHinge.lean` ↔ silence-ack vs incident-ack laundering
  refusal (one warrant — ack — cannot be reused as two distinct
  claims — attention + closure); the "no per-input
  `requires_recheck` flag" rule (recheck is the gate, not
  metadata). Weaker alignment than the three above but real at the
  warrant-reuse level.

**Cross-references inside this repo.**

- `docs/working/gaps/GAP-imported-basis-freshness.md` —
  StaleEvidenceMerge alignment.
- `docs/working/gaps/GAP-silence-aware-posture.md` —
  ProjectionLaundering alignment (boolean-laundering refusal trio).
- `docs/working/gaps/GAP-mcp-authority.md` — AmendmentFragment
  alignment (self-certifying authority).
- `docs/architecture/GAP-reack-doctrine.md` — ContractionHinge
  alignment (warrant occurrence; ack ≠ closure).
- The "verdict boundary: observable, not constructible" entry
  above — RetroactiveLegitimation alignment (post-validation
  cannot supply its own missing precondition).

**Cross-references outside this repo.**

- `~/git/lean/LeanProofs/Admissibility/ProjectionLaundering.lean`
  (filed 2026-05-30).
- `~/git/lean/LeanProofs/Admissibility/RetroactiveLegitimation.lean`
  (filed 2026-06-02).
- `~/git/lean/LeanProofs/Admissibility/AmendmentFragment.lean`
  (closeout reframe 2026-06-02).
- `~/git/lean/LeanProofs/Admissibility/StaleEvidenceMerge.lean`
  (filed 2026-06-01).
- `~/git/lean/LeanProofs/Admissibility/ContractionHinge.lean`
  (charter re-alignment 2026-06-02).

All five are scratch annexes — not imported by `LeanProofs.lean`,
not on any 1.0 surface. Their own headers carry the same
non-promotion discipline NS uses for `working/decisions/`.

**Promotion condition.**

Promote only if a live NS case needs a sharper refusal form and
one of these annexes provides the precise kernel needed. Possible
shapes:

1. A reviewer asks NS to defend a laundering-refusal invariant and
   the prose form is insufficient — the Lean kernel's paired
   theorem structure gives the right shape to point at.
2. A new NS surface (a third posture class, a new wire format, an
   inbound testimony channel beyond NQ) reaches the design stage
   where the refusal geometry is unclear, and the existing annex
   names the boundary that NS needs to mirror.
3. The Lean annex is itself promoted onto the 1.0 surface and the
   alignment becomes a *cross-component reference* rather than a
   *cross-component coincidence*. (This is upstream of NS.)

Until one of those, this entry holds the alignment as a retrieval
handle.

**Not in scope by this entry.**

- Any change to NS code, schemas, packets, ledger events, tests,
  or doctrine surface.
- Any wiring, import, or build-time dependency on the Lean repo.
- Promotion of any of the listed alignments into a NS GAP or
  CLAUDE.md invariant.
- Any external-facing claim that NS is Lean-backed, formally
  verified, or implemented from the Lean kernel.

**Hard guardrail.**

Do not say Night Shift is Lean-backed, formally verified, or
implemented from the Lean kernel. The alignment is **formal
resonance**, not formal backing. Useful, not sovereign.

**Witness.**

Triggered by operator question 2026-06-03 ("any of the new annex
stuff for admissibility hits in nightshift?"). Spot-check of the
five annexes named above against NS doctrine surfaced direct
alignment with four NS invariant families and weaker alignment
with a fifth. The breadcrumb captures the recoverable shape;
chatgpt and NS-Claude independently agreed on the
retrieval-handle framing and the hard-guardrail line.

**Update 2026-07-20: promotion condition 3 fired; entry superseded as
a posture, retained as provenance.**

All five modules named above are now `PUBLIC-SHIPPED` /
`PUBLIC-EVIDENCE` (lean v13 custody migration), and the capital-C
`Admissibility.Calculus` root (Core / Spine / Crossing / Comparison +
BreakGlass terminal instance) is on the lean repo's **stable surface**
as of v14 (2026-07-18). Operator direction (2026-07-20): runtime
conformance is now the goal; the lean repo's "no runtime conformance
claim" is a fence for third-party consumers, not a ceiling on this
program. The alignment is therefore a *conformance target*, no longer
"resonance, not backing."

The full conformance audit, findings, correspondence-map assessment,
and fix ladder live in
`AUDIT-2026-07-20-calculus-runtime-conformance.md`. The hard
guardrail above survives in narrowed form: do not claim NS is
*formally verified* or *implemented from* the Lean kernel — a
conformance claim still requires an explicit correspondence map plus
evidence/refinement, and none is asserted yet.
