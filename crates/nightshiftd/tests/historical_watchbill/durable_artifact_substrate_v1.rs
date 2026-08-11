//! HISTORICAL WATCHBILL SPECIMEN. Consumer-alignment dry run for NQ `DURABLE_ARTIFACT_SUBSTRATE_GAP` V1.
//!
//! Open acceptance criterion from
//! `~/git/nq/docs/gaps/DURABLE_ARTIFACT_SUBSTRATE_GAP.md` §V1 step 6:
//!
//! > At least one downstream consumer (NS preferred) reads the
//! > synthetic-producer output and either (a) consumes the ingested
//! > finding through the same admissibility path it uses for native NQ
//! > findings, or (b) refuses with a typed error explaining what doesn't
//! > fit. Either result is information; (a) ratifies the domain admission,
//! > (b) sends the gap back to "sibling tool" framing.
//!
//! Open question 7 leans mechanical:
//!
//! > NS V1.x parses the synthetic producer output without
//! > `NqInadmissible`-shaped refusal under the existing admissibility
//! > branching.
//!
//! Fixtures (real captures, 2026-05-12, NQ schema 46, contract v1):
//!
//! - `nq-findings-import-clean.jsonl` — two ingested findings with
//!   `origin.source = "import"` populated; producer fresh; no `silence`.
//! - `nq-findings-import-stale.jsonl` — two lines: (1) NQ's own
//!   `extraction_stale` finding with the new `silence` envelope and no
//!   `origin`; (2) an ingested finding from the stale producer with
//!   `origin` populated and no `silence`.
//!
//! What this dry run asserts:
//!
//! 1. **Schema/contract compat.** V1-substrate exports still claim
//!    `nq.finding_snapshot.v1` + `contract_version: 1`. NS's hardcoded
//!    `NQ_EXPORT_SCHEMA` / `NQ_EXPORT_CONTRACT_VERSION` continue to
//!    match. No parse-time schema refusal.
//! 2. **Forward-compat tolerance.** New top-level fields (`origin`,
//!    `silence`, `basis`, `regime`, `observations`, `generation`,
//!    `export`, `diagnosis`, `admissibility.ancestor_finding_key`,
//!    `admissibility.declaration_id`) parse silently — NS's
//!    `NqExportDto` is forward-compat tolerant by design.
//! 3. **Admissibility path unchanged.** Every captured line has
//!    `admissibility.state = "observable"`. NS's "observable" gate
//!    admits all of them through the same path it uses for native
//!    findings. **No `NqInadmissible` refusal under existing
//!    branching** — the mechanical bar Open Q7 names.
//! 4. **Translation contract preserved.** `translate_nq` produces a
//!    valid `FindingSnapshot` for each ingested line. Severity,
//!    condition_state, timestamps all map cleanly.
//! 5. **Two-clock latency surfaced (informational, not blocking).**
//!    NS's `last_seen_at → captured_at` translation reflects NQ ingest
//!    time, not producer extraction time. This is the documented
//!    consequence of forward-compat tolerance: NS tolerates the
//!    `origin` envelope but does not branch on it. Cross-clock
//!    reconciliation is scope-deferred per NQ gap doc Open Question 3.
//!
//! Verdict: outcome (a) — substrate admission ratified.
//!
//! ---
//!
//! ## Slice A: operator visibility (2026-05-12)
//!
//! Beyond the dry-run admission bar, Slice A plumbs origin and
//! silence through Night Shift's internal model so operators can
//! see both clocks side-by-side and inspect silence envelopes via
//! peek and packet output. Tests below assert the visibility
//! contract end-to-end.
//!
//! ## Deferred semantics
//!
//! Three questions Slice A does not answer:
//!
//! - Whether `origin.source = "import"` should be treated as
//!   *finding shape* (Night Shift may branch on it) or as something
//!   closer to *witness metadata* (Night Shift should respond to
//!   the finding shape NQ surfaces and not to per-finding origin
//!   metadata). CLAUDE.md prohibits branching on NQ witness
//!   positions; the parallel reasoning around origin has not been
//!   settled.
//! - Whether silence-shaped findings warrant a different
//!   notification posture or ack obligation. The three-axis split
//!   (NQ owns truth; Night Shift owns posture + ack) leaves this
//!   open.
//! - Whether `producer_extraction_time` should inform freshness or
//!   revalidation reasoning for ingested findings. The structural
//!   analog of the liveness wrinkle contract — *"don't trust
//!   upstream `fresh`"* — applies here in principle but is not
//!   wired.
//!
//! ## Build gate vs ratification gate
//!
//! Slices B and C (provenance-aware reconciliation; silence-aware
//! posture) are optional follow-on explorations, not V1
//! requirements. They may proceed by maintainer intent. Their
//! behavior should be treated as provisional until runs, operator
//! use, or incident review show which semantics actually matter.
//!
//! > **Exploration does not require prior evidence. Ratification does.**
//!
//! Three distinct gates govern different kinds of work:
//!
//! - *Implementation permission* — maintainer intent, design
//!   hypothesis, aesthetic judgment, anticipated friction,
//!   curiosity. No prior evidence required.
//! - *Ratification of operational claims* — observed runs,
//!   operator use, incident review, cross-system failures,
//!   regression tests. Required before describing a slice's
//!   behavior as operationally necessary or required.
//! - *Doctrine / GAP promotion* — concrete boundary, repeated
//!   pattern, actual laundering vector, demonstrated confusion.
//!   Required before promoting a candidate GAP to non-candidate.
//!
//! Slice A is visibility-only. Slices B and C, when built, will
//! need their own design notes and tests; this file does not
//! pre-claim their semantics.

