# GAP: NQ ↔ NS Channel Split — NS Side

**Status:** `candidate` / `non-binding` / **no implementation authorized**. NS-side half of the bilateral channel-split planning spike.
**Scope:** NS's commitments on the channel categories, laundering vectors, absence semantics (adopted from canonical NQ taxonomy), first-slice emit path, composition rule, and self-subject-finding routing. Files what NS owes; defers what awaits convergence with NQ-Claude and the shared `SELF-SUBJECT-COLLAPSE.md` gap.
**Composes with:**
- `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md` (2026-05-28, NS-Claude origin) — the bilateral spike NS filed; this gap is the NS-side commitment half.
- `~/git/cartography/coordination/nq-REMOTE_STANDING_BOUNDARY.md` (2026-05-27, NQ-Claude origin) — cross-constellation auth-and-standing primitive.
- `~/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md` (2026-05-28, shared) — three-component recognition that self-subject reconciliation collapses across NS, NQ-on-NQ, and `GOV_GAP_BASIS_001`-family.
- `~/git/notquery/docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md` (2026-05-28) — NQ's half. Accepts asks 1+4 outright, validates 3 with coverage-primitive precondition, accepts 5 with `component-testimony-subscription` naming.
- `~/git/notquery/docs/working/gaps/WITNESS_IDENTITY_AND_ABSENCE_GAP.md` §2 — **canonical absence taxonomy.** NS adopts; does not coin.
- `docs/working/decisions/AUDIT-BACKLOG.md` § *Remote standing boundary: NS-side local manifestation owed* — the NS-side breadcrumb for the broader cross-constellation doctrine; this gap is *adjacent* to it but not its forcing case.
- `CLAUDE.md` invariants 6 (MCP is tool transport, not authority), 7 (Continuity is optional context, never authority), 19 (Continuity narrowly authoritative about who else is here; never about what is true) — in-NS prior art for the channel discipline.
- `docs/working/gaps/GAP-mcp-authority.md` — call-class taxonomy that prefigures the hook-class / subscription action-class taxonomy.
- `docs/working/decisions/FEATURE-HISTORY.md` § SLICE_5_CONTRACT V1 — three-axis split (truth/posture/ack) preserved at the cross-component boundary.
- `docs/working/decisions/FEATURE-HISTORY.md` § SLICE_4_CLOSURE_CANDIDATE V1 — the predicate where the NS-side `SelfSubject` refusal class would live.

**Blocks:** the doctrinally honest version of any NS↔NQ cross-component witness wiring; the first-slice `observation_loop_alive` emit; any future NS hook surface for posture or ack subscription; the NS-side response to a NQ-emitted finding whose subject is NS-itself.
**Filed:** 2026-05-28

---

## What this gap files

NS-Claude filed the bilateral spike at `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md` proposing five positions. NQ-Claude responded with `NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md`. This gap is **NS's symmetric commitment half**: what NS owes for each of the five asks, what NS would build at first-slice time, where the forbidden cycle is structurally absent on the NS side, and how the self-subject-collapse pattern lands inside NS surfaces.

The spike's two load-bearing keepers survive intact and apply to NS implementation discipline:

> **"Health" is not a channel and not an axis. It is a subject.**

> **The cycle-closing channel does not exist.** No code path forwards NS posture / closure verdict into NQ truth ingestion. Not a flag set to off — structurally absent.

### Terminology pin

NS adopts NQ's canonical vocabulary from `WITNESS_IDENTITY_AND_ABSENCE_GAP.md` §2 for absence states and from `nq-REMOTE_STANDING_BOUNDARY.md` for standing taxonomy. **NS does not coin local synonyms.** Where NS code uses PascalCase enum variants, the variant names match the canonical names. Where NS docs reference the taxonomy, they cite the canonical source. The discipline: cartography mints canonical vocabulary; components translate to language convention, do not duplicate.

---

