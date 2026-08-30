# Nightshift Casework projection and read API V1

Nightshift Casework is a separate read-only operator tool. Canonical
`nightshiftd` remains the temporal office with exactly its two declared
production binaries. The casework package neither opens its SQLite store nor
calls AG, NQ, Docket, Switchyard, a subprocess, or an external service.

## Inputs and custody

Each repeatable `--run-dir` is explicit and supplies exactly:

```text
packet.v1.json
run-receipts.v1.json
```

The loader performs no recursive discovery. It opens the supplied run directory
once as a no-follow directory handle, opens only the two exact child names
relative to that handle with no-follow semantics, verifies the opened
descriptors are regular files, and reads those already-open descriptors. A
concurrent pathname replacement therefore cannot redirect a later read. Packet
evidence paths and receipt evidence strings are display data and are never
opened. Exact source bytes and their SHA-256 digests remain attached to the run.

Packet V1 parsing stays closed. `NightshiftPacketV1::validate_integrity` is
the narrow additive library seam for structure, content digest, and DAG
validation without evaluation-time currentness. Existing `validate_at`,
packet schema, and packet digest law are unchanged.

## Closed projection

The derived schema is `nightshift.casework-run/v1`, documented by
`schemas/nightshift.casework-run.v1.schema.json`. It contains separate
packet intent, receipt outcome, starting custody, final custody, and human
question records. It contains no aggregate result.

Currentness has three independent fields:

- `packet.integrity`: structural/content integrity;
- `packet.currentness_at_receipt_snapshot`: interval position at
  `receipts.updated_at`; and
- `packet.currentness_now`: interval position at the server's single startup
  evaluation instant.

Currentness strings are exactly `NOT_YET_CURRENT`, `CURRENT`, or
`EXPIRED`; interval endpoints are inclusive. `packet.currentness_evaluated_at`
records the exact startup instant used for `currentness_now`. If `updated_at`
is not a recognized RFC 3339 string, snapshot currentness is `UNAVAILABLE`
without rejecting the renderer-compatible receipt. An intact expired run
projects normally as historical evidence. The projection is a startup snapshot;
its digest does not change dynamically as wall-clock time advances.

The run id is the packet digest hex without the `sha256:` label. Derived ids
are SHA-256 over a domain string, one NUL byte, and RFC 8785 JCS of ordered
exact components:

- work item: domain `nightshift.casework.work-item/v1`; packet digest and id;
- base question: domain `nightshift.casework.question/v1`; packet digest,
  linked packet work-item id, and recognized exact question;
- question navigation: domain `nightshift.casework.question-row/v1`; packet
  digest and source ordinal;
- custody base row: domain `nightshift.casework.custody-row/v1`; packet
  digest, source section, recognized repository, and source ordinal; and
- final custody navigation: domain
  `nightshift.casework.custody-row-navigation/v1`; packet digest, source
  section, and source ordinal.

Unrecognized question linkage or custody repository values have null base
identities while retaining ordinal navigation identities. The projection digest
is SHA-256 over `nightshift.casework-run.digest/v1\0` followed by RFC 8785 JCS
of the full projection with only `projection_digest` omitted. Derived ids are
navigation keys, not authority or provenance claims.

## HTTP boundary

The listener checks both the requested address and the resulting socket and
refuses anything not loopback. The only successful GET routes are:

```text
GET /healthz
GET /api/v1/runs
GET /api/v1/runs/{packet-digest-hex}
GET /api/v1/runs/{packet-digest-hex}/raw/packet
GET /api/v1/runs/{packet-digest-hex}/raw/receipts
```

All other methods return 405 with `Allow: GET`. Unknown and traversal-shaped
routes return 404. Raw routes return the source bytes directly. Projection and
source identities provide ETags. Responses carry a same-origin content
security policy, same-origin resource policy, no-referrer policy, frame
refusal, and content-type sniffing refusal. No permissive CORS header exists.

INDEX-WREN deliberately serves no filesystem static path: unrecognized routes
are 404. A compiled UI may later be embedded behind a separately qualified
same-origin asset table; it must not turn a URL into a pathname or widen these
API routes.
