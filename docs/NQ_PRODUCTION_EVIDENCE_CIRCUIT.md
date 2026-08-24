# NQ to Nightshift production evidence circuit

## Standing

Canonical Nightshift consumes locally emitted
`nq.diagnostic_execution.v2` artifacts. Delivered bytes are not sufficient on
their own. Before claiming a recurrence slot, Nightshift invokes the configured
NQ-NG read boundary:

```text
exact v2 artifact
  -> nq --json diagnostics qualify ARTIFACT_ID
  -> nq.diagnostic_admission_provenance.v1
  -> exact query/provenance cross-check
  -> canonical Nightshift observation
  -> separately resolved currentness
```

The provenance carrier establishes that one configured local NQ store can
reopen the artifact's exact bytes and retained run, provider-intake,
admission, profile, and judgment/refusal history. It establishes evidence
eligibility only. Nightshift still owns observation/currentness, and no
observation path grants standing, authorization, spend, or action.

## Divergence and repair

NQ-NG committed the live `nq.diagnostic_execution.v2` producer in July 2026.
Nightshift first added a v2 consumer explicitly labeled as lacking
live-producer correspondence. The August canonical-runtime cutover then added
the production `diagnostics qualify` subprocess adapter, but NQ-NG had not
committed that command or the admission-provenance emitter. Canonical runtime
tests supplied an in-process `NqAdmissionPortV1`, so they qualified owner
semantics while bypassing the executable seam.

The repair is source-owned by NQ-NG: its read-only qualifier now reopens the
existing v2 store history and emits the carrier Nightshift already defined.
Nightshift retains the same consumer semantics and adds process-adapter hostile
tests plus an executable-capability preflight in its reference service.

## Historical finding snapshots

Classic NQ's `nq.finding_snapshot.v1` is a real stable export whose first
external consumer was the retired Nightshift Watchbill runtime. That adapter
now exists only under `tests/historical_watchbill/` and is deliberately absent
from the compiled canonical graph. Restoring it would create a competing
runtime path and discard the exact v2 source provenance required here. It is
therefore historical to this circuit, not the minimum repair.

Classic NQ's fleet `ssh://` transport is also outside this circuit. It runs
`ssh user@host cat PATH` to render per-target liveness in the Classic fleet
index. It is a real operator transport, but it supplies neither NQ-NG
diagnostic admission provenance nor a governed Nightshift source boundary.

## Executable-name collision and deployment gate

Debian package `nq` owns `/usr/bin/nq` and implements an unrelated command
queue. NQ-NG documentation and packaging already name this collision and use a
package conflict rather than silent replacement. A configured executable must
pass both:

```sh
NIGHTSHIFT_NQ_PROGRAM --help
NIGHTSHIFT_NQ_PROGRAM diagnostics qualify --help
```

The first rejects Debian's unrelated queueing utility; this is necessary
because that program can return success for some arbitrary trailing command
words. The second rejects older NQ-NG builds. The reference systemd unit
enforces both capability preflights. Local contract
qualification does not itself authorize NQ-NG installation, replacement of
Classic NQ, subject admission, or production cutover.

## Subject and vantage identities

No remote-host-specific semantic is required. The NQ artifact carries exact
subject and open `SemanticIdentityV1` vantage values. The complete Nightshift
inventory binding, including vantage, participates in content-derived posture
policy identity; that policy identity participates in observation-family
identity. Consequently, a differently configured vantage cannot silently
supersede another lineage.

Neither NQ-NG nor Nightshift currently provides a canonical DNS-alias mapping
for subject identity. Hostnames that happen to resolve to one machine are not
canonical identity evidence. Real-host admission must wait for one explicit
subject/vantage identity rather than ingesting each hostname as a separate
governed subject.

## Test boundary

NQ-NG tests invoke the actual Cargo-built `nq` executable against a real local
store, qualify a locally emitted v2 artifact after process restart, require
exact replay convergence, and refuse imported or substituted history.
Nightshift tests invoke its production command adapter, pin exact arguments,
parse the closed carrier, and refuse nonzero exit, malformed JSON, wrong
schema, trailing bytes, and exact-query substitution. Existing currentness and
canonical-store tests retain stale evidence historically and prevent later
lineages from being selected by write time or caller choice.

This is local contract qualification. Production service-principal isolation,
credential rotation/revocation, coherent live backup/restore, physical-power
cuts, and real-host source honesty remain environmental gates.
