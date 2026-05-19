# GAP: Imported Basis Freshness (Slice B of DURABLE_ARTIFACT_SUBSTRATE consumption)

> Status: spec / pre-implementation. Filed 2026-05-18. Slice B of
> Night Shift's consumption of NQ's
> `DURABLE_ARTIFACT_SUBSTRATE_GAP` V1 substrate. Slice A
> (visibility-only, landed 250fc5d) plumbed `origin` and `silence`
> through the NS internal model; Slice B introduces the smallest
> useful semantic consumption: producer-clock-aware freshness for
> ingested findings. Implementation follows immediately under the
> six-test contract below.

## The deep rule

> **Night Shift may consume NQ findings. It cannot upgrade custody into basis.**

The keeper:

> **NQ lifecycle / custody time cannot launder upstream observation time.**

The companion, in plainer prose:

> **`captured_at` may prove when Night Shift saw the finding. It does not prove when the world was observed.**

## The problem

An imported finding has at least two clocks visible to Night Shift:

1. **Producer basis clock** — `origin.producer_extraction_time`.
   When the upstream producer actually extracted or observed the
   evidence. Present only when `origin.source = "import"`.
2. **Custody / lifecycle clock** — `FindingSnapshot.captured_at` /
   `first_seen_at`. When NQ ingested the finding from the producer
   (and, transitively, when Night Shift fetched the export). Always
   present.

A finding can be **freshly ingested but evidentially stale**: a new
envelope around a six-month-old lab result. Treating
`captured_at` recency as evidence freshness for ingested findings
is the laundering path. Slice B closes it.

The pattern is structurally a Kerberos-style ticket vs. authenticator
split — a primitive WLP / NQ already lean on; Night Shift adopts the
same distinction now that ingested findings are first-class.

## Time semantics

Time is treated as a hostile substrate, not a reliable one. The
two doctrines that anchor Slice B's time handling:

> **Clocks are witnesses, not facts.**

> **A timestamp is evidence about time, not authority over time.**

And the operational compression:

> **Freshness is not transitive across custody.**

That is: a fresh artifact arriving over a fresh transport does not
make its underlying basis fresh. Each clock proves what it proves
and nothing more.

### Four clock roles

Slice B names four distinct clock roles. The first three are
visible on the reconciliation receipt; the fourth is internal-only.

1. **Producer basis time** — `origin.producer_extraction_time`.
   When the upstream producer extracted/observed the evidence.
   *The only imported clock that can support evidence freshness.*
2. **Local custody time** — `FindingSnapshot.captured_at` /
   `first_seen_at`. When NQ/NS first saw the finding.
   *Proves custody, not freshness.*
3. **Decision time** — `reconciled_at` / now (wall clock at
   reconciliation). When NS is making the freshness decision.
   *Where freshness is evaluated.*
4. **Monotonic process time** — used only internally for elapsed
   runtime measurements; never appears on receipts and never
   participates in cross-system comparisons. Wall clock is for
   inter-system meaning; monotonic is for "did N seconds pass
   inside this process."

Do not mix wall and monotonic clocks across decision boundaries.

### Skew tolerance, not skew authority

A fixed skew budget allows for small clock drift between producer
and Night Shift hosts:

```
skew_budget = 60 seconds   (configurable; default 60)
```

Skew tolerance is allowed; clock incoherence beyond skew
tolerance never becomes authorization. Specifically, two
relations are enforced:

```
producer_extraction_time MUST NOT exceed captured_at + skew_budget
  → impossible ordering: NS cannot have captured a finding before
    the producer extracted it.

producer_extraction_time MUST NOT exceed reconciled_at + skew_budget
  → future evidence: extraction in the future of the reconciler's
    wall clock.
```

Either violation degrades the finding to `cannot-assess` with a
specific reason code (case 4 below). Parse failures of the
`producer_extraction_time` string fall in the same bucket.

### Stale must not eat clock failures

Three distinct time pathologies map to three distinct receipt
reasons, even when the existing `EvidenceState` ladder has only
`Stale` available as a non-execution posture:

| Pathology | Receipt reason | Notes |
|---|---|---|
| age known, exceeds window | `imported_producer_basis_stale` | the laundering killshot |
| age unknown | `imported_producer_basis_missing` | clock absent, not old |
| clock relation invalid | `imported_producer_clock_incoherent` | future, or after-custody beyond skew, or unparseable |

`Stale` carries one meaning; the other two pathologies preserve
the distinction in the reason code so audit can tell *unknown*
from *too old*. Collapsing them silently is a different sin in
the same church.

### Import lag is audit, not basis

```
import_lag = captured_at - producer_extraction_time
```

