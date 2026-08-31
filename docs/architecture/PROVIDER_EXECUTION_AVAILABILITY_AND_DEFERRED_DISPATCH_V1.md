# Provider execution availability and deferred dispatch V1 contract freeze

Campaign: HOLDING-PATTERN

Canonical slug: `provider-execution-availability-and-deferred-dispatch-v1`

Status: accepted cross-repository owner subjects pinned; Nightshift contract checkpoint under qualification

This contract is a non-authorizing successor to the exact durable roadmap at
`70e3b734e979173ae552efb322b48bf7fb0c028b`. It does not amend the sealed
Nightshift V2 packet or the accepted worker-adapter V2 law.

## Exact owners

The accepted owner subjects consumed by this contract are:

- Codex provider-boundary owner `c36a8137638decf8b04a49611354a90f32c5a945`;
- Switchyard mapper owner `2ba25db66d8b29dd215bd87e05f4ea794024b3b7`;
- checked Switchyard schema SHA-256
  `131f1f6e0cf8cb0aea26ed225c584440c81ffedd443c68ace23adecbe493cf93`; and
- deterministic fixture SHA-256
  `cafa673ac58f60029fd6c1de229b4f57d9f42ba918b7ecb2a3bfb20cb2b41a31`.

These pins establish protocol and test custody, not an inherited executable,
provider session, credential, or dispatch authority. Nightshift independently
reopens canonical mapper snapshot bytes, verifies binding/evidence/snapshot
digests and exact identity relations, then projects only the closed owner
meanings.

The checked deterministic mapper vectors are retained byte-exactly with plain
SHA-256 custody:

- parked not admitted: `362d56fde662c92c2a34849341f6b2af890ed2ce284558d801e77753676f778e`;
- provider completed: `cee936c7d33c983a6b15892531f7167130cab90d2b0cdcd1013d59a6868960c7`;
- post-admission interrupted: `6d966b7ca74460731569c6f1f7fb41ca028217ce8a7935cb97cb5db04e67801a`;
- approval interrupted: `c05646531bacc83260fc4f4c104b31a2b858e5808214d636859feff80f1e89c7`; and
- admission indeterminate: `6b15689989daa2eaeefd602ccfb000c681217bc49ffdcb80c35d81b657e07bf1`.


Nightshift owns provider-neutral scheduling mechanism state:

- immutable work-attempt identity;
- fresh dispatch/admission-occurrence identity;
- exact provider-execution identity or explicit absence;
- availability policy and observation custody;
- durable park, wake, bounded backoff, fallback, and reconciliation;
- atomic resource release/reacquisition;
- restart replay and stop law.

Switchyard owns the Codex adapter mapping from exact App Server messages into
the provider-neutral records. It may not infer admission from a request, local
thread or turn creation, process state, exit status, or missing error.

Codex core and App Server own the provider boundary. Only they can emit a
positive upstream response-created identity or an exact ordered refusal before
that identity. Switchyard must not reconstruct this fact from free text.

FUEL-NEEDLE remains the sole owner of quota/budget observation and admission.
Quota and execution availability are independent. In particular:

- abundant quota does not imply execution availability;
- unavailable execution does not imply exhausted quota;
- an App Server `usageLimitExceeded` value does not become a rate-limit or
  availability observation; and
- absent availability evidence is `UNKNOWN`.

## Owner-corrected App Server finding

The accepted TUNNEL consumer at
`f0fe2563a99148a86fa3a0061e3af2dafdd6f077` has no positive provider
admission witness:

- `turn/start` returns after local `start_or_steer_turn`;
- `turn/started` is emitted before `run_turn` and before first sampling;
- upstream `ResponseEvent::Created` is currently consumed without an App
  Server notification or retained response identity; and
- `rawResponse/completed` exposes a response identity only after completion.

Those findings explain why the accepted TUNNEL V1 mapping remains insufficient
for automatic deferral. The accepted Codex and Switchyard owner subjects pinned
above add the exact evidence described below; Nightshift consumes that evidence
without reconstructing it from the older local-turn facts.

The additive Codex owner correction is:

1. preserve the exact upstream `response.created` response ID;
2. create an exact first-sampling request-occurrence identity before the
   provider request and retain its provider, requested model, thread, turn,
   sampling ordinal, and request order;
