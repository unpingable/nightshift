# Nightshift

Nightshift is the durable temporal observation and attention office.

It decides when to look again, consumes qualified present support and complete
NQ diagnostic artifacts, computes non-authorizing posture/attention, and may
submit one immutable exact-work proposal to AG. It does not mint standing,
authorize or execute effects, own AG continuation, or call Docket/executors.

## Runtime status

The sole production binary is `nightshift`. Its sole production cycle is:

```text
exact recurrence slot
  -> exact observation cycle
  -> qualified present-support result
  -> complete NQ diagnostic posture
  -> temporal posture and attention
  -> optional immutable exact-work proposal
  -> new AG occurrence
  -> AG status/settlement reference
  -> fresh observation required, reconciliation display, halt display, or close
```

This runtime has development and hostile-test evidence. It is not yet an
operational qualification claim.

The former Watchbill, Wicket/WLP, MVP-A, classic Governor, authority ladder,
prose action, same-generation skip, and production drill paths have been
removed from the production graph. Historical design records remain under
`docs/working/` and in explicitly marked historical shelves.

## Build

Prerequisites are stable Rust/Cargo 1.82 or newer, a native C toolchain for
bundled SQLite, and crates.io access on an uncached first build.

```sh
cargo build --locked --release --bin nightshift
./target/release/nightshift --help
./target/release/nightshift cycle --help
```

Nightshift has no sibling Wicket or WLP source dependency.

## Run one exact observation cycle

```sh
./target/release/nightshift \
  --store /var/lib/nightshift/nightshift.sqlite \
  cycle run \
  --request /etc/nightshift/cycles/current.json \
  --present-evidence-resolver /usr/local/bin/pulse-support-resolver
```

The request is a sealed `CanonicalCycleRequestV1` that binds the exact
recurrence slot, observation identity, closed diagnostic policy, complete NQ
input set, recurrence evidence, and optional temporal policy.

If—and only if—the request carries an immutable precompiled exact-work
proposal, all three AG coordinates are required:

```sh
  --ag-loopctl /usr/local/bin/ag-loopctl \
  --ag-database /var/lib/ag/campaign.sqlite \
  --ag-observation-resolver /usr/local/bin/ag-observation-resolver
```

Nightshift sends AG campaign/occurrence and exact observation/proposal basis.
It sends no standing, authorization, dispatch, retry, reconciliation mutation,
or human disposition.

## Inspect and recover

```sh
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle list
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle show --cycle-id '<id>'
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle replay --cycle-id '<id>'
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle sync-ag --help
nightshift --store /var/lib/nightshift/nightshift.sqlite cycle recover --help
```

Recovery erases local currentness. A prepared AG request is recovered by exact
AG status only and is never resubmitted. Settlement records attempt-native
facts and requires a new observation cycle before posture or work can change.

## Development gates

```sh
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
bash scripts/check_no_actuation_surface.sh
bash scripts/check_no_actuation_surface.sh --self-test-inject
```

The structural gate pins one binary, the exact two external process ports
(`pulse-support-resolver` and `ag-loopctl`), and absence of the retired
authority/execution surfaces.

See [the canonical runtime record](docs/CANONICAL_RUNTIME_C1.md), the
[operator quick start](docs/operator/README.md), and the
[systemd reference units](deploy/systemd/README.md).

## License

Apache-2.0
