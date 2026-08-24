# Linode host admission gate

## Standing

This is a **read-side reconnaissance record**, not an admission. No governed
subject was created for the Linode, no evidence was admitted, and no identity
decision was made. It records what was observed live, under which authority,
and which gates remain closed.

`NQ_PRODUCTION_EVIDENCE_CIRCUIT.md` states that real-host admission "must wait
for one explicit subject/vantage identity rather than ingesting each hostname
as a separate governed subject." This document supplies the evidence that
decision needs. It does not take the decision.

## Observed live state

Observed 2026-08-24 from `sushi-k` over the pre-existing operator SSH key
`~/git/claude/ssh/linode` as `root@labelwatch.neutral.zone`, with
`StrictHostKeyChecking=yes` (no new host key accepted; all host keys were
already present in `known_hosts`). Read-only commands only.

| Fact | Value |
|---|---|
| Kernel `hostname` / static hostname | `localhost` |
| `machine-id` (SHA-256 prefix) | `ac6412a144f548d8eb21ea8a67f2ea70` |
| `boot_id` | `5f5d4264-8b46-465b-9ee6-1b7c10e18d92` |
| Kernel | `5.15.0-171-generic` |
| OS | Ubuntu 22.04.5 LTS |
| DMI `product_uuid` | not readable |
| Uptime at observation | ~172 days |

Active units: `nq-publish`, `nq-serve`, `governor`, `gov-webui`.

Installed NQ is **Classic NQ only** — `/opt/notquery/nq-monitor` and
`/opt/notquery/nq-witness`, both dated 2026-07-27. `/usr/local/bin/nq` and
`/opt/nq/nq` are absent; no `nq` package is installed.

## The host does not know its own name

The machine's static hostname is literally `localhost`. Its NQ aggregator
config declares the source name `labelwatch-host`, and `hosts_current` records
`labelwatch-host`. The publisher self-reports `localhost`; NQ logs the
mismatch once per unique pair.

`localhost` is not a usable subject identity: it is non-unique by construction
and collides with every other host under any future cross-host aggregation.

## Alias standing

One physical machine currently answers to at least six names. All four network
names share one ed25519 host key (`a417b59cc17f63594cdfba74a1a0cd68`), which is
cryptographic evidence — not DNS inference — that they designate one machine.

| Name | Standing |
|---|---|
| `192.46.223.21` | **Locator.** Not identity. Changes on re-IP. |
| `labelwatch.neutral.zone` | **Transport configuration** (SSH target) and service identity. |
| `nq.neutral.zone` | **Service identity** (NQ dashboard vhost). |
| `sp00ky.net` | **Service identity** (governor web UI vhost). |
| `localhost` | **Not an identity.** Kernel hostname; non-unique. |
| `labelwatch-host` | **NQ source name.** The closest existing thing to a subject identity, and the identity actually recorded in NQ's durable evidence. |
| `nq.local` | **Component identity**, not a host identity. |

The SSH host key is deliberately *not* proposed as the canonical subject
identity. It is a transport credential; treating it as semantic identity is
exactly the transport/provenance confusion this boundary exists to prevent. It
is admissible here only as corroboration that the names denote one machine.

## Why the existing SSH integration cannot become an evidence adapter

The first-party profile `nq.host` v1 declares exactly one vantage:

```json
"vantages": [
  { "name": "local", "description": "Observation from a process on the subject host" }
]
```

`nq.conformance` v1 likewise declares only `local`. No first-party profile
declares a remote vantage.

Acquiring a host fact by `ssh root@host …` from `sushi-k` produces an
observation whose vantage is not local to the subject. Admitting it as
`vantage: local` would be a false provenance claim, and the store refuses the
substitution directly: mutating a collection's `vantage` from `local` to
`remote` fails with `StoreError::ReplayConflict`, as do substitutions of
`subject`, `scope`, and unsupported capability grants.

The existing SSH access is therefore **transport and deployment authority
only**. It cannot be promoted into an evidence-acquisition path without either
lying about vantage or adding a remote-vantage semantic. Neither is done here.

Classic NQ's `ssh://` fleet reader is outside this circuit for the same reason
already recorded in `NQ_PRODUCTION_EVIDENCE_CIRCUIT.md`.

## Gates remaining before first real admission

1. **Subject/vantage identity — operator decision, not derivable.** The
   mechanical facts are available (`machine-id`, `boot_id`, host key, existing
   `labelwatch-host` source name). What is *not* mechanical is whether a
   reimaged or migrated host remains the same governed subject. That question
   determines which fact is authoritative, and it must be answered before a
   persistent subject exists.
2. **NQ-NG is not installed on the host.** Classic NQ has no `diagnostics`
   subcommand (`error: unrecognized subcommand 'diagnostics'`, exit 2), emits
   no `nq.diagnostic_execution.v2`, and cannot answer the qualifier. Cutover is
   a deployment change, separately authorized.
3. **Helper service principal.** Release NQ-NG refuses a helper execution
   account equal to the daemon/operator UID
   (`helper execution UID … is the current daemon/operator UID`), and
   `allow_same_identity_in_debug` is unavailable in release builds. A dedicated
   helper account must exist on the host before NQ-NG runs there at all —
   including for read-only qualification.
4. **Agent Governor credential.** The Basic-auth credential was redacted from
   the tree; operational rotation/revocation remains outstanding. No part of
   this circuit depends on it, and it was not used or recovered.

## Executable-name collision, confirmed live

Debian's unrelated `/usr/bin/nq` was measured against both documented
preflights on `sushi-k`:

| Command | Debian `/usr/bin/nq` | NQ-NG `nq` |
|---|---|---|
| `--help` | exit **1** | exit **0** |
| `diagnostics qualify --help` | exit **0** | exit **0** |

The second preflight alone does not separate them — Debian's `nq` returns
success for those trailing words. Only the pair is sound, and both are required
in the order the reference unit already enforces. `which nq` on `sushi-k`
resolves to `/usr/bin/nq`, so a configured program name must be an absolute
path, never a bare `nq`.

## Non-claims

- No governed subject exists for this host.
- No observation was admitted, and none is claimed current.
- Live unit states above are point-in-time reads, not qualified evidence.
- SSH reachability is not standing, freshness, or authorization.
- Nothing here authorizes NQ-NG installation, Classic NQ replacement, subject
  admission, or production cutover.
