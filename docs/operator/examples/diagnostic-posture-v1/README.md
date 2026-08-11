# Diagnostic-posture exact-input specimen

This checked-in specimen exercises the pure NQ diagnostic → Nightshift posture
boundary used inside the canonical observation cycle. It contains one exact
`nq.diagnostic_execution.v1` vector, a closed inventory, exact receiver inputs,
and exact recurrence evidence.

It is not a second production command surface. Run its conformance coverage:

```sh
cargo test --locked --test diagnostic_posture_foundation
cargo test --locked --test posture_surface
```

At `2026-07-27T20:00:10Z`, the exact fixture derives a current clean posture.
At `2026-07-27T20:01:10Z`, the same immutable NQ bytes no longer satisfy the
new recurrence slot, so the posture is non-current and incomplete. The source
artifact remains byte-identical.

`headline` is a lossy display summary, not a currentness predicate. In
particular, `Incomplete` can describe either current evidence with unqualified
required delivery or non-current evidence. A consumer needing currentness must
retain the posture's `current` field or the completeness, coverage, and
recurrence axes; it must not branch on the headline alone.

## Files and boundaries

- `nq-positive.json` is the exact canonical NQ diagnostic vector.
- `inputs.json` delivers that artifact through the typed receiver contract.
- `policy.json` declares the complete expected producer/question/profile/
  vantage/evaluator/threshold/projection/claim/state basis.
- `recurrence.json` binds the deterministic source run slot and attempt.

The pure evaluator validates and preserves these records. Present support is a
separate authority-owned input supplied during a canonical cycle; persisting
these files does not recreate support currentness. Nothing here authenticates
the producer, grants reliance, proposes work, supplies standing, authorizes an
effect, or executes anything.