The receipt records `import_lag_seconds` for ingested findings
when both clocks are present. The value is audit material —
useful for noticing when "newly arrived" evidence is actually
many hours old at the producer — and **never** participates in
the freshness assessment itself. Freshness is evaluated against
producer time vs. reconciliation time; the lag between producer
and custody is interesting but not authoritative.

## Invariant

> For imported findings (`origin.source = "import"`),
> `origin.producer_extraction_time` is the basis clock for
> freshness. Custody / lifecycle clocks (`captured_at`,
> `first_seen_at`) establish when the finding entered local
> custody and **must not** substitute for evidence-basis
> freshness. If the producer clock is absent or incoherent,
> freshness is *unknown / cannot-assess* — never inferred from
> custody recency.

## Scope

Slice B is one narrow change to the reconciliation pathway:

- Imported findings only (`origin.source = "import"`).
- Freshness assessment only. No revalidation request, no upstream
  fetch, no ack-semantics change, no notification posture change.
- Single config knob (`imported_basis_freshness_window_seconds`)
  separate from any existing threshold.
- Reuse `EvidenceState::Stale` from the Slice 5 three-axis ladder
  for stale producer basis. Missing / incoherent producer time is
  *not* collapsed silently into Stale — receipt reason codes
  preserve the distinction.

Native findings (no `origin` block) follow the existing freshness
path unchanged. This is the regression sentinel.

## Five cases

The reconciler must distinguish these five cases for an ingested
finding (or one analog case for a native finding):

### 1. Imported, producer clock present, fresh

```
origin.source = "import"
origin.producer_extraction_time = T_p
now - T_p <= imported_basis_freshness_window_seconds
```

→ Reconcile normally. `EvidenceState` unchanged by Slice B logic.
   Receipt records `freshness_basis = producer_extraction_time`
   and `custody_basis = captured_at`.

### 2. Imported, producer clock stale

```
now - T_p > imported_basis_freshness_window_seconds
```

→ `EvidenceState::Stale`, regardless of how recent `captured_at`
   is. Per Slice 5: Stale → advise(revalidate-only); no execution.
   Receipt reason: `imported_producer_basis_stale`.

This is the laundering killshot — the case that closes the path
this slice exists to close.

### 3. Imported, producer clock missing

```
origin.source = "import"
origin.producer_extraction_time absent or null
```

→ Freshness *cannot be assessed*. Receipt reason:
   `imported_producer_basis_missing`. Posture: degraded /
   cannot-assess. **Do not** infer freshness from `captured_at`;
   **do not** silently treat as Stale.

If the existing `EvidenceState` ladder forces a choice (no
explicit "cannot-assess" state today), map to `Stale` *with the
distinct reason code* — never to the default reconciliation path.
The reason code is the load-bearing distinction.

### 4. Imported, producer clock incoherent

A producer clock is *incoherent* if it violates either of:

```
producer_extraction_time MUST NOT exceed captured_at + skew_budget
producer_extraction_time MUST NOT exceed reconciliation_time + skew_budget
```

Both relations matter: the first catches impossible import
ordering (extraction after ingestion); the second catches future
evidence (extraction in the future of the reconciler's wall
clock).

```
skew_budget = 60 seconds
```

→ Receipt reason: `imported_producer_clock_incoherent`. Same
   posture as case 3 (cannot-assess). Parse failures of the
   `producer_extraction_time` string also fall here.

### 5. Native finding

```
origin block absent
```

→ Existing freshness path applies unchanged. Slice B introduces
   no new behavior for native findings. The regression test
   pins this.

## Freshness window

A new pipeline-level config knob:

```
imported_basis_freshness_window_seconds   (default: 3600)
```

Independent from:

- `liveness_threshold_seconds` (different concern: liveness
  artifact age, not finding evidence age).
- Any NQ-side `extraction_stale` detector threshold (operator
  policy of the NQ instance; not Night Shift's reconciliation
  policy).

These thresholds answer different questions and **must not** be
deduplicated. NS uses
`imported_basis_freshness_window_seconds` only for reconciliation
posture on imported findings; NQ's own detector policy remains
NQ's call. The wire signal `extraction_stale` arrives as a regular
finding through the usual path (see Slice A); it does not
short-circuit Slice B logic.

## Receipt fields

Slice B exposes the basis / custody distinction on the
reconciliation receipt so downstream review can see *why* a
Stale verdict fired (or why one did not, when it might be
expected to).

Illustrative shape (not yet a frozen schema):

