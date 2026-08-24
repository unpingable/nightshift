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

Not representable. No current schema binds a subject, prior substrate,
successor substrate, and proof-bearing authority occurrence to the evidence.
Issuing an ordinary Standing assertion lease after observation would authorize
future speech within its scope; it would not provide a succession verdict and
must not canonicalize the earlier observation.

### Prior authority delivered late

Also not representable. Current records cannot prove that an independently
valid succession authority existed before the observation but became available
to the attribution resolver later.

Standing receipt chains prove append-only order within one grant lifecycle.
They do not share a causal order with NQ acquisition or Nightshift custody.
`not_before`, `issued_at`, `observed_at`, and receipt timestamps have different
owners and meanings. Comparing their wall-clock values would not prove
independence or prevent a later issuer from backdating an effective time.

These two scenarios cannot be distinguished by receipt delivery order, and
they must not be collapsed.

## Hostile-case standing

| Attempt | Current result |
|---|---|
| P1-to-P3 authority substituted for P1-to-P2 | No succession authority schema exists to validate or substitute. |
| Authority for another subject | Ordinary Standing subject binding refuses speech-scope substitution, but this is not a continuity test. |
| P2 artifact relabelled from its declared P2 binding to P1 | Exact NQ/profile/policy substitution checks refuse, including after resealing the artifact identity. |
| P2 reuses every P1 configured identity | Discontinuity is not observable under the current contract. |
| Duplicate NQ evidence or admission provenance | Exact replay converges; changed bytes or bindings refuse. |
| Duplicate Standing request proof | Replay nonce is consumed/refused within the Standing audience. |
| Backdated effective time | No governed continuity rule may rely on it; cross-system causal precedence is absent. |
| Currentness or qualification offered as succession authority | Structurally the wrong object; neither establishes attribution. |

Failure to establish attribution does not establish another subject, a safe
state, or permission to delete evidence. That negative-clearance prohibition
is consistent with existing refusal and historical-evidence doctrine.

## Decision required before implementation

A future implementation needs an explicit governance decision defining at
least:

1. the owner and versioned shape of substrate-succession authority;
2. exact binding to logical subject, prior substrate, successor substrate,
   scope, and authority occurrence;
3. a proof-bearing causal order capable of distinguishing authority that
   existed independently before an observation from authority created after
   seeing it;
4. revocation, supersession, replay, and wrong-transition behavior;
5. an append-only attribution verdict that can cite retained evidence without
   mutating its custody or acquisition time; and
6. the reliance law for unresolved/refused attribution.

Wall-clock comparison alone is explicitly insufficient. So are DNS names,
provider identifiers, conserved configuration strings, host keys, and stable
PlanNode or campaign lineage.

Until that decision exists, the safe behavior is to keep any substrate
distinction represented by existing semantic identities in distinct lineages,
retain exact refused evidence, and refuse to claim continuity. A process that
reuses a prior substrate's complete configured identity remains an unclosed
detection gap; deploying it does not make the identity claim true.

## Non-claims

- This document does not choose the canonical identity of any real host.
- It does not introduce a remote-host semantic or a substrate registry.
- It does not authorize retrospective attribution.
- It does not create a quarantine subsystem, retry worker, or remediation path.
- It does not give Standing, Nightshift, AG, NQ, or a UI new effectful authority.
- It does not authorize deployment to the Linode.
