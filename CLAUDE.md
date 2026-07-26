# CLAUDE.md — Instructions for Claude Code

## What This Is

Night Shift: deferred agent work with receipts, reconciliation, and governed promotion.

Night Shift manages admissibility across time. Its job is not to decide whether action is authorized, but to prevent old observations, stale plans, and deferred work from silently becoming current authority.

Schedules and resumes *intent* — not commands — under declared policy, with
context bundles that revalidate their own premises before execution.

Night Shift preserves deferred operational obligations, prevents unresolved work from masquerading as resolved, and detects when the kind of work has changed.

## What This Is Not

- Not cron with opinions. Cron executes blindly; Night Shift reconciles before acting.
- Not Governor. Governor is the constitutional layer (authority/policy/receipts). Night Shift is the executive calendar (intention/scheduling/promotion).
- Not an autonomous agent. The agent produces proposals under constraint. It does not get to touch production because it used confident adverbs.

## Invariants

1. No mutation without Governor authorization. Night Shift proposes; Governor permits.
2. Context bundles must be reconciled before execution. Stale context is not evidence.
3. Recheck is the gate, not metadata. Every input passes through the Reconciler by virtue of the pipe it enters on. No per-input `requires_recheck` flag.
4. Every run produces ledger events; Governor emits authority receipts. Night Shift does not manufacture authority by logging itself.
5. Lifecycle, authority, and artifact are three distinct ladders. Do not conflate them.
   - Lifecycle: `capture → reconcile → plan → review → run → verify → record`
   - Authority: `observe → advise → stage → request → apply → publish → escalate`
   - Artifact: `receipt | packet | diff | report | page | publication_update`
   A run moves through lifecycle phases but cannot exceed its authority ceiling. `escalate` is a **terminal run posture**, not a peer action of `apply`/`publish`; destinations that realize an escalation (`page`, `notify`, `request_approval`, etc.) are implementations, not authority levels. `page` as an MCP call class is transport, not posture. See `GAP-escalation.md`.
6. MCP is tool transport, not authority. Tool availability is not permission.
7. Continuity is optional context, never authority. Inputs enter as `observed`; the Reconciler may grant `committed` status for a declared scope. `committed` means "accepted for this run under this scope," not "true forever."
8. Missing intelligence dependencies (Continuity, MCP, LLM) must never increase authority. Missing safety dependencies (Governor, evidence adapter, run ledger) lower the promotion ceiling or fail closed.
9. Diagnostic review (self-check / conference) may reduce confidence, downgrade promotion, or require escalation. It may not raise the promotion ceiling or authorize force.
10. Drive to resolution ends where standing begins. Night Shift pursues resolution only while the next step remains within evidence, authority, scope, and budget. Once any boundary is crossed, the run escalates.
11. If the next diagnostic step changes the system, stop. Read-only disambiguation is fine; mutation as disambiguation is not.
12. Backend choice must not change authority semantics. Scaling the store must not scale the trust assumptions. SQLite is v1 default; Postgres is v2 production; the storage contract is the boundary (see `GAP-storage.md`).
13. A run transition must be atomic and exclusive. If the store cannot prove exclusive ownership of a run, Night Shift fails closed.
14. Operator intent has a half-life. Attention state (acknowledged, investigating, silenced) is distinct from evidence state and must carry a TTL or an explicit reason. Attention state never raises authority. See `GAP-attention-state.md`.
    - Silence is not handling.
    - Ack is not closure.
    - Suppression needs an expiry or a reason.