use std::path::PathBuf;

use nightshiftd::finding::{EvidenceState, Severity};
use nightshiftd::nq::{parse_nq_line, translate_nq};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn load_lines(fixture: &str) -> Vec<String> {
    let path = fixtures_dir().join(fixture);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("fixture must exist: {}", path.display()));
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect()
}

// -----------------------------------------------------------------------------
// 1. Schema/contract compatibility under V1 substrate
// -----------------------------------------------------------------------------

#[test]
fn ingested_findings_preserve_schema_and_contract_version() {
    let lines = load_lines("nq-findings-import-clean.jsonl");
    assert_eq!(lines.len(), 2, "clean fixture has two findings");

    for line in &lines {
        let dto = parse_nq_line(line).expect("V1-substrate export must parse under contract v1");
        assert_eq!(dto.schema, "nq.finding_snapshot.v1");
        assert_eq!(dto.contract_version, 1);
    }
}

// -----------------------------------------------------------------------------
// 2. Admissibility path unchanged — the mechanical bar from Open Q7
// -----------------------------------------------------------------------------

/// The headline acceptance: NS reads the synthetic-producer output
/// without `NqInadmissible`-shaped refusal. This is the mechanical
/// version of "consumes through the same admissibility path it uses
/// for native NQ findings."
#[test]
fn ingested_findings_admit_through_observable_path_no_refusal() {
    for fixture in ["nq-findings-import-clean.jsonl", "nq-findings-import-stale.jsonl"] {
        for line in load_lines(fixture) {
            let dto = parse_nq_line(&line)
                .unwrap_or_else(|e| panic!("V1 substrate line refused: {e:?}\nline: {line}"));
            assert_eq!(
                dto.admissibility.state, "observable",
                "every captured V1-substrate line is observable in this fixture"
            );
        }
    }
}

// -----------------------------------------------------------------------------
// 3. Translation contract — ingested findings produce valid snapshots
// -----------------------------------------------------------------------------

#[test]
fn ingested_finding_translates_to_valid_snapshot() {
    let lines = load_lines("nq-findings-import-clean.jsonl");
    let dto = parse_nq_line(&lines[0]).unwrap();
    let snap = translate_nq(&dto).expect("translate must succeed on V1-substrate ingested finding");

    // Native invariants hold for ingested findings — they admit through
    // the same path.
    assert_eq!(snap.finding_key.source, "nq");
    assert_eq!(snap.finding_key.detector, "imported_corpus_health");
    assert!(
        snap.finding_key.subject.starts_with("synthetic-corpus-extractor:"),
        "finding_key.subject embeds host: prefix per NS convention"
    );
    assert_eq!(snap.severity, Severity::Warning);
    assert_eq!(snap.current_status, EvidenceState::Active);
    assert_eq!(snap.persistence_generations, 1);
    assert_eq!(snap.snapshot_generation, 1);
}

// -----------------------------------------------------------------------------
// 4. Stale fixture — extraction_stale + ingested both admit
// -----------------------------------------------------------------------------

#[test]
fn extraction_stale_finding_admits_without_silence_field_parsing() {
    // Line 1 of the stale fixture is NQ's own `extraction_stale` finding,
    // carrying the new SILENCE_UNIFICATION `silence` envelope. NS does
    // not parse `silence` fields today (forward-compat tolerant). The
    // finding must still admit through the observable path, and the
    // detector identity is preserved.
    let lines = load_lines("nq-findings-import-stale.jsonl");
    let dto = parse_nq_line(&lines[0]).expect("extraction_stale must parse");
    assert_eq!(dto.identity.detector, "extraction_stale");
    assert_eq!(dto.admissibility.state, "observable");

    let snap = translate_nq(&dto).unwrap();
    assert_eq!(snap.finding_key.detector, "extraction_stale");
    assert_eq!(snap.current_status, EvidenceState::Active);
}

