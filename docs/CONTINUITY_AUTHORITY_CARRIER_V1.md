# Continuity-authority carrier V1

## Owner boundary

Standing owns issuance of one immutable, authenticated continuity-edge warrant.
NQ owns the pre-provider acquisition intent and its append-only invocation/intake
phases. Nightshift is a verification-only consumer. AG and Docket do not
participate in this carrier.

The only positive relation in V1 is `substrate_incarnation`. The warrant means:

> Standing authorized exact subject `S` to use exact transition `X -> Y` as an
> eligible continuity prerequisite for the pinned NQ audience.

It does **not** assert that the transition occurred, that acquired evidence is
true, that the subject is canonically attributable, that the evidence is
current, or that routine reliance or action is allowed.

## Frozen cross-office chain

Nightshift accepts continuity-bearing admission provenance only through this
closed chain:

```text
standing.continuity_authority.v1
  in standing.signed_continuity_authority.v1
        +
standing.continuity_acquisition_commitment.v1
  in standing.signed_continuity_acquisition_commitment.v1
        ↓
standing.continuity_acquisition_bundle.v1
        ↓
nq.continuity_acquisition_basis.v1
        ↓
nq.provider_acquisition_intent.v1
        ↓
provider_invocation_started
provider_intake_completed
        ↓
nq.diagnostic_admission_provenance.v2
        ↓
nightshift.continuity_applicability.v1
        ↓
nightshift.observation_record.v5
```

All JSON objects are strict closed structures. Standing envelopes authenticate
`signed-schema || NUL || JCS(payload)` with Ed25519. Nightshift pins the public
key ID, raw 32-byte public key, and NQ audience through deployment
configuration. It contains no Standing private key or signer.

The authority and commitment carry raw lowercase SHA-256 hex payload digests.
The NQ basis digest is likewise raw lowercase hex. NQ's intent identity,
complete-intent digest, and checkpoint-contract digest retain NQ's
`sha256:<hex>` convention. The different encodings are frozen owner wire
contracts; Nightshift does not normalize ambiguous input.

## Structural causality, not clock order

The commitment binds one preallocated NQ acquisition/provider-intake identity
and the digest of the complete static acquisition basis. NQ then durably stores
an intent containing that exact authenticated bundle before it records
`provider_invocation_started`. The later `provider_intake_completed` phase and
diagnostic artifact bind the same intake.

This is the causal fence:

```text
authenticated authority A
→ signed commitment to basis/acquisition Q
→ immutable NQ intent containing A
→ provider invocation
→ intake/evidence
```

`issued_at`, `committed_at`, diagnostic completion time, and Nightshift receipt
time are evidence fields only. Nightshift never compares them to prove causal
precedence. Backdating an authority cannot insert it into an already completed
NQ intent, and changing the intent requires a new intent identity and breaks
the exact intake chain.

Late delivery to Nightshift is therefore harmless to the historical causal
fact: if the received NQ intent already contains the valid authority and
commitment, Nightshift can verify the chain after the provider intake. By
contrast, an acquisition completed without that prerequisite cannot be
retrofitted with a later warrant.

## Applicability is not a mutable continuity flag

Nightshift records a content-identified applicability verdict with one of:

- `applicable`: the signed exact edge, acquisition intent, provider intake,
  diagnostic subject, independently established predecessor, and independently
  established observation-substrate coordinate all agree;
- `refused`: an exact coordinate disagrees;
- `unresolved`: the authenticated prerequisite chain is valid, but no
  independent observation-substrate coordinate is available.

The verdict is neither Standing authority nor Nightshift currentness. Matching
subject, producer, scope, or vantage tokens cannot replace the independent
substrate coordinate. In particular, a continuity-bearing acquisition from a
copy of the complete P1 configuration on P2 remains `unresolved` without those
coordinates; the signed warrant alone does not establish continuity.

This carrier does not make an undeclared physical substrate transition
observable. An ordinary V1 acquisition that reuses every configured P1
identity contains no proof that it came from P2 and no marker saying that a
continuity prerequisite was required. The carrier closes retrofit/backdating
for acquisitions that use it; it does not solve physical-origin discovery or
select its own use.

The present NQ admission wire does not yet carry independently owned
predecessor/observation-substrate coordinates. Production Nightshift therefore
refuses to rely on a V2 continuity admission whose verdict is unresolved. That is a
fail-closed integration gate, not a reason to infer a substrate from hostname,
DNS, IP address, or reused configuration.

## Replay, reissuance, and revocation

Exact replay of the same signed authority, commitment, NQ intent, and admission
converges on the same identities. A deliberate second Standing authority for
the same edge has a different authority occurrence and produces a different
applicability identity; it does not mutate a boolean.

Standing's revocation law governs whether a warrant may be committed into a
new acquisition. Nightshift retains the signed historical chain that a prior
acquisition actually named. Later revocation does not rewrite the acquisition
or erase its provenance, and this consumer does not infer current usability
from timestamps or from historical possession of the warrant.

## Nonclaims and remaining gate

- The carrier does not establish empirical transition occurrence.
- The carrier does not establish evidence truth, currentness, adequacy,
  standing, AG authority, or permission to act.
- Nightshift cannot mint, sign, amend, or revoke Standing warrants.
- NQ cannot mint Standing warrants; it can only verify and commit one before
  provider invocation.
- No Linode, host, provider, DNS, or general remote-observation semantic is
  introduced.

Before the full substrate-attribution experiment can be rerun positively, an
existing owner must provide authenticated, independently established
predecessor and observation-substrate coordinates that can be bound to the same exact NQ intake.
Until then, custody and the causal prerequisite can be verified while canonical
attribution remains unresolved. A separate owner-correct rule must also decide
when a successor acquisition is required to use the continuity path; Nightshift
cannot derive that requirement from indistinguishable ordinary V1 evidence.
