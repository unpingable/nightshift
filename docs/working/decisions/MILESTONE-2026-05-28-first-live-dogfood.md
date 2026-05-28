# Milestone — First Live-Data Dogfood (2026-05-28)

**Filed:** 2026-05-28
**Status:** milestone record, not shipped state. No code landed; no doctrine changed. This is the first time NS shipped doctrine was exercised against real ops data instead of fixture data.

## Two milestone lines

```text
live-data packet proven       (2026-05-28 16:19Z)
operator lifecycle proven     (2026-05-28 16:46Z)
pilot not yet established
```

**Not a birth certificate.** First heartbeats are not delivery dates. A repeated, scheduled, durable-store dogfood is the pilot; this is the precondition that proves the pipe is wired.

## What landed

- **Input:** live NQ substrate evidence — local nq instance (`~/nq/nq.db`, `sushi-k` witness publisher on `127.0.0.1:9847`) producing real findings about this dev box.
- **Target finding:** `nq:disk_pressure:sushi-k:` — IncidentShape, Critical severity, persisted for 22,579 generations (~15 days) at first probe.
- **Probe shape:** `/tmp/`-only — agenda fixture at `/tmp/sushi-k-disk-pressure.yaml`, NS store at `/tmp/nightshift-probe.sqlite`, `--no-governor` (ceiling capped at advise).

## What the lifecycle proved

| Axis | Before ack | After ack | What this proves |
|---|---|---|---|
| Evidence (NQ truth) | Active | Active | NQ truth-axis unmoved by operator action |
| Posture (NS) | IncidentShape | IncidentShape | NS posture-axis unmoved by operator action |
| Attention (NS operator) | Unowned | **Acknowledged** | Operator-axis transitions durably |
| Closure refusal | `unassessable (missing consequence-witness)` | `not_eligible (operator_attention_active)` | **Still refused.** Reason refined per Slice 4 doctrine ordering |
| Ledger | (no ack event) | `run_attention_changed applied=acknowledged` | Operator intent is audit material |
| Render | (no next-check / ack-expires fields) | `next check: 17:44Z / ack expires: 17:44Z` | Operator-scannable in `runs show` |

## The key sentence

> **Acknowledged did not imply closable.**

The closure refusal class moved (operator attention beats posture-class per Slice 4 ordering), but closure stayed refused. The operator owning the finding refined *why* closure is refused; it did not unlock closure. The boolean-laundering refusal trio (`silence_present ≠ incident_absent`, `acked_silence ≠ acked_incident`, `no_new_evidence ≠ resolved`) held in **lifecycle**, not just at packet-emission time.

This is the whole doctrine wearing work boots.

## Doctrine surfaces exercised on live data

- **SLICE_5_CONTRACT V1** — three-axis split (truth/posture/ack) held: NQ-owned truth axis unmoved by NS-axis transitions. None of the three axes masqueraded as another.
- **SLICE_C_1 V1** — `PostureClass::IncidentShape` derived correctly from the live finding's envelope shape.
- **SLICE_4_CLOSURE_CANDIDATE V1** — refusal classes fired correctly in both phases: `UnassessableMissingConsequenceWitness` pre-ack (no consequence-witness present), `NotEligible(OperatorAttentionActive)` post-ack (operator-attention ordering beats posture-class). The conservative-default sentinel held: no IncidentShape finding silently rounded up to `EligibleForClosureReview`.
- **SLICE_3_ATTENTION_LIFECYCLE V1** — ack persisted into `(agenda, finding)`-keyed attention table; reconciler applied read-time projection on the next run; ledger event `run_attention_changed` recorded the transition.
- **`--no-governor` ceiling-lowering** — `requested=Advise / governor_present=false`; promotion ceiling capped per CLAUDE.md invariant 8 (intelligence dependencies improve quality, not authority; safety dependencies lower the ceiling).
- **Run idempotency** — `--trigger manual` default opened both runs as independent rows; idempotency-skip path (which would fire on `--trigger scheduled` within the same NQ generation) not exercised this dogfood.

## Two polish notes (not doctrine bugs)

