# Substrate-origin V2 attribution

## Owner split

Standing authorizes an exact `substrate_incarnation` edge. NQ commits that
authenticated authority plus an independently signed expected-successor origin
proof before provider invocation. Nightshift verifies both and records an
append-only applicability verdict. Neither NQ nor Nightshift mints continuity
authority. AG and Docket do not participate.

```text
Standing edge P1 → P2
  + prior admitted Nightshift origin head P1
  + NQ V3 acquisition expecting P2
  + pinned origin attestation proving P2 key possession
→ Nightshift Applicable / Refused / Unresolved
```

Applicability is attribution custody, not evidence truth, diagnostic admission,
currentness, qualification, standing, or authority to act.

## Relying-side V3 cutover

The deployment configures a `nightshift.substrate_origin_requirement.v1` for
an exact subject. Once present, the producer cannot choose a weaker path:

* NQ admission V1 refuses because configured identity is not origin proof.
* NQ admission V2 refuses because continuity authority without origin proof is
  insufficient.
* Forging a V3 wrapper around V1 bytes fails the closed schema, intent,
  signature, intake, and phase checks.

Historical V1 evidence is not rewritten. The first V3 observation may establish
the configured exact bootstrap coordinate. After cutover, bootstrap is omitted;
a different attester coordinate requires the unique prior admitted origin-chain
head and exact Standing authority for that predecessor/successor edge.

The chain is not a mutable `current_substrate` attribute. Every applicable V3
verdict references its exact predecessor applicability record. Nightshift
reconstructs a unique append-only head and refuses forks, disconnected history,
unknown predecessors, and coordinate substitution.

## Verdict law

Nightshift emits `nightshift.substrate_origin_applicability.v1` with one of:

* `Applicable`: exact bootstrap origin, stable exact origin, or authorized exact
  transition;
* `Refused`: subject/intake/origin/edge mismatch or missing/unexpected authority;
* `Unresolved`: no admitted predecessor exists and the observed coordinate is
  not the exact bootstrap.

Only `Applicable` enters an observation record relied upon by the canonical
runtime. Refused/unresolved evidence remains durably available in NQ custody;
routine Nightshift reliance refuses rather than deleting it. A full operational
quarantine workflow is not introduced.

## Exact transition checks

For a transition Nightshift verifies:

1. V3 provenance binds the exact diagnostic bytes, run, intake, and phases.
2. The pinned attester verifies the exact acquisition basis and coordinate.
3. The predecessor comes from the admitted origin chain, not the request.
4. Standing signatures and commitment bind the same acquisition basis.
5. Subject and relation are exact and the edge is predecessor → observed origin.

DNS, hostname, IP, boot identity, and provider prose never participate. Reboot
alone therefore does not cause succession. Reimage and provider migration follow
the attester-coordinate semantics selected and qualified by deployment; the
generic runtime does not guess.

## Trust boundary and current qualification

The concrete runtime verifies an Ed25519 attester-key coordinate. It proves that
the exact pinned private key signed the exact pre-provider acquisition challenge.
It does not prove that the key was non-exportable, co-located with a particular
physical or virtual host, or protected from clone/reimage copying.

Accordingly the protocol and relying-side cutover are locally qualified, while
production physical-origin support remains unqualified until a deployment owns
and qualifies an origin source (for example provider-signed identity or a
hardware-backed enrolled key), its verifier roots, key custody, co-location,
rotation/revocation, restart behavior, and clone/reimage semantics.

No Linode-specific semantic, DNS alias rule, remote-command path, or deployment
mechanic is introduced.
