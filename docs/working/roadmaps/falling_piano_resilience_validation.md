# FALLING-PIANO — controlled failure-injection observability qualification

> **Track:** `nightshift-resilience-validation`  
> **Codename:** `FALLING-PIANO`  
> **Canonical slug:** `controlled-failure-injection-observability-qualification`  
> **Status:** **PLANNED / NOT STARTED**  
> **Result classification:** none  
> **Filed:** 2026-08-29  
> **Authority:** documentation only; this record is not authorization to activate a campaign or execute a fault.

## Roadmap boundary

This is a deferred Nightshift roadmap campaign for a bounded, evidence-producing
failure-injection qualification program. It is not active, parked, qualified,
or authorized. It adds no dependency edge to the current Nightshift execution
graph and is not part of the immutable packet for the 2026-08-29 run.

Recording this roadmap does not authorize implementation, fault execution,
infrastructure mutation, worker creation, VM or cluster creation, live-route
changes, or any production action. A future activation requires its own exact
work authority and fresh campaign and occurrence identities.

The design goal is to verify that monitoring, observation, scheduling, custody,
recovery, and closeout remain truthful when a bounded infrastructure or workload
failure is deliberately introduced. The central invariant is:

> Loss of observation must never be silently promoted into a healthy or calm
> reading.

A failed injector, an undetected fault, an observation outage, an attribution
failure, a response failure, and a recovery failure are distinct outcomes.

## Qualification chain

The work is not complete merely because an alert fires. Each future scenario
must preserve evidence across the complete chain:

```text
fault proposal
    -> fault admission and exact occurrence identity
    -> confirmed fault execution
    -> observation or explicit observation gap
    -> detection and attribution
    -> alert/state transition
    -> scheduler or operator response
    -> recovery
    -> teardown and custody verification
    -> independent result classification
```

Initial work must prefer exact, deterministic, receipt-bearing scenarios over a
generalized random-chaos framework. The system must first prove what it proposed,
what it admitted, what it changed, what actually occurred, and what it observed.
Randomized coverage may follow only after that scenario contract qualifies.

## Ordered target families

### 1. Disposable VM substrates

Potential qualification cases include clean shutdown, abrupt power loss, forced
reboot, failed boot, QEMU or guest-process death, guest pause or bounded scheduler
starvation, CPU or memory pressure, bounded OOM conditions, disk pressure,
read-only filesystems, bounded disk exhaustion, network loss or degradation, DNS
failure, asymmetric partition, service crash or hang, missing listeners, stale
PID state, host/reported-host identity mismatch, stale or missing exporters, and
orphaned VMs, overlays, processes, sockets, or teardown obligations.

Begin with disposable local fixtures and one fault occurrence at a time. This
stage does not authorize arbitrary production VM destruction.

### 2. Disposable or isolated k3s substrates

Potential qualification cases include Pod deletion or crash loops, node
shutdown/drain/pause/partition, API-server unavailability, controller restart,
CNI or overlay disruption, readiness/liveness failure, image-pull failure,
storage or PVC unavailability, workload identity mismatch, stale workloads or
observations, claim/fence contention, unreleased fences, duplicate-work
prevention, and loss of the executor, inert runtime carrier, or durable
claim/fence interface identified by BEDROCK.

The first k3s qualification must use an isolated, recoverable cluster after the
necessary BEDROCK seams independently qualify. It must not consume, reopen, or
rewrite BEDROCK's terminal occurrences.

### 3. Isolated `atproto-*watch` systems

Potential qualification cases include upstream timeout or 5xx, malformed or
unreachable sources, ingest pause or process death, cursor stall/corruption or
invalid continuation state, database locks, storage pressure or bounded
disk-full state, stale observation, exporter identity mismatch, delayed
acquisition, skipped scheduling slots, total or partial observation loss,
receipt or provenance mismatch, and false-negative or false-calm readings.

This stage must reproduce the Labelwatch observation-adequacy failure class in
which failed observation suppressed positive findings without qualifying the
negative reading. It must also use applicable Driftwatch and Weatherwatch
observation paths as retained evidence, without altering those systems merely by
recording this roadmap.

### 4. Composed cross-layer failures

Combine failures only after their individual forms qualify. Candidate examples
include a workload failure during monitor degradation, node loss while a fence
is held, or upstream loss during local storage pressure.

### 5. Other Nightshift-managed infrastructure and control-plane components

