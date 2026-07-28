# NQ Diagnostic Consumer v2

Status: implemented consumer contract. NQ package and live-emitter
correspondence remain external, commit-specific qualification claims; this
decision does not assert either one by itself.

## Decision

Nightshift accepts both immutable NQ diagnostic contracts:

- `nq.diagnostic_execution.v1`;
- `nq.diagnostic_execution.v2`.

The v1 path remains byte-compatible. Its numeric `clock_uncertainty_ms` is
treated as the bounded producer claim defined by that frozen contract. The one
defensive exception is an artifact that explicitly carries the limitation code
`absolute_clock_quality_unqualified`; that contradiction cannot earn
`Current`.

V2 retains two distinctions that v1 cannot encode:

- acquisition intervals carry either a tagged `bounded` qualification with an
  identified basis and maximum error, or an exact tagged `unqualified` reason;
- received-input and outcome refusals retain the exact
  `nq.governed_refusal.v1` carrier instead of a code/reason projection.

An unqualified interval produces `ClockUnqualified` standing and operator
status. It never produces `Current`, even if its timestamp is recent. Exact v2
refusals remain in the embedded NQ trace. The trace also carries a typed
input-failure summary for operator distinction; the full exact failure carrier
remains in the embedded artifact. Nightshift reclassifies neither.

Even a bounded v2 producer interval cannot earn `Current` against the current
bare Nightshift `evaluated_at` instant. That instant has no identified,
qualified clock or admitted comparison relation. Nightshift therefore retains
the NQ condition/refusal but reports `ClockUnqualified` standing with an
explicit consumer-clock reason.

The same boundary applies to recurrence. Nightshift preserves the frozen v1
recurrence containment check, but does not apply that raw wall-clock
comparison to v2: the NQ interval now identifies and qualifies its clock,
while the current Nightshift recurrence carrier does not identify or qualify
the invocation clock. Request, run-slot, and artifact identities still bind
the completion. A future contract may add a qualified cross-clock relation;
this transition does not invent one.

## Explicit Nightshift contract transition

The widened structures are not emitted under their former v1 schema names.
Current emission uses:

- `nightshift.diagnostic_posture_policy.v2`;
- `nightshift.diagnostic_inputs.v2`;
- `nightshift.recurrence_evidence.v2`;
- `nightshift.nq_diagnostic_sources.v2`;
- `nightshift.nq_source_import_receipt.v2`;
- `nightshift.operational_posture.v2`;
- `nightshift.operator_projection.v2`;
- `nightshift.concordance_policy.v2`;
- `nightshift.operational_posture_concordance.v2`.

The posture-policy, diagnostic-input, recurrence, source-manifest,
source-receipt, and concordance-policy readers retain their v1 schema readers.
A v1 Nightshift carrier may contain only its original NQ v1/interval surface;
in particular, a v1 recurrence artifact reference must omit the v2
`contract_schema` and `profile_semantic_id` fields. It refuses a v2 artifact or
v2 profile-semantic binding rather than silently widening v1. A v2 posture
policy binds the exact NQ
`profile_semantic_id`; cross-vantage comparison additionally requires one
exact semantic identity for the complete declared comparison set. Unknown
contract versions refuse.

The posture evaluator and concordance evaluator advance to semantic version 2.
That identity change is independent of NQ artifact identity and does not
rewrite the source execution.

This slice does not ship a frozen v1 posture re-evaluator. Previously emitted
`nightshift.operational_posture.v1` bytes are historical artifacts, not inputs
that the v2 evaluator silently reopens under new semantics. The v1 foundation
defined posture evaluation as a pure, read-only CLI operation that did not
open the SQLite store, persist a snapshot, or provide a posture read/reopen
command; see the
[v1 foundation](NQ-NIGHTSHIFT-STAGE6-FOUNDATION.md) and
[v1 operator specimen](../../operator/examples/diagnostic-posture-v1/README.md).
That absence of a durable historical posture store/read contract is the
bounded compatibility premise for this transition. If v1 posture bytes later
require product-supported historical interpretation, they need their own
frozen v1 reader/evaluator rather than best-effort v2 decoding.

## Consumer boundary

Nightshift validates and retains the exact NQ artifact bytes made available
through the pinned package/source manifest. It compares only the bounded NQ
claim/outcome surface already admitted by NQ. It does not reinterpret raw
observations, qualify a producer clock by itself, select a winning vantage,
grant reliance beyond its declared consumer policy, authorize action, or
execute anything.

The package pin is version-specific: v1 uses
`share/nq/diagnostic-contract/manifest.json` and asset-manifest schema
`nq.diagnostic_contract_assets.v1`; v2 uses
`share/nq/diagnostic-contract-v2/manifest.json` and
`nq.diagnostic_contract_assets.v2`. A pin cannot relabel one asset namespace
as the other.

The importer verifies the exact payload-manifest, contract-manifest, contract
asset, and artifact bytes. It also evaluates every NQ-owned valid and hostile
fixture declared by that exact package through the selected Nightshift
consumer before admitting a delivered artifact. Valid fixtures must be
accepted and hostile fixtures must remain rejected; Nightshift does not copy a
second fixture corpus into its repository.

The source manifest's repository, commit, and release fields are
consumer-supplied declarations: they are syntax-checked but not attested by
the current NQ package. Human output labels them
`nq_source_declaration` with `attestation=unverified` and reports the
digest-verified surface separately as `nq_package_bytes`. This slice claims
exact identity for the consumed artifact bytes, contract assets, and declared
payload-manifest bytes—not authenticated source provenance or the identity of
every installed payload byte.

The v2 detector-refusal correspondence is checked against the exact profile
semantic identity, detector boundary, `cannot_evaluate` refusal code, and
summary. Received-input refusals retain their exact refusal and explicit
artifact-profile or foreign-input-role binding. Other exact refusal-origin
payloads remain typed versioned carriers but have no Nightshift-owned
diagnostic meaning.

## Remaining integration gate

The implementation is not a live-engine correspondence claim until NQ's
production engine emits canonical v2 artifacts and supplies NQ-owned valid and
hostile vectors. Required integration specimens are:

1. completed bounded-clock execution;
2. completed unqualified-clock execution;
3. exact governed refusal;
4. typed provider no-response;
5. unknown/substituted hostile documents.

Those exact bytes must cross the package/process boundary unchanged and pass
Nightshift's read-only posture and concordance paths.
