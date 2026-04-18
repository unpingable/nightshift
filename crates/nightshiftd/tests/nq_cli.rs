//! NQ CLI interface contract tests.
//!
//! These are skip-if-missing: if the `nq` binary is not reachable, the
//! test prints a skip notice and passes. When it IS reachable, we
//! invoke `nq findings export --help` and confirm the flag-name
//! interface Night Shift relies on is present. This catches schema
//! drift on NQ's side without requiring a live NQ database in the
//! test run.
//!
//! Resolution order: NIGHTSHIFT_NQ_BIN env var, then PATH, then the
//! in-tree debug binary at ~/git/nq/target/debug/nq.

use std::path::PathBuf;
use std::process::Command;

fn resolve_nq_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("NIGHTSHIFT_NQ_BIN") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    // Try "nq" on PATH.
    if let Ok(out) = Command::new("which").arg("nq").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                let pb = PathBuf::from(s);
                if pb.exists() {
                    return Some(pb);
                }
            }
        }
    }
    // Fall back to in-tree debug build next to this repo.
    if let Some(home) = std::env::var_os("HOME") {
        let candidate = PathBuf::from(home)
            .join("git")
            .join("nq")
            .join("target")
            .join("debug")
            .join("nq");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

#[test]
fn nq_findings_export_help_advertises_expected_flags() {
    let Some(bin) = resolve_nq_bin() else {
        eprintln!("skipping: nq binary not reachable (set NIGHTSHIFT_NQ_BIN)");
        return;
    };

    let out = Command::new(&bin)
        .arg("findings")
        .arg("export")
        .arg("--help")
        .output()
        .expect("invoking nq must not fail once the binary resolves");
    assert!(
        out.status.success(),
        "nq findings export --help exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let help = String::from_utf8_lossy(&out.stdout);

    // These are the flag names Night Shift's CliNqSource relies on.
    // If any of these moves, we want a clear test failure, not a
    // runtime surprise.
    for flag in [
        "--db",
        "--format",
        "--finding-key",
        "--changed-since-generation",
        "--detector",
        "--host",
    ] {
        assert!(
            help.contains(flag),
            "nq findings export --help missing expected flag {flag}\nhelp output:\n{help}"
        );
    }
}

#[test]
fn nq_findings_export_default_format_is_jsonl() {
    let Some(bin) = resolve_nq_bin() else {
        eprintln!("skipping: nq binary not reachable (set NIGHTSHIFT_NQ_BIN)");
        return;
    };

    let out = Command::new(&bin)
        .arg("findings")
        .arg("export")
        .arg("--help")
        .output()
        .expect("nq invocation");
    let help = String::from_utf8_lossy(&out.stdout);

    // Night Shift parses the default output as JSONL. If the default
    // changes (e.g., to json arrays), this test catches it before the
    // parser does.
    assert!(
        help.contains("jsonl"),
        "nq help should document jsonl default; got:\n{help}"
    );
}
