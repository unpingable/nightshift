# CALCULUS-CONFORMANCE.md — Night Shift ↔ Admissibility Calculus

> **PRE-CUTOVER / RETIRED — DO NOT CITE AS CURRENT RUNTIME CONFORMANCE.**
>
> This manifest's runtime-side symbols and modules (`bundle.rs`,
> `reconciler.rs`, `horizon.rs`, the Governor receipt path, and the rest of
> the Watchbill-era pipeline) were deleted or quarantined by the canonical
> runtime cutover. Its HELD/PARTIAL/BLOCKER rows describe that deleted
> runtime and have not been rehabilitated against current code. Retained for
> provenance only. The current runtime boundary contract is
> `docs/CANONICAL_RUNTIME_C1.md`.

---

**Status: CANDIDATE (unratified).** Authoring this manifest is routine
implementation. Ratifying the constitutional transition it describes —
from *formal reference* to *runtime governing specification* — is an
operator act. Until ratified, the correspondence claims below are
candidate claims: they name the obligation and record current evidence,
they do not certify conformance.

**Filed:** 2026-07-20. Companion to
`AUDIT-2026-07-20-calculus-runtime-conformance.md` (the findings) and
`~/git/lean` v14 `Admissibility.Calculus` (the shapes).

---

## 0. The constitutional transition (the whole point of this file)

There is a loophole this repository left open, and it was driven
through. It has a precise shape:

> **Lean claim (correct):** the calculus proves exact governed shapes
> and their *non-implications*. It explicitly does **not** prove that
> any runtime system conforms — "no runtime correspondence" is stated
> as a nonclaim throughout `WHAT-THIS-PROVES.md`.
>
> **The silent promotion (the bug):** *"Lean does not prove runtime
> conformance"* → *"therefore runtime conformance is not a
> requirement."*

Those are different statements. The first is epistemically correct: a
theorem does not govern Rust by osmosis. The second turns the calculus
into decorative doctrine — under it, exact correspondence degrades to
"resonance," and every mismatch becomes a footnote instead of a defect.
That frame is how an implementation can preserve the *general idea*
while flattening the *exact distinctions* and still feel compliant.

The correction is to make the transition explicit and four-part, in the
repository itself:

1. **Lean claim** — these are the exact governed shapes and the
   non-implications. (Owned upstream, in `~/git/lean`.)
2. **Runtime claim** — *this implementation claims correspondence to
   those shapes.* (Owned here, by this file.)
3. **Evidence obligation** — a type map, a refinement/representation
   map, executable preservation tests, and qualification receipts for
   every required distinction.
4. **Nonclaim** — Lean alone does not establish (2). The runtime repo
   must supply (3). A green theorem is not a conformant binary.

Once (2)–(4) are named, **"no correspondence exists here" is a blocker,
not interpretive freedom.** That is the sentence this file exists to
enforce.

### Proposed upstream companion (NOT applied here — lean repo is custody-affecting)

`WHAT-THIS-PROVES.md` may still be literally right but is
operationally incomplete: it states the nonclaim without stating the
obligation it creates for consumers. The proposed second half, for the
operator to place upstream:

> The formalization does not prove runtime conformance; runtime systems
> claiming conformance must supply an exact correspondence map and
> evidence that every required distinction survives implementation and
> transport.

Editing the released-v14 canonical claim doc is a custody-affecting act
and is left to the operator; it is drafted here, not committed to
`~/git/lean`.

---

## 1. Correspondence map (the manifest core)

Each row: the Calculus object (`Admissibility.Calculus.Core` /
`Spine` / `Crossing`), the runtime construct that plays it, and the
conformance status. Status is one of **HELD** (mapped + preserved,
with executable evidence), **PARTIAL** (mapped but a required
distinction is unenforced or diffuse), **BLOCKER** (no correspondence
exists at a guarantee-typed seam).

