# Nightshift canonical runtime operational qualification harness

Status: harness preparation only. Nothing in this directory is a qualification
claim, certificate, support certificate, diagnostic result, or authority
artifact.

This harness prepares evidence collection for the operational premises that
remain after production correspondence closed. It does not change recurrence,
currentness, diagnostic, AG, or execution semantics.

## Gate classification

| Gate | Required environment | Safe local evidence available here |
| --- | --- | --- |
| real systemd missed/catch-up behavior | trusted host | deterministic slot/missed/catch-up tests |
| timer restart | trusted host | explicit canonical store restart tests |
| wall-clock rollback | fault-injection environment | owner-clock mismatch and endpoint tests |
| Pulse receiver/currentness behavior | deployed service | exact command-port fixture and equality-expired tests |
| deployed NQ/AG correspondence | deployed service | complete NQ basis and real `ag-loopctl` adapter tests |
| SQLite/filesystem power-loss behavior | fault-injection environment | logical crash/reopen/replay and process contention |

`gates.json` is the machine-readable matrix. `trusted-host-plan.json`,
`deployed-service-plan.json`, and `fault-injection-plan.json` define the
additional evidence needed to close the non-local premises honestly.

## Local development evidence

Run from the repository root and write evidence outside the source tree:

```sh
python3 qualification/operational/run_local.py \
  --output /tmp/nightshift-operational-local-$(date -u +%Y%m%dT%H%M%SZ)
```

The runner refuses a dirty source tree by default. During harness development,
`--allow-dirty-development` records the dirty-status digest and labels the
bundle as development evidence.

The local suite uses the real canonical `nightshift` binary, real SQLite files,
independent processes, and a deterministic present-evidence fixture. The
fixture is named `pulse-support-resolver` only to exercise the production
command-port grammar. It is explicitly not Pulse and cannot demonstrate Pulse
receiver operation.

The result always says `qualification_status: not_assessed`.

## Environment preflight

```sh
python3 qualification/operational/preflight.py \
  --profile trusted-host \
  --output /tmp/nightshift-systemd-preflight

python3 qualification/operational/preflight.py \
  --profile deployed-service \
  --output /tmp/nightshift-deployed-preflight

python3 qualification/operational/preflight.py \
  --profile fault-injection \
  --output /tmp/nightshift-fault-preflight
```

The deployed-service profile recognizes these path-valued coordinates without
executing them:

- `NIGHTSHIFT_QUAL_PULSE_RESOLVER`
- `NIGHTSHIFT_QUAL_NQ_ADAPTER`
- `NIGHTSHIFT_QUAL_AG_LOOPCTL`
- `NIGHTSHIFT_QUAL_AG_DATABASE`
- `NIGHTSHIFT_QUAL_AG_OBSERVATION_RESOLVER`
- `NIGHTSHIFT_QUAL_DEPLOYMENT_MANIFEST`

The trusted-host profile additionally requires
`NIGHTSHIFT_QUAL_HOST_DECLARATION`. The fault-injection profile requires
`NIGHTSHIFT_QUAL_FAULT_VM_IMAGE` and `NIGHTSHIFT_QUAL_FAULT_CONTROLLER`.
These declarations prevent installed systemd or QEMU tools from being mistaken
for a designated qualification environment.

Preflight captures identities and prerequisite presence only. It does not
demonstrate currentness, diagnosis, AG correspondence, timer behavior, or
durability.

Path records larger than 32 MiB are not hashed by preflight, and every path
hash is labeled as an unlocked read rather than a coherent service snapshot.

## Evidence discipline

- Every evidence directory is create-once.
- Commands retain argv, UTC bounds, exit status, and stdout/stderr hashes.
- Source, harness, executable, unit, and host identities are captured first.
- The local support fixture grants no currentness outside its exact live query.
- Historical diagnostic bytes never become current merely because the harness
  can replay them.
- A process kill or ordinary reopen is not represented as power loss.
- Real systemd behavior is collected only on an isolated trusted host or VM.
- Wall-clock rollback is injected only where clock control cannot affect other
  workloads.
- Deployed NQ and AG evidence must identify the actual adapters/services used.
