# HOLDING-PATTERN — provider execution availability and deferred dispatch V1

> **Track:** `provider-execution-availability`
> **Codename:** `HOLDING-PATTERN`
> **Canonical slug:** `provider-execution-availability-and-deferred-dispatch-v1`
> **Status:** **AUTHORIZED SUCCESSOR / PLANNED / NOT STARTED**
> **Result classification:** none
> **Filed:** 2026-08-30
> **Codename collision search:** clear in local Nightshift source, local and remote branch names, and established remote heads at filing time.
> **Authority:** direct operator successor authorization; activation is automatic after the current sealed V2 run closes cleanly with the prerequisites below. This record does not amend, reseal, widen, or add an edge to that nine-item packet.

## Architectural fact and evidence boundary

Provider quota and provider execution availability are independent scheduling
inputs. The operator-observed motivating condition was:

- weekly account quota reported abundant;
- short-window account quota reported abundant;
- the selected model reported temporary capacity unavailability; and
- the provider client performed no automatic delayed redispatch.

This operator statement establishes the design requirement. It is not itself a
qualified provider observation and must not be promoted into a synthetic live
receipt. Any later live occurrence is supplemental evidence only when its exact
raw source and custody are retained. Qualification must not repeatedly contact a
provider to induce a capacity refusal.

FUEL-NEEDLE remains the owner of quota/budget observation and budget admission.
HOLDING-PATTERN owns the separate execution-availability observation,
pre-execution provider-admission mechanism, persisted deferred-dispatch state,
and reconciliation law. Neither source may infer the other:

- abundant quota does not imply model availability;
- unavailable model execution does not imply exhausted quota;
- absent availability evidence is `UNKNOWN`, never `AVAILABLE`; and
- the two dimensions retain independent classifications and raw evidence.

## Activation gate

This campaign remains **PLANNED / NOT STARTED** until all necessary exact
predecessors from the current V2 DAG are independently accepted and that run
closes cleanly. At minimum activation requires:

1. the durable foreman and its restart/query-only custody laws;
2. the independently qualified worker-adapter owner and consumer contracts;
3. FUEL-NEEDLE as exact budget evidence, including UNKNOWN-safe admission;
4. the accepted current V2 integration result that binds those predecessors;
5. live Casework if the campaign exposes the new mechanism history there; and
6. no unresolved predecessor question that changes dispatch identity,
   acceptance, custody, or stop law.

When these conditions qualify, the direct operator instruction filed by this
record authorizes campaign start without another operator question. A failed or
unqualified prerequisite leaves this campaign **PLANNED / NOT STARTED**, with
the exact unmet predecessor reported. This authorization does not answer an
approval request, authorize provider credentials, enable a timer, activate a
service, or permit production dispatch.

## Closed mechanism distinctions

The normalized contract must preserve at least these distinct meanings, using
an existing qualified vocabulary where it is semantically exact:

- `quota_exhausted`;
- `rate_limited`;
- `model_at_capacity`;
- `provider_unavailable`;
- `authentication_refused`;
- `transport_error`;
- `protocol_error`;
- `admission_indeterminate`;
- `execution_started`; and
- `waiting_approval`.

Wire spellings are adapter-owned raw evidence. The normalized record must bind
each accepted spelling to one closed meaning and preserve unrecognized evidence
raw-only or fail closed. Authentication refusal, protocol failure, transport
failure, rate limiting, model capacity, budget exhaustion, approval waiting,
and provider execution acceptance must never collapse into one generic failure.

## Identity law

The following identities are independent:

1. **Work-attempt identity** — one intended execution of one bounded worker
   brief. It is stable across proven pre-admission deferrals.
2. **Dispatch/admission-occurrence identity** — one bounded attempt to obtain
   provider execution capacity for that work attempt. Every redispatch creates
   a fresh occurrence.
3. **Provider-execution identity** — the exact provider-side execution, turn,
   session, or equivalent identity established only at the qualified provider
   acceptance boundary.

A process occurrence, connection, thread container, status request, or exit code
does not by itself establish provider execution. The dispatch receipt must bind
the exact work attempt, dispatch occurrence, adapter identity/version, provider,
selected model identity or class, availability evidence, acceptance or refusal
evidence, and all relevant times without treating a locator as identity.

## Provider acceptance boundary

The Codex adapter campaign must determine and freeze the exact App Server
response or event that mechanically proves a model turn was admitted. It must
not infer admission from process exit, successful initialization, thread
existence, request transmission, connection establishment, or the absence of an
error.

Qualification must prove the boundary using exact raw App Server evidence and
closed owner/consumer validation. Until that event is known, a lost response or
ambiguous connection transition is `admission_indeterminate`. The contract must
also state which exact canonical status/session operation can reconcile the
indeterminate occurrence and how its identity binds back to the work attempt.

## Dispatch and deferred-wake state law

A representative state relation is:

```text
READY
  -> DISPATCHING
       -> PARKED_NOT_ADMITTED(wake_at, exact refusal)
       -> ADMISSION_INDETERMINATE(reconciliation required)
       -> EXECUTION_ADMITTED(provider execution identity)
            -> ACTIVE | WAITING_APPROVAL | terminal custody
```

Only positive evidence that execution was not admitted permits automatic
deferred redispatch. Examples include an explicit model-capacity refusal,
pre-admission rate-limit refusal, or explicit provider-unavailable refusal,
each received before an execution identity exists.

