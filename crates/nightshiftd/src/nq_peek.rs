//! `nightshift nq peek` — translation-only inspection of a live NQ DB.
//!
//! Answers the baseline question: *does Night Shift read the same
//! world the CLI reads?* This module owns rendering only — query
//! semantics live in `crate::nq::CliNqSource::list_findings`.
//!
//! Output discipline (per chatty's slice-4 constraints):
//! - JSON is deterministic: findings sorted by translated finding_key,
//!   field order fixed by struct layout, no run-specific timestamps.
//! - `--show-raw` is opt-in; default shows only Night Shift's
//!   translated view (the thing the reconciler actually consumes).
//! - Identity fields are always present so the operator can
//!   cross-check against `nq findings export` directly.

use serde::Serialize;

use crate::finding::{EvidenceState, Severity};
use crate::nq::ListedFinding;

pub const PEEK_SCHEMA: &str = "nightshift.nq_peek.v1";

#[derive(Debug, Serialize)]
pub struct PeekDocument {
    pub schema: &'static str,
    pub findings: Vec<PeekFinding>,
}

#[derive(Debug, Serialize)]
pub struct PeekFinding {
    /// Night Shift's canonical finding_key for this finding.
    /// Format: `<source>:<detector>:<host>:<subject>` per
    /// `FindingKey::as_string()`.
    pub finding_key: String,
    pub translated: TranslatedView,
    /// Present only when `--show-raw` is set. Carries NQ's full
    /// JSONL output verbatim (parsed back from string into a
    /// `serde_json::Value` so it nests inside the JSON document).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

/// The Night-Shift-internal view of a finding, in stable shape for
/// diffing across peek runs. Everything here is deterministic for a
/// fixed NQ state — no captured-at-now timestamps, no run_ids, no
/// per-invocation salt.
#[derive(Debug, Serialize)]
pub struct TranslatedView {
    pub source: String,
    pub detector: String,
    pub host: String,
    pub subject: String,
    pub severity: Severity,
    pub evidence_state: EvidenceState,
    pub persistence_generations: u32,
    pub snapshot_generation: u64,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub evidence_hash: String,
}

impl PeekDocument {
    /// Build a deterministic document from a list of `ListedFinding`s.
    /// Findings are sorted by `finding_key` so JSON diffs across runs
    /// reflect actual world deltas, not iteration order.
    pub fn build(items: &[ListedFinding], show_raw: bool) -> Self {
        let mut findings: Vec<PeekFinding> = items
            .iter()
            .map(|item| {
                let snap = &item.translated;
                let translated = TranslatedView {
                    source: snap.finding_key.source.clone(),
                    detector: snap.finding_key.detector.clone(),
                    host: snap.host.clone(),
                    // FindingKey.subject embeds "<host>:<subject>" by
                    // Night Shift convention; render the actual NQ
                    // subject by stripping the leading host segment.
                    subject: snap
                        .finding_key
                        .subject
                        .split_once(':')
                        .map(|(_h, s)| s.to_string())
                        .unwrap_or_else(|| snap.finding_key.subject.clone()),
                    severity: snap.severity,
                    evidence_state: snap.current_status,
                    persistence_generations: snap.persistence_generations,
                    snapshot_generation: snap.snapshot_generation,
                    first_seen_at: snap.first_seen_at.to_rfc3339(),
                    last_seen_at: snap.captured_at.to_rfc3339(),
                    evidence_hash: snap.evidence_hash.clone(),
                };
                let raw = if show_raw {
                    serde_json::from_str::<serde_json::Value>(&item.raw_line).ok()
                } else {
                    None
                };
                PeekFinding {
                    finding_key: snap.finding_key.as_string(),
                    translated,
                    raw,
                }
            })
            .collect();
        findings.sort_by(|a, b| a.finding_key.cmp(&b.finding_key));
        PeekDocument {
            schema: PEEK_SCHEMA,
            findings,
        }
    }

