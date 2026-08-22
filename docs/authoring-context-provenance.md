# Maude authoring-context provenance v1

Status: canonical runtime seam for new proposal handoffs. This is lineage, not
permission.

## Owner and handoff

The canonical handoff is
`PrecompiledWorkflowProposalV2::compile` → durable
`CanonicalStore::prepare_ag_occurrence`. Nightshift owns it because this is the
first boundary that simultaneously possesses the exact Maude source context
and the final AG campaign, occurrence, proposal, work, and Nightshift intent
identities. Maude and Phosphor-ng are read-only consumers of the resulting
relation; AG remains Maude- and NQ-agnostic.

An optional `authoring_context` member of
`nightshift.canonical_cycle_request.v1` has schema
`nightshift.maude_authoring_context_handoff.v1`; its inner
`nightshift.maude_authoring_context_input.v1` carries `plan_ref`, `session_id`,
and `plan_text`. New authoring inputs must pass the separate producer/session
authentication contract in
[`authoring-context-custody.md`](authoring-context-custody.md). Nightshift
hashes the exact UTF-8 `plan_text` and requires equality with Maude's
`plan_ref`. A context without an exact proposal is refused. The source text
participates in the sealed cycle-request identity but is not copied into the
durable provenance record.

## Durable relation

`nightshift.authoring_context_provenance.v1` binds:

```text
Maude plan_ref + supervised session_id
Nightshift source_intent_id
AG campaign_id + occurrence_id + proposal_id
exact_work_id
producer_component + recorded_at + source plan byte length
```

`proposal_id` uses AG-NG's domain-separated
`ag.governed-loop.proposal/v1` JCS identity law; a cross-repository test vector
pins both implementations. `provenance_id` is SHA-256 of the JCS record with
that field omitted. Nightshift validates the self-digest and then separately
validates campaign, occurrence, proposal, work, and intent against the exact
prepared AG request. Recomputing a self-digest after substituting relationship
facts therefore cannot make them match the independently persisted request.

The relation is stored atomically with proposal preparation in
`canonical_authoring_context_provenance`. Campaign plus occurrence and cycle
are unique. There is no update API. A repeated cycle request is deterministically
refused by existing slot/occurrence uniqueness; a conflicting context cannot
replace the winner. Concurrent conflicting proposal preparation is serialized
by the same immediate transaction and unique occurrence claim, so at most one
relation wins. Replay, reopen, and export compare the append-only relation with
the authoritative cycle snapshot and exact prepared request.

## Read projection

The read-only command accepts exactly one complete query form:

```text
nightshift --store STORE cycle export-authoring-context \
  --campaign-id CAMPAIGN --occurrence-id OCCURRENCE

nightshift --store STORE cycle export-authoring-context \
  --proposal-id PROPOSAL

nightshift --store STORE cycle export-authoring-context \
  --plan-ref PLAN --maude-session-id SESSION
```

The output schema is `nightshift.authoring_context_export.v1`; it echoes the
typed query and returns zero or more exact records. Zero means not recorded,
not invalid. A read-only currentness consumer may still inspect a store from
before this projection existed; only this new export refuses that unmigrated
schema.

## Cut line and successors

Historical cycles are not backfilled. Existing snapshots deserialize with no
relation, and the new query table starts empty for them. `unlinked` therefore
means unlinked. The cut line is operational: only a new cycle request that
actually carries the exact v1 Maude context can mint a relation; neither
Nightshift nor either UI discovers old context after the fact.

Each new occurrence receives a relation only from authoring context present in
its own sealed cycle request. No predecessor-copy or mutable "current Maude
link" operation exists. Runtime-generated successors without new Maude context
remain unlinked.

## Authority neutrality and nonclaims

Authoring-context provenance establishes lineage, not permission. The record
is absent from AG's open-occurrence request and is not an input to NQ admission,
Nightshift currentness, standing, admissibility, authorization, spend, Docket
custody, retry eligibility, or human disposition. Missing provenance neither
admits nor refuses otherwise valid work; present provenance upgrades no trust.

The lineage record's `producer_component` still names a software boundary, not
an authority. For new handoffs, the separate custody record identifies and
authenticates the Maude session issuer and delivery producer without changing
this lineage meaning. Maude/session-service honesty, host credential custody,
executable integrity, and external-world truth remain environmental
assumptions.