| Calculus object | Runtime construct in `nightshiftd` | Status |
| --- | --- | --- |
| `Claim` (the unit of judgment; includes origin where history matters) | The reconcile-time claim about a `FindingKey`: "the captured premise still holds / this deferred obligation is still tolerable." Carried across `ReconciliationResult` + target `FindingKey`, with captured basis (origin) in the persisted `Bundle`. | HELD |
| `Witness : Claim → Type` (data an auditor replays; multiplicity lives here) | **No first-class type.** Admission evidence for the accepted (`Committed`) case is diffuse: reconciliation status + `current_evidence_hash` + the frozen `ReconciliationAcquisition`, plus (for authorized deferral) the Governor receipt. | **BLOCKER** (§3.1) |
| `Refusal : Claim → Type` (family-native, no shared enum) | `packet::RefusalKind` (Liveness{Stale,Skewed}, BasisInvalidated, ToleranceBasisInvalidated, PreflightHeld), `governor_client::NonDischargeKind`, `errors::NightShiftError`. Family-native, per-seam, no unified spine — matches the Calculus. | PARTIAL (§3.2: `InvalidAgenda` catch-all) |
| `decide : (c) → Sum (Witness c) (Refusal c)` (returns evidence, never a bit) | `liveness::verdict_for`, `reconciler::reconcile_nq_input`/`adjudicate`, `reconcile_horizon::action_for`, `freshness::assess_freshness`, `posture_class::derive_posture_class` — all total, all return typed data. | HELD, with the caveat that every `refusal: None` on a refused claim is a point where `decide`'s totality fails to inhabit. The largest such hole (basis-invalidation) was closed 2026-07-20. |
| `witness_requires_standing` / `Standing : Claim → Prop` | `RelianceScope.valid_for` (Authorization/Proposal/Diagnosis/…) + `AuthorityLevel` ceiling + Governor authorization. | **BLOCKER** at three seams (§3.3): horizon-defer-on-stale, unenforced `governor_binding`, unwitnessed preflight. |
| `witness_preserves_custody` / `Custody : Claim → Prop` | Evidence-hash chains (`previous`/`current_evidence_hash`), `receipt_references.governor_receipts`, `ToleranceRecord.receipt_id`. | HELD (tolerance custody closed 2026-07-20). |
| `Obligation : Claim → Prop` (no generic law; family-native lifecycle) | `NonDischargeClaim`/`NonDischargeKind` (unsettled claims), `Packet.unsettled`, horizon tolerance lifecycle. Matches "obligation carries no generic law." | HELD |
| `Authority := Nonempty (Witness c)` — admission by witness, no other rule; `authority_has_no_multiplicity` | Authority is not a field NS can set: apply/publish posture requires a Governor receipt; NS caps at Advise without one. NQ witness-position metadata is render-only, never a gate input (no multiplicity counting). | HELD |
| **Stored-decision crossing** (`check` is the only native-evaluation boundary; decide once, store, derive) | `acquire_current` → frozen `ReconciliationAcquisition` (single live read), `adjudicate` pure over it; `HorizonReceipt.unsettled` retained "without re-deriving." One-shot reconcile enforced by `RunAlreadyCompleted`. | HELD |
| **`RefusalPacket` losslessness** (decode recovers the complete claim+refusal; no opaque `false`; no vocabulary laundering) | Refusal serialization on `Packet` (round-trips) + the closed `RefusalKind`. | PARTIAL → HELD for the wired variants; see §2 evidence. |

## 2. Transport boundaries and their preservation evidence

A distinction is only "preserved" if it survives *every* boundary it
crosses. NS's boundaries and the executable qualification receipts that
guard them:

| Transport boundary | Required distinction preserved | Qualification receipt (test) |
| --- | --- | --- |
| `Packet` → JSON (`store.save_packet`, render) | A refused claim carries its typed refusal, not only free-text `blocked[]`; the refusal survives storage. | `reconciler_pipeline::pipeline_renders_invalidated_when_finding_disappears` (asserts typed `BasisInvalidated` + `stored.refusal == packet.refusal`); `packet::refusal_kind_*_round_trips` |
| Refusal encoding injectivity | Distinct native refusals encode to distinct values (no catch-all). | `packet::refusal_kind_preflight_held_distinguishes_hold_outcomes`; `packet::refusal_kind_tolerance_basis_invalidated_is_a_distinct_tag`; `LivenessSkewed` omits threshold |
| Run ledger events (append-only) | Ledger is NOT authority; write-only in decision paths (RetroactiveLegitimation shape). | `Store::list_events` has zero production callers (structural); `ledger.rs` header fence |
| Governor receipts (`record_receipt`) | `receipt_id` reaches packet + ledger + tolerance record (custody). | `horizon_cross_run` (grant carries `fixture_receipt_0001`); receipt pipe-through to `receipt_references` |
| `tolerance_state` persistence (SQLite) | The grant round-trips and stays bound to its authorizing receipt. | `store::sqlite::tolerance_round_trip`; receipt_id custody assertion |
| capture → reconcile freeze | The 3am agent does not re-decide on 11pm inputs; one frozen acquisition clock. | `reconciler::adjudicate_is_pure_no_live_source_needed`, `adjudicate_is_deterministic_over_fixed_inputs` |
| imported basis → reconciliation | `ok_to_proceed` is NOT an authorization summary; custody time does not launder observation time. | `nq_integration::b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization` |

