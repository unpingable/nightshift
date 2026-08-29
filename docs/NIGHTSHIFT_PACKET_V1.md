# Nightshift orientation packet V1

Status: campaign-qualified prototype.

`NightshiftPacketV1` is a deterministic orientation and scheduling envelope.
It is not a work proposal, standing record, authorization, native approval,
Docket custody record, execution request, retry, outcome, observation, or
settlement.

The operator prompt authorizes a bounded Codex run. The packet only preserves
the run graph and immutable references. A work item may cite AG's canonical
`ExactWorkProposalV1` or explicitly label a repository's actual equivalent.
The reference does not promote that object or grant authority.

## Content identity

The schema is `nightshift.orientation-packet/v1`. Serialization uses RFC 8785
JCS and SHA-256. The digest preimage is the following versioned,
packet-specific frame:

1. the ASCII bytes `nightshift.orientation-packet.digest/v1`;
2. one NUL byte (`00` hex);
3. the RFC 8785 JCS bytes of the whole packet object with the two derived
   locator fields omitted:

- `packet_digest`
- `switchyard.plan_ref`

Every other field is normative and changes the computed digest. This domain
frame prevents a bare JCS object used by another protocol from sharing packet
identity. The plan
reference is exactly `nightshift-packet://<packet digest hex>`. Changing
either derived field without recomputing the packet fails validation.

The typed closed decoder and shared structural validator are the executable
equivalent of `schemas/nightshift.orientation-packet.v1.schema.json`. Both
sealing and validation reject schema-invalid packets before output or digest
admission. Validation additionally rejects stale/future packets, unknown
dependencies, dependency cycles, duplicate work-item or campaign identities,
malformed commit/digest identities, unbounded worker budgets, and transport
field lists other than exactly `alias`, `plan_ref`, `nonce`.

## Transport boundary

A transport may carry only:

1. a pre-registered alias;
2. the immutable `plan_ref`;
3. a replay-resistant nonce;
4. minimal mechanism metadata required for validation and a receipt.

The browser, a prompt, a mailbox message, or a metadata field cannot widen the
packet. Switchyard owns alias registration, digest resolution, expiry checks,
nonce custody, replay refusal, queueing, and receipts. Nightshift does not
accept or settle Switchyard nonces.

## CLI

```bash
cargo run --locked --bin nightshift -- packet seal \
  --packet qualification/nightshift-packet-v1/velvet-orrery/packet.draft.json

cargo run --locked --bin nightshift -- packet validate \
  --packet qualification/nightshift-packet-v1/velvet-orrery/packet.v1.json \
  --evaluated-at 2026-08-29T17:30:00Z

cargo run --locked --bin nightshift -- packet render \
  --packet qualification/nightshift-packet-v1/velvet-orrery/packet.v1.json
```

`seal` emits only the exact canonical bytes with the digest and plan reference
derived: it adds no line delimiter, so direct stdout redirection produces a
packet accepted by `validate`.
`validate` requires already-canonical bytes and an explicit evaluation time.
`render` produces a clearly marked non-authorizing Markdown projection.

## Reports

`scripts/render_nightshift_reports.py` accepts the sealed packet and an exact
`nightshift.run-receipts/v1` snapshot. It rejects unknown, missing, or
duplicate work-item receipts and packet-digest mismatch, then derives:

- `NIGHTSHIFT-RUN-LEDGER.md`
- `HUMAN-QUESTIONS.md`
- `MORNING-REPORT.md`

These reports summarize independent campaign classifications. They never
create an aggregate semantic result.
