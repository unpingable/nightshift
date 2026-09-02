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
reopens canonical mapper snapshot bytes, verifies the exact vendored schema
SHA, executes that schema against the retained snapshot, verifies
binding/evidence/snapshot digests and exact identity relations, then projects
only the closed owner meanings.

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
`PRE_RESPONSE` terminal error may establish non-admission. For the accepted
Codex/Switchyard owner pair, only its literal
`NOT_ADMITTED_MODEL_AT_CAPACITY` disposition, derived by that owner from the
exact typed pre-created `serverOverloaded` record, permits automatic parking.
No other refusal vocabulary is promoted into that meaning. Quota exhaustion
remains FUEL-owned; `usageLimitExceeded`/rate-limited, provider-unavailable/
server-overload ambiguity, authentication-refused, transport, protocol, and
unknown records retain their distinct raw/observation categories. Under the
current owner they stop or remain admission-indeterminate and cannot redispatch,
fall back, or auto-park.

HOLDING also owns one additive qualification-only deterministic adapter
contract. Its exact source artifact is
`qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/fixtures/deterministic-fake-adapter-v1.py`
with SHA-256
`e8a310d46cb40b0aef6399a8da6c97ac99f0fc5eab6a78c5e7007600d5cbfa82`.
The closed record binds producer and source-artifact identity, work attempt,
dispatch, provider request, provider, model, stable typed outcome,
`response_created`, explicit non-admission proof, retry-after, observed and
received times, exact source bytes, and a domain-separated digest. It has no
production/default adapter registration, provider profile, network operation,
listener, or service. Its explicit non-admission `RATE_LIMITED` and
`PROVIDER_UNAVAILABLE` outcomes exercise the ordinary provider-neutral
park/wake path under policy. `AUTHENTICATION_REFUSED` stops without automatic
retry. `TRANSPORT_ERROR` and `PROTOCOL_ERROR`, including possible-admission
cuts, remain indeterminate and cannot redispatch. These qualification meanings
do not widen or reinterpret the separately accepted Codex/Switchyard path.

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
- `nightshift.worker-start-request/v3`;
- `nightshift.provider-dispatch-occurrence/v1`;
- `nightshift.provider-admission-disposition/v1`;
- `nightshift.holding-deterministic-provider-admission-evidence/v1` plus the
  qualification-only `nightshift.provider-admission-disposition/v2`; and
- `nightshift.deferred-provider-dispatch/v1`.

The HOLDING-only start V3 retains the complete canonical start V2 predecessor
bytes, predecessor request digest, and plain byte SHA. It duplicates and binds
all V2 packet/run/work/attempt/adapter/brief/workspace/model-class/boundary
fields, then adds the exact work-attempt and fresh dispatch occurrence,
profile, selected provider/model/class and model ordinal, and accepted
Codex/Switchyard owner/schema/fixture pins. V2 remains unchanged and readable.
V3 has no provider execution identity at start, internal provider retry,
semantic retry, approval-response authority, or target-effect authority.
Construction is acyclic, and the graph check is executable as validation:
Nightshift first derives and seals V3 from the exact V2 predecessor, sealed
execution profile, sealed availability requirement, selected requirement
ordinal, and fresh dispatch occurrence ID.
It then seals the dispatch occurrence over the V3 digest. The decision-bearing
graph validator reopens both and requires exact profile/admission/requirement,
run/work/attempt/packet/adapter/brief, provider/model/class/ordinal, occurrence,
and opening-time equality. The work-attempt and dispatch-occurrence identities
must be distinct. Standalone structural V3 validation is not dispatch admission.

V3 has two closed adapter branches. The ordinary branch remains exactly
`switchyard.codex-app-server/v2` with its accepted binding, evidence, and
snapshot schemas. The SECOND-WATCH qualification branch is available only when
the profile, requirement, V2 predecessor, and V3 request all name the exact
accepted HOLDING deterministic-fake adapter ID, protocol, version, executable
SHA-256, and empty bounded arguments. In that branch the single closed
`nightshift.holding-deterministic-provider-admission-evidence/v1` record is the
qualification binding, evidence, and snapshot carrier. Mixed branches or any
coherently resealed alternate tuple refuse before attempt or dispatch mutation.
This branch grants no production activation and does not widen the ordinary
Switchyard path.