    pub fn to_json_pretty(&self) -> String {
        // Pretty for human reading + diffing. serde_json::to_string_pretty
        // is deterministic for our struct shape.
        serde_json::to_string_pretty(self).expect("PeekDocument is always serializable")
    }
}

/// Plain-text rendering for the operator at a terminal. The text
/// shape is human-friendly; for repeated diffs use `--format json`.
pub fn render_peek_text(doc: &PeekDocument, show_raw: bool) -> String {
    if doc.findings.is_empty() {
        return "(no findings match)\n".into();
    }
    let mut out = String::new();
    out.push_str(&format!("schema: {}\n", doc.schema));
    out.push_str(&format!("findings: {}\n", doc.findings.len()));
    out.push('\n');
    for f in &doc.findings {
        out.push_str(&f.finding_key);
        out.push('\n');
        let t = &f.translated;
        out.push_str(&format!(
            "  detector: {}   host: {}   subject: {}\n",
            t.detector, t.host, t.subject
        ));
        out.push_str(&format!(
            "  severity: {:?}   evidence_state: {:?}\n",
            t.severity, t.evidence_state
        ));
        out.push_str(&format!(
            "  generations: {}   snapshot_gen: {}\n",
            t.persistence_generations, t.snapshot_generation
        ));
        out.push_str(&format!(
            "  first_seen: {}   last_seen: {}\n",
            t.first_seen_at, t.last_seen_at
        ));
        out.push_str(&format!("  evidence_hash: {}\n", t.evidence_hash));
        if show_raw {
            if let Some(raw) = &f.raw {
                out.push_str("  raw (NQ): ");
                out.push_str(&serde_json::to_string(raw).unwrap_or_else(|_| "?".into()));
                out.push('\n');
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{FindingKey, FindingSnapshot};
    use crate::nq::ListedFinding;
    use chrono::{TimeZone, Utc};

    fn mk_listed(detector: &str, host: &str, subject: &str, generation: u64) -> ListedFinding {
        let snap = FindingSnapshot {
            finding_key: FindingKey {
                source: "nq".into(),
                detector: detector.into(),
                subject: format!("{host}:{subject}"),
            },
            host: host.into(),
            severity: Severity::Warning,
            domain: None,
            persistence_generations: 3,
            first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
            current_status: EvidenceState::Active,
            snapshot_generation: generation,
            captured_at: Utc.with_ymd_and_hms(2026, 4, 17, 0, 0, 0).unwrap(),
            evidence_hash: format!("sha256:{generation:08x}"),
        };
        let raw_line = format!(
            r#"{{"schema":"nq.finding_snapshot.v1","contract_version":1,"finding_key":"local/{host}/{detector}/{subject}","placeholder":true}}"#
        );
        ListedFinding {
            raw_line,
            translated: snap,
        }
    }

    #[test]
    fn empty_input_renders_no_findings_line() {
        let doc = PeekDocument::build(&[], false);
        assert!(doc.findings.is_empty());
        let text = render_peek_text(&doc, false);
        assert!(text.contains("(no findings match)"));
    }

    #[test]
    fn findings_are_sorted_by_finding_key_for_diff_stability() {
        let items = vec![
            mk_listed("z_detector", "host-a", "/p1", 1),
            mk_listed("a_detector", "host-b", "/p2", 2),
            mk_listed("a_detector", "host-a", "/p1", 3),
        ];
        let doc = PeekDocument::build(&items, false);
        let keys: Vec<&str> = doc.findings.iter().map(|f| f.finding_key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "findings must be sorted by finding_key");
    }

    #[test]
    fn translated_view_strips_host_prefix_from_subject() {
        let items = vec![mk_listed(
            "wal_bloat",
            "labelwatch-host",
            "/var/lib/db",
            42,
        )];
        let doc = PeekDocument::build(&items, false);
        assert_eq!(doc.findings[0].translated.subject, "/var/lib/db");
        assert_eq!(doc.findings[0].translated.host, "labelwatch-host");
    }

    #[test]
    fn show_raw_includes_nq_payload_when_requested() {
        let items = vec![mk_listed("d", "h", "s", 1)];
        let with_raw = PeekDocument::build(&items, true);
        assert!(with_raw.findings[0].raw.is_some());
        let without = PeekDocument::build(&items, false);
        assert!(without.findings[0].raw.is_none());
    }

    #[test]
    fn json_output_is_deterministic_across_calls() {
        // Same input → byte-identical output. This is what makes
        // diff-across-runs meaningful for the baseline stability watch.
        let items = vec![mk_listed("d1", "h1", "s1", 1), mk_listed("d2", "h2", "s2", 2)];
        let a = PeekDocument::build(&items, true).to_json_pretty();
        let b = PeekDocument::build(&items, true).to_json_pretty();
        assert_eq!(a, b);
    }

    #[test]
    fn json_output_carries_schema_tag() {
        let doc = PeekDocument::build(&[], false);
        let json = doc.to_json_pretty();
        assert!(json.contains("\"schema\""));
        assert!(json.contains("nightshift.nq_peek.v1"));
    }

    #[test]
    fn text_render_includes_identity_for_cross_check() {
        let items = vec![mk_listed("wal_bloat", "labelwatch-host", "/var/lib/db", 42)];
        let doc = PeekDocument::build(&items, false);
        let text = render_peek_text(&doc, false);
        // Operator must be able to grep for finding_key, detector,
        // host, subject, severity, and evidence_state.
        assert!(text.contains("nq:wal_bloat:labelwatch-host:/var/lib/db"));
        assert!(text.contains("wal_bloat"));
        assert!(text.contains("labelwatch-host"));
        assert!(text.contains("/var/lib/db"));
        assert!(text.contains("Warning"));
        assert!(text.contains("Active"));
    }

    #[test]
    fn text_render_omits_raw_unless_requested() {
        let items = vec![mk_listed("d", "h", "/s", 1)];
        let plain = render_peek_text(&PeekDocument::build(&items, false), false);
        assert!(!plain.contains("raw (NQ):"));
        let with_raw = render_peek_text(&PeekDocument::build(&items, true), true);
        assert!(with_raw.contains("raw (NQ):"));
    }
}
