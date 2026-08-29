# Nightshift packet summary (non-authorizing)

> This rendering is an orientation and scheduling aid. It grants no authority, approval, retry, execution custody, or settlement.

- Packet: `nightshift-20260829-autonomous-convergence`
- Digest: `sha256:01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`
- Current: `2026-08-29 17:00:00 UTC` through `2026-08-30 12:00:00 UTC`
- Switchyard alias: `nightshift-convergence-20260829`
- Immutable plan reference: `nightshift-packet://01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`

## Campaign DAG

- `packet-v1` — **VELVET-ORRERY** / `nightshift-immutable-run-packet-v1-20260829`; depends on: none
- `switchyard-transport` — **QUIET-BRIDGE** / `switchyard-explicit-plan-ref-transport-v1`; depends on: packet-v1
- `authorization-continuation` — **IRON-CHORUS** / `ag-codex-exact-occurrence-authorization-continuation-v1`; depends on: none
- `worker-vm-custody` — **SILVER-KINGFISHER** / `worker-vm-custody-admission-successor-v1`; depends on: none
- `caliper-mode-contract` — **GROUP-ANCHOR** / `generation-store-mode-contract-successor-v1`; depends on: none
- `caliper-authority-lifecycle` — **CROWNED-STORE** / `caliper-authority-lifecycle-successor-v1`; depends on: caliper-mode-contract
- `lanternwake-port` — **AMBER-COMPASS** / `reported-host-identity-successor-port-v1`; depends on: none
- `copper-artifact-custody` — **FORGE-VAULT** / `matched-k-n-release-custody-successor-v1`; depends on: none
- `copper-deployment` — **TWIN-HARBOR** / `same-version-reference-deployment-successor-v1`; depends on: copper-artifact-custody
- `bedrock-docket-executor` — **RIVER-CLERK** / `live-docket-executor-prerequisite-v1`; depends on: none
- `bedrock-runtime-carrier` — **QUIET-EMBER** / `inert-runtime-release-carrier-prerequisite-v1`; depends on: none
- `bedrock-claim-fence` — **GRANITE-FENCE** / `durable-nq-claim-fence-prerequisite-v1`; depends on: none
- `bedrock-first-live` — **OPEN-QUARRY** / `bedrock-first-live-occurrence-successor-v1`; depends on: authorization-continuation, bedrock-docket-executor, bedrock-runtime-carrier, bedrock-claim-fence
- `glasshopper-closeout` — **GLASSHOPPER** / `passive-linode-canonical-relation-relaunch-v1`; depends on: none
