# Casework live provider-execution projection V1

`nightshift.casework-live-provider-execution/v1` is an additive query-only
projection over exact HOLDING foreman journal history. It does not revise the
sealed `nightshift.casework-live-run/v1` contract.

The projection is available through the existing live API's `GET`-only method
law at:

`/api/v1/active-runs/{navigation-id}/provider-execution`

The foreman owner reopens each canonical journal event, validates its retained
raw digest and exact nested typed/raw equality, and rejects unknown placement.
Casework then cross-binds the singular requirement and policy to the exact
packet, admission, execution profile, adapter registration, provider identity,
and transaction-consistent read snapshot. The projection retains ordered
dispatch, disposition, deferral, wake, resume, and resource release/reacquire
facts with source-byte SHA-256 custody.

Observation currentness is explicit: `NOT_YET_CURRENT` before receipt,
`CURRENT` from receipt until but excluding expiry, and `EXPIRED` afterward.
Expired evidence remains historical evidence. Provider capacity from FUEL is
reported as a distinct status; it is neither derived from nor overwritten by
provider-execution availability.

Legacy or non-HOLDING runs return the explicit
`NOT_RECORDED_BY_FOREMAN` absence record. Provider journal rows without the
owner history, any typed/raw split, sequence/cardinality disagreement, or
identity substitution refuse the projection.

This surface exposes mechanism evidence only. It supplies no target-effect,
approval-response, retry, dispatch, wake, timer, provider, campaign-result, or
other authority, and it creates no composite classification.