```json
{
  "freshness_basis": {
    "kind": "producer_extraction_time",
    "timestamp": "2026-05-18T18:00:00Z"
  },
  "custody_basis": {
    "kind": "finding_snapshot.captured_at",
    "timestamp": "2026-05-18T19:30:00Z"
  },
  "import_lag_seconds": 5400,
  "freshness_assessment": "stale",
  "freshness_reason": "imported_producer_basis_stale"
}
```

For case 3:

```json
{
  "freshness_basis": {
    "kind": "producer_extraction_time",
    "timestamp": null
  },
  "custody_basis": {
    "kind": "finding_snapshot.captured_at",
    "timestamp": "2026-05-18T19:30:00Z"
  },
  "freshness_assessment": "cannot_assess",
  "freshness_reason": "imported_producer_basis_missing"
}
```

For native findings:

```json
{
  "freshness_basis": {
    "kind": "native_lifecycle",
    "timestamp": "2026-04-17T03:00:00Z"
  },
  "custody_basis": {
    "kind": "finding_snapshot.captured_at",
    "timestamp": "2026-04-17T03:00:00Z"
  },
  "freshness_assessment": "fresh",
  "freshness_reason": "none"
}
```

(`import_lag_seconds` is omitted for native findings — there is
no upstream producer clock to lag against.)

## Internal type sketch

Implementation may use either a flat or a variant-shape enum, but
**must not** conflate clock source with assessment quality, and
**must not** make "unknown" look like a valid basis kind. The
following variant shape captures both the four clock pathologies
and the three coarse assessment buckets the reconciler reads:

```rust
enum FreshnessBasis {
    NativeLifecycle { timestamp: DateTime<Utc> },
    ProducerExtraction { timestamp: DateTime<Utc> },
    MissingProducerExtraction,
    IncoherentProducerExtraction,
}

enum FreshnessAssessment {
    Fresh,
    Stale,
    CannotAssess,
}
```

A finer-grained representation may be useful internally for
classifying which incoherence fired (future vs. after-custody vs.
parse failure). One option:

```rust
enum ImportedTimeAssessment {
    Fresh { evidence_age: Duration, import_lag: Option<Duration> },
    Stale { evidence_age: Duration, freshness_window: Duration },
    MissingProducerTime,
    ProducerTimeInFuture { skew_budget: Duration },
    ProducerTimeAfterCustody { skew_budget: Duration },
    TimestampParseFailed,
}
```

The reason code surfaced on the receipt remains a small,
audit-friendly string (`imported_producer_basis_stale`,
`imported_producer_basis_missing`,
`imported_producer_clock_incoherent`, `none` for native). The
variants above are illustrative; the spec pins the distinctions,
not the exact field shape.

## Acceptance tests

Eight tests pin the contract. They land before the implementation
(commit 2) and pass once the implementation lands (commit 3).

1. `native_finding_uses_existing_freshness_path`
   — regression sentinel; Slice B introduces no behavior change
   for native findings.
2. `imported_finding_uses_producer_time_for_freshness`
   — producer fresh → reconcile fresh; custody recency does not
   alter the verdict.
3. `imported_finding_recent_custody_does_not_override_stale_producer_basis`
   — **the laundering killshot.** `captured_at = now - 30s`,
   `producer_extraction_time = now - 4h`, window = 1h →
   `EvidenceState::Stale`, reason `imported_producer_basis_stale`.
4. `imported_finding_missing_producer_time_cannot_assess_freshness`
   — absent producer clock yields cannot-assess /
   `imported_producer_basis_missing`; never fresh-by-default and
   not silently collapsed into Stale.
5. `imported_finding_future_producer_time_is_clock_incoherent`
   — `producer_extraction_time` > `reconciled_at + skew_budget` →
   `imported_producer_clock_incoherent`.
6. `imported_finding_producer_time_after_custody_beyond_skew_is_incoherent`
   — `producer_extraction_time` > `captured_at + skew_budget` →
   `imported_producer_clock_incoherent` (the impossible-ordering
   case, distinct from the future-time case).
7. `imported_finding_allows_small_producer_custody_skew`
   — `producer_extraction_time` slightly after `captured_at`
   within `skew_budget` → admissible; not flagged as incoherent.
   Positive test for the skew tolerance band.
8. `import_lag_is_recorded_not_used_as_freshness_basis`
   — receipt carries `import_lag_seconds` for ingested findings;
   the value does not influence the freshness assessment (an
   ingested finding with huge import lag but fresh producer time
   reconciles as fresh).

## Non-goals

Slice B explicitly does *not*:

- Contact upstream producers or attempt revalidation.
- Re-prove the finding or mutate NQ truth in any way.
- Change `EvidenceState` semantics for native findings.
- Introduce a new top-level `EvidenceState` variant beyond the
  Slice 5 ladder — reuse `Stale` with distinct receipt reasons.