Every record uses RFC 8785 serialization and a distinct versioned digest
domain. Exact raw evidence uses hex encoding, an
explicit byte length, plain SHA-256, and a 16 KiB per-evidence ceiling. The
complete canonical Switchyard mapper snapshot is a distinct carrier with
representation `RFC8785_SWITCHYARD_MAPPER_SNAPSHOT` and a 16 MiB ceiling.
Nightshift executes the exact vendored Switchyard schema and then replays every
retained raw frame: lowercase hex, line framing, duplicate-key refusal,
acquisition lane/method, client request/response digests, provider occurrence
and execution identities, refusal, response completion, approval, normalized
record, cut, and snapshot state must all agree. A present source timestamp that
cannot be represented is refused; only an absent timestamp uses the explicitly
defined receipt-time fallback.
LOSS evidence is exact arbitrary frame custody and is never JSON-decoded;
rawless UNKNOWN acquisition discrepancies retain their distinct exact meaning.
Decision-bearing admission is deliberately narrower than the complete accepted
Switchyard compatibility domain: every non-cut record must carry a non-null
`acquisition_ordinal` and an exact retained `acquisition_kind`. This early
presence/lane gate is the strict ordered acquisition surface used by the
campaign binding and runs before semantic replay. It does not assert literal
ordinal continuity: the subsequent exact owner replay admits contiguous
positive history or exact gap/duplicate/reorder discrepancy testimony. The
three retained ordering-discrepancy histories (`[1,0]`, `[1]`, and `[0,0]`)
remain decision-bearing only as `ADMISSION_INDETERMINATE`. Legacy
qualification-only unordered records have neither retained lane nor sufficient
evidence to distinguish `server_request=true` from `server_request=false` for
the same raw approval message. Nightshift retains all 126 exact owner-generated
terminal prefixes as compatibility analysis, separately identifies the 118 the
owner's generic replay helper reopens, and refuses all 61 prefixes containing
unordered evidence as provider-admission or dispatch-decision basis. They may
remain raw evidence; they cannot park, wake, fallback, redispatch, or establish
execution admission. No fixture-specific snapshot allowlist restores missing
lane testimony.

Provider sampling ordinals and
request order start at zero and advance together, completed occurrence IDs are
unique, and request/response/refusal boundary times are monotonic. The
qualification-only `verify_switchyard_owner_parity.py` predicate pins exact
Switchyard head `2ba25db...`, preserves all 126 captured owner outputs,
classifies eight generic-helper exceptions, and proves owner replay equality
for the 118 generic-replayable compatibility snapshots. Seven helper exceptions
are unordered and receive the same early raw-only refusal as the other 54;
the remaining helper exception is strict ordered owner testimony for a retained
wire/parsed-message discrepancy and reopens only as `ADMISSION_INDETERMINATE`.
The independent Rust
decision-bearing matrix covers only the 65 strict ordered snapshots and
requires every unordered snapshot—including the same-raw approval watermark
and server-request discrepancy pair—to return the exact raw-only refusal before
binding inspection, semantic raw-frame replay, or any scheduling meaning.
The mechanism store enforces a distinct cumulative 16 MiB journal-history
ceiling for execution-availability rows. Each availability append also creates
an immutable metadata row binding run, sequence, event identity, closed event kind, and byte length; an independent
immutable anchor binds the same run, sequence, and event identity, and a
run-level marker records that HOLDING history is required. Independently, the
immutable admitted `runs.execution_availability_required` value anchors that
requirement outside the marker, metadata, and anchor tables. A required run
therefore cannot become legacy through deletion or relabeling of the auxiliary
custody rows. Query-only and
mutating reopen compare the complete metadata, anchor, and event row sets, then
query count and `length(raw_bytes)`, refuse
empty or per-event-oversized rows, and checked-add the cumulative length before
selecting or materializing any raw BLOB. Relabeling a provider event as a legacy internal row, deleting one custody
row, or dropping a required custody table therefore cannot bypass preflight.
The same preflight runs during
query-only reopen and every mutating transition under the immediate transaction.
Legacy non-availability rows remain readable under their predecessor law.
The pure contract graph requires the exact ordered prior history for the same
work attempt. Every entry carries and reopens its exact dispatch occurrence,
admission disposition, and deferral receipt. The graph binds their digests and
identities, requires requirement admission before dispatch and dispatch before
disposition, requires each prior wake before the next dispatch and refusal,
recomputes every duration from exact timestamps or the immutable policy rather
than trusting stored seconds, and checked-adds the result even when the current
occurrence admits execution. Model ordinals cannot advance when fallback is
disabled. Storage must later prove that the supplied slice is the complete
append-only history.

The worker-adapter successor is V3. V2 remains readable and unchanged, but a
run admitted with an execution-availability requirement refuses the V2 start
path. V3 directly retains the exact predecessor request, packet, brief,
profile digest, work attempt, fresh dispatch occurrence, provider, selected
model/class/ordinal, adapter protocol/version, expected receipt schema, and
accepted owner/schema/fixture pins. The sealed profile, availability
requirement, and dispatch graph validates executable registration and policy
bindings transitively; those are not direct V3 fields. An adapter process
occurrence, App Server session estate, and resource-lock acquisition are not
selected or proven by V3. The mechanism store selects those facts, binds them
into the dispatch occurrence, and validates the complete V2/profile/
requirement/V3/dispatch graph before any future adapter process creation.

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

Dispatch dispositions use the closed provider-neutral vocabulary:

- `NOT_ADMITTED_MODEL_AT_CAPACITY`;
- `NOT_ADMITTED_PROVIDER_UNAVAILABLE`;
- `NOT_ADMITTED_RATE_LIMITED`;
- `AUTHENTICATION_REFUSED`;
- `QUOTA_EXHAUSTED_FUEL_OWNED`;
- `ADMISSION_INDETERMINATE`; and
- `EXECUTION_ADMITTED`.

