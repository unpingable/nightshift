# Nightshift packet summary (non-authorizing)

> This rendering is an orientation and scheduling aid. It grants no authority, approval, retry, execution custody, or settlement.

- Packet: `nightshift-operational-spine-ecad-20260829-v2`
- Digest: `sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`
- Current: `2026-08-30 03:07:18 UTC` through `2026-09-13 03:07:18 UTC`
- Switchyard alias: `nightshift-operational-spine-ecad-v2-20260829`
- Immutable plan reference: `nightshift-packet://1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`

## Campaign DAG

- `foreman-core` — **CLOCKWORK-MOTH** / `nightshift-durable-foreman-state-machine-v1`; depends on: none
- `budget-observation` — **FUEL-NEEDLE** / `provider-capacity-observation-and-scheduling-policy-v1`; depends on: none
- `worker-adapter` — **TUNNEL-FINCH** / `nightshift-worker-adapter-and-switchyard-codex-bootstrap-v1`; depends on: foreman-core
- `live-casework` — **LEDGER-FOX** / `nightshift-live-run-casework-projection-v1`; depends on: foreman-core
- `integrated-dogfood` — **MIDNIGHT-RAIL** / `nightshift-restartable-autonomous-foreman-qualification-v1`; depends on: foreman-core, budget-observation, worker-adapter, live-casework
- `operational-spine` — **FIELD-CLOCK** / `monitor-nq-nightshift-operational-evidence-spine-v1`; depends on: none
- `observation-wire` — **DISTANT-BELL** / `authenticated-store-and-forward-observation-ingress-v1`; depends on: operational-spine
- `operational-casework` — **SHIFT-ATLAS** / `nightshift-operational-condition-casework-v1`; depends on: operational-spine, live-casework
- `ecad-pilot` — **SILICON-ORCHARD** / `ecad-operational-observation-golden-journey-v1`; depends on: operational-spine, observation-wire, operational-casework
