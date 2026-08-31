# Nightshift operational-condition Casework V1

`nightshift.casework-operational-condition/v1` is a third, distinct read-only
Casework family. It does not revise the sealed `nightshift.casework-run/v1`
projection or the live `nightshift.casework-live-run/v1` projection. A
scheduler state, campaign classification, Monitor acquisition outcome, NQ finding,
and Nightshift re-observation disposition remain different owner facts.

## Explicit input and exact-byte custody

Each repeatable `--condition-dir` supplies exactly five files:

```text
monitor.v1.json
nq.v1.json
lineage.v1.json
profile.v1.json
evaluation.v1.json
```

The loader performs no recursive discovery and never interprets evidence text as a
pathname. It opens the supplied directory and fixed children with descriptor-relative
no-follow operations. For every child it requires one nonempty regular file of at
most one MiB, binds the opened device and inode, records size, mtime, and ctime before
the read, and requires the same metadata plus the admitted byte count after the read.
Pathname replacement cannot redirect the retained descriptor. Content mutation
during acquisition is refused.

All five exact byte streams, plain SHA-256 identities, lengths, and validation
dispositions remain in the loaded case. The raw API serves those exact bytes. Plain
source-byte digests remain distinct from the owner semantic digests carried by
EPOCH-LANTERN.

## Owner rederivation and temporal history

Casework parses the closed EPOCH-LANTERN compatibility contracts and then re-runs
`admit_operational_lineage` over the exact Monitor and NQ bytes. The supplied
lineage must equal that deterministic owner result. The supplied changing evaluation
must pass the EPOCH `validate_against` relation for the exact lineage and
profile. Duplicated subject, producer, outcome, profile, lineage, and evaluation
fields are copied only after their owner digests and relationships agree.

History supplied to one admission is restricted to the same exact subject identity,
producer identity, and epoch. An unrelated condition directory cannot affect
admission. A successor in the same branch participates in EPOCH replay, missing
predecessor, and fork law.

The projection preserves independently:

- typed operational subject and stable-basis contract;
- typed Monitor producer and exact key identity;
- acquisition outcome and Monitor/NQ raw and semantic custody;
- NQ support, cannot-testify, refusal, contradiction, and nonclaim records;
- acquisition start/end, producer observation, receiver custody, NQ qualification,
  Nightshift admission, and evaluation times;
- epoch, sequence, and predecessor observation identity;
- profile-owned maximum age, current-until horizon, re-observation disposition,
  trigger, and next lawful action.

Unknown source extensions remain only in exact raw bytes. Casework cannot promote an
unknown field into a subject attribute, finding, currentness rule, or disposition.

## Presentation questions

A cannot-testify, refusal, or contradiction may receive a deterministic navigation
record. It embeds the exact upstream finding, its source ordinal, and the exact
EPOCH next lawful action. `presentation_only` is always true. The text is a
deterministic operator label; it is not a new finding, answer, or disposition. The
surface supplies no answer or resume operation.

## HTTP and browser boundary

The new API routes are:

```text
GET|HEAD /api/v1/operational-conditions
GET|HEAD /api/v1/operational-conditions/{navigation-id}
GET|HEAD /api/v1/operational-conditions/{navigation-id}/raw/monitor
GET|HEAD /api/v1/operational-conditions/{navigation-id}/raw/nq
GET|HEAD /api/v1/operational-conditions/{navigation-id}/raw/lineage
GET|HEAD /api/v1/operational-conditions/{navigation-id}/raw/profile
GET|HEAD /api/v1/operational-conditions/{navigation-id}/raw/evaluation
```

HEAD is admitted only for those exact new routes. Its status, headers, ETag, and
Content-Length equal GET while its response body is empty. The sealed, live, health,
asset, and predecessor UI route families retain their established GET-only method
contract byte-for-byte. Every write method returns 405 across every family.

Stable browser routes are:

```text
/operational-conditions
/operational-conditions/{navigation-id}
/operational-conditions/{navigation-id}/questions/{question-navigation-id}
/operational-conditions/{navigation-id}/raw
```

Direct refresh uses the same loopback-served application. Native links, headings,
landmarks, tables, definition lists, a skip link, visible focus, and focusable exact
raw blocks provide keyboard navigation. All runtime assets are local. There is no
approval, answer, dispatch, retry, remediation, execution, merge, promotion, remote
control, or filesystem browser.

The projection's fixed authority effect is
`read_only_projection_no_authority`. It creates no combined health or campaign
classification and authorizes no target effect.
