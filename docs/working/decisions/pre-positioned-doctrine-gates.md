# Pre-positioned doctrine gates (from lean + tooltheory)

**Status:** non-binding pointer doc. Not a spec; not a roadmap; not authorization to build any of the triggered work.
**Filed:** 2026-05-27
**Source:** `~/git/lean` (formalized refusal kernels) + `~/git/papers/working/tooltheory/` (operational sketches)

## What this is

Catalog of doctrine that has been formalized or sketched in upstream projects and that Night Shift will need to honor **when NS reaches the implementation point that triggers each gate**. Pre-positioned here so a future NS session building toward a gate sees the doctrine without re-discovering it in the upstream projects.

Each entry names:

- **Trigger** — the NS implementation state that activates the gate
- **Doctrine** — what the upstream formalism establishes
- **What NS owes when triggered** — the concrete obligation
- **Provenance** — file references

## What this is NOT

- Not authorization to build any of the triggered work now
- Not a commitment to any specific implementation shape — the upstream sketch is sketch, not spec
- Not a substitute for reading the upstream formalism when the gate fires
- Not a roadmap; gates fire in the order NS hits them in real work, not in this doc's order
- Not an inventory of every cross-project reference — only the ones that are *NS-owed* under a named trigger condition

---

## Gate 1: Incident-closure predicate

**Status (2026-05-27): partially fired.** Slice 4 (`SLICE_4_CLOSURE_CANDIDATE V1` in FEATURE-HISTORY) implemented the *refusal* side: NS emits a `ClosureCandidate` verdict on every packet, refusing closure under six named blocker classes and emitting `UnassessableMissingChannelClassification` for IncidentShape findings without channel classification. The doctrine's full firing — distinguishing proxy-observation from consequence-bearing testimony — requires NQ-side claim-support classification that does not yet exist; until then, `EligibleForClosureReview` is unreachable. The seam is named in [`ADVISORY-nq-claim-support.md`](ADVISORY-nq-claim-support.md) (candidate; owner=nightshift, recipient=nq).

**Trigger.** NS implements incident-closure logic (today: NS surfaces evidence state and posture; it does not yet authorize *closure* of an incident).

**Doctrine.** Dashboard / proxy-channel normalization is not a settlement receipt for incident closure. A consequence-channel witness (customer-impact, downstream-effect) is required. "Quiet" on the observation surface does not equal "recovered" in substrate.

**What NS owes when triggered.** A closure-authorization predicate that explicitly distinguishes proxy-quiet from substrate-recovered. The predicate must reject closure on proxy-only evidence. The predicate is enforceable; it is not advisory.

**Provenance.**
- `~/git/papers/working/tooltheory/dashboard-quiet-is-not-recovery.md` — operational doctrine.
- Lean candidate name: `proxy_quiet_does_not_authorize_target_closure` (refusal-kernel family).

