# GAUGE-LATCH qualification

Campaign: GAUGE-LATCH

Canonical slug:
`nightshift-foreman-provider-capacity-admission-binding-v1`

Track: `nightshift-foreman-provider-capacity`

Sealed V2 packet:
`sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`

MIDNIGHT integration parent:
`8c5ee9f26df998b690b3deb37fc6106979d413e7`

Exact independently accepted qualified subject:
`62f11d6a5f2622fff2edd4e3eaee140803ab09f0`

Classification:
`GAUGE-LATCH-FOREMAN-PROVIDER-CAPACITY-ADMISSION-BINDING-V1-QUALIFIED`

This is an independent GAUGE-LATCH classification. It is not an aggregate
campaign or run classification and grants no provider, approval, target-effect,
retry, or production authority.

## Entry and owner correction

MIDNIGHT-RAIL's sealed mutation surface is qualification-only. Read-only entry
evaluation found that the accepted foreman and FUEL contracts did not yet bind
one exact provider-capacity decision atomically to attempt admission. That was
an owner-level acceptance gap, so GAUGE-LATCH was created as a separate,
non-rewriting successor correction. MIDNIGHT remained clean at its integration
head while this owner contract was implemented and independently audited.

The accepted subject descends the exact integrated MIDNIGHT parent and the
non-rewriting GAUGE checkpoints `18b3c62`, `90c9f7d`, `e64b8da`, and
`62f11d6`. No predecessor or default-branch history was rewritten.

## Qualified subject

A capacity-aware run admits one immutable
`nightshift.foreman-capacity-requirement/v1` record atomically with
`RunAdmitted`. It binds packet, run admission, profile, policy ID, provider
identity, and the packet's complete closed model-class-to-cost-class mapping.
Legacy runs remain readable and retain their predecessor preparation law;
capacity-required runs refuse legacy attempt preparation.

Each capacity-required attempt atomically retains one exact
`nightshift.foreman-capacity-admission/v1` and the exact canonical FUEL
observation, policy, and decision bytes before `AttemptCreated`. The foreman
recomputes the exact FUEL decision at the retained decision time and requires
full equality. It binds policy ID separately from policy digest, provider,
packet/profile model class, adapter, run, work item, attempt, start request,
currentness, and account-wide or exact matching model-family scope.

The exact FUEL disposition is enforced without inventing another policy:
`NO_NEW_WORK` refuses new attempts; `CHEAP_BOUNDED_ONLY` admits only the
closed cheap model classes; and `ORDINARY_BOUNDED` admits according to its
exact expensive-work flag. Speculative admission is unbound by the packet and
profile and therefore fails closed.

Restart, mutating preparation, replay, and query-only snapshot use one shared
capacity-history validator. It proves singular requirement placement directly
after run admission and one capacity-admission event directly before each
attempt creation, with equal timestamps and complete cross-event identities.
Digest-consistent separation, reordering, omission, substitution, or
same-attempt/different-work histories are refused before scheduler state is
usable.

Capacity journal rows have an independent per-event profile ceiling and a fixed
16 MiB checked cumulative raw-byte ceiling. The exact row-count ceiling is one
requirement plus at most one admission per packet work item. Projection and
query-only snapshot run a transaction-local SQL metadata preflight over the
special capacity row kinds before selecting any event BLOB. Append checks both
the retained history and candidate row atomically. Legacy non-capacity
`internal` rows are excluded from the new capacity-only byte law.

The query-only projection exposes exact retained capacity facts without adding
Casework mutation or inferred provider health.

## Deterministic qualification

Final replay against exact subject
`62f11d6a5f2622fff2edd4e3eaee140803ab09f0`:

- full locked Rust workspace: 374 passed, 0 failed, 14 documented
  environment-dependent ignores;
- nightshift-foreman integration suite: 28 passed, 0 failed;
- foreman schema suite: 7 passed, 0 failed;
- all-target/all-feature warnings-denied nightshift-foreman Clippy: passed;
- workspace formatting and diff checks: passed;
- canonical Nightshift no-actuation gate and deterministic negative control:
  passed;
- provider-capacity boundary gate and deterministic negative control: passed;
- GAUGE capacity-admission boundary gate and deterministic negative control:
  passed.

Direct qualification includes the complete FUEL state/admission matrix:
ABUNDANT expensive and cheap success; NORMAL expensive and cheap success;
CONSERVE cheap success and expensive refusal; UNKNOWN with an exact
cheap-admitting decision admits cheap and refuses expensive; UNKNOWN
`NO_NEW_WORK` refuses cheap and expensive; CRITICAL refuses both. Every row
asserts the exact derived state and disposition.

Negative cases cover legacy-path bypass, a concurrent alternate transition,
stale evidence, policy/provider/model-family substitution, speculative
admission, digest-consistent substituted FUEL outcome and decision time,
requirement/admission/attempt reordering or omission, prior-history mutation,
same-attempt/different-work substitution, per-event oversize, cumulative
history overflow, and legacy non-capacity compatibility. A 16 MiB-plus invalid
capacity `zeroblob` and an over-count set of malformed one-byte capacity rows
both return the exact metadata-bound refusal from projection and query-only
snapshot before raw digest or JSON decoding.

## Custody and limitations

The campaign branch is
`campaign/gauge-latch-nightshift-foreman-provider-capacity-admission-binding-v1-20260830`.
Publication is limited to the already-established Nightshift `origin`; one
bounded push and exact remote-SHA verification occurs only after this result
record is committed. No default branch changes.

No provider probe, model turn, App Server process, provider session,
authentication-profile copy, approval response, protected effect, listener,
service, browser profile, production activation, or target-effect mutation was
created. No credential or secret was read or retained.

TUNNEL-FINCH's real-provider lifecycle remains independently
`NOT-RUN-AUTHORITY-REQUIRED`. GAUGE-LATCH neither answers nor weakens that
human authority boundary.

Successor base policy: exact result ancestry.

## Human questions

None.

## Next lawful action

After exact result publication and verification, merge this GAUGE result
non-rewriting into the held MIDNIGHT-RAIL integration branch. MIDNIGHT may then
run only its sealed deterministic fixture qualification. It must preserve the
real-provider lifecycle as not run and must not activate HOLDING-PATTERN before
the current sealed V2 run closes.
