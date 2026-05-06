//! `nightshift liveness peek` — operator inspection of the same
//! liveness DTO the pipeline gate consumes.
//!
//! Answers two questions the operator may want to ask without running
//! a full agenda:
//!
//! 1. *What does NQ say?* — the raw `LivenessSnapshot` from
//!    `nq liveness export`, including its own `freshness.fresh` flag.
//! 2. *What does Night Shift see?* — the verdict computed by
//!    `liveness::verdict_for`, the threshold that produced it, and
//!    whether the gate would let a run proceed.
//!
//! Both views ship in every peek output. The peek command exists
//! because the wrinkle in `liveness::verdict_for` (negative `age_seconds`
//! → `Skewed`, even when upstream says `fresh: true`) means the two
//! views can disagree — and the operator must be able to see the
//! disagreement directly.
//!
//! This module owns rendering only. The verdict computation and DTO
//! parsing live in `crate::liveness`.

use serde::Serialize;

use crate::liveness::{verdict_for, LivenessSnapshot, LivenessVerdict};

pub const LIVENESS_PEEK_SCHEMA: &str = "nightshift.liveness_peek.v1";

/// Where the threshold used by the verdict came from. Surfaced so the
/// operator sees whether their `--nq-liveness-threshold-secs` was
/// honored or whether the Night Shift default applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdSource {
    /// Threshold was passed in via `--nq-liveness-threshold-secs`.
    Operator,
    /// Threshold defaulted to `liveness::DEFAULT_STALENESS_THRESHOLD_SECONDS`.
    Default,
}