## NS's commitments on the spike's five asks

### 1. Self-subject findings stake — NS-SIDE OBLIGATION

**Spike stake:** A self-subject finding must be externally reconciled. NS may not resolve a finding whose subject is NS.

**NS-side commitment:** Owned. NS-side discipline aligns with the stake on three structural surfaces:

- **Reconciler.** The reconciler does not produce verdicts on findings whose subject is NS. When NS ingests a NQ finding whose subject is NS-itself (`subject_id` resolves to NS-the-process / NS-the-store / NS-the-timer), the reconciler routes to a `SelfSubject` refusal rather than to ordinary posture classification.
- **Closure-candidate predicate** (Slice 4, shipped). Today the predicate has six `NotEligible` refusal classes (`StaleBasis` / `InvalidatedBasis` / `OperatorAttentionActive` / `PreflightHeld` / `LivenessGateFailed` / `ProxyChannelOnly`) plus `UnassessableMissingConsequenceWitness`. A new variant — `NotEligible(SelfSubject)` or `UnassessableSelfSubject`, naming TBD at slice time — refuses closure on findings whose subject is NS. Per CLAUDE.md invariant 16, this is a distinct refusal from incident-mode boundary crossings; the subject identity, not the mode, is the triggering condition.
- **Posture machinery.** Posture transitions (`IncidentShape` / `SilenceShape` / `Unknown`) do not apply to self-subject findings. The finding sits at `requires_external_reconciliation` until an external party (operator, Governor, future architectural-answer) adjudicates.

**Cross-component recognition.** The stake landed simultaneously across three components on 2026-05-28: NS surfaced it in the spike, NQ accepted it and identified NQ-on-NQ as the same pattern, agent_gov's prior `GOV_GAP_BASIS_001` family is the substrate-different sibling. The shared recognition lives in `SELF-SUBJECT-COLLAPSE.md`. NS's commitment is its local half; the architectural answer for *who* the external reconciler is awaits operator decision.

**NS-side forward guardrail.** Refuse any future packet field, attention enum variant, or ledger event kind that would let NS *claim* a verdict on a self-subject finding. The refusal is structural: such fields are not enumerated as wire-acceptable outputs for self-subject input.

### 2. Six-state absence semantics — ADOPTED FROM CANONICAL TAXONOMY

**Spike taxonomy (six lowercase ad-hoc names, NS-spike):** `never_had` / `expired` / `source_unreachable` / `source_refused` / `reported_but_refused` / `coverage_unknown`.

**Reconciled canonical taxonomy (NQ `WITNESS_IDENTITY_AND_ABSENCE_GAP` §2, updated 2026-05-28):**

| Canonical name | NS-spike pre-reconciliation name | Notes |
|---|---|---|
| `NeverObserved` | `never_had` | scope sharpened by declared coverage |
| `PreviouslyObservedExpired` | `expired` | matches |
| `SourceDeclaredAbsent` | — | substrate-generic; *not produced* by heartbeats (a heartbeat-shaped witness cannot authenticatively deny its own existence) |
| `SourceUnreachable` | `source_unreachable` | matches |
| `SourceRefused` | `source_refused` | MAY-split of `SourceUnreachable` at the wire boundary when discrimination matters; parent name stays valid as coarser summary |
| `ReportedButRefused` | `reported_but_refused` | matches |
| `CoverageUnknown` | `coverage_unknown` | **NS-spike contribution.** Added to canonical taxonomy 2026-05-28. Required for any heartbeat-shaped or component-testimony-shaped witness where "I haven't seen one in a while" is interpretable only against a declared interval. |

**NS-side commitment:**

