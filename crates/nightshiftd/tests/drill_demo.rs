//! D0e — `nightshift watchbill demo wal-bloat-review --drill`
//! end-to-end smoke test.
//!
//! Asserts:
//!
//!   1. The command exits 0 and emits a deterministic poster on
//!      stdout — a receipt render of the seven gauntlet runs (six
//!      scenarios + D3), not LLM narration.
//!   2. The poster carries the codex-ratified header / incident /
//!      DRILL paragraph / harness assertions section.
//!   3. Two invocations against fresh poster roots produce
//!      byte-identical posters (determinism).
//!
//! Skip protocol: if AG's `python3 -m governor.drill_poster` is not
//! importable, the test is skipped. Keeps scheduler CI green on
//! stock dev machines while still exercising the cross-repo wiring
//! when both sides are present.

use std::path::PathBuf;
use std::process::Command;

use nightshiftd::drill;

fn nightshift_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nightshift"))
}

fn skip_if_drill_poster_missing() -> bool {
    if !drill::drill_poster_module_importable(None) {
        eprintln!(
            "[drill_demo skip] python3 -m governor.drill_poster is \
             not importable; install agent_gov to exercise this test."
        );
        return true;
    }
    false
}

/// Normalize the poster across two fresh-root invocations. Receipt
/// ids are content-addressed and stable per identical inputs; the
/// scan below strips any future renderer-level identifier-length
/// drift so the determinism contract is robust against cosmetic
/// changes. Stdlib-only — no regex dependency.
fn normalize_poster(text: &str) -> String {
    // Replace every `ag_rcpt_<hex>` substring with `ag_rcpt_<rid>`
    // where `<hex>` is a contiguous run of 12+ hex chars.
    let needle = "ag_rcpt_";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        out.push_str(needle);
        rest = &rest[idx + needle.len()..];
        let hex_len = rest
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        if hex_len >= 12 {
            out.push_str("<rid>");
            // Skip past the hex run we just normalized.
            // Each hex char is ASCII so byte-length == char-length.
            rest = &rest[hex_len..];
        } else {
            // Not a real receipt id — pass through the prefix and
            // continue scanning from where we were.
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn demo_command_emits_deterministic_ticket_poster() {
    if skip_if_drill_poster_missing() {
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let root_a = tmpdir.path().join("demo_a");
    let root_b = tmpdir.path().join("demo_b");

    let out_a = Command::new(&bin)
        .args([
            "watchbill",
            "demo",
            "wal-bloat-review",
            "--drill",
            "--poster-root",
        ])
        .arg(&root_a)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        out_a.status.success(),
        "nightshift watchbill demo exited non-zero (code={:?}); stderr:\n{}",
        out_a.status.code(),
        String::from_utf8_lossy(&out_a.stderr),
    );
    let poster_a = String::from_utf8(out_a.stdout)
        .expect("poster must be UTF-8");

    // Codex-ratified header text.
    assert!(
        poster_a.contains("AG MVP Demo: Refusal Is a Product Surface"),
        "missing ratified header:\n{poster_a}"
    );
    assert!(
        poster_a.contains("Incident: WAL bloat review — DRILL"),
        "missing ratified incident line:\n{poster_a}"
    );
    assert!(
        poster_a.contains("SpendabilityAuthority: LA_ONLY"),
        "missing LA_ONLY banner:\n{poster_a}"
    );
    assert!(
        poster_a.contains("[BA3 — POST-MVP DEBT]"),
        "missing BA3 debt tag:\n{poster_a}"
    );
    assert!(
        poster_a.contains("origin_mode=drill minted at NQ"),
        "missing DRILL paragraph leader:\n{poster_a}"
    );
    assert!(
        poster_a.contains("Harness assertions"),
        "missing harness assertions header:\n{poster_a}"
    );
    // Codex revision 5: D3 row surfaces the dangling kind explicitly.
    assert!(
        poster_a.contains("validator_refused (dangling_receipt_reference)"),
        "missing D3 ratified outcome text:\n{poster_a}"
    );

    // Second invocation — determinism.
    let out_b = Command::new(&bin)
        .args([
            "watchbill",
            "demo",
            "wal-bloat-review",
            "--drill",
            "--poster-root",
        ])
        .arg(&root_b)
        .output()
        .expect("invoking nightshift must not fail (second time)");
    assert!(out_b.status.success());
    let poster_b = String::from_utf8(out_b.stdout)
        .expect("poster must be UTF-8");
    let norm_a = normalize_poster(&poster_a);
    let norm_b = normalize_poster(&poster_b);
    assert_eq!(
        norm_a, norm_b,
        "posters must be byte-identical after normalization\n--- A ---\n{norm_a}\n--- B ---\n{norm_b}"
    );
}

#[test]
fn demo_command_refuses_unknown_workload() {
    if skip_if_drill_poster_missing() {
        return;
    }
    let bin = nightshift_bin();
    let out = Command::new(&bin)
        .args(["watchbill", "demo", "not-a-real-workload", "--drill"])
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "demo must refuse unknown workload; got success:\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn demo_command_requires_drill_flag() {
    if skip_if_drill_poster_missing() {
        return;
    }
    let bin = nightshift_bin();
    let out = Command::new(&bin)
        .args(["watchbill", "demo", "wal-bloat-review"])
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "demo without --drill must fail; got success:\nstdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