15. Continuity availability is not Continuity use. The reconciler queries shared substrate for concurrent activity in declared scope by default; the run ledger writes observational breadcrumbs to Continuity at surprise / partial / escalation / completion events, not only at run end. Hooked in ≠ used. See `GAP-parallel-ops.md`.
16. Incident modes (incident / remediation / architecture) are distinct and do not share a success condition. A run declares mode, objective, allowed actions, and exit criteria; crossing mode bounds is an invariant breach. Incident state, remediation state, and architectural-followup state are tracked separately. Stabilized ≠ remediated. Deployed ≠ verified. See `GAP-incident-modes.md`.
17. Protected services (observation-critical, control-plane-critical) resist casual turn-down regardless of promotion ceiling or policy verdict. A proposed action that disables a `protected` service requires explicit operator confirmation in all modes. See `GAP-incident-modes.md`.
18. Coordination safety is distinct from authorization safety. Continuity is optional for authorization safety (missing Continuity never raises authority) but required for coordination safety in named risky classes — shared-infrastructure ops, topology/config/publisher/source changes, mode transitions, or protected-class scopes. For these classes, a Continuity preflight is a guardrail: the run cannot leave capture phase without preflight clearance or a named, receipt-generating operator override. The failure mode is not forgetting; it is failing to ritualize recall. See `DESIGN.md` (Continuity role) and `GAP-parallel-ops.md`.
19. The coordination channel is narrowly authoritative. Overlap *existence* and *classification* may gate coordination; breadcrumb *contents* remain `observed` / `hint`. Continuity is authoritative about who else is here; Continuity is never authoritative about what is true.
20. Backup and restore for continuity-bearing workloads are first-class operational truths, not operator folklore. Backup scope must be explicitly declared (never "the whole box"); off-host destinations are required for continuity protection; restore drills are required and non-live by default; SQLite captures must use a safe method. Continuity (cross-run state / coordination / memory) and backup (disaster survival of declared protected state) are related concerns but MUST NOT be collapsed. See `GAP-backup-restore.md`.

## Quick Start

```bash
cargo build
cargo test
cargo run --bin nightshift -- --help
```

Tier-2 horizon path (cross-run tolerance + Governor receipts)
requires `--horizon-policy <path>` AND `--governor-socket <path>`
in pair. Without both, the pipeline runs in Tier-1 observe/advise
mode against the NQ source + liveness gate.

## Project Structure

- `crates/nightshiftd/` — single Rust crate (daemon + library);
  agenda, bundle, reconciler, pipeline, NQ + liveness consumer,
  freshness (Slice B), posture_class (Slice C.1), horizon,
  horizon_policy, reconcile_horizon, governor_client, store,
  packet, main.rs CLI entry. ~216 tests.
- `tests/fixtures/` — agenda + NQ fixtures used by integration tests
- `docs/` — lifecycle-organized documentation. See [`docs/README.md`](docs/README.md) for the canonical inventory and routing.

```
docs/
├── architecture/     # ratified design: DESIGN, FLOW-tolerability-horizon, SCHEMA-*, GAP-reack-doctrine
├── theory/           # positioning / why-this-shape: DEPLOYMENT-MATURITY
├── operator/         # placeholder; operator docs owed at MVP exit
└── working/
    ├── gaps/         # open spec-shaped GAPs (14 entries)
    ├── decisions/    # candidate doctrine, working notes, FEATURE-HISTORY ledger, AUDIT-BACKLOG, planning memos
    └── roadmaps/     # build ladders sequencing runtime surfaces toward deployable targets
```

The Rust+Python language split in `architecture/DESIGN.md` is preserved as the
architectural target for **Code mode** (deferred coding sessions
that produce reviewable diffs). v1 is effectively Rust-only —
`pyproject.toml` is a placeholder; no `src/nightshift/` Python
package exists.

**Shipped state lives in `docs/working/decisions/FEATURE-HISTORY.md`, not in gap-doc front-matter.** Gap docs are design records; the ledger answers "what's actually in production, with what evidence."

### Doctrine pointers (load-bearing across sessions)

These one-liners are the doctrine the inventory used to carry inline. They survive document moves; if you need the full spec, follow to the linked doc.