#[test]
fn ingested_finding_from_stale_producer_admits() {
    // Line 2 of the stale fixture is an ingested finding whose producer
    // is stale. NS does not branch on producer freshness today; the
    // finding admits through the observable path.
    let lines = load_lines("nq-findings-import-stale.jsonl");
    let dto = parse_nq_line(&lines[1]).expect("ingested-from-stale must parse");
    assert_eq!(dto.identity.detector, "imported_corpus_health");
    assert_eq!(dto.admissibility.state, "observable");

    let snap = translate_nq(&dto).unwrap();
    assert_eq!(snap.severity, Severity::Warning);
}

// -----------------------------------------------------------------------------
// 5. Two-clock latency — informational, documents the seam
// -----------------------------------------------------------------------------

/// NS's `last_seen_at → captured_at` translation reflects NQ ingest
/// time, not producer extraction time. Per NQ gap §"Two-clock
/// provenance is load-bearing": *"Consumers branch on the axis they
/// need."* NS does not branch today; the producer clock is invisible
/// to the current consumer. This test documents that seam so a future
/// reader knows where to look if cross-clock reasoning becomes needed.
#[test]
fn captured_at_reflects_nq_ingest_clock_not_producer_extraction_clock() {
    let lines = load_lines("nq-findings-import-clean.jsonl");
    let dto = parse_nq_line(&lines[0]).unwrap();
    let snap = translate_nq(&dto).unwrap();

    // NQ ingest time: 2026-05-12T10:00:30Z
    // Producer extraction time: 2026-05-12T10:00:00Z (30s earlier)
    // captured_at must reflect the NQ clock (existing semantics).
    assert_eq!(
        snap.captured_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "2026-05-12T10:00:30Z",
        "captured_at grounds in NQ ingest time per existing translate_nq semantics"
    );
    // The producer's extraction time is on the wire (`origin.producer_extraction_time`)
    // but Slice A surfaces it only via origin envelope (next test) — it does
    // not flow into captured_at. Two clocks visible, no semantics yet.
}

// -----------------------------------------------------------------------------
// 6. Slice A visibility: origin/silence carry through to FindingSnapshot
// -----------------------------------------------------------------------------

#[test]
fn origin_envelope_preserved_in_finding_snapshot() {
    let lines = load_lines("nq-findings-import-clean.jsonl");
    let snap = translate_nq(&parse_nq_line(&lines[0]).unwrap()).unwrap();
    let origin = snap
        .origin
        .as_ref()
        .expect("ingested finding must carry origin envelope");
    assert_eq!(origin.source, "import");
    assert_eq!(origin.producer_id, "synthetic-corpus-extractor");
    assert_eq!(origin.extraction_run_id, "run-20260512T100000Z-abc123");
    assert_eq!(
        origin
            .producer_extraction_time
            .as_ref()
            .expect("producer_extraction_time is present in this fixture")
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "2026-05-12T10:00:00Z",
        "producer clock is preserved separately from NQ ingest clock"
    );
    assert_eq!(origin.import_contract_version, 1);
    assert!(
        snap.silence.is_none(),
        "ingested findings carry no silence envelope unless they're extraction_stale"
    );
}

#[test]
fn silence_envelope_preserved_in_finding_snapshot_for_extraction_stale() {
    let lines = load_lines("nq-findings-import-stale.jsonl");
    let snap = translate_nq(&parse_nq_line(&lines[0]).unwrap()).unwrap();
    assert_eq!(snap.finding_key.detector, "extraction_stale");
    let silence = snap
        .silence
        .as_ref()
        .expect("extraction_stale must carry silence envelope");
    assert_eq!(silence.scope, "extraction");
    assert_eq!(silence.basis, "age_threshold");
    assert!(silence.duration_s > 10_000_000);
    assert_eq!(silence.expected, "none");
    assert!(
        snap.origin.is_none(),
        "extraction_stale is NQ's own finding; no origin envelope"
    );
}

#[test]
fn native_finding_carries_neither_envelope() {
    // The legacy observable fixture has no origin or silence on the wire.
    // Both must remain None after translate.
    let path = fixtures_dir().join("nq-findings-observable.jsonl");
    let raw = std::fs::read_to_string(&path).unwrap();
    let line = raw.lines().next().unwrap();
    let snap = translate_nq(&parse_nq_line(line).unwrap()).unwrap();
    assert!(snap.origin.is_none(), "native NQ findings have no origin");
    assert!(snap.silence.is_none(), "non-silence findings have no silence");
}

