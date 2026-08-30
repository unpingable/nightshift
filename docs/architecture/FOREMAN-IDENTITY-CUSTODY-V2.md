# Foreman identity custody correction

Campaign: BOLT-LOOM

Canonical slug: `nightshift-durable-foreman-identity-binding-correction-v1`

Predecessor subject: exact CLOCKWORK-MOTH result head
`30373353d4472720bf62f60d378056658d068e88`.

The predecessor remains immutable and addressable, but its acceptance is
superseded for successor admission. Its execution profile bound adapter ID,
protocol, and executable digest without binding an adapter version. Accepted
events and terminal receipts were checked against adapter ID only, and replay
could replace provider custody identities with later values. Closeout also
accepted a snapshot time earlier than retained terminal evidence.

This successor corrects those exact custody boundaries without rewriting the
predecessor:

- `nightshift.foreman-execution-profile/v2` binds an `adapter_version` and
  uses the V2 profile-digest domain. The historical V1 schema is not edited.
- Every adapter event and terminal receipt must match the profile-bound
  adapter ID and version.
- Provider, model, session, thread, turn, and queue identities are
  write-once-per-attempt observations. A field may first appear on a later
  event, and later events may omit it, but a contradictory value is refused.
- A terminal receipt may supply an identity not previously observed, but it
  must match every identity already frozen by accepted events.
- A final snapshot time may equal or follow, but never precede, the latest
  exact accepted terminal end time or not-started recorded time.

The scheduler still assigns no semantic meaning to provider identity strings,
worker state, or result classification. The correction establishes custody
consistency only; it grants no target-effect authority and adds no approval
response, provider process, retry, aggregate result, or canonical
`nightshiftd` surface.

Successors must use the exact remote-verified BOLT-LOOM result head and
execution profile V2. Exact head `3037335...` remains historical evidence, not
an accepted integration base.