For a permitted deferral:

- preserve the work-attempt identity;
- create a fresh dispatch/admission occurrence on the next dispatch;
- retain the exact refusal and provider/model binding;
- persist an exact `wake_at` and bounded backoff state before releasing custody;
- release provider-only capacity as defined by policy;
- retain repository or resource locks only when an explicit profile policy says
  so, otherwise release them and reacquire under the ordinary lock law; and
- admit the next dispatch only after the persisted eligibility boundary.

This is provider admission deferral, not semantic work retry, because qualified
evidence establishes that worker execution did not begin. It must not create a
new work attempt.

## Indeterminate admission and reconciliation

When it is unknown whether provider execution began:

- do not redispatch automatically;
- persist `admission_indeterminate` against the work attempt and dispatch
  occurrence;
- retain the ambiguous raw evidence and process/session identities;
- reconcile only through the provider's canonical status or session interface;
- bind every reconciliation result to the exact occurrence and provider
  execution identity; and
- preserve indeterminate state and apply the existing stop law when exact
  reconciliation cannot prove non-admission or admission.

A connection loss, timeout, lost response, process stop, or restart must not by
itself create duplicate work. Reconciliation evidence received after restart
does not rewrite the original mechanism history.

## After execution admission

Once the qualified provider boundary establishes execution:

- provider/session resume may continue the same work attempt when the protocol
  supports exact identity-bound resume;
- no fresh turn or prompt may be started as an automatic substitute;
- no new work attempt may be silently created;
- model migration is forbidden; and
- semantic retry remains outside V1.

Approval waiting remains a distinct mechanism state. This campaign does not
answer approvals or convert unanswered approval into provider unavailability.

## Model fallback

Optional fallback is lawful only before provider execution admission and only
when an execution profile contains an ordered allowed set of exact provider/model
identities or closed classes. Every selection is retained in the dispatch
receipt. A missing policy, unrecognized model, or exhausted allowed set refuses
fallback.

Fallback must not be silent, must not widen provider identity, and must not
occur after execution begins. A model becoming unavailable after admission does
not authorize migration.

## Availability observation

The owner contract should define a normalized provider-execution-availability
record binding at least:

- provider identity;
- exact model identity or closed model class;
- observation time;
- closed availability state;
- source/provenance identity and version;
- optional provider-supplied retry-after time;
- derived exact wake time when policy supplies one;
- exact raw evidence digest and retained raw bytes where permitted;
- expiry/currentness interval; and
- an explicit UNKNOWN disposition when no qualified observation exists.

Observation time, provider refusal time, receiver custody time, foreman
evaluation time, and wake time remain independent. A stale observation is not
current availability, and a wake event does not refresh it.

## Wake behavior and bounded backoff

The parked state is durable and contains an exact `wake_at`, backoff ordinal,
policy identity/digest, last dispatch occurrence, and remaining permitted model
set. Backoff is finite, persisted, bounded, and replay-stable. Duplicate wake
notifications converge on one reevaluation and do not create two dispatches.

An operator CLI, cron, a systemd timer, or another qualified local scheduler may
invoke reevaluation. The wake source supplies no evidence of provider
availability, grants no authority, and does not bypass the foreman transition
law. Reference timer or unit definitions may be created for local qualification
only and must be disabled/inactive at closeout.

## Required deterministic qualification

Use a deterministic fake adapter to qualify:

1. 99% reported quota with selected model at capacity;
2. explicit retry-after retention and exact `wake_at`;
3. repeated capacity refusals with bounded backoff;
4. capacity later returning;
5. rate limit later returning;
6. transport failure before known admission;
7. lost response after possible admission;
8. crash while parked;
9. crash after wake and before redispatch;
10. duplicate wake convergence;
11. permitted ordered model fallback;
12. forbidden model fallback;
13. model change after execution start refused;
14. bounded backoff and no provider hammering;
15. independent lane progress while another lane is parked;
16. exact replay and concurrent-writer alternate transition history;
17. dispatch-occurrence substitution and provider-execution substitution;
18. indeterminate reconciliation proving non-admission;
19. indeterminate reconciliation proving admission; and
20. unresolved reconciliation preserving the stop law.

If a capacity refusal occurs naturally during an explicitly authorized live
qualification, retain it as a separate supplemental dimension. Do not generate
traffic for the purpose of inducing the condition.

Qualification must include full locked workspace tests, warnings-denied Clippy,
formatting, closed schema/runtime parity, deterministic negative controls,
restart/concurrent-writer cases, exact raw-custody checks, read-only Casework
presentation if in scope, and a final process/listener/timer/unit/temp-root and
credential-artifact census.

## Non-goals and closeout

HOLDING-PATTERN adds no semantic retry, approval response, credential brokerage,
production activation, browser write control, remote target actuation, hidden
model substitution, quota inference, aggregate provider-health score, or
authority. Its classification is independent of FUEL-NEEDLE.

At closeout, any reference timer/unit remains disabled and inactive; no provider
session, adapter process, listener, copied profile, temporary credential, or
campaign-owned store remains unless an exact qualification receipt explicitly
assigns retained custody. Until activation succeeds, status remains
**PLANNED / NOT STARTED** and classification remains **none**.

