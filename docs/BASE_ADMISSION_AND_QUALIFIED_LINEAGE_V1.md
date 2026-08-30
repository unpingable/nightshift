# Base admission and qualified lineage v1

Status: Nightshift campaign-practice contract.

## Purpose

A repository can contain qualified implementation content without containing
the exact campaign result that qualified it. Successor admission must state
which identity it requires instead of treating implementation, qualification,
and repository topology as interchangeable.

The load-bearing principle is:

> Qualification attaches to the exact subject it qualified. Repository
> ancestry establishes custody lineage. Content equivalence establishes
> implementation provenance. None may silently stand in for another.

This contract changes no Nightshift packet schema, digest law, authority
boundary, result classification, or production binary.

## Distinct identities

### Qualified subject

The exact commit or commits whose behavior and artifacts were exercised by a
qualification. A qualified subject need not be the campaign's final commit
when later commits only record publication or closeout.

### Campaign result head

The immutable terminal commit named by the campaign's result and custody
record. It may include qualification records, publication facts, limitations,
and closeout material beyond the qualified subject.

### Successor integration base

The exact repository commit from which a successor is admitted to develop.
Its ancestry proves custody lineage. It inherits qualification only according
to each predecessor's explicit successor-base policy.

## Base-admission dispositions

### EXACT-RESULT-ANCESTRY

The exact predecessor campaign result head is an ancestor of the proposed
successor base. This satisfies a predecessor policy requiring exact result
ancestry, subject to the predecessor's other entry predicates.

### EXACT-SUBJECT-ANCESTRY

Every explicitly declared qualified-subject commit is an ancestor, while the
immutable result evidence remains separately and exactly addressable. This is
usable only when the predecessor contract explicitly permits subject ancestry
as its successor-base policy.

### VERIFIED-CONTENT-EQUIVALENCE

A cherry-pick, restack, or other adoption is mechanically shown to have the
same relevant patch or content identity. This establishes implementation
provenance only. It does not establish inherited qualification and does not
satisfy a requirement for exact result-head ancestry.

### DIVERGED-UNRESOLVED

No permitted exact ancestry or verified-equivalence relationship can be
established, or reconciliation would require choosing materially different
runtime semantics. Mutation stops.

## Evaluation order

1. Resolve the exact repository and refreshed predecessor result heads.
2. Read each predecessor's declared successor-base policy.
3. Test exact result-head ancestry.
4. If the policy explicitly permits it, test every qualified-subject commit.
5. Record content-equivalence evidence independently; never use it as a
   qualification substitute.
6. Refuse an unresolved semantic divergence.
7. Bind the evaluated heads, checks, and disposition in a closed receipt when
   the campaign needs machine-readable admission evidence.

A merge created solely to converge custody must retain every required exact
head as an ancestor. It may reconcile mechanically equivalent changes and
evidence placement. A blanket tree preference cannot manufacture qualified
lineage.

## Future closeout practice

A campaign intended to serve as a successor base should record separately:

- `result_head`;
- `qualified_subject_commit(s)`;
- `successor_base_policy`.

The policy should be one of exact result ancestry or explicitly permitted exact
subject ancestry. Content equivalence is always an additional provenance fact,
never a successor-base policy.

Historical sealed packets, receipts, and campaign artifacts are not retrofitted
to add these fields. A later campaign may record the missing relationship in
its own immutable evidence.

## Machine-readable receipt

`nightshift.base-admission-receipt/v1` is a small campaign-practice artifact,
not a packet field and not a production runtime input. Its schema is
`schemas/nightshift.base-admission-receipt.v1.schema.json`.

The receipt records the repository, candidate base, predecessor result and
qualified-subject commits, predecessor policy, exact checks, integration
parents, and independent disposition. It grants no scheduling, execution,
authorization, qualification, or publication authority.
