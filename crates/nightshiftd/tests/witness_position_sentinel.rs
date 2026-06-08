//! Witness-position absence sentinel test.
//!
//! Pins the NS-side render discipline for NQ's `WitnessPosition`
//! (`nq-witness-api`, ad26dc4 2026-06-08). NS adopts NQ canonical
//! taxonomy exactly — three values: `substrate`,
//! `application_internal`, `platform`. The render surface shows the
//! lane when NQ stamps it. When NQ does not stamp it, the render
//! says **absent** in operator prose, NOT an inferred lane.
//!
//! ## What this test guards
//!
//! The forbidden inference: `position.is_none() &&
//! witness_type/detector == "<x>" => inferred_position`. NS-side
//! per-detector lookup tables, NS-side classification heuristics, or
//! any other shape that would silently default `None` to a taxonomy
//! value at the render boundary. The render output for an absent
//! position must say so explicitly.
//!
//! Test fixture deliberately picks `git_status` as the detector —
//! NQ's own production code classifies the `git_status` witness as
//! `Platform`
//! (`crates/nq-monitor/src/cmd/witness/git_status.rs:60`). A naive
//! reverse-engineering implementation would map "detector starts
//! with git" or "detector is in {git_status, diff_scope}" to
//! `platform` and silently render the lane. This test refuses
//! that path: even with the temptation-shaped detector, the render
//! must say `not testified` when `position == None`.
//!
//! ## How updating this test signals doctrine has moved
//!
//! Same sentinel shape as
//! `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`
//! and `forbidden_cycle_structural_absence_sentinel_ns_does_not_write_to_nq`:
//! the test is the doctrine surface, not a regression guard. If a
//! future NS slice intentionally adds detector-to-position
//! inference (the rule that says "absence + witness_type X →
//! position Y"), updating this test deliberately is the signal
//! that the no-inference invariant has moved. Until that, the test
//! must refuse the temptation.
//!
//! ## Why "not testified" is operator prose, not a taxonomy value
//!
//! `WitnessPosition` carries three variants. `None` is the absence
//! state, surfaced for operator visibility as the string
//! `"not testified"`. That string is render prose only — it must
//! not be added as a fourth enum variant, must not appear in any
//! match arm that branches semantics, and must not be parsed back
//! from the wire as a position value. The render is a one-way
//! projection: `Option<WitnessPosition> → display string`. Going
//! the other way would coin a synonym NQ does not ship.

use chrono::{TimeZone, Utc};
use nightshiftd::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity, WitnessPosition};
use nightshiftd::nq::ListedFinding;
use nightshiftd::nq_peek::{render_peek_text, PeekDocument};

fn snapshot_without_position(detector: &str) -> FindingSnapshot {
    FindingSnapshot {
        finding_key: FindingKey {
            source: "nq".into(),
            detector: detector.into(),
            subject: "tempting-host:/tmp/example".into(),
        },
        host: "tempting-host".into(),
        severity: Severity::Warning,
        domain: None,
        persistence_generations: 1,
        first_seen_at: Utc.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap(),
        current_status: EvidenceState::Active,
        snapshot_generation: 1,
        captured_at: Utc.with_ymd_and_hms(2026, 6, 8, 0, 0, 0).unwrap(),
        evidence_hash: "sha256:00000000".into(),
        origin: None,
        silence: None,
        // The whole point of the sentinel: position is None, and
        // the detector below is named such that an inference path
        // would map it to a real lane.
        position: None,
    }
}

fn snapshot_with_position(
    detector: &str,
    position: WitnessPosition,
) -> FindingSnapshot {
    FindingSnapshot {
        position: Some(position),
        ..snapshot_without_position(detector)
    }
}

fn render_for(snap: FindingSnapshot) -> String {
    let item = ListedFinding {
        raw_line: "{}".to_string(),
        translated: snap,
    };
    let doc = PeekDocument::build(&[item], false);
    render_peek_text(&doc, false)
}

#[test]
fn position_absence_sentinel_witness_type_does_not_infer_position() {
    // Detector picked to tempt inference: NQ's own
    // `git_status` witness is classified `Platform` in
    // `crates/nq-monitor/src/cmd/witness/git_status.rs`. An NS-side
    // synonym table would render `position: platform` here. The
    // sentinel refuses.
    let snap = snapshot_without_position("git_status");
    let out = render_for(snap);

    // Must explicitly say absence.
    assert!(
        out.contains("position: not testified"),
        "render must say `position: not testified` when position is None — \
         got:\n{out}"
    );

    // Must NOT have inferred any taxonomy value. None of the three
    // canonical tokens may appear on a `position:` line.
    for token in ["substrate", "application_internal", "platform"] {
        let needle = format!("position: {token}");
        assert!(
            !out.contains(&needle),
            "render leaked inferred position `{token}` despite \
             position == None — got:\n{out}"
        );
    }
}

