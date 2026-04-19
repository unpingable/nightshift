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

/// Regression for the 2026-04-18 real-world failure: NQ's local DB
/// had a pending migration (`absent_gens` column missing) and exited
/// non-zero with a schema error. Night Shift MUST propagate that
/// failure honestly — with identifying content — never silently
/// return "no findings" and never produce a packet pretending
/// reconciliation worked.
///
/// Implementation note: we use `/bin/sh -c '<script>' --` as the NQ
/// invocation rather than writing a fake binary to disk; this avoids
/// the Linux ETXTBSY race where a freshly-written executable can
/// briefly be unexecutable.
#[test]
fn cli_source_propagates_upstream_non_zero_exit() {
    use nightshiftd::finding::FindingKey;
    use nightshiftd::nq::{CliNqSource, NqSource};

    // The trailing `--` and following Night-Shift-injected args
    // become $1..$N to the shell script and are ignored.
    let fake_nq = "echo 'Error: no such column: absent_gens in SELECT host, kind, subject ...' >&2; exit 1";

    let src = CliNqSource::new("/dev/null/placeholder.db").with_nq_argv([
        "/bin/sh",
        "-c",
        fake_nq,
        "--",
    ]);
    let key = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "labelwatch-host:/var/lib/labelwatch.sqlite".into(),
    };

    let err = src
        .snapshot(&key)
        .expect_err("upstream non-zero must surface as an error, not Ok(None)");

    let msg = format!("{err}");
    assert!(
        msg.contains("nq findings export failed"),
        "error did not mention the failing command: {msg}"
    );
    assert!(
        msg.contains("absent_gens"),
        "identifying upstream content missing from error: {msg}"
    );
    assert!(
        msg.contains("no such column"),
        "identifying upstream content missing from error: {msg}"
    );
}

/// Companion: a binary that exits non-zero without writing anything
/// to stderr still fails loudly (the status code alone is enough).
#[test]
fn cli_source_propagates_upstream_non_zero_exit_with_empty_stderr() {
    use nightshiftd::finding::FindingKey;
    use nightshiftd::nq::{CliNqSource, NqSource};

    let src = CliNqSource::new("/dev/null/placeholder.db").with_nq_argv([
        "/bin/sh",
        "-c",
        "exit 2",
        "--",
    ]);
    let key = FindingKey {
        source: "nq".into(),
        detector: "wal_bloat".into(),
        subject: "h:/p".into(),
    };

    let err = src
        .snapshot(&key)
        .expect_err("non-zero exit with empty stderr must still error");
    let msg = format!("{err}");
    assert!(
        msg.contains("nq findings export failed") && msg.contains("exit status"),
        "silent non-zero must still name the failure: {msg}"
    );
}

/// Regression: schema-version drift on the wire is rejected by the
/// parser, not silently translated. NQ might one day ship a
/// nq.finding_snapshot.v2 that Night Shift hasn't been updated for;
/// when that happens the consumer must complain visibly, not pretend
/// the v2 payload is a v1.
#[test]
fn cli_source_rejects_drifted_schema_on_wire() {
    use nightshiftd::finding::FindingKey;
    use nightshiftd::nq::{CliNqSource, NqSource};

    // Emit a single JSONL line that is structurally similar to a v1
    // snapshot but advertises a newer schema. parse_nq_line must
    // reject it; the error must surface, not pass through as None.
    let drifted = r#"{"schema":"nq.finding_snapshot.v99","contract_version":1,"finding_key":"local/h/d/s","identity":{"scope":"local","host":"h","detector":"d","subject":"s"},"lifecycle":{"first_seen_gen":0,"first_seen_at":"2026-01-01T00:00:00Z","last_seen_gen":0,"last_seen_at":"2026-01-01T00:00:00Z","consecutive_gens":1,"severity":"info","condition_state":"open"}}"#;
    // The fake-nq script must produce the canonical key Night Shift
    // is asking about, so the drifted line is actually consumed
    // (otherwise the consumer skips non-matching keys silently).
    let canonical = "local/h/d/s";
    let script = format!("printf '%s\\n' '{drifted}' | grep -F '{canonical}'; exit 0");

    let src = CliNqSource::new("/dev/null/placeholder.db").with_nq_argv([
        "/bin/sh",
        "-c",
        &script,
        "--",
    ]);
    let key = FindingKey {
        source: "nq".into(),
        detector: "d".into(),
        subject: "h:s".into(),
    };

    let err = src
        .snapshot(&key)
        .expect_err("drifted schema must surface as an error");
    let msg = format!("{err}");
    assert!(
        msg.contains("schema mismatch") && msg.contains("nq.finding_snapshot.v99"),
        "drift error must name the unexpected schema: {msg}"
    );
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
