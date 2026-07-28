# NQ–Nightshift Lean correspondence

This record pins the private formal north star used for the Stage 6 runtime
foundation. It claims correspondence only where named below. It does not
claim extraction, proof-carrying code, runtime equivalence, or full formal
verification.

## Exact pin

- repository: private skunkworks formalization
- commit: `5302a09256160c8fcfb08903a122ece218d40d66`
- tree: `3e9241cde3adfffe2d6b5609964cf84f2b3334a5`
- root module: `formalization/Skunkworks/NightshiftOperationalPosture.lean`

Pinned module blobs:

| Module | Git blob |
|---|---|
| `Core.lean` | `670e4339827bab47417d344ce935ccf29b4e6bdf` |
| `Evaluation.lean` | `596d6ac1e9c5d1bcc356f8bbe491278d94ee6a35` |
| `Scheduling.lean` | `0125e113e9a6a1e642fc2c188b6f4339242d961b` |
| `ProposalBoundary.lean` | `f66e37deaad21ea282efbb182c2087ee778fe818` |
| `OperatorProjection.lean` | `9f023f39884fc8e0aa22ede613b39195aef17091` |
| `Hostile.lean` | `831d72723f2b21462aae60f2c855ffdd2ee71728` |
| `Qualification.lean` | `500a4846c08a9844266a298750e94ccdefb36d66` |

The runtime's NQ consumer mirror is independently pinned to local NQ-NG
commit `81489644fef4cf56b9fb61e943e18d2313728931`, tree
`921b45d082eea93f338f80300b9c8216d45718d5`. That repository has no
configured remote; this is an exact local source/vector correspondence, not a
publication claim.

## Runtime-to-model map

| Runtime surface or vector | Lean definition/theorem | Correspondence claim |
|---|---|---|
| closed `inventory` and `assess_entry` | `PosturePolicy`, `assessEntry`, `mandatory_missing_head_blocks_completeness` | A favorable subset cannot satisfy a missing mandatory row. |
| delivered NQ refusal versus receiver `NoResponse` | `no_response_is_not_nq_refusal`, `nq_refusal_remains_distinct_from_nightshift_no_response` | Producer refusal and receiver silence remain different constructors and output states. |
| exact producer/question/profile/vantage/state binding | `assessDelivered`, `individual_freshness_does_not_establish_joint_state` | Freshness cannot repair a binding or subject-state mismatch. |
| source-age checks | `expired_clean_is_not_current`, `custody_arrival_does_not_refresh_source_evidence`, `future_dated_artifact_is_not_current` | Receipt/completion/evaluation time cannot refresh source evidence; future evidence is not age zero. |
| duplicate declared input and retained undeclared input | `duplicate_input_is_not_latest_wins`, `undeclared_input_is_retained_but_not_minted_into_inventory` | Duplicate matching input blocks; an extra input remains visible without changing the denominator. |
| deterministic jitter, slot, attempt and artifact checks | `deterministicJitter`, `makeRunSlot`, `assessRecurrenceFor`, `recurrence_is_derived_from_bound_slot_and_time`, `recurrence_must_bind_the_exact_diagnostic_artifact` | Recurrence is derived from declared policy and exact current-slot evidence. Retained records for other slots neither duplicate nor replace the due slot. |
| missed recurrence with identical NQ bytes | `last_clean_is_not_current_after_missed_recurrence`, `later_recurrence_loss_invalidates_without_rewriting` | Nightshift currentness may change while the immutable diagnostic artifact does not. |
| internal operator-rendering projection retaining omitted rows | `headlineFor`, `ProjectionConforms`, `visual_omission_is_visible_and_cannot_strengthen`, `dropped_row_cannot_make_a_favorable_subset_clean` | Hiding a row cannot yield a clean headline. This correspondence does not publish a standalone projection contract. |
| contradiction and partial-coverage vectors | `operator_projection_preserves_dissenting_component_state`, `stale_refused_and_no_response_keep_distinct_status_and_origin` | Presentation preserves material dissent and blockers. |
| absence of proposal/authorization/execution fields | `supported_proposal_not_authorized_in_empty_ledger`, `authorization_not_execution_in_empty_ledger`, `empty_authorization_ledger_keeps_proposal_inert` | This runtime unit stops before even the model's inert proposal boundary. |

## Deliberate refinement differences

The Lean model uses finite `Nat` identities and one schematic artifact
`acquiredAt`. The runtime uses bounded strings, cryptographic JCS identities,
and retains every primary-claim dependency's exact acquisition interval,
clock, and uncertainty. The runtime checks each interval independently and
does not collapse cross-clock evidence into one invented timestamp. This is a
required implementation strengthening, not a theorem that the Rust structure
is equivalent to `DiagnosticArtifact`.

Lean's `deterministicJitter` is a small arithmetic model. Rust derives a stable
offset from SHA-256 over the JCS schedule/key preimage. Both establish
determinism for equal declared inputs; no claim is made that offsets are
numerically equal across the two implementations.

The Lean proposal, authorization, and execution structures remain boundary
models only. The Stage 6 Rust foundation implements no proposal API and no
authority or execution carrier. Likewise, Rust serialization, SHA-256,
RFC 8785, timestamp parsing, arithmetic overflow behavior, authentication,
custody, persistence, and transport are outside the cited proofs and require
ordinary machine-facing qualification.

Any later change to the pinned Lean commit, runtime contract, or correspondence
claim requires a new record. A source comment saying “corresponds” is not a
substitute for this exact pin and the named hostile vectors.