#[test]
fn position_absence_sentinel_holds_for_every_tempting_detector() {
    // Walk every NQ-side detector name that NQ's own code maps to
    // a canonical position. If the rendered output reverse-engineers
    // any of these to a lane, the no-inference invariant has
    // collapsed. The list is sourced from NQ's
    // `nq-monitor/src/cmd/witness/*.rs` and
    // `nq-db/src/*_witness_projection.rs` production classifications
    // (commit message `ad26dc4`):
    //
    // - dns_resolver_legacy_projection             Substrate
    // - disk_state legacy projections              Substrate
    // - zfs / smart / disk_pressure                Substrate
    // - ingest_generation_legacy_projection        ApplicationInternal
    // - ingest_source_legacy_projection            ApplicationInternal
    // - sqlite_wal_legacy_projection               ApplicationInternal
    // - pytest                                     ApplicationInternal
    // - git_status                                 Platform
    // - diff_scope                                 Platform
    let tempting = [
        "dns_resolver_legacy_projection",
        "zfs",
        "smart",
        "disk_pressure",
        "ingest_generation_legacy_projection",
        "ingest_source_legacy_projection",
        "sqlite_wal_legacy_projection",
        "pytest",
        "git_status",
        "diff_scope",
    ];

    for detector in tempting {
        let snap = snapshot_without_position(detector);
        let out = render_for(snap);
        assert!(
            out.contains("position: not testified"),
            "detector `{detector}` rendered without absence label — got:\n{out}"
        );
        for token in ["substrate", "application_internal", "platform"] {
            let needle = format!("position: {token}");
            assert!(
                !out.contains(&needle),
                "detector `{detector}` leaked inferred position `{token}` — got:\n{out}"
            );
        }
    }
}

#[test]
fn position_present_renders_the_lane_verbatim() {
    // Positive side. When NQ does stamp the lane, the render shows
    // it verbatim using the NQ wire token (snake_case canonical
    // taxonomy). Renaming this string requires renaming NQ.
    let cases = [
        (WitnessPosition::Substrate, "position: substrate"),
        (
            WitnessPosition::ApplicationInternal,
            "position: application_internal",
        ),
        (WitnessPosition::Platform, "position: platform"),
    ];
    for (pos, expected) in cases {
        let snap = snapshot_with_position("zfs", pos);
        let out = render_for(snap);
        assert!(
            out.contains(expected),
            "expected `{expected}` in render — got:\n{out}"
        );
        // And the absence label must NOT appear.
        assert!(
            !out.contains("position: not testified"),
            "render showed absence label even though position == {pos:?} — got:\n{out}"
        );
    }
}

#[test]
fn legacy_finding_snapshot_without_position_field_parses_to_none() {
    // Acceptance criterion: "existing fixtures still parse". A
    // FindingSnapshot JSON without the `position` field must
    // deserialize successfully with `position == None`. This guards
    // the forward-compat invariant on the wire — NQ may add the
    // field later, but existing packets must continue to parse.
    let legacy_json = r#"{
        "finding_key": {
            "source": "nq",
            "detector": "wal_bloat",
            "subject": "host:/path"
        },
        "host": "host",
        "severity": "warning",
        "persistence_generations": 1,
        "first_seen_at": "2026-06-08T00:00:00Z",
        "current_status": "active",
        "snapshot_generation": 1,
        "captured_at": "2026-06-08T00:00:00Z",
        "evidence_hash": "sha256:00"
    }"#;

    let snap: FindingSnapshot = serde_json::from_str(legacy_json)
        .expect("legacy snapshot without position must parse");
    assert!(snap.position.is_none(), "missing field must deserialize to None");
}

#[test]
fn new_finding_snapshot_with_position_field_parses_to_canonical_variant() {
    // Acceptance criterion: "fixture with position parses". A
    // FindingSnapshot JSON with the `position` field set to a
    // canonical NQ token must deserialize to the matching variant.
    for (token, expected) in [
        ("substrate", WitnessPosition::Substrate),
        ("application_internal", WitnessPosition::ApplicationInternal),
        ("platform", WitnessPosition::Platform),
    ] {
        let with_position = format!(
            r#"{{
                "finding_key": {{
                    "source": "nq",
                    "detector": "wal_bloat",
                    "subject": "host:/path"
                }},
                "host": "host",
                "severity": "warning",
                "persistence_generations": 1,
                "first_seen_at": "2026-06-08T00:00:00Z",
                "current_status": "active",
                "snapshot_generation": 1,
                "captured_at": "2026-06-08T00:00:00Z",
                "evidence_hash": "sha256:00",
                "position": "{token}"
            }}"#
        );
        let snap: FindingSnapshot = serde_json::from_str(&with_position)
            .expect("snapshot with position must parse");
        assert_eq!(snap.position, Some(expected));
    }
}
