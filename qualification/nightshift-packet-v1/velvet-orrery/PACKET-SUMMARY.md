# Nightshift packet summary (non-authorizing)

> This rendering is an orientation and scheduling aid. It grants no authority, approval, retry, execution custody, or settlement.

- Packet: `nightshift-20260829-autonomous-convergence`
- Digest: `sha256:9b4e1a2cea4010dbc620368fe3adfe51a7c1e2551fa7af61f27f94a0a8bc692b`
- Current: `2026-08-29 17:00:00 UTC` through `2026-08-30 12:00:00 UTC`
- Switchyard alias: `nightshift-convergence-20260829`
- Immutable plan reference: `nightshift-packet://9b4e1a2cea4010dbc620368fe3adfe51a7c1e2551fa7af61f27f94a0a8bc692b`

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
