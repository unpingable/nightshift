# Subject attribution across substrate discontinuity

## Classification

**Semantic gap exposed — decision required.**

This is an audit result, not a substrate-succession contract. It authorizes no
deployment, evidence admission, subject alias, standing grant, or runtime
effect.

The candidate law remains plausible but is not implementable from the current
proof objects:

> Raw evidence may claim a subject reference, but canonical attribution is a
> governed verdict. Custody and provenance can survive when subject
> attribution refuses.

The existing system can preserve exact evidence and can refuse an exact
subject, scope, producer, or vantage mismatch. It does not have a distinct
canonical-attribution verdict or a proof-bearing authority for succession from
one substrate to another.

## Existing distinctions

The current repositories already preserve several adjacent distinctions:

- NQ-NG owns exact acquisition, input admission/refusal, diagnostic subject,
  scope, producer, and vantage identities in `nq.diagnostic_execution.v2`.
  Profile validation prevents provider output from widening or substituting
  the NQ-owned request binding.
- `nq.diagnostic_admission_provenance.v1` proves exact local NQ-store custody
  and evidence eligibility. It does not establish currentness, standing, or
  authorization.
- Nightshift inventory matching compares the complete declared producer and
  semantic binding, including subject and vantage. A mismatch becomes an
  inspectable `BindingMismatch`; it does not become current support.
- Nightshift observation families include policy identity, configuration,
  subject, scope, and scheduler clock. Vantage participates through the full
  inventory-bound policy identity, so two declared vantages cannot silently
  supersede each other.
- Standing authorizes an actor to introduce a claim kind about a concrete
  subject to an audience. That is speech standing, not proof that one logical
  subject continued across two substrates.
- AG standing/currentness/authorization concern exact governed proposals and
  work. They contain no substrate-succession judgment.

These distinctions are useful substrate for a future attribution verdict, but
none of them is that verdict.

## Where claim and verdict still collapse

NQ's `DiagnosticSubjectV1` is an exact logical identity embedded in the
diagnostic artifact. Nightshift requires it to match the predeclared policy
binding. This prevents an artifact from changing the declared identity while
keeping the same policy.

It does not prove that the process producing the artifact is still operating
on the substrate previously associated with that logical identity. Physical
substrate identity is not a separate coordinate in the NQ diagnostic contract,
and Nightshift has no attribution decision between acquisition custody and
canonical observation construction.

Consequently:

- if `substrate:test-b` is represented as a different exact producer, scope,
  or vantage identity, the mismatch is retained and cannot support the
  `substrate:test-a` binding;
- if a process on `substrate:test-b` reuses all identities configured for
  `substrate:test-a`, the present contracts cannot observe the discontinuity.

Conserved identifiers therefore establish exact equality of the declared
contract, not continuity of the underlying substrate.

## Scenario results

Use the following minimal vocabulary:

```text
subject:   observer:test-office
prior:     substrate:test-a
candidate: substrate:test-b
```

### Baseline

An NQ diagnostic whose complete producer/subject/scope/vantage binding matches
the Nightshift inventory is eligible for ordinary posture evaluation. Existing
NQ provenance, Nightshift support, currentness, and AG gates remain separate.

### Unauthorized discontinuity

When the substrate distinction is carried by an existing exact binding field,
Nightshift refuses it as a binding mismatch or records it in a different
policy/observation family. Exact artifact and source-admission provenance remain
inspectable.

There is no typed state that says all of:

```text
custody:                  established
claimed subject:          observer:test-office
origin substrate:         substrate:test-b
canonical attribution:    refused or unresolved
```

The retained mismatch is therefore a useful analogue, not a complete governed
attribution model.

### Genuinely ex-post authority

The cross-office causal carrier is now specified and implemented in
[`CONTINUITY_AUTHORITY_CARRIER_V1.md`](CONTINUITY_AUTHORITY_CARRIER_V1.md).
A completed NQ acquisition cannot be amended to add a later Standing warrant.
Backdated evidence times do not change the immutable acquisition intent.

### Prior authority delivered late

The immutable NQ intent now commits the exact authenticated Standing authority
before provider invocation. Nightshift may receive the intent/evidence before
it separately learns or configures the Standing verification material; later
verification preserves the historical prerequisite relation. Consumer delivery
order is not used as issuance order.

The remaining attribution gate is empirical rather than causal: current NQ
diagnostic provenance supplies no independently owned observation-substrate
coordinate. A cryptographically valid chain is therefore `unresolved` rather
than silently attributed when complete configured identity is reused across
P1 and P2.

## Hostile-case standing

| Attempt | Current result |
|---|---|
| P1-to-P3 authority substituted for P1-to-P2 | Exact continuity-edge and acquisition-basis checks refuse. |
| Authority for another subject | Exact signed edge and diagnostic-subject checks refuse. |
| P2 artifact relabelled from its declared P2 binding to P1 | Exact NQ/profile/policy substitution checks refuse, including after resealing the artifact identity. |
| P2 reuses every P1 configured identity | An explicit continuity-bearing acquisition is unresolved without independent edge context. An ordinary V1 acquisition remains physically indistinguishable; the carrier cannot select its own use. |
| Duplicate NQ evidence or admission provenance | Exact replay converges; changed bytes or bindings refuse. |
| Duplicate Standing authority or NQ intent | Exact replay converges; deliberate reissuance has a distinct authority occurrence. |
| Backdated effective time | Asserted times are not consulted; a later authority cannot be inserted into a completed NQ intent. |
| Currentness or qualification offered as succession authority | Structurally the wrong object; neither establishes attribution. |

Failure to establish attribution does not establish another subject, a safe
state, or permission to delete evidence. That negative-clearance prohibition
is consistent with existing refusal and historical-evidence doctrine.

## Ratified carrier and remaining attribution gate

The first four questions above are now answered by the exact Standing warrant,
signed acquisition commitment, immutable NQ intent, and append-only provider
phases documented in
[`CONTINUITY_AUTHORITY_CARRIER_V1.md`](CONTINUITY_AUTHORITY_CARRIER_V1.md).
Nightshift's applicability record cites that exact chain without changing
evidence custody or acquisition time.

The remaining positive-attribution prerequisite is authenticated,
independently established predecessor and observation-substrate context bound
to the same provider intake. Current NQ diagnostic provenance does not carry
that context. Without it, Nightshift can verify that the warrant was a
prerequisite but cannot prove that the evidence came from the warrant's
successor substrate in the required edge; the result is `unresolved` and
routine reliance refuses.

Wall-clock comparison alone is explicitly insufficient. So are DNS names,
provider identifiers, conserved configuration strings, host keys, and stable
PlanNode or campaign lineage.

Until an existing owner supplies that context and an owner-correct rule selects
the continuity acquisition path for a successor, the safe behavior is to keep
any available substrate distinctions in separate lineages, retain exact
provenance, and refuse to claim continuity. A process that reuses a prior
substrate's complete configured identity through ordinary V1 evidence remains
an undetectable physical discontinuity under the current wire contract;
deploying it does not make the identity claim true.

## Non-claims

- This document does not choose the canonical identity of any real host.
- It does not introduce a remote-host semantic or a substrate registry.
- It does not authorize retrospective attribution.
- It does not create a quarantine subsystem, retry worker, or remediation path.
- It does not give Standing, Nightshift, AG, NQ, or a UI new effectful authority.
- It does not authorize deployment to the Linode.
