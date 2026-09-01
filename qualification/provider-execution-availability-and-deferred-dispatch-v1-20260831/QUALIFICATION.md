# HOLDING-PATTERN qualification

Campaign: HOLDING-PATTERN

Canonical slug: provider-execution-availability-and-deferred-dispatch-v1

Qualified subject: 57c165fb246a530bc9448afbe3a26c17a5118ebd

The result is the non-rewriting commit containing this closeout, with the
qualified subject as its first parent. No aggregate classification is created.

## Exact accepted topology

| Stage | Exact accepted subject | Relationship |
| --- | --- | --- |
| Codex provider-admission owner | c36a8137638decf8b04a49611354a90f32c5a945 | Pinned external owner; no inherited executable identity |
| Switchyard admission mapper | 2ba25db66d8b29dd215bd87e05f4ea794024b3b7 | Pinned sole-local owner and exact corpus |
| Nightshift contract graph | 8c11e8c45978a10bc32dd75a9091d1a5ad8fda32 | Exact ancestor |
| Worker-start V3 | 540f3945b695e5c7400454e0d99b005041c893f7 | Exact ancestor |
| Durable foreman storage | 8bf9099a3481fa83e93e1806cca5397b97763be8 | Exact ancestor |
| Query-only Casework backend | 0d2f735fbd2a4a00adad43688b28582dcc51ff22 | Exact ancestor |
| Read-only Casework UI | 57c165fb246a530bc9448afbe3a26c17a5118ebd | Qualified subject |

Nightshift also retains MIDNIGHT-RAIL result
6160a7fac9845aaefefbc11847e55786b35749e6, SILICON-ORCHARD result
f6e95c8a51982a9381c27c4792c8d9fd6f1daf47, and roadmap
70e3b734e979173ae552efb322b48bf7fb0c028b as ancestors. Codex and
Switchyard remain separately owned repositories, pinned rather than
represented as Nightshift ancestry.

## Independent dimensions

| Dimension | Classification |
| --- | --- |
| Codex first-sampling/admission owner | QUALIFIED_DETERMINISTIC_MOCK_PROVIDER |
| Switchyard exact ordered mapper | QUALIFIED_DETERMINISTIC_MOCK_APP_SERVER |
| Nightshift contract graph | QUALIFIED |
| Worker-start V3 | QUALIFIED |
| Durable foreman mechanism | QUALIFIED_DETERMINISTIC |
| Twenty-case matrix | QUALIFIED_DETERMINISTIC |
| FUEL independence | QUALIFIED_DETERMINISTIC |
| Query-only Casework backend | QUALIFIED_READ_ONLY |
| Read-only Casework UI | QUALIFIED_DETERMINISTIC_UI |
| Real provider lifecycle and egress | NOT_RUN |
| Real Codex App Server executable identity | NOT_RUN |
| Live timer, unit, service, or wake source | NOT_RUN |
| Production/default route or default branch | NOT_RUN |
| External authority or protected effect | NOT_EXERCISED |
| Installed/headless browser | NOT_RUN |
| Supplemental outer capacity refusal | OBSERVED_ONLY_NOT_QUALIFICATION_EVIDENCE |

These dimensions are independent. Deterministic mechanism qualification does
not claim a real provider lifecycle, and FUEL capacity does not determine
provider execution availability.

## Twenty-case deterministic matrix

All cases use campaign-owned local fixtures and the production mechanism
transition path, with no provider connection or live timer.

1. **99% quota plus selected-model capacity:** passed; abundant FUEL evidence
   remains separate while provider execution parks the same attempt.
2. **Retry-after and wake-at:** passed with exact refusal time, retry-after,
   derived wake time, policy, and dispatch identity.
3. **Repeated capacity refusal:** passed; fresh dispatches advance bounded
   backoff without a new work attempt.
4. **Capacity later returning:** passed through wake, fresh dispatch, exact
   execution admission, and completion.
5. **Rate limit later returning:** passed through the qualification-only fake
   owner and the common park/wake/store path.
6. **Transport failure before known admission:** passed as a distinct
   no-silent-redispatch record.
7. **Lost response after possible admission:** passed as indeterminate and
   requiring exact reconciliation.
8. **Crash while parked:** passed; restart preserves attempt, refusal, backoff,
   resource policy, and wake time without refreshing evidence.
9. **Crash after wake before dispatch:** passed through a bounded fault seam;
   atomic rollback leaves no partial group.
