# Nightshift Worker Adapter Protocol V1

Status: BOLT-LOOM corrected generic contract, built as a non-rewriting successor to CLOCKWORK-MOTH. TUNNEL-FINCH may implement a
Switchyard adapter against it without changing packet semantics.

The closed records are defined by
schemas/nightshift.worker-adapter.v1.schema.json and executable Rust types.
All identity-bearing records use domain-separated RFC 8785/JCS digests.

## Start request

nightshift.worker-start-request/v1 binds:

- exact packet, run, work-item, attempt, and deterministic worker-brief digest;
- adapter protocol, workspace identity, and provider/model routing class;
- timeout and output bounds;
- recursive-worker prohibition;
- approval policy fixed to SURFACE_ONLY_NO_RESPONSE;
- expected terminal-receipt schema.

It is a request to spend bounded local agent compute. It is not target-effect
authority. The generic protocol has no packet-level provider fields and no
approval-response operation.

## Event

nightshift.worker-adapter-event/v1 supports adapter acceptance, provider
identity, worker start, checkpoint, waiting approval, human question, provider
completion observation, diagnostic, and mechanism-indeterminate events.

Every event echoes packet/run/work-item/attempt and adapter identities. The adapter version is bound by execution profile V2. Each provider, model, session, thread, turn, and queue identity freezes when first observed; later events may add previously absent fields but cannot change a frozen value.
Provider, model, session, thread, turn, and queue identities remain distinct
nullable fields. A duplicate event identity is refused. Wrong attempt or
adapter identity is refused. Unknown material belongs only in the bounded
extensions object; exact raw bytes remain inspectable and extension values
gain no scheduler meaning.

A waiting-approval event is testimony that the provider is waiting. The
foreman records it and sends no response.

## Terminal receipt

nightshift.worker-terminal-receipt/v1 binds all start identities plus exact
times, raw state and classification strings, repository custody, tests,
evidence, live/production mutations, remaining trigger, next lawful action,
human questions, and teardown declarations.

Acceptance validates identity, structure, digest, size, and required fields. Adapter ID and version must match the exact execution profile, and every provider identity already frozen by accepted events must match the terminal receipt.
It does not certify the semantic truth of a worker's classification. Provider
completion and process exit do not substitute for this receipt.

nightshift.work-item-not-started-receipt/v1 is separate. It is accepted only
for an entry-eligible work item with no attempt and records raw state,
classification, exact evidence, trigger, next lawful action, and questions.

There is no implicit second attempt. Transport/session resume after foreman
restart uses the original attempt identity.