1. **Empty-subject rendering.** When the NQ finding key has an empty subject (`nq:disk_pressure:sushi-k:`), NS renders the subject field as `'sushi-k:'` in the packet — concatenating the host with a trailing colon. Cosmetic. Worth a one-line render fix when convenient; doesn't affect classification, refusal, or ledger.
2. **Ack note re-uses `silence_reason` packet field.** Operator passed `--note "known, watching"` to `attention ack`; packet rendered it as `attention.silence_reason: known, watching`. Field is being shared across ack and silence operator paths. Naming polish: introduce a neutral `attention.operator_note` field (or rename the existing field) so an ack's note doesn't read as a silence reason in the packet. Operator-scannability item, not a semantic bug — the note text is preserved correctly.

## What this dogfood was NOT

- **Not a stable pilot.** `/tmp/` paths, manual invocation, no schedule, no systemd unit, no repeated runs. The pilot conversation is downstream of this dogfood, not this dogfood.
- **Not a full lifecycle exercise.** Silence path not tried; re-ack with `--disposition` not tried; investigate / handoff / request-revalidation not tried. The single ack → re-run transition was the milestone; the rest of the operator-loop surface waits for the next exercise.
- **Not a liveness gate exercise.** `--nq-liveness` was not passed. The LIVENESS_CONSUMER V1 path on real data is owed but not landed here.
- **Not a horizon path exercise.** `--horizon-policy` + `--governor-socket` not passed. Tier-2 horizon receipts on real data is owed but not landed here.
- **Not a wal-bloat-review pilot.** The `tests/fixtures/wal-bloat-review.yaml` agenda targets a `wal_bloat` detector that has zero current findings. The originally-named pilot needs the agenda updated to point at a live finding-class (e.g., `freelist_bloat` on the linode side, where labelwatch's sqlite is bloated) before *that* pilot can run.
- **Not committed.** No code, no agenda, no doctrine change. The `/tmp/` artifacts can be discarded; this milestone note is the durable record.

## What's next

After this milestone documentation lands:

- **(a) Stable local pilot setup.** Repo-located agenda (`agendas/sushi-k-disk-pressure.yaml` or similar), durable NS store path (`~/nightshift/` or similar), repeated `--trigger manual` runs at first, eventually a systemd timer for `--trigger scheduled` to exercise idempotency on real data.
- **(b) Second-half lifecycle.** Silence path with `--until` + `--reason`; re-ack with `--disposition` to exercise the re-ack-as-mini-re-triage path; investigate / handoff if scope grows.
- **(c) Liveness path on live data.** Pull `~/nq/liveness.json` in via `--nq-liveness` to see the gate fire (or pass) against real liveness state. Not high signal if liveness is fresh; useful regression test.
- **(d) Slice 5 (proxy-shock recognition) design.** Coordinates with NQ on wire shape for regime-change findings. Today's snapshot had no proxy-shock-shape findings to design against, so this is still pre-data; the dogfood doesn't unblock Slice 5 directly but does prove the pipe is ready when shock-shape findings appear.
- **(e) Render polish for the two notes above.** Cosmetic; lands when an interested operator pass cycle happens.

## Provenance

Dogfood plan formed in this session after the cartography channel-split landing and the AUDIT-BACKLOG breadcrumb commits. The probe was the smallest move that exercised shipped doctrine end-to-end on real data:

- Local nq binary located at `/home/jbeck/git/notquery/target/release/nq` (live publisher + aggregator running, feeding `~/nq/nq.db`).
- Local nq publishing findings about `sushi-k` (this dev box) via the `sushi-k` witness publisher source.
- NS shells out to `nq findings export` and consumes the canonical snapshot contract.

The lifecycle order was directed by chatgpt's framing during the dogfood: *"Do (2) first. Run the operator lifecycle... silence/re-ack is useful, but it's a second lifecycle exercise. Capture the first one while it's crisp."* The single-transition discipline (ack-then-rerun, not ack-then-silence-then-reack) protected the milestone from getting smeared into "one more lifecycle edge."

Key acceptance sentence pinned by chatgpt:

> *"Acknowledged must not imply closable."*

Pass. End of milestone.