3. emit an ordered `rawResponse/started` App Server notification binding
   that request occurrence to thread ID, turn ID, exact provider, exact model,
   upstream response ID, and sampling ordinal;
4. make malformed or missing response-created identity an explicit protocol
   discrepancy, never silent absence;
5. label each terminal error with the owner-observed admission phase
   `PRE_RESPONSE`, `POST_RESPONSE`, or `INDETERMINATE`; and
6. retain the request occurrence, provider/model/order, typed
   `codexErrorInfo`, `willRetry`, exact raw message, and nullable provider
   retry-after without promoting message text into meaning.

The first exact `rawResponse/started` notification for the admitted local
turn is the V1 Codex provider-execution boundary. Its response ID is the first
provider execution step identity. Nightshift's provider-execution identity is
the tuple of provider, exact selected model, App Server session estate, thread,
turn, and first response ID. Later response IDs remain ordered steps inside the
same admitted execution; they do not create new work attempts or dispatches.

Only an exact, acquisition-complete, `willRetry=false`,
`PRE_RESPONSE` terminal error may establish non-admission. Initially the
Codex mapping recognizes only typed `serverOverloaded` as
`MODEL_AT_CAPACITY`. Coarse `usageLimitExceeded`, internal-server,
connection, stream, protocol, and unknown errors remain independently recorded
and cannot establish a retryable availability meaning. Authentication refusal
stops. Transport or protocol uncertainty is admission-indeterminate.

The pinned Codex and Switchyard subjects qualified these meanings with a
deterministic mock provider only. They do not qualify a real provider lifecycle,
credential profile, inherited executable, or production activation.

## Identity law

The following values never substitute for one another:

1. `work_attempt_id`: stable intended execution of one exact worker brief;
2. `dispatch_occurrence_id`: fresh for every bounded provider-admission
   request;
3. `provider_request_occurrence_id`: one exact initial sampling request
   inside the dispatch; it is not provider admission;
4. `adapter_process_occurrence_id`: one local adapter process occurrence;
5. `app_server_thread_id` and `app_server_turn_id`: local protocol
   mechanism identities; and
6. `provider_execution_id`: created only at the positive owner boundary.

A pre-admission redispatch preserves `work_attempt_id` and exact brief bytes,
but creates a new dispatch occurrence and a new local App Server turn. This is
lawful only after exact evidence proves the preceding occurrence was not
admitted. It is not a semantic retry.

After provider admission, no new turn, prompt, model, dispatch, or work attempt
may replace the admitted execution. Restart may resume only the exact retained
session/thread/turn/provider-execution tuple.

## Versioned contracts

The implementation adds closed, domain-separated records:

- `nightshift.provider-execution-availability-observation/v1`;
- `nightshift.provider-execution-availability-policy/v1`;
- `nightshift.foreman-execution-availability-requirement/v1`;
- `nightshift.provider-dispatch-occurrence/v1`;
- `nightshift.provider-admission-disposition/v1`; and
- `nightshift.deferred-provider-dispatch/v1`.

Every record uses RFC 8785 serialization and a distinct
`<schema>.digest/v1\0` domain. Exact raw evidence uses hex encoding, an
explicit byte length, plain SHA-256, and a 16 KiB per-evidence ceiling. The
complete canonical Switchyard mapper snapshot is a distinct carrier with
representation `RFC8785_SWITCHYARD_MAPPER_SNAPSHOT` and a 16 MiB ceiling.
This contract checkpoint does not yet claim a cumulative journal-history bound;
that metadata-first pre-acquisition invariant belongs to the held storage
implementation and must qualify before journal mutation is accepted.

The worker-adapter successor is V3. V2 remains readable and unchanged, but a
run admitted with an execution-availability requirement refuses the V2 start
path. V3 binds exact request, packet, brief, profile, attempt, dispatch,
provider, selected model, adapter protocol/version/executable, session estate,
resource policy, and result schema before adapter process creation.

## Closed meanings

Availability observation states are:

- `AVAILABLE`;
- `MODEL_AT_CAPACITY`;
- `PROVIDER_UNAVAILABLE`;
- `RATE_LIMITED`;
- `AUTHENTICATION_REFUSED`;
- `TRANSPORT_ERROR`;
- `PROTOCOL_ERROR`; and
- `UNKNOWN`.

Dispatch dispositions are:

- `NOT_ADMITTED_MODEL_AT_CAPACITY`;
- `NOT_ADMITTED_PROVIDER_UNAVAILABLE`;
- `NOT_ADMITTED_RATE_LIMITED`;
- `AUTHENTICATION_REFUSED`;
- `QUOTA_EXHAUSTED_FUEL_OWNED`;
- `ADMISSION_INDETERMINATE`; and
- `EXECUTION_ADMITTED`.

`WAITING_APPROVAL` is a post-admission worker mechanism state, not an
availability disposition. It never answers an approval or permits redispatch.
Unknown wire values remain raw-only and produce
`ADMISSION_INDETERMINATE`.

## Policy and model selection

An immutable run-level execution-availability requirement is admitted
atomically with a new run. It binds the packet, admission, execution profile,
provider identity, adapter identity/version, availability policy ID/digest,
and an ordered exact model-selection list for each work item.

Each model selection contains the same exact provider identity, an exact model
identity, and the packet/profile model class. No alias or class heuristic may
change provider or model. Missing policy, an unrecognized selection, or an
exhausted list refuses dispatch.

Fallback is allowed only before provider admission and only to the next exact
selection in the ordered list. Every selection and refusal remains in the
journal. After admission, model migration is forbidden.

The policy fixes:

- at most 16 dispatch occurrences per work attempt;
- a finite list of positive backoff seconds, each at most 86,400;
- at most seven days total deferral;
- one lock policy: `RELEASE_AND_REACQUIRE` or explicit
  `RETAIN_WHILE_PARKED`;
- `reconcile_indeterminate=true`;
- `automatic_semantic_retry=false`; and
- `approval_response_authorized=false`.

The adapter capability contract and exact App Server launch identity must prove
the internal provider-request retry count is zero. Qualification emits one
request-occurrence record and proves one mock-provider request. An adapter that
can internally retry without Nightshift custody is not admitted. A future
nonzero retry implementation would have to expose every actual request and
retry occurrence before it could satisfy this contract.

## Atomic transition law

The durable journal order is:

```text
AttemptCreated
DispatchOccurrenceOpened
  -> ProviderAdmissionDisposition(EXECUTION_ADMITTED)
  -> ProviderAdmissionDisposition(NOT_ADMITTED_*)
       immediately followed by DeferredProviderDispatch
  -> ProviderAdmissionDisposition(ADMISSION_INDETERMINATE)
```

A not-admitted disposition and parked record are one immediate transaction.
The parked row retains the same attempt, last dispatch, refusal evidence,
selected model, policy, backoff ordinal, exact `wake_at`, remaining model
set, and lock disposition before any provider capacity or resource lock is
released.

`wake_at` is derived from exact refusal-received time and either an exact
provider retry-after or the policy backoff. A wake invocation supplies no
availability evidence and grants no authority. One immediate transaction
checks eligibility, reacquires required locks, and creates at most one fresh
dispatch. Duplicate wake invocations converge.

A stale availability observation cannot start a dispatch. A wake never
refreshes evidence. Restart replays exact journal state and cannot advance
`wake_at`, reset backoff, or replace evidence.

## Indeterminate and reconciliation law

Indeterminate admission is monotonic until exact reconciliation:

- no automatic redispatch or fallback;
- no new local turn or prompt;
- preserve exact process/session/thread/turn and raw evidence;
- reconcile only through an adapter-declared canonical status operation; and
- apply the existing lane-local stop law when reconciliation is unavailable or
  cannot prove admission or non-admission.

Reconciliation creates a new append-only record. It never rewrites the
ambiguous occurrence. Proven admission binds the exact provider execution and
permits same-attempt resume only. Proven non-admission permits the ordinary
park law. Unresolved reconciliation remains indeterminate.

## Qualification boundary

The initial qualification is deterministic fake-adapter only. It must cover
the twenty roadmap cases, exact schema/runtime parity, restart at both parked
boundaries, duplicate wake, concurrent writers, identity substitution,
cumulative bounds, same-attempt continuity, policy-bounded fallback,
independent lane progress, and no semantic retry.

No provider profile, credential, provider process, network contact, live
session, timer activation, service installation, or production route is
authorized. No aggregate provider-health or campaign result is created.

SECOND-WATCH remains `PLANNED / NOT STARTED`.
