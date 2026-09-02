# Self-hosted foreman bootstrap V1

Campaign: SECOND-WATCH

Canonical slug: nightshift-self-hosted-foreman-bootstrap-v1

Status: accepted contract; active bounded runtime/store qualification checkpoint

This contract defines one bounded local operator invocation that transfers
ordinary scheduling custody to the durable Nightshift foreman. It does not make
Nightshift an authority office and does not make a model conversation part of
the scheduler.

## Claim boundary

The future qualification may establish only that one fresh harmless packet can
be admitted, scheduled, parked, woken, recovered, dispatched, collected, and
closed by local durable mechanism after one bounded operator invocation.
Protected decisions remain outside this surface. The record is not a campaign
result, aggregate verdict, provider-quality signal, authorization, approval, or
target-effect receipt.

The implementation remains in the separate nightshift-foreman operator tool.
Canonical nightshiftd retains exactly its two production binaries.

## Versioned record

The closed record is nightshift.self-hosted-foreman-bootstrap/v1. Its
bootstrap_digest is SHA-256 over:

1. the ASCII domain nightshift.self-hosted-foreman-bootstrap.digest/v1;
2. one NUL octet; and
3. RFC 8785/JCS bytes of the complete record with bootstrap_digest omitted.

Digests and commit identities use lowercase hexadecimal. evaluated_at uses the
same canonical UTC lexical law as the accepted foreman owner contracts.
Unknown fields refuse.

The record pins:

- exact HOLDING result and qualified subject;
- exact durable roadmap, MIDNIGHT result, and SILICON result;
- exact accepted Codex and Switchyard provider-admission owner subjects;
- a fresh packet identity distinct from the sealed V2 packet;
- exact foreman admission and execution profile;
- exact capacity requirement and capacity policy;
- exact execution-availability requirement and policy;
- one local runtime identity;
- one bootstrap occurrence and one distinct run identity;
- expected work-item and initially runnable-lane counts;
- the presentation-only question work item;
- bounded driver steps and wall time; and
- fixed non-authorizing, non-recursive policies.

Standalone record validation proves the closed shape, content identity, fixed
predecessors, fresh-packet distinction, bounds, and authority constants. It
does not admit a run.

## Exact graph preflight

validate_graph reopens exact supplied bytes for the packet, admission, profile,
capacity requirement, capacity policy, execution-availability requirement, and
execution-availability policy. It runs every owner validator and then requires
complete digest and identity equality.

The graph must establish:

- packet currentness at evaluated_at;
- packet, admission, profile, both requirements, and both policies name one
  exact run and packet;
- the profile budget-policy reference equals the capacity policy ID while the
  separate capacity policy digest remains exact;
- the availability requirement policy ID and digest equal the policy;
- packet, profile, and availability work-item key sets are equal;
- work-item packet model class, profile model class, capacity cost-class map,
  and ordered availability model selections agree;
- exactly one admitted qualification adapter is used by every work item;
- its ID, protocol, version, and executable identity agree with the
  availability requirement;
- at least two packet lanes have no dependencies and both packet and admission
  concurrency bounds permit two workers;
- admission concurrency never exceeds the packet worker-budget concurrency
  ceiling;
- the named question work item exists;
- recursive worker swarms are forbidden; and
- no packet work item is itself the SECOND-WATCH campaign.

The qualification adapter registration is not selected by cross-record
agreement alone. For `CAMPAIGN_QUALIFICATION_DETERMINISTIC_FAKE`, the graph
requires the exact accepted HOLDING owner mapping:

- adapter ID `nightshift:holding-pattern-deterministic-fake-adapter`;
- protocol `nightshift.holding-deterministic-provider-admission-evidence/v1`;
- adapter version `v1`;
- executable identity
  `sha256:e8a310d46cb40b0aef6399a8da6c97ac99f0fc5eab6a78c5e7007600d5cbfa82`; and
- an empty bounded-argument vector.

All of this occurs before a SQLite path is admitted or created. A pathname is
not contract identity. Future runtime work must preserve the existing
no-follow, exact-file, and query-only store custody laws.

## Identity separation

These identities remain distinct:

1. operator bootstrap occurrence;
2. durable run;
3. packet and admission;
4. scheduler process occurrence;
5. work attempt;
6. dispatch/admission occurrence;
7. provider request occurrence;
8. adapter process and App Server session estate;
9. provider execution; and
10. closeout occurrence.

A process restart never creates a new run, attempt, or provider execution.
A safe pre-admission wake creates a fresh dispatch occurrence for the same
unstarted attempt. A post-admission restart may resume only the exact retained
provider execution where the accepted adapter permits it.

## No self-approval

The bootstrap record fixes approval_response_authorized,
protected_effect_authorized, and target_effects_authorized to false.

A worker may surface an exact human question. The durable foreman retains the
question and continues resource-independent work. Neither bootstrap, scheduler,
adapter, report, nor Casework can answer it. Authority-required work reaches an
exact waiting or not-started disposition under its owner law. No successful
local test, provider admission, receipt, restart, wake, or closeout converts
that question into approval.

## No authority widening

The only authority effect is LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY. The
bootstrap cannot:

