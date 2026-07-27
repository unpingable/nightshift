# Night Shift — systemd deployment

Slice 1 close-out artifact. A timer-driven, non-actuating
reconciliation loop against a real NQ database, with idempotency in
the daemon so timer cadence and reconciliation cadence are
independent.

## What this is

Three files:

| File | Purpose |
|---|---|
| `nightshift-watchbill.service` | the one-shot unit that runs `nightshift watchbill run --trigger scheduled` against a configured agenda + finding |
| `nightshift-watchbill.timer` | wakes the service every 5 minutes (default; configurable) |
| `watchbill.env.example` | the `EnvironmentFile` template — copy to `/etc/nightshift/watchbill.env`, fill in paths |

## What this is not

- Not a multi-finding orchestrator. One timer watches one
  `(agenda, finding)` pair. Multi-finding scheduling is later work
  (see `working/decisions/GAP-slice-cycle.md`).
- Not an actuating daemon. NS proposes; Governor authorizes. This
  unit runs with hardened systemd sandboxing (`ProtectSystem=strict`,
  `NoNewPrivileges=true`, etc.) — see `MemoryDenyWriteExecute=true`
  on the service file.
- Not a packaging story. No `.deb` / `.rpm` here. Install paths
  assume `/usr/local/bin/nightshift` and `/var/lib/nightshift/`;
  adapt to your distribution.

Before installing the unit, satisfy the source-build prerequisites and
sibling `nightshift` / `wicket` / `wlp` layout in the
[operator quick start](../../docs/operator/README.md). Install
`nq-monitor` separately at an absolute path executable by the `nightshift`
service user. Because the unit sets `ProtectHome=true`, neither binary nor
live input artifacts may be hidden under a protected home directory.

## Install (manual)

```bash
# 1. From the Night Shift source tree, build and install the binary
cargo build --locked --release
sudo install -m 0755 target/release/nightshift /usr/local/bin/nightshift

# 2. Create the service user + state directory
sudo useradd --system --no-create-home --shell /usr/sbin/nologin nightshift
sudo install -d -m 0755 -o nightshift -g nightshift /var/lib/nightshift

# 3. Configure the environment file
sudo install -d -m 0750 -o root -g nightshift /etc/nightshift
sudo install -d -m 0755 -o root -g nightshift /etc/nightshift/agendas
sudo install -m 0640 -o root -g nightshift \
    deploy/systemd/watchbill.env.example /etc/nightshift/watchbill.env
sudo editor /etc/nightshift/watchbill.env
# Set NIGHTSHIFT_NQ_BIN to the separately installed nq-monitor executable
# and fill in every other required site-owned path.

# 4. Drop the agenda
sudo cp tests/fixtures/wal-bloat-review.yaml /etc/nightshift/agendas/

# 5. Install the unit + timer
sudo install -m 0644 deploy/systemd/nightshift-watchbill.service /etc/systemd/system/
sudo install -m 0644 deploy/systemd/nightshift-watchbill.timer   /etc/systemd/system/

# 6. Enable + start
sudo systemctl daemon-reload
sudo systemctl enable --now nightshift-watchbill.timer
```

## Operator surfaces

| What you want to see | Where to look |
|---|---|
| Timer firings + skip lines | `journalctl -u nightshift-watchbill -n 50` |
| Persisted runs | `nightshift --store /var/lib/nightshift/nightshift.sqlite runs list` |
| One run in detail | `nightshift --store /var/lib/nightshift/nightshift.sqlite runs show <run_id>` |
| NQ peek (cross-check what NS would consume) | `nightshift --nq-bin /absolute/path/to/nq-monitor --nq-db /absolute/path/to/nq.db nq peek` |
| Liveness peek (if configured) | `nightshift --nq-bin /absolute/path/to/nq-monitor --nq-liveness /absolute/path/to/liveness.json liveness peek` |

## Idempotency

The timer fires every 5 minutes. NQ scans run on their own cadence.
If the timer fires when NQ has not produced a new
`snapshot_generation` since the most recent completed run for the
configured `(agenda, finding)`, NS prints a one-line skip message
and exits 0 *without* opening a new run:

```
scheduled-skip: nq snapshot_generation=4217 already reconciled in run run_abc123 at 2026-05-27T03:00:14+00:00
```

The skip line is what shows up in `journalctl`. Operators looking
for the canonical receipt for the current generation follow the
`run_<id>` and inspect via `runs show`.

Idempotency applies only when `--trigger scheduled` is in effect
(this unit sets it). `nightshift watchbill run` invoked manually
defaults to `--trigger manual` and always opens a fresh run.

## Hardened sandboxing notes

The service file enables a broad systemd sandbox: `ProtectSystem`,
`ProtectHome`, `PrivateTmp`, `PrivateDevices`, `NoNewPrivileges`,
`MemoryDenyWriteExecute`, `RestrictAddressFamilies` to
`AF_UNIX|AF_INET|AF_INET6` only, `RestrictNamespaces`,
`RestrictRealtime`, `SystemCallFilter=@system-service`.

If you add features that require additional capabilities (e.g.
publication to a network artifact target, MCP tool transports
outside the listed address families), update the service file
*deliberately* and document the reason in the FEATURE-HISTORY
entry that ships the feature. Don't loosen the sandbox by accident.

## Removing

```bash
sudo systemctl disable --now nightshift-watchbill.timer
sudo rm /etc/systemd/system/nightshift-watchbill.{service,timer}
sudo systemctl daemon-reload
# State (/var/lib/nightshift) and config (/etc/nightshift) preserved
# for forensics; delete deliberately if no longer needed.
```