- Rewrite the liveness gate or its threshold.
- Treat silence-shaped findings as a separate posture class
  (that is Slice C — *"a silence ack is not an active-finding
  ack"* — and stays untouched here).
- Affect ack semantics or notification posture.
- Treat imports as inherently less true — only as having a
  distinct basis clock that the reconciler must respect.

## Connection to existing doctrine

- **Slice 5 three-axis split** (NQ owns truth; NS owns posture +
  ack obligation). Slice B preserves the split. `Stale` is an NS
  posture verdict already in the ladder; producer-clock-aware
  staleness gives it a new *reason* to fire, not a new axis.
- **Liveness wrinkle contract** (*"don't trust upstream
  `fresh`"*, `project_liveness_consumer_pending.md`). Structural
  analog: Slice B is the same discipline applied to ingested
  findings — Night Shift computes its own freshness assessment
  rather than trusting recency-of-receipt as a proxy.
- **Slice A visibility** (250fc5d). The wire fields Slice B reads
  (`origin.producer_extraction_time`, `origin.source`) are
  already in the NS internal model; Slice B uses them for the
  first time as control input rather than display.
- **Build-gate doctrine** (test module doc on
  `durable_artifact_substrate_v1.rs`): Slice B is being built by
  maintainer intent, ahead of an operationally-ratifying
  incident. The spec's behavior should be treated as *provisional*
  until operator use, incident review, or live coordination
  pressure supports the chosen threshold and reason taxonomy.

## Build shape

Three commits for the primitive:

1. **This spec doc.**
2. **Red tests + fixtures** — the eight tests above; tests fail until commit 3 lands.
3. **Implementation** — new `FreshnessBasis` / `FreshnessAssessment`
   types; the primitive `assess_freshness`; the config knob and its
   default.

Then two more commits for behavior:

4. **B.1 — observe-only integration** (`8821efd`). The pipeline calls
   `assess_freshness` between adjudicate and packet build; the
   verdict lands on `bundle::ReconciliationResult.freshness` as a
   `FreshnessReceipt`; `EvidenceState` is not yet mutated. *Make the
   clock seam visible before making it binding.*
5. **B.2 — bind stale imported basis to the Slice 5 path** (`f510a44`).
   `FreshnessOutcome::Stale` with reason
   `imported_producer_basis_stale` is mutated into
   `InputStatus::Stale` + `RelianceClass::Historical` + `valid_for =
   [PacketContext]`; `Attention.evidence_state` cascades to
   `EvidenceState::Stale`; the packet's regime starts with `stale`
   and `ProposedAction` proposes revalidate-only steps.
   `MissingProducerExtraction` and `IncoherentProducerExtraction`
   stay at `cannot_assess` with their distinct receipt reasons;
   Stale does not eat clock failures.

Slice C remains untouched.

## Closeout (2026-05-19)

Slice B is behavior-complete for the producer-stale case. One
consumer-safety invariant pinned during closeout audit, recorded
here as Slice B provenance:

> **`reconciliation_summary.ok_to_proceed` is NOT an authorization
> summary.** Under v1 semantics it means exactly *"no input was
> hard-invalidated."* A run with stale imported basis (or any other
> downgraded input) leaves `ok_to_proceed = true` and surfaces the
> caution through `ProposedAction` (revalidate-only),
> `Attention.evidence_state` (`Stale`), `result.reliance_class`
> (`Historical`), `result.scope.valid_for` (`[PacketContext]` only),
> and `summary.downgraded`.

The audit reached the following findings:

- No production NS code branches on `ok_to_proceed` alone. The field
  is informational + ledger material; production action requires
  Governor authorization, which keys on receipts, not on this bool.
- The `ReconciliationSummary` struct had no doc comments before
  this audit. A future Rust consumer reading the type alone could
  plausibly misread the field. Fixed: doc comments added to both
  the struct and the `ok_to_proceed` field naming what they do and
  don't promise.
- `SCHEMA-bundle.md` showed an example with `ok_to_proceed: true`
  alongside `downgraded: [...]` populated, with no disambiguation.
  Fixed: an inline note next to the example pins the consumer
  caution and points readers at the fields they must inspect.
- A sentinel test
  (`b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`)
  asserts the full gating surface in one place. If a future change
  flips `ok_to_proceed = false` on Stale (which would be a Slice 5
  doctrine change, not a Slice B change), this test must be
  updated deliberately.

No semantics were changed during closeout. The audit confirmed the
contract; the doc and test changes document and tripwire it.

Slice B status: **behavior-complete; consumer caution pinned.** Slice
C (silence posture / ack lineage) remains untouched; its design space
is left in its current `cannot_assess`-visible-only state under
Slice B.