- **Re-ack doctrine** (`architecture/GAP-reack-doctrine.md`) — re-ack is mini re-triage with typed disposition, not a courtesy tap. Six-value enum frozen (advanced / unchanged-waiting / blocked / handed-off / escalated / resolved). Nine invariants pin ack-lineage scope, re-ack-on-context-change, silence-ack ≠ active-ack, stale-ack ≠ invalidated-ack, ack ≠ resolution/safety/freshness/truth. Class-aware disposition extension is a non-goal of v1 doctrine; design-space explored in `working/decisions/class-aware-disposition-design-space.md`.
- **Imported basis freshness** (`working/gaps/GAP-imported-basis-freshness.md`) — NQ lifecycle/custody time cannot launder upstream observation time. `captured_at` proves when NS saw the finding, not when the world was observed. `FreshnessBasis` enum + five reconciliation cases. Closeout pins "**`ok_to_proceed` is NOT an authorization summary**" — guarded by sentinel test `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`.
- **Silence-aware posture (Slice C.1 landed)** (`working/gaps/GAP-silence-aware-posture.md`) — NQ owns truth/classification, NS owns posture + ack. Three anti-laundering invariants: `silence_present ≠ incident_absent`, `acked_silence ≠ acked_incident`, `no_new_evidence ≠ resolved`. `PostureClass::{IncidentShape, SilenceShape, Unknown}` with envelope-presence derivation rule.
- **Slice 5 contract** (load-bearing primitive) — three-axis split: truth (NQ-owned) / notification posture (NS) / ack obligation (NS). `Stale → advise(revalidate-only)`. `Invalidated → emits packet`. Every subsequent slice composes onto this.
- **Horizon CLI + pipe-through receipts (landed)** (`working/gaps/GAP-governor-contract.md` partial) — Tier-2 reachable via paired `--horizon-policy` + `--governor-socket`. Receipt-id from Governor reaches `packet.receipt_references.governor_receipts` and `RunHorizonOutcome` ledger events. `check_policy` / `authorize_transition` (the remaining two `nightshift.*` Governor methods) still open.
- **Constellation maturity ladder** (`theory/DEPLOYMENT-MATURITY.md`) — shared v1 local → v2 shared → v3 service curve; NS / NQ / Continuity share it, Governor does not.

### Candidate doctrine (not ratified; `working/decisions/`)

- **Workflow Routing Boundary** — "Route by claim type, not by tool identity"; six keepers; claim-type taxonomy.
- **Architectural Promotion Boundary** — roadmap ≠ authorization; three action classes (containment / tuning / promotion).
- **Solution-Family Exhaustion** — "repeated repair inside the same context is evidence against the context"; mitigation chain + bucket migration; detect, do not resolve.
- **Lesson Distillation Boundary** — meta-meta: rules under which NS extracts candidate motifs without applying them as authority; five-stage lifecycle, no skipping. Sharpening: *transitions, not nouns*.
- **Autonomous Execution Boundary** (real witness: labelwatch) — trigger ≠ authority; alarm-vs-remediation policy split; "an alarm may wake the operator, it may not become the operator."
- **Rumination Boundary** (no internal witness) — synthesis ≠ standing; "a dream may propose memory; it may not become memory without standing."
- **Narrowing Posture Transition** (witness pair + live counterexample) — "the same narrowing move can finish one workstream and abort another"; "narrow after contact, do not narrow before contact"; explicit refusal of `work_phase` / `narrowing_role` enum fields.
- **Slice Cycle** — work-cycle cadence; reconciliation-vs-lifecycle-ladder open question pending review.

### Registries (`working/decisions/`)

- **FEATURE-HISTORY** — shipped-state ledger (what landed, when, with evidence).
- **AUDIT-BACKLOG** — audit-owed breadcrumbs only. Not GAPs, not accepted doctrine, not implementation plans. Promote an entry only after repo-local audit finds an actual construction/provenance boundary or laundering vector.

## Conventions

- License: Apache-2.0
- Rust: stable toolchain, clippy clean, no unsafe without justification
- Receipts: content-addressed, append-only, deterministic
- Python tooling is deferred until Code mode lands (see DESIGN.md).

## Neighboring projects

