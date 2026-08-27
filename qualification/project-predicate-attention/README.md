# Generic project-predicate attention qualification

`unfamiliar-project/` is intentionally absent from Nightshift production
code. Its `project.concerns/v1` declaration and `project.ops.status/v1`
producer expose the attention-worthy bounded predicate
`queue.depth >= 18` under Cogwheel-only identities.

The opt-in Rust control
`crates/nightshiftd/tests/project_predicate_attention_e2e.rs` requires:

```sh
MONITOR_CONCERNS_BIN=/qualified/path/monitor-concerns \
NQ_MONITOR_BIN=/qualified/path/nq-monitor \
PULSE_PROJECT_PREDICATE_SUPPORT_BIN=/qualified/path/pulse-project-predicate-support \
cargo test -p nightshiftd --test project_predicate_attention_e2e -- --ignored
```

It proves real generic Monitor acquisition, real NQ semantic admission, real
Pulse qualification and exact replay, three distinct support occurrences,
Nightshift attention at the third occurrence, and duplicate refusal on replay
of the first receipt. CLI/test invocation itself is not evidence.
