# Nightshift run ledger

- Packet: `nightshift-20260829-autonomous-convergence`
- Packet digest: `sha256:01e9f695fd89af789023cea0b9220a8e5178f807066779c9f7a4b7b3b67d4ba7`
- Receipt snapshot: `2026-08-29T17:22:22Z`
- Aggregate verdict: none; every campaign retains its own classification.

## Campaign DAG

```text
packet-v1 <- root
switchyard-transport <- packet-v1
authorization-continuation <- root
worker-vm-custody <- root
caliper-mode-contract <- root
caliper-authority-lifecycle <- caliper-mode-contract
lanternwake-port <- root
copper-artifact-custody <- root
copper-deployment <- copper-artifact-custody
bedrock-docket-executor <- root
bedrock-runtime-carrier <- root
bedrock-claim-fence <- root
bedrock-first-live <- authorization-continuation, bedrock-docket-executor, bedrock-runtime-carrier, bedrock-claim-fence
glasshopper-closeout <- root
```

## Per-workstream state

| Work item | Campaign | Dependencies | State | Classification |
|---|---|---|---|---|
| packet-v1 | VELVET-ORRERY / nightshift-immutable-run-packet-v1-20260829 | none | QUALIFIED | NIGHTSHIFT-ORIENTATION-PACKET-V1-IMPLEMENTATION-QUALIFIED |
| switchyard-transport | QUIET-BRIDGE / switchyard-explicit-plan-ref-transport-v1 | packet-v1 | AUDIT-IN-PROGRESS | WORKER-REPORTED-QUALIFIED-WITH-CUSTODY-LIMITATION |
| authorization-continuation | IRON-CHORUS / ag-codex-exact-occurrence-authorization-continuation-v1 | none | IN-PROGRESS | NO-CAMPAIGN-RESULT-YET |
| worker-vm-custody | SILVER-KINGFISHER / worker-vm-custody-admission-successor-v1 | none | IN-PROGRESS | NO-CAMPAIGN-RESULT-YET |
| caliper-mode-contract | GROUP-ANCHOR / generation-store-mode-contract-successor-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| caliper-authority-lifecycle | CROWNED-STORE / caliper-authority-lifecycle-successor-v1 | caliper-mode-contract | WAITING-PREREQUISITE | NO-CAMPAIGN-RESULT-YET |
| lanternwake-port | AMBER-COMPASS / reported-host-identity-successor-port-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| copper-artifact-custody | FORGE-VAULT / matched-k-n-release-custody-successor-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| copper-deployment | TWIN-HARBOR / same-version-reference-deployment-successor-v1 | copper-artifact-custody | WAITING-PREREQUISITE | NO-CAMPAIGN-RESULT-YET |
| bedrock-docket-executor | RIVER-CLERK / live-docket-executor-prerequisite-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| bedrock-runtime-carrier | QUIET-EMBER / inert-runtime-release-carrier-prerequisite-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| bedrock-claim-fence | GRANITE-FENCE / durable-nq-claim-fence-prerequisite-v1 | none | SCHEDULED | NO-CAMPAIGN-RESULT-YET |
| bedrock-first-live | OPEN-QUARRY / bedrock-first-live-occurrence-successor-v1 | authorization-continuation, bedrock-docket-executor, bedrock-runtime-carrier, bedrock-claim-fence | WAITING-PREREQUISITES | NO-CAMPAIGN-RESULT-YET |
| glasshopper-closeout | GLASSHOPPER / passive-linode-canonical-relation-relaunch-v1 | none | WAITING-FIXED-BOUNDARY | NO-NEW-CAMPAIGN-RESULT-YET |
