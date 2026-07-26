# Actuation Boundary

<!--
Adopted verbatim from ~/git/cartography/doctrine/nightshift-actuation-boundary.md
2026-05-28; re-adopted verbatim 2026-07-26 after the cartography amendment
adding the "bounded diagnostic execution" category and correcting the
2026-05-28 grounded-reality paragraph. Single canonical source is in
cartography; do not amend locally — if amendment needed, file feedback in
cartography (PR or coordination note) and the doctrine evolves there.

The declared operations this repository classifies under that category live
in `crates/nightshiftd/src/diagnostic_operations.rs` and are enforced by
`scripts/check_no_actuation_surface.sh`.
-->


# Nightshift actuation boundary

### How to keep your action triggers without holding the trigger

Nightshift was meant to grow small action triggers. The danger analysis says
"don't hold the capability to act." Those only look like they conflict. The
resolution is one distinction:

> **A trigger is an emission across a boundary. Actuation is holding the creds.**

Nightshift keeps every trigger. It just fires an *inert proposal* into a
separately-privileged executor instead of pulling the trigger on the substrate
itself. Nothing is lost except the part that was dangerous.

## What Nightshift may and may not do

| May do (inside its own boundary) | May never do |
|---|---|
| Read (shell to `nq … export`, `liveness export`) | Execute a substrate actuator (ssh/salt/ansible/systemctl/kubectl/docker) |
| Classify posture; cook a Wicket intent | Hold credentials that could mutate the substrate |
| Emit a `ProposedAction` (the trigger) | Write to a control plane (HTTP post/put/delete) |
| Present options / notify | Both decide *and* enact in one trust boundary |

**Grounded reality (checked, not assumed) — as of 2026-05-28:** Nightshift was
already on the right side of this line. Every `Command::new` in the codebase
shelled to `nq` / `liveness` with `export` verbs — pure reads. The danger was
entirely *additive*: a future `Command::new("salt")`, an `ssh … reboot`, a
control-plane `POST`, or an ssh client in `Cargo.toml`. So the boundary
preserved exactly what existed and forbade only what would creep in.

> **Correction, 2026-07-26.** The paragraph above stopped being true on
> 2026-06-10, when `crates/nightshiftd/src/drill.rs` landed with subprocess
> sites that are not `export` verbs and not pure reads. It is retained above as
> the 2026-05-28 statement it was, and corrected here rather than silently
> rewritten. The two-column table was also *incomplete*, not wrong: it had no
> row for an operation that executes something bounded while remaining unable to
> mutate any governed substrate. See **Bounded diagnostic execution** below.

## Bounded diagnostic execution (added 2026-07-26)

A third category. **It is not a read.** Do not call it one merely because it is
non-actuating — that is the same flattening the boundary exists to refuse.

> **A read observes without executing. A bounded diagnostic executes a named
> operation in order to observe. Neither holds the creds.**

### What it establishes

That a **named, enumerated** operation invoked an **explicitly identified**
diagnostic dependency, observed or computed evidence, and returned testimony.

### What it does not establish

That anything was authorized, executed against a governed substrate, settled, or
made safe. Diagnostic output is evidence, not permission.

### Constraints — conjunctive; all eleven must hold

An operation qualifies only if it:

1. invokes an **explicitly identified** diagnostic/probe operation — declared,
   not inferred from context;
2. observes, computes, or collects evidence;
3. writes only to **declared disposable/local evidence state**;
4. emits testimony, findings, proposals, or advisory packets only;
5. possesses **no** standing, authority, lease, capability, credential, or
   reusable execution token;
6. cannot mutate Git refs, repositories, control planes, services, system
   configuration, or governed application state;
7. cannot issue AG authorization or Docket standing;
8. cannot call a repair or actuator;
9. cannot convert its result into an automatic action;
10. remains replayable and operator-visible;
11. is **structurally enumerable and testable**.

An operation that violates any one of these is **not** in this category. It
remains blocked, or the enclosing function is split until the qualifying part
stands alone.

### The mechanical statements

- **Non-actuating does not imply read-only.**
- **Diagnostic execution is not authority.**
- **A proposal is not an effect.**
- **An advisory packet is not an execution lease.**
- **An acknowledgment does not discharge or rewrite source evidence.**
- **Adding an actuator to a diagnostic module invalidates the classification.**
  The classification attaches to declared *operations*, never to a file, a
  directory, or a module name.
