# Nightshift Casework operator quick start

Build and start the read-only tool against the VELVET-ORRERY fixture:

```bash
cargo run --locked -p nightshift-casework --bin nightshift-casework -- \
  --run-dir qualification/nightshift-packet-v1/velvet-orrery \
  --bind 127.0.0.1:4177
```

The process prints the exact loopback URL. It remains in the foreground and
stops with Ctrl-C. The command installs no service.

Inspect the index and full golden run:

```bash
curl --fail http://127.0.0.1:4177/api/v1/runs
curl --fail http://127.0.0.1:4177/api/v1/runs/01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7
```

Inspect exact source bytes:

```bash
curl --fail http://127.0.0.1:4177/api/v1/runs/01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7/raw/packet
curl --fail http://127.0.0.1:4177/api/v1/runs/01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7/raw/receipts
```

Repeat `--run-dir PATH` to load another explicit completed run. Duplicate
packet digests are refused. Each directory must contain real, non-symlink
`packet.v1.json` and `run-receipts.v1.json` entries. The tool does not
search for runs or follow evidence references.

For deterministic qualification, `--evaluated-at RFC3339` pins the
`currentness_now` evaluation. Ordinary operation omits it and uses one UTC
instant captured before all runs load.

There are no write, approval, answer, retry, dispatch, resume, merge,
promotion, or execution endpoints. Stop the foreground process when
inspection is complete.