- **Authority (role: "Governor")** — canonical office: **AG ng** (`~/git/ag_ng`, Rust exact-work authorization/issuance). Legacy implementation: `~/git/agent_gov` (Python, JSON-RPC daemon) — retained here only for bounded-diagnostic drill helpers and the optional, disabled-by-default RPC wire
- **NQ** (`~/git/nq`): observatory/failure-domain classifier — Rust, HTTP API
- **Grid Dependency Atlas** (`~/git/grid-dependency-atlas`): publication-mode target
- **Continuity**: cross-project context via MCP (workspace: observatory-family)

## Don't

- Don't let Night Shift become the authority layer. It schedules intent; Governor authorizes force.
- Don't auto-repair without explicit promotion. Start propose-only; mutation comes later via allowlists.
- Don't treat context bundles as junk drawers. Every input has standing: authoritative, hint, stale, or inadmissible.
- Don't skip reconciliation. The 3am agent must not act on 11pm vibes.
- Don't conflate the three ladders (lifecycle/authority/artifact). Terminology drift breeds bugs in the drywall.
- Don't give Python workflows production credentials, mutable tool handles, or unrestricted shell. Workflows read context JSON and emit proposal JSON. Nothing else.
- Don't make Continuity a hard dependency. Optional context, never authority.
- Don't add per-input `requires_recheck` flags. Recheck is the gate, not metadata.
- Don't let a smarter model unlock higher authority. Intelligence dependencies improve quality, never permission.
- Don't treat `committed` as "true forever." It means "accepted for this run under this scope, after reconciliation."
- Don't treat acknowledgment as closure. Ack needs a TTL; silence needs a reason or a timestamp. Attention state without a half-life is a graveyard.
- Don't assume Continuity is being used just because it's hooked in. The reconciler queries it for concurrent activity by default; the run ledger writes breadcrumbs by default. Availability ≠ use.
- Don't mix incident modes. Stabilization is not a license for redesign; architectural insight is not a substitute for stabilization; shipped remediation is not closed remediation. Cross-mode work requires explicit operator override.
- Don't treat `protected` services as a flag checked at the end. The reconciler resists casual turn-down throughout the run, not just at authorization.
- Don't collapse Continuity (cross-run state / coordination / memory) into backup (disaster survival of declared protected state). Related concerns, different failure modes. See `GAP-backup-restore.md`.
- Don't treat "same host, different directory" as off-host. That is staging, not continuity protection. Off-host means out of the protected host's primary failure domain.
- Don't let Nightshift become a universal archive substrate. Nightshift owns backup *orchestration and visibility*, not preservation theory. Evidentiary archive is a separate concern if it ever needs to exist.
- Don't treat Continuity as advisory for risky work. For shared-infrastructure ops, topology/config/publisher/source changes, mode transitions, or protected-class scopes, preflight is a guardrail — a run that skips it is not a faster run, it is an unsafe run.
- Don't propose execution on stale evidence. Staleness escalates to revalidation, not action. Night Shift may schedule a recheck; it may not propose mutation against evidence the reconciler flagged stale.
- Don't branch Night Shift behavior on NQ witness positions. NQ witness-position taxonomy (substrate / application_internal / application_external / platform_internal / platform_external) lives entirely inside NQ's grammar. Night Shift consumes the finding *shape* NQ surfaces; it does not interpret which witness wins. If two positions disagreeing should change scheduling, NQ encodes that into the finding shape and Night Shift responds to the shape — not to the witness metadata.
- Don't mint governance doctrine without grepping corpus first. Audit-owed entries, candidate GAPs, motif extractions, promotions, and ratifications are each corpus mutations. Before authoring any of them: if canonical placement already exists, point to it; if only memory exists, weigh whether row pressure or discoverability justifies surfacing into repo text; if nothing exists, mint narrowly with provenance. The failure mode is duplicate doctrine with perfect posture and no memory of the corpse in the next room. `AUDIT-BACKLOG.md` captures this rule for the backlog→GAP transition specifically; the general rule is named here because tacit does not exist for Night Shift — the reflex an experienced operator has ("surely we've hit this; grep first") is not available across NS sessions by default.
