# Exact load-pressure production support deployment

Status: qualified on 2026-08-25 for the dormant Linode observation office and
for exactly `nq.host.load_pressure/v1`.

## Evidence chain

```text
unchanged NQ acquisition/artifact
  sha256:54c50dfca0acfaf369d7e800d585b35a26768ffefbad5741cea62c04b63bfad3

independent Pulse-owned support occurrence P1
  sha256:225924007ed48fd0dffe4ac6f7174e05647359d43f08513f9fb5901e3bca899f

receiver custody
  sha256:fdae6a2958cd0e0d1c25a564e0bb707f7f184151a1e4c79b6a833f1b4718d52e

query-bound resolver result
  sha256:bb06e58ed4a94bbb908ba233e41de3cd2421c1babcad99b223cf52242203b238

Nightshift cycle
  cycle:cb11146d-5d68-48b2-8b91-dda91b12f437

canonical observation
  sha256:11dd56509e57e8f094aa03e1ace29889b92e014a1d3262b65b53a8762fce5ba2
```

The original Nightshift refusal remains historical. The successful cycle is a
new ordinary manual slot, not mutation or recovery laundering of the refused
cycle. Both cycles point at the same unchanged NQ diagnostic artifact. The
support family is independent evidence of the exact same proposition, not a
replacement diagnostic acquisition.

## Deployed custody

The static musl executable digest is:

```text
sha256:1b6a197db7eaec77e690b41bfc71a9d0636481f91db63be22e8031c85583e566
```

One inode is exposed under three closed basenames:

- `pulse-load-pressure-producer` reads only the fixed kernel sources and writes
  its append-only outgoing directory as `pulse-load-support`;
- `pulse-load-pressure-receiver` verifies exact signed evidence and writes the
  receipt directory as `pulse-support-receiver`; and
- `pulse-support-resolver` reads receipts as the `nightshift` process and
  accepts no arguments or evidence-generation operation.

The producer private key is a software-exportable Ed25519 key readable only by
the producer principal under the tested Unix permissions. It authenticates the
producer occurrence; it is not host or hardware attestation. Nightshift and NQ
could not read it. Root remains part of the host trust base.

One root-run staging preflight accidentally created a valid signed outgoing
occurrence named `invalid-preflight`. It was never received, never considered
by the resolver, and never used by Nightshift. It remains retained as an
explicit unreceived hostile witness; it is not silently deleted or promoted.

## Temporal and replay witnesses

P1 used receiver boot-clock expiry at exactly 300 seconds. Exact producer and
intake replay returned the same identities while preserving byte digests,
mtimes, observation tick, receipt tick, and expiry. At the exclusive boundary
the resolver reported `expired` and retained P1's evidence reference.

A genuine P2 occurrence then produced distinct support evidence
`sha256:3cae710d1839f382492d393060b32e4e41233604fda719019daf5ffaec0c10b7`
and receiver receipt
`sha256:0d829230e676c9afdc31548428b1ec03fc54ca16da3c760255b77ec61458501b`.
The resolver returned `current` using P2. The original diagnostic artifact was
unchanged throughout.

## Boundaries

This deployment proves independently acquired, proposition-exact support under
one local kernel vantage. It does not prove whole-host health, workload cause,
absolute UTC accuracy, cross-vantage separation, physical-host identity, or
authority to act. It creates no recurring diagnostic or pulse timer. The
operator-supplied Linode bootstrap value remains independent of runtime
metadata but is not backed by a mechanically authenticated Linode API receipt.

The office remains installed and dormant. Enabling recurrence still requires
the separately qualified repeat-diagnostic acquisition law and an explicit
bounded cadence/retry policy.