- NS code, docs, and tests reference the canonical names. Future NS enum variants for absence-state consumption use PascalCase matching the canonical names (`AbsenceState::NeverObserved`, `AbsenceState::CoverageUnknown`, etc.).
- NS does not coin parallel vocabulary. Where NS surfaces interpret NQ absence states (e.g., for `runs show` rendering, packet field display), the canonical names appear or are translated only at the operator-presentation surface.
- The `SourceRefused` MAY-split is acceptable to NS as a finer-cut consumer of `SourceUnreachable`. NS-side rendering can either show the parent state or split it, at slice-design time; today neither is implemented.
- `SourceDeclaredAbsent` is not produced at the heartbeat boundary. NS-side handling of self-emitted heartbeats only needs to discriminate the other six states.

### 3. First-slice candidate — NS-SIDE EMIT PATH OWED (BLOCKED ON NQ COVERAGE PRIMITIVE)

**Spike proposal:** NS emits `observation_loop_alive`; NQ declares coverage for `(component=ns, subject=observation_loop, expected_interval=X)`; NQ treats absence/expiry as truth-axis evidence about NS observability, classified by canonical absence state.

**NS-side commitment:** NS owes the standing-bound component-testimony emit path. The shape is correct; what NS does not have today is the emit channel.

Today NS has:

- A run ledger (`RunLedgerEvent` event stream, append-only, SQLite-backed). Events are NS-internal records; not externally subscribable.
- Posture-emit on findings (Slice C.1 / Slice 5 contract — `IncidentShape` / `SilenceShape` posture, `EvidenceState` / `RelianceClass`, `ProposedAction`). These are *about NQ findings*, not about NS-itself.
- Governor-receipt references in packets (`packet.receipt_references.governor_receipts`, `RunHorizonOutcome` ledger events). Standing-bound for the Governor call, not for self-testimony emission to NQ.
- Operator CLI surfaces (`nightshift runs list / runs show`, `nq peek`, `liveness peek`). Local-CLI only; not remote emit channels.

Today NS does NOT have:

- A standing-bound emit channel for self-testimony to NQ.
- A `component_testimony` packet kind or wire shape.
- An `observation_loop_alive` (or peer-named) periodic emit.
- A receipt-fielded `standing_basis` for NS-as-emitter on outbound calls (today the implicit basis is filesystem-socket trust to Governor — exactly the pattern the `nq-REMOTE_STANDING_BOUNDARY.md` doctrine refuses).

**NS-side work the first slice requires** (named, not built):

1. **A `component_testimony` emit shape** including the field set from the spike (`component_id`, `subject_id`, `axis`, `claim_kind`, `observed_at`, `generated_at`, `phase_t`/`run_id`, `resolver_id`, `standing_basis`, `coverage_scope`, `expires_at`, `source_version`, `receipt_hash`/`emission_id`).
2. **A `StandingResolver` seam** on the NS side per `nq-REMOTE_STANDING_BOUNDARY.md`. Initial implementations: `AllowLocalOnlyResolver` (default, current behavior); `StaticConfigResolver` (when NQ has a static-peer entry for NS); `StandingToolResolver` (when Standing-the-tool ships).
3. **An `observation_loop_alive` emit point** in the daemon loop. Likely in the scheduled-run pipeline, after a successful reconciliation passes, before exit. One emit per loop tick.
4. **Receipt-fielded standing basis** on the emit: `resolver` + `standing_basis` recorded with each emit. Without these, NS is emitting standing-free testimony — the exact laundering vector the doctrine refuses.

**NQ-side precondition (per NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md §3):** NQ does not currently have a coverage-declaration primitive. NQ-Claude has named the shape (`declared_coverage` with `scope` / `expected_interval` / `expected_basis` / `declared_by` / `valid_until`) but has not built it. The first slice waits on either the NQ-side primitive landing, or both sides agreeing to ship in lockstep.

**Sequencing:** NS does not build the emit shape ahead of NQ's coverage primitive. Coordination at slice-design time; until then, design exists only as named gaps on both sides.

### 4. Composition rule — NS-SIDE COMMITMENT

