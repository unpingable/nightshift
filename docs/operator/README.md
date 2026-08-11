# Nightshift operator quick start

**Status:** canonical development runtime; not yet operationally qualified.
Nightshift observes and records posture. It does not authorize or execute an
effect.

## Build contract

```sh
cargo build --locked --release --bin nightshift
./target/release/nightshift cycle --help
```

The workspace has no sibling Wicket/WLP dependency. A native C toolchain is
required for bundled SQLite.

## Required external ports

One observation cycle requires:

- a sealed `CanonicalCycleRequestV1` produced by the workflow integration;
- an executable whose basename is exactly `pulse-support-resolver` and which
  returns an exact query-bound `QualifiedSupportV1`;
- an operator-owned Nightshift SQLite path.

If the request contains exact work, the cycle additionally requires paths to
`ag-loopctl`, AG's database, and AG's observation resolver. Those are the only
consequence-adjacent Nightshift calls. Nightshift never accepts a Docket or
executor endpoint.

```sh
nightshift \
  --store /var/lib/nightshift/nightshift.sqlite \
  cycle run \
  --request /etc/nightshift/cycles/current.json \
  --present-evidence-resolver /usr/local/bin/pulse-support-resolver
```

For exact work, append:

```sh
  --ag-loopctl /usr/local/bin/ag-loopctl \
  --ag-database /var/lib/ag/campaign.sqlite \
  --ag-observation-resolver /usr/local/bin/ag-observation-resolver
```

AG options are forbidden on posture-only requests, and all three are mandatory
on requests containing a proposal.

## Read local cycle state

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle list
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle show --cycle-id '<cycle-id>'
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle replay --cycle-id '<cycle-id>'
```

Text output is presentation only. The durable cycle record retains exact
support, diagnostic, posture, AG occurrence, attempt, and settlement
references.

## Restart and AG status

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle recover \
  --ag-loopctl /usr/local/bin/ag-loopctl \
  --ag-database /var/lib/ag/campaign.sqlite \
  --ag-observation-resolver /usr/local/bin/ag-observation-resolver \
  --observed-at 2026-08-11T12:00:00Z
```

Restart never recreates support currentness, an AG authorization, or an
execution capability. Locally in-flight observation work becomes recovery
required. Prepared AG work is queried by exact occurrence status and is never
resubmitted by Nightshift. Reconciliation and human disposition remain AG
operations.

## Settlement law

An AG settlement supplies only exact occurrence/attempt/outcome provenance.
Nightshift records it and enters observation-required posture. Success does not
mean healthy, failure does not diagnose the cause, and indeterminate never
causes a repeat.

## Scheduling

Reference systemd files are under [`../../deploy/systemd/`](../../deploy/systemd/).
They preserve the existing boot delay, five-minute wake cadence, jitter, and
`Persistent=true`. Every firing attempts one exact recurrence slot; generation
equality never satisfies a slot.

The `deploy/systemd/user/` shelf is historical and nonportable. Do not install
it.

## Diagnostic specimen

The checked-in [diagnostic-posture fixture](examples/diagnostic-posture-v1/README.md)
documents the complete NQ/posture basis consumed inside a canonical cycle. It
is a library/conformance specimen, not a second production CLI.