- **The category is closed and explicitly enumerated.** Membership is a listed
  declaration naming the exact executable and verb, checkable by a gate. An
  undeclared subprocess site is not in the category by resemblance.

### Where it sits

| category | executes a subprocess? | may write? | holds authority? |
|---|---|---|---|
| read / export | yes — `export`-class verbs | no | no |
| **bounded diagnostic execution** | **yes — a declared operation** | **declared disposable evidence only** | **no** |
| proposal | no | Nightshift-local state only | no |
| actuation | — | — | **forbidden** |

The "May never do" column above is unchanged and continues to bind this
category in full: no substrate actuator, no credentials that could mutate the
substrate, no control-plane write, and never deciding *and* enacting inside one
trust boundary.

## The contract: `ProposedAction` (inert by construction)

Nightshift's terminal output is a descriptor, not an instruction it can run. The
contract has **no `enact` field, no command string, no connection target, no
credentials** — there is nothing in the artifact that *performs* anything. See
`proposed_action.example.json`. Key properties:

- It **must** carry an admitted Wicket verdict (`admissibility`). A proposal with
  no `authorized`/`gap` verdict is not enactable, full stop.
- Its `subject` content-addresses the originating NQ finding, so custody walks
  back to the observation (same property the receipts and the in-toto sketch
  have).
- `reversibility` drives the executor: **irreversible → human, always.**
  Reversible may be auto-eligible, but only via a standing grant (below).
- `enactment` is intentionally empty. If an action happens, a *different program*
  did it.

## The executor lives on the other side of a privilege boundary

This is the part that makes "doesn't act" a property rather than a promise.
Nightshift holds no actuation credentials. A separate **Executor** process —
different privilege, different creds — consumes `ProposedAction`s and enacts. Three
flavors, differing only in where the trigger-pull lands:

- **human** — Nightshift presents the options + their verdicts; a person selects.
- **agent** — the proposal is handed to an agent (e.g. Claude) whose own tool
  boundary should itself be Wicket-gated. Same seam, one layer down.
- **auto** — for `reversibility=reversible` only, and only with a matching,
  unexpired **Wicket standing-grant** scoped to that `action.kind`. The
  auto-executor holds the creds; Nightshift still does not.

The "small automatic triggers" you wanted live here: a standing-grant for, say,
`cache.flush` lets an auto-executor enact it fast — but the credential to do so
is in the executor, scoped to that one reversible class, never in Nightshift.

## The HITL loop closes itself

The human path isn't a bolted-on approval step. When a person picks an option
from what Nightshift presents, **that selection is the `operator_approval` token
Continuity already requires** to commit anything at `actionable` reliance. So:

```
NQ observe → NS classify + emit ProposedAction → Wicket admit
   → human selects an option  (== operator_approval)
   → Executor enacts (separate privilege)
   → Continuity commit(actionable, approved_by = the human)  → receipt
```

The components compose to enforce human-in-the-loop end to end. You don't wire a
special approval path; the gates you already built demand the human's choice.

## Enforcement: the refusal as a build gate

Intentions drift; build gates don't. `check_no_actuation_surface.sh` is the
Nightshift equivalent of NQ-witness's `assert_no_authority_claims()` and
Continuity's `test_adapter_module_exposes_no_transport_surface`. It:

- hard-fails on actuator execution, execution-client deps, Python actuation
  primitives, and control-plane HTTP writes;
- flags any **new** subprocess call site outside the vetted read-adapter
  allowlist (`nq.rs`, `liveness.rs`) — because that is exactly where mutation
  sneaks in.

Run against current Nightshift it reports **clean**. Run against an injected
`systemctl restart` / `ssh reboot` it **blocks the build**. Wire it into CI and
the convenience PR that adds an `apply()` six months from now gets caught by the
gate, not by whoever happens to review it.

## Why this is overdetermined

Don't-actuate is demanded by four independent things at once: blast-radius
safety, the refusal-architecture (NS doing the executor's job
collapses the decomposition), the human-in-the-loop emphasis, and self-subject
collapse (a component that both decides and enacts is "I'm fine, said the
database" wearing a remediation hat). When four constraints point at the same
line, it isn't a constraint to manage — it's the load-bearing wall.
