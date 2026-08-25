# Substrate-origin V2 attribution

## Owner split

Standing authorizes an exact `substrate_incarnation` edge. NQ commits that
authenticated authority plus an independently signed expected-successor origin
proof before provider invocation. Nightshift verifies both and records an
append-only applicability verdict. Neither NQ nor Nightshift mints continuity
authority. AG and Docket do not participate. Origin meaning is bounded by an
exact profile: the software attester-key profile proves key possession only;
`linode_instance_metadata_v1` proves only the exact logical Linode-instance
coordinate reported through its separately qualified local helper chain.

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
the configured exact bootstrap coordinate. Bootstrap is an independently
pinned deployment input, never the first runtime coordinate accepted merely
because it was observed. After cutover, bootstrap is omitted;
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

The concrete runtime always verifies an Ed25519 signature over the exact
pre-provider acquisition challenge. Under the software-key profile that proves
only key possession. Under the Linode profile it additionally verifies the
closed typed metadata evidence and exact independently pinned logical-instance
coordinate. The metadata response itself is not provider-signed, so the latter
also depends on separately qualified helper locality and provider routing. It
does not prove that the helper key was non-exportable, physical-host identity,
or guest-installation identity.

Accordingly the protocol and relying-side cutover are locally qualified, while
production deployment support remains unqualified until a deployment owns and
qualifies the fixed Linode source, its verifier root, helper/key custody,
co-location and endpoint isolation, rotation/revocation, and restart behavior.

The carrier itself adds no DNS alias rule, remote-command path, or deployment
mechanic.

## Linode logical-instance profile

The closed profile `linode_instance_metadata_v1` reuses the V3 carrier. Its
coordinate is `substrate:linode-instance:v1:<digest>` over namespace
`akamai_linode` and the SHA-256 digest of the decimal provider instance ID.
Nightshift requires the exact profile ID, namespace, instance-ID digest,
helper issuer/key, and optional exact bootstrap coordinate. It rejects:

* attester-key evidence substituted for the Linode profile;
* Linode evidence with another instance-ID digest;
* absent or malformed typed profile evidence;
* V1/V2 downgrade once the exact subject has a V3 requirement;
* runtime metadata attempting to choose its own bootstrap coordinate.

The signed profile evidence retains a canonical metadata-response digest and
optional hashed `host_uuid`. `host_uuid` is supplemental only because its
lifecycle is not sufficiently documented; it is not used by applicability.
The profile identifies one logical Linode object, not physical hypervisor
placement, guest installation, boot, subject, or network name.

The CLI flag `--substrate-origin-linode-instance-id-sha256` supplies the
independently pinned exact logical-instance digest. The observed metadata
cannot populate that flag. The public key remains the local helper's signing
key: Linode metadata is instance-local but is not provider-signed portable
attestation. Production helper isolation, fixed-endpoint routing, executable
custody, service principal, and signing-key custody remain deployment gates.
