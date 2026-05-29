# Actuation Boundary

<!--
Adopted verbatim from ~/git/cartography/doctrine/nightshift-actuation-boundary.md
2026-05-28. Single canonical source is in cartography; do not amend locally —
if amendment needed, file feedback in cartography (PR or coordination note)
and the doctrine evolves there.
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

**Grounded reality (checked, not assumed):** Nightshift today is already on the
right side of this line. Every `Command::new` in the codebase shells to `nq` /
`liveness` with `export` verbs — pure reads. The danger is entirely *additive*:
a future `Command::new("salt")`, an `ssh … reboot`, a control-plane `POST`, or
an ssh client in `Cargo.toml`. So the boundary preserves exactly what exists and
forbids only what would creep in.

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