## 3. Open blockers ("no correspondence exists here")

These are not footnotes. Each is a seam where a required Calculus
distinction has **no** runtime counterpart today. Under the ratified
transition, each blocks a full conformance claim.

### 3.1 `Witness` has no first-class type
The accepted-claim receipt is diffuse (status + hashes + acquisition),
so the type-level laws that attach to `Witness` — `exclusive` (witness
⊕ refusal are mutually exclusive by construction) and witness
multiplicity living in `Type` — have nothing to bind to. NS gets
`Authority = Nonempty Witness` semantics operationally (apply/publish
needs a Governor receipt) but cannot *prove* witness/refusal
exclusivity at a boundary. **Needs:** a design record for a typed
admission witness, or an explicit refinement argument that the diffuse
representation is faithful. (Design-scoped; not a mechanical fix.)

### 3.2 `InvalidAgenda(String)` catch-all — narrowed 2026-07-20, residue tracked
The four inbound NQ-export parsing refusals (schema mismatch,
contract-version mismatch, two malformed-timestamp fields) were moved
out of `InvalidAgenda` into a typed `NqContractViolation { kind, detail }`
(`kind` a closed `SchemaMismatch`/`ContractVersionMismatch`/`MalformedField`),
so NQ's refusals arrive in their own stratum. The taxonomy is by
*category*, deliberately not pinned to today's wire schema — nq-ng
(`~/git/skunkworks/nq-ng`) is being rebuilt correctness-first.
**Residue (not folded, on purpose):** two adjacent non-NQ-contract
sites still use `InvalidAgenda` — `nq_canonical_key` (an NS-internal
`FindingKey` convention violation on the *outbound* NQ-query path) and
`parse_target_from_bundle` (a bundle-integrity issue). Folding these
into `NqContractViolation` would re-launder an NS-side / bundle-side
problem as an NQ-contract one. They want their own classification, not
this one.

### 3.3 `Standing` seams with no witness
Three guarantee-typed seams where authority is granted without checking
standing — each falsifies `witness_requires_standing` at that seam:
- **Horizon defer on stale evidence** (audit F2): `action_for` never
  sees `result.status`; a `Stale`/`Invalidated` finding can still be
  granted tolerance. Design question named in the audit: *missing
  conjunct vs documented freeze.*
- **`governor_binding` unenforced** (audit F4): the declared "Governor
  required above level X" gate is an empty `if`.
- **Preflight cleared from an unwitnessed boolean** (audit F5): a
  `--continuity-configured` flag stands in for an actual Continuity
  query; the clearance receipt attests a check that never ran.

### 3.4 Break-glass advertised but unimplemented (audit F6)
Invariant 18's "named, receipt-generating operator override" is
pointed at in an operator-facing packet string but has no verb, type,
or ledger event. The rung-7 `BreakGlass` instance is the spec when it
is built: origin-bound (who), history-bound (a ledger entry settlement
never cleans), refused as ordinary authority.

## 4. What landed 2026-07-20 (evidence, not resonance)

The audit's first pass closed three holes with executable receipts:
- **F1** — `BasisInvalidated` wired at both live sites; horizon gets a
  distinct `ToleranceBasisInvalidated`. Refused claims now reach the
  typed stratum (`funnel_never_opaque`).
- **F3** — tolerance grant bound to its Governor receipt and saved only
  after it exists (`witness_preserves_custody`).
- **Finding 6** — `PreflightHeld` carries its coordination outcome;
  three hold outcomes stop collapsing to one value (injectivity).

See `FEATURE-HISTORY.md` for the shipped-state ledger and the audit doc
for the full findings.

---

## 5. Ratification checklist (for the operator)

Ratifying this manifest as NS's runtime governing specification means
accepting that, going forward:
- [ ] A change that adds a seam without a correspondence-map row is
      incomplete, not shippable.
- [ ] A `BLOCKER` row is a release gate for any claim of conformance in
      the affected book, not a known-limitation footnote.
- [ ] Each transport boundary added must ship its qualification receipt
      (an executable preservation test) in the same change.
- [ ] The four-part transition (§0) is the standard artifact shape for
      every runtime repo that consumes the calculus (NQ next).