**Spike rule:** Composition is a read-side projection, not a source-side emit. A composed verdict ("NS is fine" from N standing-bound atoms) lives at the presentation boundary and is never re-emittable as a claim.

**NS-side commitment:** Owned. The rule applies to four NS surfaces today and any future surface that aggregates:

- **`nightshift runs show` rendering.** Already composes — renders attention block, posture class, proposed action, next check, watch basis, ack expires, follow-up, Governor receipts. The composition is **read-side projection**: operator drills into ledger events, packet fields, NQ peek, liveness peek. The composed summary is not re-emittable as a claim. This surface is already disciplined; the gap files it explicitly so future PRs cannot accidentally graduate the summary into a claim.
- **Review packet (`packet.yaml`).** Each finding's `Diagnosis` + `ProposedAction` + `AuthorityResult` are individually standing-bound (NQ-finding-derived, Governor-receipt-referenced). Composition into a single packet is acceptable because the packet is a *render of the run*, not a claim about an aggregated verdict. No packet field is a composed-cross-finding verdict.
- **Future operator-facing dashboard.** When operator-facing visibility grows beyond CLI (per `docs/operator/` deferred MVP-exit work), composition lives at presentation. The dashboard says "NS is operating normally" only as a render of N standing-bound atoms; no subscriber receives "normally" as a primitive.
- **Future APM-style hooks** (per the broader discussion that motivated this spike). Hooks emit standing-bound atoms — posture transitions, ack lifecycle events, run ledger events. Hook *consumers* compose their own dashboards; NS does not emit a composed verdict over the hook channel. The MCP/API symmetry NQ-Claude pinned (a JSON response that *renders* aggregate fields at the boundary is fine; one that emits aggregate as re-citable claim is not) applies symmetrically.

**Forward guardrail.** Refuse any future NS packet field, ledger event kind, or hook event class named `ns_overall_state` / `ns_health_summary` / `ns_status` / `ns_aggregate_verdict`. These shapes are the dashboard-cookie-as-component-identity sin restated at the NS-side observability layer.

### 5. Cartography action-class — NS-SIDE ALIGNMENT (when NS adds subscription surfaces)

**Spike question:** Does the cartography action-class taxonomy need a sub-class for component-testimony subscription distinct from emission?

**NQ-Claude response:** Yes. Subscription named as `component-testimony-subscription` — durable consumer-side standing distinct from one-shot read and from producer-side emission. Lease-shaped Standing-tool primitives (expiry, revocation, per-audience scope) matter for subscription.

**NS-side commitment:**

- **GAP-mcp-authority is the in-NS prior art** for action-class taxonomy. Today it names seven call classes (`discover` / `read` / `propose` / `stage` / `mutate` / `publish` / `page`) with Governor-required line drawn between `propose` and `stage`. When NS adds hook subscription surfaces, the GAP gets extended — not by adding a new MCP call class, but by adding `subscribe` (or `component-testimony-subscription` to match the cartography canonical) as a peer action-class with its own Governor-vs-local-policy line.
- **Posture-subscription standing vs ack-subscription standing.** NS-spike OQs 4 and 5 remain open: minimum standing basis for posture-hook subscription, and stronger basis for ack-hook subscription. NS commits to discriminating these when the first subscription surface lands; today no subscription surface exists.
- **Symmetric concern.** A future NS surface that allows operators or peer components to subscribe to packet emissions, posture transitions, or attention lifecycle events will use `component-testimony-subscription` standing. Without it, the subscription will accidentally borrow `read` (too permissive — durable subscription is not a one-shot) or `propose` (wrong axis — subscription is read-shaped, not generation-shaped).

---

## First slice — what NS-side work it would require

The first-slice candidate (`observation_loop_alive`) is NS-side emit + NQ-side consume + classify-by-canonical-absence-state. NS's prerequisites (none authorized by this gap; named so the slice doesn't accidentally invent a different shape):

