# Canonical Nightshift systemd reference units

These system-level files invoke the sole production `nightshift` binary and
its exact `cycle run` interface:

| File | Role |
|---|---|
| `nightshift-observation-cycle.service` | one exact canonical observation cycle |
| `nightshift-observation-cycle.timer` | preserved five-minute wake cadence, boot delay, jitter, and `Persistent=true` catch-up |
| `observation-cycle.env.example` | site-owned store, request, present-support resolver, and optional AG paths |

The timer grants permission only to attempt observation. Exact recurrence-slot
identity and overlap exclusion are durable Nightshift facts. Present support is
resolved by the configured evidence authority, and any immutable exact-work
proposal goes only to AG. The unit has no Governor, Wicket, WLP, Docket, or
executor option.

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

The request generator/rotation policy and production readiness of the external
Pulse/AG services remain site integration responsibilities. These files are not
a qualification claim.

The `user/` directory is a clearly marked historical, nonportable dogfood
specimen. It is not a supported unit set and remains only as archaeology.
