# NQ as a reliance source

Night Shift consumes NQ as an **evidence and reliance source, not an authority**. NQ
determines what a configured consumer may rely upon; Night Shift determines what read-only
posture to propose next. Neither executes anything.

## Supported NQ schemas

| schema | how it arrives | what Night Shift does with it |
|---|---|---|
| `nq.finding_snapshot.v1` | `nq-monitor findings export --format jsonl` | existing finding intake; non-observable admissibility states are refused at parse time |
| `nq.reliance.receipt.v1` | `nq-monitor reliance evaluate --format json` | consumer-indexed reliance → read-only disposition |

Night Shift reads NQ's **published machine contracts only**. It does not parse human CLI
prose, open NQ's SQLite, read Docket storage, or share Rust types across the repository
boundary.

## Configured consumer, not authenticated

Night Shift is the `nightshift-readonly` consumer profile. That profile is **selected from
local configuration** — NQ has no transport authentication, so nothing here is an
authenticated identity.

Every reliance receipt carries `caller_binding_disclosure` stating this, and Night Shift
**refuses a receipt whose disclosure is missing**: an undisclosed binding cannot be
distinguished from an authenticated one by a later reader. The disclosure is copied into
every disposition's `does_not_establish`.

A receipt addressed to a different consumer profile is refused, not reinterpreted.

## Freshness and timeout — Night Shift's own policy

Two separate numbers, both Night Shift's:

- **`timeout_seconds`** — how long Night Shift waits for the NQ command to answer.
- **`max_age_seconds`** — how old a returned receipt may be and still count as fresh.

The invocation is bounded by polling rather than blocking, because a blocking call would
hang forever and produce **no observation to record**. NQ cannot report its own
unresponsiveness, so this policy cannot live on the NQ side.

## A fresh refusal is not a missing answer

This is the distinction the integration exists to hold.

| source state | whose observation |
|---|---|
| `Fresh` | **NQ testimony** |
| `Stale { age_seconds, max_age_seconds }` | **NQ testimony**, aged past Night Shift's policy |
| `NoResponse { elapsed_seconds, timeout_seconds }` | **Night Shift's** timeout observation |
| `TransportUnavailable { detail }` | **Night Shift's** observation |
| `Malformed { detail }` | **Night Shift's** integrity observation |

A typed NQ refusal — `cannot_testify`, `claim_not_verified`, `contradiction_retained` — is
NQ *speaking*, and arrives as `Fresh`. No answer at all is Night Shift *observing silence*.

**No synthetic NQ receipt is ever fabricated.** When nothing arrived, the disposition record
has no `source` block at all, and says that absence of a response is not evidence of health
or of failure.

## Read-only disposition vocabulary

| disposition | meaning |
|---|---|
| `continue_observing` | bounded evidence is fresh; read-only consideration may continue |
| `wait_for_fresh_evidence` | the answer may change with newer evidence |
| `evidence_unavailable` | Night Shift's own observation that no fresh NQ testimony arrived |
| `request_additional_evidence` | a named further piece of evidence would resolve this |
| `human_judgment_required` | a person must decide |
| `stop` | do not proceed on this line |

**None is an instruction to act.** `human_judgment_required` is a statement that a person
must decide — it does not send anyone a message. Every disposition record carries, whatever
the outcome:

- no action was executed or authorized;
- this is a read-only posture proposal, not execution authority;
- this disposition is Night Shift's, not an NQ claim.

## Mapping

| NQ decision | disposition |
|---|---|
| `authorized_reliance` | `continue_observing` |
| `claim_not_verified` (incl. underlying `needs_more_evidence`) | `request_additional_evidence` — **never a retry** |
| `cannot_testify` | `human_judgment_required` — **never proceed** |
| `contradiction_retained`, `premise_not_accepted`, `residual_obligation_blocks` | `human_judgment_required` |
| `stale_evidence` | `wait_for_fresh_evidence` |
| `consumer_unknown`, `claim_not_authorized_for_consumer`, `purpose_not_authorized` | `stop` — configuration/policy error |
| `malformed_request` / substituted | `stop` — integrity failure |
| *(no response)* | `evidence_unavailable` |

## Premises, contradictions, residuals

Carried into the disposition **verbatim**, with their exact counts, and echoed into
`does_not_establish`. Night Shift does not reinterpret, resolve, or discharge any of them.
A carried residual stays undischarged; a carried contradiction stays unresolved.

## What Night Shift will not do

No automatic retry, repair, clearing, or escalation. No AG call, no Docket call, no Git
mutation, no execution lease. This is enforced structurally, not by convention:

- `scripts/check_no_actuation_surface.sh` blocks new subprocess call sites outside a vetted
  two-file allowlist;
- `tests/forbidden_cycle_sentinel.rs` requires every NS → NQ subprocess call to be a
  **read-only verb** (`export` or `evaluate`) and pins the structural absence of the
  NS → NQ-truth back-edge;
- the disposition tests assert no record ever serializes an action, capability, lease,
  grant, or retry.

## Bootstrap gap

When NQ cannot run at all, no receipt exists, and **absence is not a verdict**. Night Shift
records its own `evidence_unavailable` observation with both its elapsed time and its
configured timeout, and proposes nothing further. Closing that gap is the operator's
judgment, not an inference Night Shift is entitled to make.

## Running the isolated specimen

```sh
cargo test --test nq_reliance_disposition     # 17 behavioural tests
cargo test --test nq_reliance_conformance     # 5 tests over NQ's golden vectors
```

The conformance fixtures under `crates/nightshiftd/tests/fixtures/nq_reliance/` were
produced **by NQ** and are verified here independently — no shared library guarantees
agreement, and none of NQ's evaluator is copied into this repository.

## Not claimed

Night Shift proposes read-only posture from NQ testimony. It cannot authorize or execute
work, and nothing in this document should be read as saying otherwise.

*(Correction, 2026-07-26: this section originally opened with "There is no four-office
execution integration." That sentence was true when written and was falsified on
2026-07-26, when the first four-office pilot — Night Shift proposal → Docket preparation →
AG ng authorization → Docket execution → NQ evaluation → Night Shift disposition —
completed against this repository. The jurisdictional claim above survives unchanged:
Night Shift held no authority and executed nothing in that pilot; Docket's broker authored
the commit, and Night Shift's terminal artifact was a read-only `stop` disposition. See
`docs/FOUR_OFFICE_PILOT_01.md`.)*