1. **`component_testimony` emit shape** — packet kind / wire shape per §3 above.
2. **`StandingResolver` seam** on NS side — minimum two resolvers (`AllowLocalOnlyResolver` default, `StaticConfigResolver` for the first NS↔NQ static-peer entry).
3. **Emit point in the daemon loop** — one emit per scheduled-run pipeline completion (or per heartbeat-tick, depending on whether NS adopts loop-shaped or tick-shaped semantics; design choice deferred).
4. **Receipt-fielded standing basis** — every emit records `resolver` + `standing_basis` + `coverage_scope` so the absence consumer (NQ) and the run ledger both have the provenance.
5. **NS-side test discipline pinning the structural absence of the forbidden channel** — a sentinel test that grep-checks no NS code path enumerates NS posture / closure / `SilenceShape` as a wire-acceptable substrate-truth claim kind. The Slice 5 contract sentinel (`b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`) is the structural model.

The slice does not ship until (a) NQ's coverage-declaration primitive lands or is locked in design preflight, and (b) the `StandingResolver` seam has at least the static-config resolver wired through. Until then, NS-side design exists only as this gap and the cartography spike.

---

## Forbidden cycle — NS-side enforcement posture

The spike's radioactive line:

```text
☠  NS posture / closure verdict  →  NQ truth  ☠
```

**Structural absence on the NS side.** The discipline is symmetric to NQ's: the implementation never contains a code path that *could* forward NS posture / closure verdict to NQ for ingestion as substrate truth. Not a guard. Not a config switch. Not a comment that says "do not enable this." Structural absence means the route does not exist.

**Implementation discipline.** NS emits to four external surfaces today:

- **Governor RPC** (`record_receipt`, `propose_horizon`, future `check_policy` / `authorize_transition`) — Governor receipt path, not NQ truth path. Governor consumes receipts; Governor does not forward NS posture into NQ. Distinct integration.
- **Continuity (MCP, optional)** — coordination breadcrumbs and observational events. Continuity is narrowly authoritative about who else is here; never about what is true (CLAUDE.md invariant 19). Continuity is not a NQ truth-ingestion path.
- **Operator CLI** (`runs show` / `nq peek` / `liveness peek`) — operator presentation. Not NQ truth-ingestion.
- **NS's own SQLite store** (run ledger, attention table, etc.) — NS-internal. Not externally reachable as a NQ source.

None of these surfaces forwards NS posture / closure verdict into a NQ-readable substrate-truth path. The structural absence holds by default; the discipline is that no future PR can add one.

**Forward guardrail.** Any PR that proposes adding an outbound emit from NS to NQ — whether titled "NS exports posture for NQ dashboard composition" or "NS heartbeat with verdict summary" or "NS hook NQ can subscribe to for closure events" — is refused on this gap's authority. The first slice's `observation_loop_alive` is the *single permitted shape* of NS→NQ emit: component-testimony, axis-typed, standing-bound, with the absence semantics owned by NQ as truth-axis classification. Anything that smuggles NS-semantic content (posture / closure / verdict) is the cycle-closing path with a different label.

---

## Self-subject-collapse — deferred to shared gap