- mutate AG standing or authorization;
- call Docket execution;
- produce ABSD effect custody;
- synthesize an approval response;
- contact a protected target;
- infer authority from packet possession, provider admission, or test success;
- refresh stale packet/admission evidence;
- change model, provider, prompt, workspace, or exact brief after admission;
- turn mechanism history into a semantic classification; or
- create an aggregate result.

Restart and reconciliation consume exact retained evidence. They do not renew
or replace operator authority.

## Bootstrap recursion law

V1 is depth zero. parent_bootstrap_occurrence_id is null. bootstrap_may_nest
and worker_may_invoke_bootstrap are false.

The bootstrap command may not appear as an admitted worker adapter command,
worker brief action, wake hook, closeout hook, or subprocess callback. A
bootstrap record whose packet contains the SECOND-WATCH campaign refuses before
store mutation. A bootstrap occurrence cannot be rebound to another run,
packet, or local runtime.

The initiating outer conversation supplies no scheduler step after the bounded
command. It cannot serve as queue, wake loop, transition memory, receipt
collector, or closeout coordinator. Read-only auditing is separate.

## Bounded driver and wake law

The eventual driver has an exact maximum step count and wall-time ceiling. It
does not spin, hammer a provider, or wait past the bound. Reaching the bound
produces a retained incomplete/stop disposition; it never fabricates terminal
receipts.

A wake source only says that reevaluation may occur. It supplies no capacity
testimony, availability testimony, standing, authorization, or provider
admission. Wake uses current exact evidence and the accepted HOLDING atomic
transition law. Duplicate wake and concurrent scheduler openers converge to at
most one fresh dispatch.

No timer or service is authorized by V1. Reference timer or unit artifacts may
be qualified only while disabled and absent from active process/service state.

## Stop laws

The bootstrap stops before store mutation on:

- malformed, noncanonical, unknown-field, oversized, or digest-invalid input;
- any predecessor, packet, run, admission, profile, policy, adapter, model,
  question-path, or runtime-identity substitution;
- any alternate qualification-adapter registration, even if coherently resealed;
- stale packet or admission;
- reuse of the sealed V2 packet;
- fewer than two independent runnable lanes;
- bootstrap recursion or a SECOND-WATCH packet work item;
- any target-effect, approval-response, protected-effect, semantic-retry,
  production, timer, or service authorization;
- missing exact capacity or execution-availability evidence; or
- an untrusted store path failing existing custody admission.

After admission, the scheduler stops or parks only under exact existing owner
law:

- UNKNOWN or unsafe capacity cannot start new work;
- admission-indeterminate execution cannot redispatch until reconciliation
  proves non-admission;
- admitted interruption can only resume the same execution;
- unanswered approval remains waiting without response;
- maximum dispatch/backoff/deferral bounds stop new dispatch;
- contract or history discrepancy stops the affected lane;
- a lane-local question does not block independent lanes; and
- closeout refuses until every item has an exact terminal or not-started
  receipt.

No lane-local stop is promoted into an aggregate campaign classification.

## First runtime/store checkpoint

The bounded bootstrap CLI acquires all eight exact input files as nonempty regular
files with no-follow semantics and a 16 MiB per-file ceiling. It completes the
accepted graph preflight before opening or initializing the SQLite destination.
Run admission keeps the immutable admission time distinct from the later
bootstrap evaluation time.

One immediate transaction retains the normal run, capacity, and
execution-availability owner history together with the exact bootstrap and
capacity-policy bytes. Two append-only tables retain:

- one exact bootstrap occurrence per run, its digest, bounded driver limits,
  evaluation time, and deadline; and
- canonical driver-step bytes in exact ordinal order.

Update and delete triggers protect both tables. Query-only reopen first uses
the existing contract owner loader to cross-bind every authoritative runs-row
digest, canonical byte, admission time, expiry, and concurrency column. It then
revalidates the full bootstrap graph from the exact retained packet, admission,
profile, requirements, and policies before returning typed or raw custody.

A driver step is an observation, not a dispatch. It binds the exact bootstrap,
run, scheduler-process occurrence, ordinal, observed scheduler-projection
digest, time, and one closed disposition. Worker dispatch, approval response,
protected effect, semantic retry, and aggregate-result fields are fixed false.
An already-retained expected ordinal returns the exact winner record so
concurrent writers converge; skipped ordinals and wrong bootstrap identities
refuse. Driver timestamps never move backward. `BOUND_REACHED` and
`ALL_ITEMS_EXPLICIT_TERMINAL` close the append history while exact
duplicate-ordinal reopen remains idempotent.
A failed append rolls back, and restart may retain that same unused ordinal.

This checkpoint does not yet invoke the deterministic fake adapter or complete
the self-hosted golden journey. It starts no provider, subprocess, listener,
timer, service, browser, or production route.

## Qualification boundary

The first runtime qualification must use deterministic campaign-owned adapter
evidence only. It must not read/copy an authentication profile, contact a real
provider, start a browser, install or activate a service/timer, or change a
production/default route.

This checkpoint activates only append-only bootstrap custody and observation-only driver steps. Worker dispatch, wake, receipt collection, final closeout, and the full golden journey remain outside this first runtime freeze; no provider or timer is active. The contract checkpoint was independently
accepted.