10. **Duplicate wake convergence:** passed across concurrent writers with one
    reacquisition, wake, and fresh dispatch.
11. **Permitted ordered fallback:** passed; only the next exact pre-admission
    model selection is used.
12. **Forbidden fallback:** passed; disabled/exhausted fallback cannot advance
    the model ordinal.
13. **Post-admission model change:** refused; resume retains the exact
    provider/model/session/thread/turn/response identity.
14. **Bounded backoff/no hammering:** passed; dispatch and total-deferral
    ceilings stop occurrences and semantic retry remains false.
15. **Independent lane progress:** passed under exact worker-slot and resource
    policy while another lane is parked.
16. **Exact replay/concurrent writer:** passed; alternate histories converge or
    refuse without smoothing journal differences.
17. **Dispatch/execution substitution:** refused across all bound identities.
18. **Reconciliation proving non-admission:** passed into the park law without
    rewriting the indeterminate occurrence.
19. **Reconciliation proving admission:** passed only for the same attempt and
    exact execution identity.
20. **Unresolved reconciliation:** passed; indeterminate state and the
    lane-local stop law remain with no redispatch/fallback.

Direct Rust coverage includes 17 contract cases and 29 named durable-store
HOLDING cases. Exact Switchyard parity reopens five terminal snapshots and all
126 owner terminal prefixes, including 118 generic-replayable compatibility
histories, while the strict ordered decision subset remains enforced.

## Owner versus qualification fixture

The accepted Codex/Switchyard owner permits automatic parking only for literal
pre-response-created modelAtCapacity with ordered raw testimony and a clean
cut. Provider-unavailable, rate/usage-limit, authentication, quota, transport,
protocol, coarse, unknown, and post-created conditions gain no automatic
parking authority from that owner.

The campaign-owned fake adapter is qualification-only. Its closed record binds
producer and fixture executable identities, provider/model, attempt, dispatch,
response-created, retry-after, times, raw bytes, and digest. Exact rate-limited
and provider-unavailable non-admission exercise park/wake/recovery;
authentication has no automatic retry; transport/protocol remain
no-redispatch/indeterminate. The fixture has no production registration,
binary, or listener and cannot widen the production taxonomy.

## FUEL and Casework

FUEL quota/capacity and execution availability are separate owner records.
Abundant quota does not erase model-at-capacity; UNKNOWN or NO_NEW_WORK refuses
before dispatch and is not overwritten. The fake owner never constructs
QUOTA_EXHAUSTED_FUEL_OWNED.

The additive nightshift.casework-live-provider-execution/v1 projection reopens
the foreman snapshot query-only. It shows explicit absence or exact
requirement/model ordering, attempt versus dispatch, all dispositions and time
axes, wake/backoff/fallback, resource predecessor edges, approval, execution,
interruption, completion, same-execution resume, independent FUEL digests, and
raw event routes. The browser uses GET only; HEAD and writes remain 405. It has
no form, approval response, retry, dispatch, execution, merge, promotion,
aggregate result/health, or other control.

## Final replay

- locked Rust workspace: **463 passed, 0 failed, 16 documented ignores**;
- foreman contract **17**, integration **59**, worker-start V3 **5** passed;
- executable schema/report suite **58 passed**;
- frontend **32 passed across 7 files**; production build passed;
- all-targets/all-features warnings-denied Clippy and formatting passed;
- Switchyard parity: **5 snapshots, 126 terminal prefixes, 118
  generic-replayable, passed**;
- V3 boundary passed and canonical nightshiftd binary count remained two;
- no-actuation, HOLDING mechanism, HOLDING qualification-receipt, foreman
  query-only, foreman capacity, provider-capacity, sealed Casework, live
  Casework, and UI gates passed;
- all nine deterministic negative controls passed, including the closed
  qualification-receipt substitution control.

The browser journey was not run because no local API listener was started.
Deterministic DOM qualification and production build remain separately passed.

## Custody and teardown

No default branch changed. No production route, unit, timer, or service was
installed or activated. No provider was contacted and no authentication
profile/credential was copied, inspected, or added. No campaign provider
session, App Server/adapter process, browser, API server, listener, store, or
mutable fixture remains.

The census found only unrelated Codex orchestration processes and the
pre-existing loopback CUPS listener on port 631. Historical reviewer files
under /tmp are retained artifacts, not campaign runtime state, profiles,
credentials, or teardown obligations.

SECOND-WATCH remains PLANNED / NOT STARTED.