The vocabulary preserves distinct evidence categories; it does not make every
category actionable. With the accepted Codex/Switchyard V1 owner, only
`NOT_ADMITTED_MODEL_AT_CAPACITY` permits automatic parking. The unavailable,
rate/usage-limit, authentication, quota, transport, protocol, coarse, and
unknown paths retain independent testimony but do not authorize automatic
parking or redispatch through that owner. The separate campaign-owned
qualification fixture permits unavailable/rate parking only when its exact
closed evidence explicitly proves non-admission. Authentication still stops;
transport/protocol remain indeterminate. `QUOTA_EXHAUSTED_FUEL_OWNED` is not
constructed from execution-availability evidence in either path.

`WAITING_APPROVAL` is a post-admission worker mechanism state, not an
availability disposition. It never answers an approval or permits redispatch.
Unknown wire values remain raw-only and produce
`ADMISSION_INDETERMINATE`.

## Policy and model selection

An immutable run-level execution-availability requirement is admitted
atomically with a new run. A run may simultaneously retain the independent
FUEL capacity requirement; attempt preparation then validates and appends the
exact FUEL admission and the exact HOLDING dispatch in one immediate
transaction. Abundant quota never overwrites an exact execution refusal, and
UNKNOWN/NO_NEW_WORK capacity never creates a dispatch. Neither owner silently
stands in for the other. It binds the packet, admission, execution profile,
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
  -> ProviderAdmissionDisposition(NOT_ADMITTED_MODEL_AT_CAPACITY)
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
checks eligibility, atomically reacquires both the maximum-concurrent-worker slot and required locks, records the resource
reacquisition and wake, and creates at most one fresh dispatch. Duplicate wake
invocations converge.

A stale availability observation cannot start a dispatch. A wake never
refreshes evidence. Restart replays exact journal state and cannot advance
`wake_at`, reset backoff, or replace evidence.

Availability-required runs refuse the legacy V2-only preparation path. Initial
attempt creation, resource-lock acquisition, V3 construction, and first
dispatch append share one immediate transaction. A construction or validation
failure therefore leaves no attempt, lock, or dispatch. A parked wake similarly reacquires the exact locks and appends the resource
reacquisition, wake, and fresh dispatch atomically. Under
`RELEASE_AND_REACQUIRE`, the parked disposition is immediately followed by a
closed resource-release event and matching mutable claims are removed in the
same transaction; `RETAIN_WHILE_PARKED` keeps them. Replay reconstructs the
exact expected claims from attempt, release, reacquisition, and terminal
history and refuses any mutable-table discrepancy. Attempt creation is
immediately followed by its first dispatch at the same time; every wake is
immediately followed by the bound next dispatch at the same time.

The shared restart validator runs before scheduler state is usable. It requires
the singular immutable requirement adjacent to run admission (after the exact
capacity requirement when both owners are configured), exact canonical row
bytes and retained digests, the complete dispatch/disposition/deferred/wake/
resume and resource-transition ordering, exact V3 graph binding, and the accepted execution-availability
graph for every retained occurrence. Query-only snapshots expose these exact
facts but perform no scheduling transition.

For an availability-required attempt, the generic worker-adapter event surface
cannot change scheduler state. Every `AdapterEventV1` kind is refused before a
provider disposition and after one is retained. `WAITING_APPROVAL` is accepted
only from the exact owner disposition; post-admission continuation uses the
exact same-execution resume transition, and `PROVIDER_COMPLETED` can close only
through the exact identity-bound terminal receipt. The legacy generic event
surface remains available only to runs that did not admit the HOLDING
requirement.

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

The fake-adapter matrix is mechanism-valid rather than an enum-only test: its
six closed outcome rows enter the same `record_provider_disposition` graph,
journal, query-only replay, resource-release, wake, and legacy-transition stop
laws as accepted mapper evidence. Exact rate-limited and provider-unavailable
non-admission cases park, wake, and then reach a separately exact admitted
completion. Authentication, pre-created transport, possible-admission
transport, and protocol cases remain no-redispatch states. Independent FUEL
fixtures retain abundant and UNKNOWN quota decisions separately; the fake
owner never creates quota evidence.

Exact mapper parity is qualified from the accepted Switchyard owner itself,
not inferred from five terminal examples. The qualification harness runs the
complete checked owner transition suite, terminalizes each completed public
mapper-operation prefix through the owner's own acquisition-cut law, and pins
the resulting exact-dispatch/executable-bound corpus. Nightshift must reopen
every retained prefix and reproduce its mechanism state, execution identity,
client-lane custody, ordinal high-water behavior, and normalized discrepancy
vocabulary. The corpus includes distinct gap, duplicate, and reorder histories.
Alternate binding and executable substitutions remain graph-level negative
cases and are not admitted into the positive transition corpus.

No provider profile, credential, provider process, network contact, live
session, timer activation, service installation, or production route is
authorized. No aggregate provider-health or campaign result is created.

SECOND-WATCH remains `PLANNED / NOT STARTED`.
