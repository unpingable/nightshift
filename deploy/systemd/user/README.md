# Night Shift — user-level systemd deployment

Sibling to `deploy/systemd/` (system-level, hardened sandboxing,
intended for production-shaped deployment). This `user/` shelf is
for **local dogfood pilots** — runs as the invoking user, lighter
sandboxing, paths under `$HOME`, scoped to one operator.

Modeled after the user-level `nq-serve` / `nq-publish` services in
`~/git/notquery/deploy/examples/` — same minimal shape, parallel
install pattern.

## What's here

| File | Purpose |
|---|---|
| `nightshift-watchbill.service` | user-level oneshot wired to `deploy/agendas/sushi-k-disk-pressure.yaml` + `~/nightshift/local.sqlite` + local nq db |
| `nightshift-watchbill.timer` | 5-minute jittered cadence, idempotency-skip safe |

## Install

```bash
# 1. Build the release binary (the service file points at target/release/)
cd ~/git/scheduler && cargo build --release

# 2. Create the durable state directory if it doesn't exist
mkdir -p ~/nightshift

# 3. Install the unit + timer to user-level systemd config
mkdir -p ~/.config/systemd/user
install -m 0644 deploy/systemd/user/nightshift-watchbill.service \
    ~/.config/systemd/user/
install -m 0644 deploy/systemd/user/nightshift-watchbill.timer \
    ~/.config/systemd/user/

# 4. Reload + enable + start
systemctl --user daemon-reload
systemctl --user enable --now nightshift-watchbill.timer
```

## Operator surfaces

| What you want to see | Command |
|---|---|
| Timer next-fire time | `systemctl --user list-timers nightshift-watchbill.timer` |
| Recent invocations + skip lines | `journalctl --user -u nightshift-watchbill -n 50` |
| Persisted run history | `nightshift --store ~/nightshift/local.sqlite runs list` |
| One run in detail | `nightshift --store ~/nightshift/local.sqlite runs show <run_id>` |

## Idempotency

The timer fires every 5 minutes (jittered ±30s). The local nq
aggregator runs on a 60s cycle, so most timer firings will land on
a NQ `snapshot_generation` that hasn't advanced since the last
run. Those will skip with:

```
scheduled-skip: nq snapshot_generation=<gen> already reconciled in
  run run_<id> at <timestamp>
```

Real work only happens when NQ has produced a new generation. The
journal will be sparse — that's correct behavior.

## What this user-level shelf is NOT

- Not the production deployment story. For that, use the
  system-level units in `deploy/systemd/` with the hardened
  sandboxing.
- Not multi-finding. One timer watches one `(agenda, finding)`
  pair. Adding a second pilot means a second service+timer with
  different names (e.g., `nightshift-freelist-bloat.service`).
- Not actuating. NS proposes; Governor authorizes. This user-level
  unit runs with `--no-governor` because there's no Governor
  daemon on this dev box; promotion ceiling is capped at advise.
- Not signed/secured for remote operator access. If the dev box
  has other users, the SQLite store at `~/nightshift/local.sqlite`
  is readable by them by default; tighten with `chmod 0700
  ~/nightshift` if that matters.

## Removing

```bash
systemctl --user disable --now nightshift-watchbill.timer
rm ~/.config/systemd/user/nightshift-watchbill.{service,timer}
systemctl --user daemon-reload
# State (~/nightshift) preserved for forensics; delete deliberately
# if no longer needed.
```