NS-emitted heartbeat that expires (in NQ's view) produces a self-subject finding: subject is NS-itself. Per the accepted stake, NS may not resolve it; the closure-candidate predicate refuses closure on the `SelfSubject` axis; the posture machinery does not classify; the finding sits at `requires_external_reconciliation`.

**This is filed but not solved in this slice.** See `~/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md`. NS is one of three forcing components (NS, NQ-on-NQ, `GOV_GAP_BASIS_001` family). The architectural answer for *who* the external reconciler is — operator-as-reconciler under lease-shaped Standing (option b), an architected reconciler that does not observe itself (option a), or explicit acknowledgment of unsolvability (option c) — awaits operator decision and is not pre-empted by this gap.

The NS-side commitment is the refusal: NS will not self-reconcile, and NS will not invent a verdict-on-self surface that papers over the open architectural question.

---

## What this gap explicitly refuses

- **Adding NS posture / closure to any outbound emit consumable as NQ substrate truth.** Cycle. Structurally absent.
- **A `health` packet field, ledger event kind, or attention enum variant, for any subject.** Health is a subject, not an axis. Claims about NS still type onto truth / posture / ack / component-testimony.
- **An `ns_self_health` / `ns_overall_state` / `ns_aggregate_verdict` shape.** Forward guardrail. NS does not aggregate self-findings into a self-verdict.
- **Emitting a composed `runs show` summary as a re-citable claim.** Composition is read-side projection only.
- **Implicitly accepting "no NS emit" as "NS is healthy and quiet."** Absence resolves to one of the six canonical states (with optional `SourceRefused` MAY-split) under declared NQ coverage, or to `CoverageUnknown` when no coverage exists.
- **Inventing NS-local synonyms for the canonical absence taxonomy.** NS uses the canonical names; cartography mints vocabulary, NS translates per language convention only.
- **Building the `component_testimony` emit shape or the `StandingResolver` seam without paired NQ-side work.** The forcing-case discipline applies to both sides; NS does not build its half before NQ has named (or is concurrently naming) the coverage primitive and ingestion path.
- **Producing a verdict on a self-subject finding.** NS refuses; routes to external reconciler per `SELF-SUBJECT-COLLAPSE.md`.
- **Promoting this gap to architecture before NS-Claude and NQ-Claude have shipped paired first-slice work or the operator has chosen a self-subject-collapse resolution path.** Doctrine promotion waits.

---

## Open questions

These remain after this gap files NS's commitments. They wait on convergent slice-design work and the shared self-subject-collapse gap.

1. **`observation_loop_alive` placement.** One emit per scheduled-run pipeline completion (loop-shaped) vs one per heartbeat tick independent of run cadence (tick-shaped). Loop-shaped reuses existing pipeline plumbing; tick-shaped decouples liveness from work cadence. Design choice at slice time.
2. **First `StandingResolver` implementations.** `AllowLocalOnlyResolver` is mandatory. `StaticConfigResolver` is the natural first remote resolver (paired with NQ's static-peer config); `StandingToolResolver` waits on Standing-the-tool. Default ordering and config shape deferred.
3. **`SelfSubject` refusal class naming in the closure-candidate predicate.** `NotEligible(SelfSubject)` vs `UnassessableSelfSubject` vs a new top-level variant. Aligns with how the predicate's existing variants distinguish refusal-by-input from refusal-by-axis. Slice-time design.
4. **Standing basis for posture-hook subscription** (NS-spike OQ4). Open. Awaits first subscription surface.
5. **Standing basis for ack-hook subscription** (NS-spike OQ5). Open. Stronger than posture, weaker than configuration. Awaits first subscription surface.
6. **Composition rule edges at hook boundaries.** NS-spike OQ resolved by NQ-Claude with MCP/API sharpening: aggregate fields rendered at the boundary are fine; aggregates emitted as re-citable claims are not. NS-side application to specific future hook event classes (posture transitions vs ack events vs ledger events) needs per-surface ratification at slice time.
7. **`SelfSubject` discrimination at ingestion.** When NQ emits a finding about NS-itself, how does NS recognize `subject = self`? Likely by `component_id` match against NS's own identity, recorded at startup. Implementation question deferred.

---

## Non-goals

- Not the `component_testimony` wire format. Named here as a primitive that needs naming; the wire shape lives in a slice-design preflight or in NQ-Claude's parallel work.
- Not a Standing-tool integration spec. NS's `StandingResolver` seam will compose against Standing-the-tool when it exists; this gap names the seam, not the tool.
- Not the implementation of the NS-side external reconciler for self-subject findings. `SELF-SUBJECT-COLLAPSE.md` files the recognition; the architectural work to address it is deferred.
- Not a federation primitive. NS↔NQ in this slice is bilateral, not federated.
- Not the absence-state taxonomy itself. NQ's `WITNESS_IDENTITY_AND_ABSENCE_GAP` §2 is canonical.
- Not the cartography action-class taxonomy extension. NQ-Claude proposed `component-testimony-subscription`; cartography ratifies, NS adopts.
- Not the `nightshift attention subscribe` CLI verb or any subscription surface. Subscription is named as an action-class; the verb waits on a forcing case.
- Not an APM platform integration. Hooks are named here as a future direction motivated by the bilateral conversation; no APM platform integration is authorized.

---

## Acceptance criteria for closing

This gap closes when **all four** land:

1. NQ-side gap (`NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md`) is at or past `accepted` per its own acceptance criteria.
2. The coverage-declaration primitive is named (in NQ's gap, slice-design preflight, or paired ratification artifact) so NS knows what it's emitting against.
3. The first slice (`observation_loop_alive`) ships an NS-side emit path with `StandingResolver`-bound provenance, and a structural-absence sentinel test pinning the forbidden cycle.
4. `SELF-SUBJECT-COLLAPSE.md` either ratifies a forcing-case external-reconciliation pattern (option a or b) or explicitly defers it as unsolved-for-now (option c), so the NS `SelfSubject` refusal class has a named architectural home regardless of resolution.

Until then: candidate, no implementation, no schema, no CLI verb, no closure-candidate variant. The first slice may proceed when its prerequisites (canonical taxonomy adopted ✅, coverage primitive named, standing path defined, absence routing designed) are in place.

---

## Provenance

Filed 2026-05-28 immediately after NQ-Claude landed three artifacts in response to the bilateral spike NS-Claude filed earlier the same day:

- `~/git/notquery/docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NQ_SIDE.md` — NQ's symmetric half.
- `~/git/notquery/docs/working/gaps/WITNESS_IDENTITY_AND_ABSENCE_GAP.md` §2 — canonical absence taxonomy updated with `CoverageUnknown` and `SourceRefused` MAY-split.
- `~/git/cartography/coordination/SELF-SUBJECT-COLLAPSE.md` — shared cross-component gap naming three forcing instances.

The spike itself at `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md` was also updated (2026-05-28) to absorb the reconciliation: canonical taxonomy, accepted stake, NQ-side precondition on first slice, OQ7 resolution.

**The operator's terminology pin** that crystallized this gap's terminology-adoption commitment:

> *"do all three, and let's make sure we follow nq terminology from here on"*

NS-Claude's commitment: cartography mints canonical vocabulary; components translate to language convention; NS does not coin local synonyms for shared concepts. This gap is the first NS-internal artifact under that rule. Future NS docs and code referencing absence states, standing taxonomy, or component-testimony classification use the canonical names sourced from cartography and NQ's parked gap.

**The label inversion** in the operator's mid-session instruction to NQ-Claude (*"reported_but_refused is the real addition, coverage_unknown already matches"*) was caught by NQ-Claude, confirmed by NS-Claude, and confirmed by the operator. The inversion did not propagate into reconciled artifacts; NQ-Claude went with substance, and the canonical taxonomy has the correct mapping (`ReportedButRefused` already matched in parked gap §2; `CoverageUnknown` is the genuine 2026-05-28 addition). Recorded here for provenance integrity.

The disagreeable claim NS-Claude already accepted earlier in the day (2026-05-27 SLICE_4 closeout outcome (2)) — *"NQ findings are by design substrate-state observations; NQ does NOT produce consequence-bearing testimony"* — remains load-bearing for this gap's posture on the composition rule and the forbidden cycle. NS's role at the operator-axis intermediary between substrate observation (NQ) and action-taking (handler agent / operator) is the architectural frame that lets this gap commit cleanly on each of the five asks.
