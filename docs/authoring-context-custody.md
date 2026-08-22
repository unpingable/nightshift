# Maude authoring-context custody v1

Status: canonical operational custody seam for new Maude-authored proposal
handoffs. Authentication establishes custody, not governed authority.

## Actual process and transport path

Maude still uses its existing `runtime.session.create` RPC and does not submit a
Nightshift cycle. Immediately after that RPC returns the supervised
`session_id`, and before launch, Maude optionally records the exact `session_id`
and exact UTF-8 plan bytes in its local append-only custody store. A separate
one-shot deployment mediator invokes `maude-authoring-handoff` to bind that
receipt and those same bytes to an already-sealed Nightshift base cycle
`request_id`. The mediator writes one handoff JSON file. The canonical receiver
remains:

```text
nightshift cycle run --request BASE_REQUEST \
  --maude-authoring-handoff HANDOFF \
  --maude-custody-credential PRODUCER_KEY \
  --maude-producer-principal-id PRODUCER \
  --maude-producer-key-id PRODUCER_KEY_ID \
  --maude-session-custody-credential SESSION_ISSUER_KEY \
  --maude-session-issuer-principal-id SESSION_ISSUER \
  --maude-session-issuer-key-id SESSION_ISSUER_KEY_ID \
  --nightshift-runtime-id RUNTIME_ID \
  ...existing NQ/currentness/AG arguments...
```

This is a bounded local file handoff, not a service or message bus. Maude does
not automatically run this command and no submit/approve control is added.

## Two authenticated roles

`maude.supervised_session_custody.v1` is minted by the configured Maude session
issuer at the canonical `runtime.session.create` result boundary. It binds:

- session issuer principal and key identities;
- exact `maude_session_id`;
- SHA-256 `maude_plan_ref` and exact byte length;
- record time evidence and a content-derived `session_record_id`;
- a domain-separated HMAC-SHA-256 authentication tag.

`nightshift.maude_authoring_context_handoff.v1` is minted by the distinct
handoff producer. It carries that immutable receipt and binds:

- producer principal and key identities;
- exact plan text, plan digest, and session identity;
- the sealed base Nightshift `target_request_id`;
- the intended `target_runtime_id`;
- creation-time evidence and a content-derived `handoff_id`;
- a separately domain-separated HMAC-SHA-256 tag.

The two key identities and key bytes must differ. The handoff producer can read
and carry a session receipt but its API has no session-receipt constructor and
does not receive the session-issuer key. Nightshift independently verifies both
credentials. Possession of the producer credential therefore does not entitle
the producer to fabricate an arbitrary Maude session.

## Exact binding and acceptance

The base request is sealed before authoring context is attached. Nightshift
recomputes that base identity from the received request with `request_id` and
`authoring_context` removed, checks the handoff target, verifies both roles,
then attaches the handoff in memory and validates the final request seal.
Authentication happens before NQ qualification, recurrence-slot claim, or any
canonical cycle write. Wrong principal/key, session, plan bytes, request,
runtime, schema, or authentication therefore produces no lineage record.

Input files are opened as non-symlink regular files with close-on-exec, read
without newline or encoding normalization, limited to 1 MiB for Maude plans
and 16 MiB for Nightshift request/handoff JSON, and rejected on malformed or
trailing JSON. The Maude handoff output uses a 0600 temporary file, `fsync`, and
atomic rename when written to a path.

## Durable evidence and read projection

After verification, Nightshift mints
`nightshift.authoring_context_custody_provenance.v1` separately from
`nightshift.authoring_context_provenance.v1`. The custody record retains both
identity roles, the session and handoff receipt IDs, target request/runtime,
exact Maude context, and the final campaign/occurrence/proposal/work relation.
Its `recorded_at` is the request's caller-sealed cycle evaluation time, not a
claim about the receiver's physical wall clock. It deliberately does not
retain either secret or either HMAC tag. It is stored
atomically with proposal preparation in
`canonical_authoring_context_custody`; there is no update API.

Read-only inspection uses the same exact query forms as authoring lineage:

```text
nightshift --store STORE cycle export-authoring-custody \
  --campaign-id CAMPAIGN --occurrence-id OCCURRENCE

nightshift --store STORE cycle export-authoring-custody --proposal-id PROPOSAL

nightshift --store STORE cycle export-authoring-custody \
  --plan-ref PLAN --maude-session-id SESSION
```

The output schema is `nightshift.authoring_context_custody_export.v1`.
Phosphor-ng reads only this command and presents custody separately from
lineage and authority. Zero matches means custody was not recorded; it is not
silently interpreted as refusal or authorization failure.

## Replay and restart law

- Exact retransmission at Maude returns the first stored session/handoff bytes.
- One `(session_id, plan_ref)` may target only one base request. A successor or
  other campaign needs its own session/context; the old handoff is refused.
- One target request accepts at most one handoff; concurrent conflicts yield at
  most one stored winner.
- Before Nightshift's proposal-preparation transaction, a crash leaves no
  custody or lineage fact. Exact resend can be verified again.
- After the atomic commit, custody and lineage reopen together. A lost AG
  response moves the cycle to recovery/status reconciliation; exact resend is
  refused by existing slot uniqueness and never remints provenance or AG work.
- A malformed, truncated, wrong-key, or wrong-runtime resend fails before a
  cycle fact exists.

Historical authoring lineage is not backfilled. Lineage predating this custody
schema remains honestly `custody not recorded`. Runtime-generated successors
without a new Maude context remain unlinked and do not inherit predecessor
custody.

## Authority neutrality and environmental boundary

Producer authentication establishes custody of the authoring-context
assertion. It does not authorize governed work.

Authoring-context provenance establishes lineage, not permission.

Neither receipt is present in AG open-occurrence or authorization wire
material. Neither is consumed by NQ admission, Nightshift currentness,
standing, admissibility, authorization, spend, Docket, retry, or human
disposition. A governed proposal without Maude context remains governed under
the pre-existing contract; an authenticated context does not upgrade it.

The deployment must protect both raw 32-byte credential files, the Maude
custody database, the request and handoff directories, and the service
principals that can read them. Nightshift has receiver copies of both HMAC
keys, so these receipts provide deployment authentication, not third-party
non-repudiation. Host-principal isolation, credential rotation/revocation,
executable integrity, filesystem durability, and the honesty of the Maude
session service remain environmental assumptions. `target_runtime_id` binds a
configured semantic deployment identity; executable and filesystem paths are
locators, not authority.

Maude still has no browser-addressed historical plan/session service. Building
one would be a separate read-only product surface, so the existing exact
Phosphor-ng display remains the bounded choice here.
