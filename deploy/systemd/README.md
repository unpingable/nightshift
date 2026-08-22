# Canonical Nightshift systemd reference units

These system-level files invoke the sole production `nightshift` binary and
its exact `cycle run` interface:

| File | Role |
|---|---|
| `nightshift-observation-cycle.service` | one exact canonical observation cycle |
| `nightshift-observation-cycle.timer` | preserved five-minute wake cadence, boot delay, jitter, and `Persistent=true` catch-up |
| `observation-cycle.env.example` | site-owned store, request, NQ-NG source, present-support resolver, and optional AG paths/profile |

The timer grants permission only to attempt observation. Exact recurrence-slot
identity and overlap exclusion are durable Nightshift facts. The configured
NQ-NG source qualifies exact local admission provenance before Nightshift
claims a slot; present support is resolved separately by its evidence
authority, and any immutable exact-work proposal goes only to AG. Admission is
not authorization. The unit has no Governor, Wicket, WLP, Docket, or executor
option.

Install as reference scaffolding after replacing every site-owned path:

```sh
sudo install -d -m 0750 -o root -g nightshift /etc/nightshift
sudo install -m 0640 -o root -g nightshift \
  deploy/systemd/observation-cycle.env.example \
  /etc/nightshift/observation-cycle.env
sudo install -m 0644 deploy/systemd/nightshift-observation-cycle.service \
  /etc/systemd/system/
sudo install -m 0644 deploy/systemd/nightshift-observation-cycle.timer \
  /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now nightshift-observation-cycle.timer
```

The reference unit does not grant cross-service filesystem authority. The site
must arrange narrowly read-only access for the `nightshift` service identity to
the configured NQ-NG config and SQLite snapshot (including SQLite sidecar
files when present), without granting NQ mutation rights. The configured
`NIGHTSHIFT_NQ_SOURCE_ID`, not either path, is the source identity checked at
the boundary.

## Optional Maude authoring custody

The reference timer remains suitable for cycles with no Maude authoring
context. Do not configure custody flags globally and then infer context for
ordinary cycles. A site-owned one-shot authoring invocation supplies the
separate handoff plus the complete two-role verifier arguments documented in
[`../../docs/authoring-context-custody.md`](../../docs/authoring-context-custody.md).

For system services, provision the two raw 32-byte keys through host credential
custody (for example systemd `LoadCredential=`) and pass only the resulting
read-only credential paths. The session-issuer and handoff-producer keys and
key IDs must differ. The Maude custody database and handoff/request directory
must be owned by the intended local service principal. This repository does
not provide or embed those credentials.

The request generator/rotation policy and production readiness of the external
Pulse/AG services remain site integration responsibilities. These files are not
a qualification claim.

The `user/` directory is a clearly marked historical, nonportable dogfood
specimen. It is not a supported unit set and remains only as archaeology.