impl ThresholdSource {
    fn as_str(self) -> &'static str {
        match self {
            ThresholdSource::Operator => "operator",
            ThresholdSource::Default => "default",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LivenessPeekDocument {
    pub schema: &'static str,
    /// The unmodified `LivenessSnapshot` returned by NQ, including
    /// `freshness.fresh` (Night Shift does NOT consume that field
    /// directly — see `crate::liveness` module docs).
    pub snapshot: LivenessSnapshot,
    /// Night Shift's own derived view: verdict, threshold applied,
    /// and gate disposition.
    pub nightshift_view: NightshiftView,
}

#[derive(Debug, Serialize)]
pub struct NightshiftView {
    /// One of "fresh", "stale", "skewed". Lower-case for
    /// pipeline-friendly grep/jq.
    pub verdict: String,
    pub verdict_explain: String,
    /// Echoed from the snapshot for convenience — operators inspecting
    /// a peek should not need to grep for it elsewhere.
    pub age_seconds: i64,
    pub threshold_seconds_used: u64,
    /// "operator" or "default" — see [`ThresholdSource`].
    pub threshold_source: &'static str,
    /// True iff `verdict == fresh`. The pipeline gate halts on Stale
    /// and on Skewed; both surface a Stale-shape revalidate-only
    /// packet.
    pub would_pass_gate: bool,
}

impl LivenessPeekDocument {
    pub fn build(
        snapshot: LivenessSnapshot,
        threshold_seconds: u64,
        threshold_source: ThresholdSource,
    ) -> Self {
        let verdict = verdict_for(&snapshot, threshold_seconds);
        let nightshift_view = NightshiftView {
            verdict: verdict_label(&verdict).into(),
            verdict_explain: verdict.explain(),
            age_seconds: snapshot.freshness.age_seconds,
            threshold_seconds_used: threshold_seconds,
            threshold_source: threshold_source.as_str(),
            would_pass_gate: verdict.is_fresh(),
        };
        LivenessPeekDocument {
            schema: LIVENESS_PEEK_SCHEMA,
            snapshot,
            nightshift_view,
        }
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("LivenessPeekDocument is always serializable")
    }
}

fn verdict_label(v: &LivenessVerdict) -> &'static str {
    match v {
        LivenessVerdict::Fresh => "fresh",
        LivenessVerdict::Stale { .. } => "stale",
        LivenessVerdict::Skewed { .. } => "skewed",
    }
}

/// Plain-text rendering for an operator at a terminal. Always shows
/// the upstream `freshness.fresh` value side-by-side with Night
/// Shift's verdict — the cross-check is the whole point of peek.
pub fn render_peek_text(doc: &LivenessPeekDocument) -> String {
    let snap = &doc.snapshot;
    let view = &doc.nightshift_view;

    let mut out = String::new();
    out.push_str(&format!("schema: {}\n", doc.schema));
    out.push('\n');

    out.push_str("witness:\n");
    out.push_str(&format!("  instance:        {}\n", snap.instance_id));
    out.push_str(&format!(
        "  generation_id:   {}\n",
        snap.witness.generation_id
    ));
    out.push_str(&format!(
        "  generated_at:    {}\n",
        snap.witness.generated_at.to_rfc3339()
    ));
    out.push_str(&format!("  status:          {}\n", snap.witness.status));
    out.push_str(&format!(
        "  findings:        {} observed, {} suppressed\n",
        snap.witness.findings_observed, snap.witness.findings_suppressed
    ));
    out.push_str(&format!(
        "  detectors_run:   {}\n",
        snap.witness.detectors_run
    ));
    out.push('\n');

    out.push_str("freshness:\n");
    out.push_str(&format!(
        "  age_seconds:     {}\n",
        snap.freshness.age_seconds
    ));
    out.push_str(&format!(
        "  threshold_secs:  {} ({})\n",
        view.threshold_seconds_used, view.threshold_source
    ));
    let upstream = match snap.freshness.fresh {
        Some(b) => b.to_string(),
        None => "null".to_string(),
    };
    out.push_str(&format!(
        "  upstream_fresh:  {upstream}  (NQ's own flag — Night Shift does NOT consume directly)\n"
    ));
    out.push('\n');

    out.push_str("nightshift verdict:\n");
    out.push_str(&format!("  verdict:         {}\n", view.verdict));
    out.push_str(&format!("  explain:         {}\n", view.verdict_explain));
    out.push_str(&format!("  would_pass_gate: {}\n", view.would_pass_gate));
    out.push('\n');

    out.push_str("source:\n");
    out.push_str(&format!(
        "  artifact_path:   {}\n",
        snap.source.artifact_path
    ));
    out.push_str(&format!(
        "  artifact_kind:   {}\n",
        snap.source.artifact_kind
    ));
    out.push_str(&format!(
        "  exported_at:     {}\n",
        snap.export.exported_at.to_rfc3339()
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liveness::{parse_snapshot, DEFAULT_STALENESS_THRESHOLD_SECONDS};

    fn fresh_dto_json() -> &'static str {
        r#"{
            "schema": "nq.liveness_snapshot.v1",
            "contract_version": 1,
            "instance_id": "labelwatch-host",
            "witness": {
                "generation_id": 43755,
                "generated_at": "2026-04-20T17:38:17Z",
                "schema_version": 29,
                "status": "ok",
                "findings_observed": 9,
                "findings_suppressed": 0,
                "detectors_run": 3,
                "liveness_format_version": 1
            },
            "freshness": {
                "age_seconds": 25,
                "stale_threshold_seconds": null,
                "fresh": null
            },
            "source": {
                "artifact_path": "/opt/notquery/liveness.json",
                "artifact_kind": "file"
            },
            "export": {
                "exported_at": "2026-04-20T17:38:42Z",
                "source": "nq",
                "contract_version": 1
            }
        }"#
    }

    fn stale_dto_json() -> &'static str {
        r#"{
            "schema": "nq.liveness_snapshot.v1",
            "contract_version": 1,
            "instance_id": "labelwatch-host",
            "witness": {
                "generation_id": 43000,
                "generated_at": "2026-04-20T17:30:00Z",
                "schema_version": 29,
                "status": "ok",
                "findings_observed": 9,
                "findings_suppressed": 0,
                "detectors_run": 3,
                "liveness_format_version": 1
            },
            "freshness": {
                "age_seconds": 600,
                "stale_threshold_seconds": null,
                "fresh": false
            },
            "source": {
                "artifact_path": "/opt/notquery/liveness.json",
                "artifact_kind": "file"
            },
            "export": {
                "exported_at": "2026-04-20T17:40:00Z",
                "source": "nq",
                "contract_version": 1
            }
        }"#
    }

    fn skewed_dto_json() -> &'static str {
        r#"{
            "schema": "nq.liveness_snapshot.v1",
            "contract_version": 1,
            "instance_id": "labelwatch-host",
            "witness": {
                "generation_id": 99999,
                "generated_at": "2030-01-01T00:00:00Z",
                "schema_version": 29,
                "status": "ok",
                "findings_observed": 0,
                "findings_suppressed": 0,
                "detectors_run": 3,
                "liveness_format_version": 1
            },
            "freshness": {
                "age_seconds": -116750929,
                "stale_threshold_seconds": 60,
                "fresh": true
            },
            "source": {
                "artifact_path": "/opt/notquery/liveness.json",
                "artifact_kind": "file"
            },
            "export": {
                "exported_at": "2026-04-20T17:38:42Z",
                "source": "nq",
                "contract_version": 1
            }
        }"#
    }

    #[test]
    fn fresh_snapshot_yields_would_pass_gate_true() {
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(snap, 90, ThresholdSource::Default);
        assert_eq!(doc.nightshift_view.verdict, "fresh");
        assert!(doc.nightshift_view.would_pass_gate);
        assert_eq!(doc.nightshift_view.age_seconds, 25);
        assert_eq!(doc.nightshift_view.threshold_seconds_used, 90);
        assert_eq!(doc.nightshift_view.threshold_source, "default");
    }

    #[test]
    fn stale_snapshot_yields_would_pass_gate_false() {
        let snap = parse_snapshot(stale_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(snap, 90, ThresholdSource::Default);
        assert_eq!(doc.nightshift_view.verdict, "stale");
        assert!(!doc.nightshift_view.would_pass_gate);
        assert_eq!(doc.nightshift_view.age_seconds, 600);
    }

    /// **Wrinkle test for the peek surface.** Even when the upstream
    /// snapshot reports `freshness.fresh: true`, a negative
    /// `age_seconds` must produce `verdict: skewed` and
    /// `would_pass_gate: false`. The peek surface must NOT silently
    /// adopt the upstream's flag.
    #[test]
    fn skewed_snapshot_does_not_inherit_upstream_fresh() {
        let snap = parse_snapshot(skewed_dto_json()).unwrap();
        assert_eq!(
            snap.freshness.fresh,
            Some(true),
            "test precondition: upstream says fresh"
        );
        assert!(snap.freshness.age_seconds < 0, "test precondition");

        let doc = LivenessPeekDocument::build(snap, 60, ThresholdSource::Operator);
        assert_eq!(doc.nightshift_view.verdict, "skewed");
        assert!(!doc.nightshift_view.would_pass_gate);
        assert!(doc.nightshift_view.verdict_explain.contains("clock skew"));
        assert_eq!(doc.nightshift_view.threshold_source, "operator");
    }

    #[test]
    fn document_carries_schema_tag() {
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(snap, 90, ThresholdSource::Default);
        assert_eq!(doc.schema, LIVENESS_PEEK_SCHEMA);
        let json = doc.to_json_pretty();
        assert!(json.contains("\"schema\""));
        assert!(json.contains("nightshift.liveness_peek.v1"));
    }

    #[test]
    fn json_output_is_deterministic_across_calls() {
        // Same input → byte-identical output. Operators using
        // `nightshift liveness peek --format json | diff` need this.
        let snap_a = parse_snapshot(fresh_dto_json()).unwrap();
        let snap_b = parse_snapshot(fresh_dto_json()).unwrap();
        let a = LivenessPeekDocument::build(snap_a, 90, ThresholdSource::Default).to_json_pretty();
        let b = LivenessPeekDocument::build(snap_b, 90, ThresholdSource::Default).to_json_pretty();
        assert_eq!(a, b);
    }

    /// The text rendering must surface the cross-check directly: the
    /// upstream `fresh` value AND Night Shift's verdict on the same
    /// page, with a label that the operator does not have to decode.
    #[test]
    fn text_render_shows_upstream_and_nightshift_side_by_side_for_skewed() {
        let snap = parse_snapshot(skewed_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(snap, 60, ThresholdSource::Operator);
        let text = render_peek_text(&doc);

        // Upstream's flag, with the explicit annotation that Night
        // Shift does not consume it directly.
        assert!(text.contains("upstream_fresh:  true"), "got:\n{text}");
        assert!(
            text.contains("Night Shift does NOT consume directly"),
            "got:\n{text}"
        );

        // Night Shift's own verdict, divergent from the upstream flag.
        assert!(text.contains("verdict:         skewed"), "got:\n{text}");
        assert!(text.contains("would_pass_gate: false"), "got:\n{text}");

        // Threshold source surfaced.
        assert!(text.contains("(operator)"), "got:\n{text}");
    }

    #[test]
    fn text_render_shows_witness_identity_for_cross_check() {
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(snap, 90, ThresholdSource::Default);
        let text = render_peek_text(&doc);

        // Operator must be able to grep for instance, generation_id,
        // status, findings counts, and artifact path.
        assert!(text.contains("labelwatch-host"));
        assert!(text.contains("43755"));
        assert!(text.contains("status:          ok"));
        assert!(text.contains("9 observed, 0 suppressed"));
        assert!(text.contains("/opt/notquery/liveness.json"));
        assert!(text.contains("(default)"));
    }

    /// Sanity: the peek surface uses the same default threshold the
    /// pipeline gate uses. Drift between the two would mean a peek
    /// could say "would_pass_gate: true" while the gate would actually
    /// halt (or vice versa).
    #[test]
    fn default_threshold_matches_pipeline_gate_default() {
        // The peek caller (main.rs) is responsible for passing the
        // default through; the constant lives in `liveness`. This
        // test pins the contract by exercising it.
        let snap = parse_snapshot(fresh_dto_json()).unwrap();
        let doc = LivenessPeekDocument::build(
            snap,
            DEFAULT_STALENESS_THRESHOLD_SECONDS,
            ThresholdSource::Default,
        );
        assert_eq!(
            doc.nightshift_view.threshold_seconds_used,
            DEFAULT_STALENESS_THRESHOLD_SECONDS
        );
    }
}
