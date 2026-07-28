# Diagnostic posture v1-input operator specimen

This checked-in specimen exercises the read-only NQ-NG → Nightshift
foundation from a clean Nightshift checkout. It uses one exact canonical
`nq.diagnostic_execution.v1` positive vector, a closed one-row host
inventory, and the exact recurrence slot that produced it. The current
Nightshift evaluator accepts those frozen v1 inputs and emits the explicitly
versioned v2 posture contract; it does not relabel the NQ artifact.

From the repository root:

```sh
cargo run --quiet -p nightshiftd -- \
  diagnostics posture \
  --policy docs/operator/examples/diagnostic-posture-v1/policy.json \
  --inputs docs/operator/examples/diagnostic-posture-v1/inputs.json \
  --recurrence docs/operator/examples/diagnostic-posture-v1/recurrence.json \
  --evaluated-at 2026-07-27T20:00:10Z \
  --format text
```

The text output must identify an immutable source posture and its internal
truth-preserving rendering projection and report:

```text
headline: Clean
completeness: Complete
condition: Clean
coverage: Complete
recurrence: Current
delivery: NotRequired
diagnostic: ... status=ExplicitlyAbsent requirement=Mandatory visibility=Shown
```

Use `--format json` to obtain the canonical
`nightshift.operational_posture.v2` artifact. See the
[v2 consumer transition](../../../working/decisions/NQ-DIAGNOSTIC-CONSUMER-V2.md)
for the compatibility boundary.

The same immutable NQ input remains source-current under this specimen's
300-second source-age policy at the next schedule slot, while the absence of
that slot's recurrence record changes Nightshift posture:

```sh
cargo run --quiet -p nightshiftd -- \
  diagnostics posture \
  --policy docs/operator/examples/diagnostic-posture-v1/policy.json \
  --inputs docs/operator/examples/diagnostic-posture-v1/inputs.json \
  --recurrence docs/operator/examples/diagnostic-posture-v1/recurrence.json \
  --evaluated-at 2026-07-27T20:01:10Z \
  --format text
```

That second evaluation reports `recurrence: Incomplete` and
`headline: Incomplete`. The cross-repository test verifies that both posture
identities differ while `nq-positive.json` remains byte-identical.

## Files and boundaries

- `nq-positive.json` is copied byte-for-byte from NQ-NG's canonical Stage 6
  positive vector. The Nightshift cross-repository conformance test verifies
  its exact JCS round trip and self-identity.
- `inputs.json` delivers that same immutable artifact through the typed
  self-identified receiver contract.
- `policy.json` declares the complete subject/role inventory and exact
  producer, question, profile, vantage, evaluator, threshold, projection,
  claim, and state bindings expected by this posture.
- `recurrence.json` binds the deterministic current run slot to the exact NQ
  request, run, artifact, attempt interval, primary claim, and source
  acquisition dependency under a self-identified recurrence record.

This command performs structural contract validation and deterministic
posture evaluation only. It does not authenticate the NQ producer, admit new
evidence, persist state, run a schedule worker, grant reliance, propose an
operation, authorize anything, or execute anything.