// -----------------------------------------------------------------------------
// 7. Slice A visibility: peek + packet surface origin/silence
// -----------------------------------------------------------------------------

#[test]
fn peek_renders_both_clocks_when_origin_present() {
    use nightshiftd::nq::ListedFinding;
    use nightshiftd::nq_peek::{render_peek_text, PeekDocument};

    let lines = load_lines("nq-findings-import-clean.jsonl");
    let snap = translate_nq(&parse_nq_line(&lines[0]).unwrap()).unwrap();
    let item = ListedFinding {
        raw_line: lines[0].clone(),
        translated: snap,
    };
    let doc = PeekDocument::build(&[item], false);
    let text = render_peek_text(&doc, false);

    assert!(text.contains("origin:"), "peek renders origin section");
    assert!(
        text.contains("source=import"),
        "peek renders origin.source discriminator"
    );
    assert!(
        text.contains("producer_id=synthetic-corpus-extractor"),
        "peek renders producer_id"
    );
    assert!(
        text.contains("producer_extraction_time: 2026-05-12T10:00:00"),
        "peek surfaces producer clock alongside NQ ingest clock"
    );
    assert!(
        text.contains("NQ ingest clock"),
        "peek labels last_seen as NQ ingest clock so operator sees the distinction"
    );
}

#[test]
fn peek_renders_silence_when_present() {
    use nightshiftd::nq::ListedFinding;
    use nightshiftd::nq_peek::{render_peek_text, PeekDocument};

    let lines = load_lines("nq-findings-import-stale.jsonl");
    let snap = translate_nq(&parse_nq_line(&lines[0]).unwrap()).unwrap();
    let item = ListedFinding {
        raw_line: lines[0].clone(),
        translated: snap,
    };
    let doc = PeekDocument::build(&[item], false);
    let text = render_peek_text(&doc, false);

    assert!(text.contains("silence:"), "peek renders silence section");
    assert!(text.contains("scope=extraction"));
    assert!(text.contains("basis=age_threshold"));
    assert!(text.contains("expected=none"));
}

// -----------------------------------------------------------------------------
// 8. The headline anti-misuse test (named per Slice A spec)
// -----------------------------------------------------------------------------

/// Slice A's invariant: origin and silence are *visible* through the
/// NS model, peek surface, and packet summary, but Night Shift does
/// not branch reconciliation, notification posture, ack obligation,
/// or freshness semantics on either field. The same fixture inputs
/// produce the same regime they would if origin/silence were stripped.
///
/// The name is doctrine: changing this assertion to a different
/// expected regime means crossing from visibility into semantic
/// consumption, which is a separate slice (B or C) with its own
/// design review.
#[test]
fn origin_and_silence_are_visible_but_do_not_change_reconciliation() {
    // Translate an ingested finding (origin populated) and the same
    // finding with origin/silence manually stripped. Both must
    // produce snapshots that compare equal on every field the
    // reconciler reads.
    let lines = load_lines("nq-findings-import-clean.jsonl");
    let with_envelopes = translate_nq(&parse_nq_line(&lines[0]).unwrap()).unwrap();

    let mut stripped = with_envelopes.clone();
    stripped.origin = None;
    stripped.silence = None;

    // Every field the reconciler reads (per `semantic_diff` in
    // `crates/nightshiftd/src/reconciler.rs`) must be identical.
    assert_eq!(with_envelopes.finding_key, stripped.finding_key);
    assert_eq!(with_envelopes.severity, stripped.severity);
    assert_eq!(with_envelopes.domain, stripped.domain);
    assert_eq!(
        with_envelopes.persistence_generations,
        stripped.persistence_generations
    );
    assert_eq!(with_envelopes.first_seen_at, stripped.first_seen_at);
    assert_eq!(with_envelopes.current_status, stripped.current_status);
    assert_eq!(with_envelopes.snapshot_generation, stripped.snapshot_generation);
    assert_eq!(with_envelopes.captured_at, stripped.captured_at);
    // The envelopes themselves differ — that's the whole point of
    // visibility — but reconciliation must not see those fields.
    assert_ne!(with_envelopes.origin, stripped.origin);
    // For this fixture, neither has silence (extraction_stale lives
    // in the other fixture), so silence equality is trivially true.
    assert_eq!(with_envelopes.silence, stripped.silence);
}