**Composition.** Sibling to NS's existing `silence_present ≠ incident_absent` invariant from Slice C.1 (`docs/working/gaps/GAP-silence-aware-posture.md`). The Slice C.1 invariant is about *evidence-shape* (silence findings don't imply incident-absent); this gate is about *closure-authorization* (dashboard normalization doesn't authorize closure). Two related disciplines, different surfaces; both refuse the same family of laundering.

---

## Gate 2: Proxy-shock workflow

**Trigger.** NS or NQ surfaces sudden-shock classification (a regime-change witness firing) that current code cannot distinguish from a target-witness event.

**Doctrine.** A regime-change observation ("we saw a shock") does not authorize action on the target ("we know what it means"). Workflow under proxy-shock must enter watch / defer mode, preserve uncertainty, and refuse premature closure.

**What NS owes when triggered.** Operational workflow states that distinguish regime-change witness from target witness. Under proxy-shock: defer / revalidate, do not advance posture, do not authorize action. Concrete shape TBD by NS at trigger time.

**Provenance.**
- `~/git/papers/working/tooltheory/proxy-shock-mismatch.md` — workflow shape.
- Lean candidate name: `proxy_shock_does_not_authorize_target_closure`.

**Composition.** Aligns with NS's Slice 5 contract: `Stale → advise(revalidate-only)`. The proxy-shock case is a *cause* for entering the Stale-shape revalidate path; the existing Slice 5 plumbing should carry it once NS recognizes proxy-shock as a Stale-trigger class. Likely no new state; new *recognition* feeding existing state.

---

## Gate 3: Consolidation interrupt (operational instantiation)

**Trigger.** NS builds out actuation (today: NS proposes, Governor authorizes, NS does not actuate). Specifically, when NS gains a capability to drive system-state-changing action against a substrate that accumulates settlement debt.

**Doctrine.** Lean's `ConsolidationDenial` (commit `57f4543` 2026-05-25) proves that *fluency* (rate of activity / freshness of observation) does not constitute a *settlement receipt*. **"Decay can clear the buffer without settling the debt. Audited discard is not rot."** Refusal-kernel family: `Surface ⇏ Substance`.

The Lean kernel is the *refusal* statement. The operational *interrupt* is a separate artifact:

- A forced non-actuating phase that clears settlement debt before further actuation or authority change.
- A health-index trigger `H_trigger(t) = B(t) + α · r_window(t) + β · A(t)` (buffer + rot pressure + authority risk).
- Four-stock dynamics over B (buffer) / K (settlement) / X (actuation) / R (residual rot).
- Schmitt-trigger controller with hysteresis to prevent flapping.
- Mode-specific safety bounds (different invariants under incident vs remediation vs architecture mode per `docs/working/gaps/GAP-incident-modes.md`).

**What NS owes when triggered.** The operational interrupt. Lean kernel proves the refusal is well-formed; NS implementation must operationalize the interrupt that *enforces* the refusal. Significant new machinery — not table-stakes for v1 MVP, which doesn't actuate.

**Provenance.**
- `~/git/lean/LeanProofs/Admissibility/ConsolidationDenial.lean` — refusal kernel (formal, sorry-free, annex-only). Header names NS as consumer (lines 1-100).
- `~/git/papers/working/tooltheory/consolidation-denial.md` — operational doctrine.
- `~/git/papers/working/tooltheory/consolidation-denial-formal-sketch.md` — Schmitt-trigger controller, four-stock dynamics, mode-specific invariants, illustrative Lean pseudo-code.

**Composition.** Cross-axis discipline; touches all three temporal axes from `project_tri_temporal_decomposition` (memory):
- **Metric-time:** the rate term `r_window(t)` and the buffer-decay dynamics.
- **Phase-time:** the consolidation interrupt as a forced lifecycle phase NS enters before actuation.
- **Operator-time:** the authority-risk term `A(t)` and the relationship to operator-authorized actuation windows.

The consolidation interrupt is plausibly the cleanest worked example of where all three axes carry load simultaneously — worth re-reading the tri-temporal memory alongside this gate when it fires.

---

## Gate 4: Cross-kernel refusal propagation (Wicket edge)

**Trigger.** Wicket (or any second upstream refusal-bearing tool) joins NS's dependency graph. Today, NS's only upstream refusal source is NQ (via finding-snapshot admissibility) and Governor (via policy verdict). Wicket is named in upstream doctrine but is not in NS's world.

**Doctrine.** When an upstream kernel refuses or emits `cannot_testify`, downstream consumers must treat *absence of authorization* as *denial-equivalent*. Refusal propagates across reconciliation cycles, not just within a single one.

**What NS owes when triggered.** NS reconciler treats Wicket refusal (or any new upstream refusal) as denial-equivalent in the same way it currently treats NQ Stale / Invalidated and Governor unreachable. The discipline is *already implemented* for the NQ → NS edge (Slice 5: `Stale → advise(revalidate-only)`, `Invalidated → emits packet`, no remediation; liveness fail → hold) and the Governor edge (unreachable → ceiling lowered to advise per `--no-governor`). When a Wicket edge appears, the same discipline applies — but the routing surface needs to be wired.

**Provenance.**
- `~/git/lean/LeanProofs/Admissibility/RefusalPropagation.lean` — formal theorem in `Annex.Nightshift` namespace (~lines 813-851, per upstream survey).
- `~/git/papers/working/tooltheory/cross-kernel-disposition.md` — "Where the temporal load lives" section (lines 100-109) names Wicket as the load-bearing seam.

**Composition.** This gate is *recognition-shaped* for the existing NQ / Governor edges (Lean now formally proves what NS already implements). It is *to-be-built* only for the future Wicket edge. The recognition matters because future NS work on similar refusal-bearing dependencies (NQ V2, NS-on-NS coordination, etc.) should be checked against the formalized propagation theorem before being implemented ad hoc.

---

## Composition with existing NS-side discipline

These gates do not introduce new doctrine families. They name *where* existing NS discipline must be honored when NS reaches specific implementation points.

- **Refusal propagation** (gate 4) composes with `docs/working/gaps/GAP-nq-nightshift-contract.md`, Slice 5 routing, and `--no-governor` ceiling-lowering. Existing implementation; future surface.
- **Consolidation interrupt** (gate 3) composes with `docs/working/gaps/GAP-incident-modes.md` (mode-specific bounds), the tri-temporal recognition (cross-axis machinery), and the eventual actuation surface NS does not yet have.
- **Closure predicate** (gate 1) composes with Slice C.1's `silence_present ≠ incident_absent` invariant and re-ack doctrine's `ack ≠ resolution` keeper.
- **Proxy-shock workflow** (gate 2) composes with Slice 5's `Stale → advise(revalidate-only)` path; likely a new recognition feeding the existing routing.

Cross-references to memory (load-bearing across sessions):
- [[tri-temporal-decomposition]] (`project_tri_temporal_decomposition`) — reading lens for gates 3 and 4 especially.
- [[shipping-vs-thinking-discipline]] — this doc is *recognition* of upstream doctrine, not *commitment* to build any of it now. Don't apply the shipping gate to the recognition.
- [[conservative-default-convergence]] — when reading these gates, the trap is to treat each as a self-evident "park until forcing case" without checking whether the gate is *already* firing in a real NS case.

## Provenance

Filed 2026-05-27 after a survey of `~/git/lean` and `~/git/papers/working/tooltheory/` requested by the maintainer. The user worked the upstream artifacts the same day; this doc is the NS-side mirror so future NS sessions don't have to re-discover the gates in the upstream projects when they get to the trigger points.

No NS code changed by this filing. No GAP promoted, no architecture ratified. Pure pointer doc; survives or gets retired based on whether any of the gates actually fires in future NS work.