Later work may cover worker disappearance, stalled or duplicate dispatch,
missing or contradictory receipts, stale packet references, scheduler restart,
provider-budget telemetry becoming stale or unknown, provider exhaustion during
bounded active work, foreman loss and restart, and incomplete closeout or custody
handoff. A fuel-gauge UI is not a prerequisite.

## Required scenario and evidence contract

Before any future activation, every scenario must declare:

- the exact target and substrate;
- immutable fixture or release identity;
- a fresh fault occurrence identifier;
- the expected precondition;
- the exact permitted mutation;
- the blast-radius boundary;
- maximum duration or TTL;
- a kill switch;
- rollback or teardown procedure;
- the expected observation path and detector;
- the detection deadline and expected attribution;
- recovery criteria and a post-recovery observation period;
- custody verification; and
- explicit stop conditions.

Fault execution requires independent confirmation. A command returning exit
status zero is not sufficient evidence that the intended state transition
occurred.

A retry must not reuse a consumed fault occurrence. A new attempt receives a
fresh occurrence identity and preserves the prior outcome. Missing monitoring
coverage is a valid qualification result. Repairing a detector during the same
occurrence must not retroactively turn the original result into success.

## Suggested stages

### F0 — fault and safety contract

Define the scenario envelope, occurrence identity, evidence requirements, stop
law, recovery law, and classifications. Produce no infrastructure fault.

### F1 — disposable VM qualification

Exercise a small deterministic matrix against disposable VMs. Qualify monitor
identity, service/process detection, bounded resource pressure, network failure,
recovery, teardown, and custody.

### F2 — isolated k3s qualification

Exercise bounded Pod, node, control-plane, network, storage, and
executor/claim/fence cases against an isolated cluster after the BEDROCK
prerequisites independently qualify.

### F3 — isolated watch-stack qualification

Run disposable instances of selected `atproto-*watch` systems and exercise
acquisition, identity, scheduling, database, and observation failures. Prove
explicitly that "unable to observe" cannot become "calm."

### F4 — composed failure qualification

Combine only individually qualified scenarios and retain the exact independent
and composed occurrence identities.

### F5 — Nightshift control-plane qualification

Extend the same contract to dispatch, receipt, scheduler, provider-budget,
foreman, and closeout failure cases.

### F6 — explicitly authorized canary environments

Consider bounded canary deployments only after disposable qualification is
mature. Every destructive or externally visible action requires separate
explicit authority. This roadmap does not imply production failure injection.

## Classification and campaign boundaries

There is no aggregate "chaos qualified" result. VM, k3s, watch-stack,
composed-failure, and control-plane work may require separate successor campaigns
and independent classifications. Terminal predecessors and consumed occurrences
remain immutable.

A scenario may qualify detection while failing attribution, response, recovery,
or custody. Those dimensions remain separate. Nightshift may eventually schedule
and supervise this work, but a Nightshift packet is an orientation and scheduling
envelope, not authority to perform a fault.

## Design intersections

- [`../../NIGHTSHIFT_PACKET_V1.md`](../../NIGHTSHIFT_PACKET_V1.md) supplies the
  current non-authorizing packet/foreman boundary; FALLING-PIANO must cite exact
  work authority rather than embed it.
- [`../gaps/GAP-nightshift-coordination-mode.md`](../gaps/GAP-nightshift-coordination-mode.md)
  holds the local-first, adapter-shaped provider budget observation requirement.
  Stale or unknown budget telemetry is itself a later control-plane scenario.
- [`../../GENERIC_PROJECT_PREDICATE_ATTENTION_V1.md`](../../GENERIC_PROJECT_PREDICATE_ATTENTION_V1.md)
  records current Weatherwatch, Labelwatch, and Driftwatch observation/support
  boundaries.
- [`../decisions/GAP-autonomous-execution-boundary.md`](../decisions/GAP-autonomous-execution-boundary.md)
  and [`../../PRESENT_EVIDENCE_SUPPORT_SOURCE_GATE.md`](../../PRESENT_EVIDENCE_SUPPORT_SOURCE_GATE.md)
  preserve the Labelwatch observation-adequacy and support-source boundary.
- LANTERNWAKE's reported-host integration, GLASSHOPPER's scheduled observation
  and closeout evidence, CALIPER's VM lifecycle evidence, and BEDROCK's executor,
  inert-carrier, and durable claim/fence prerequisites are predecessor evidence
  only. This roadmap does not alter their campaigns or classifications.

## Activation gate

This item remains **PLANNED / NOT STARTED**, with result classification **none**.
It has no active dependency edge and does not block current work. Any future
activation must begin with F0 under separate explicit authority and preserve the
existing campaign, occurrence, and classification boundaries described above.
