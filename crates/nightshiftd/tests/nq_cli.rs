//! NQ CLI interface contract tests.
//!
//! These are skip-if-missing: if the `nq` binary is not reachable, the
//! test prints an explicit `[nq_cli skip]` notice on stderr and passes.
//! When it IS reachable, we invoke `nq findings export --help` and
//! confirm the flag-name interface Night Shift relies on is present.
//! This catches schema drift on NQ's side without requiring a live NQ
//! database in the test run.
//!
//! Resolution order: `NIGHTSHIFT_NQ_BIN` env var, then PATH (`which
//! nq`), then in-tree builds at `~/git/notquery/target/{release,debug}`
//! and `~/git/nq/target/debug`.
//!
//! ## Why a smoke-check is required, not just `path.exists()`
//!
//! Two real failure modes the bare resolution can't tell apart from
//! NotQuery:
//!
//! 1. The unix `nq` job-queue tool (often shipped as `/usr/bin/nq`,
//!    ~14KB, 2020-vintage) — a completely different program. `which nq`
//!    happily returns it, and the test then invokes
//!    `/usr/bin/nq findings export --help`, which the queue tool
//!    accepts as a queued command name, exits 0, and produces nothing
//!    the test expects. Without a smoke-check, the test fails not
//!    because NotQuery's contract drifted, but because we tested the
//!    wrong binary.
//! 2. Sandboxed test environments that wrap `exec` and refuse the
//!    call. The wrapper returns success and writes a path-shaped
//!    refusal marker (e.g. `,19ea827b1e7.671531`) to stdout. The test
//!    parses the marker as if it were `nq --help` output and fails on
//!    missing flags. The `.gitignore` rule `,[0-9a-f]*.[0-9]*` keeps
//!    those markers out of `git status`; this smoke-check keeps them
//!    from breaking the test.
//!
//! The smoke-check invokes candidate `--help` and requires the output
//! to mention `findings` — a subcommand only NotQuery has. Both
//! failure modes above flunk that check, and the test skips with a
//! visible `[nq_cli skip]` notice. Harness hardening only — the actual
//! `findings export --help` / `--format jsonl` expectations are
//! unchanged.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Smoke-check that a candidate binary at `path` actually looks like
/// NotQuery — not the unix `nq` job-queue tool, not a sandbox wrapper
/// that refuses the exec. The discriminator is the literal substring
/// `findings` in the candidate's top-level `--help` stdout, which only
/// NotQuery's CLI emits (it's a subcommand name).
///
/// Returns `false` for any failure mode: process spawn fails, exit
/// non-zero, stdout missing `findings`, stdout looks like a sandbox
/// refusal marker. Conservative by design — if we can't confirm
/// NotQuery, we skip rather than test the wrong thing.
fn looks_like_notquery(path: &Path) -> bool {
    let Ok(out) = Command::new(path).arg("--help").output() else {
        return false;
    };
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Refuse anything that looks like the sandbox-refusal marker
    // pattern (a single line of the form `,<hex>.<digits>`).
    let trimmed = stdout.trim();
    if trimmed.starts_with(',') && !trimmed.contains('\n') {
        return false;
    }
    // NotQuery's top-level help advertises the `findings` subcommand.
    // The unix `nq` job-queue tool prints `usage: nq [-c] [-q] ...`,
    // which does not contain that word.
    stdout.contains("findings")
}

fn resolve_nq_bin() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(p) = std::env::var("NIGHTSHIFT_NQ_BIN") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            candidates.push(pb);
        }
    }
    // Try "nq" on PATH.
    if let Ok(out) = Command::new("which").arg("nq").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                let pb = PathBuf::from(s);
                if pb.exists() {
                    candidates.push(pb);
                }
            }
        }
    }
    // Fall back to in-tree builds. Prefer release (smaller, faster) over
    // debug; check both `notquery` (current repo name) and `nq` (legacy
    // location).
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for tail in [
            ["git", "notquery", "target", "release", "nq"],
            ["git", "notquery", "target", "debug", "nq"],
            ["git", "nq", "target", "release", "nq"],
            ["git", "nq", "target", "debug", "nq"],
        ] {
            let mut candidate = home.clone();
            for seg in tail {
                candidate.push(seg);
            }
            if candidate.exists() {
                candidates.push(candidate);
            }
        }
    }

    candidates.into_iter().find(|p| looks_like_notquery(p))
}

#[test]
fn nq_findings_export_help_advertises_expected_flags() {
    let Some(bin) = resolve_nq_bin() else {
        eprintln!(
            "[nq_cli skip] no NotQuery binary reachable (resolved candidates \
             either absent or failed the `nq --help` smoke-check). Set \
             NIGHTSHIFT_NQ_BIN to a real NotQuery binary to exercise this test."
        );
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
    let drifted = r#"{"schema":"nq.finding_snapshot.v99","contract_version":1,"finding_key":"local/h/d/s","identity":{"scope":"local","host":"h","detector":"d","subject":"s"},"lifecycle":{"first_seen_gen":0,"first_seen_at":"2026-01-01T00:00:00Z","last_seen_gen":0,"last_seen_at":"2026-01-01T00:00:00Z","consecutive_gens":1,"severity":"info","condition_state":"open"},"admissibility":{"state":"observable","reason":"none"}}"#;
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
        eprintln!(
            "[nq_cli skip] no NotQuery binary reachable (resolved candidates \
             either absent or failed the `nq --help` smoke-check). Set \
             NIGHTSHIFT_NQ_BIN to a real NotQuery binary to exercise this test."
        );
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

// ---------------------------------------------------------------------------
// `nightshift nq disposition` — the supported machine surface (2026-07-26).
//
// These tests drive the real binary against a fake NQ (a script that answers
// with a canned receipt), so they need no live NQ. The contract under test:
// exit 0 means "a disposition was derived" — including refusals and stop —
// and the record on stdout, not the shell status, is the product.
// ---------------------------------------------------------------------------

fn nightshift_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nightshift"))
}

fn canned_receipt(profile: &str, decision: &str, with_supporting: bool) -> serde_json::Value {
    let supporting = if with_supporting {
        serde_json::json!([{
            "claim": "continuity_rely_eligible",
            "content_hash": "sha256:ssss",
            "status": "verified",
            "subject": "continuity-record:mem-1"
        }])
    } else {
        serde_json::json!([])
    };
    serde_json::json!({
        "schema": "nq.reliance.receipt.v1",
        "decision_id": "sha256:dddd",
        "request_digest": "sha256:rrrr",
        "evidence_context_digest": "sha256:eeee",
        "consumer_profile_id": profile,
        "caller_binding": "configured",
        "caller_binding_disclosure":
            "consumer profile was selected from local configuration; this is not an \
             authenticated consumer identity",
        "purpose": "continue_observing",
        "claim": "docket_attempt_settled",
        "receipt_content_hash": "sha256:cccc",
        "underlying_status": "verified",
        "decision": decision,
        "premises": [],
        "coverage_limits": [],
        "unresolved_residuals": [],
        "retained_contradictions": [],
        "refusal_reasons": [],
        "establishes": [],
        "does_not_establish": ["this decision grants no execution authority"],
        "supporting_receipts": supporting,
        "policy_version": "v1",
        // Freshly stamped so the freshness policy sees a young receipt.
        "generated_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    })
}

fn fake_nq(dir: &Path, receipt: &serde_json::Value, exit_code: u8) -> PathBuf {
    let receipt_file = dir.join("canned-receipt.json");
    std::fs::write(&receipt_file, serde_json::to_vec(receipt).unwrap()).unwrap();
    let script = dir.join("fake-nq.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\ncat {}\nexit {}\n",
            receipt_file.display(),
            exit_code
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    script
}

fn run_disposition(fake: &Path, dir: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(nightshift_bin());
    cmd.arg("--nq-bin")
        .arg(fake)
        .arg("nq")
        .arg("disposition")
        .arg("--request")
        .arg(dir.join("req.json"))
        .arg("--receipt")
        .arg(dir.join("rec.json"))
        .arg("--profiles")
        .arg(dir.join("profiles.json"));
    for e in extra {
        cmd.arg(e);
    }
    cmd.output().expect("nightshift invocation")
}

#[test]
fn disposition_cli_continuity_profile_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_nq(
        dir.path(),
        &canned_receipt("nightshift-readonly-continuity", "authorized_reliance", true),
        0,
    );
    let sup = dir.path().join("sup.json");
    std::fs::write(&sup, "{}").unwrap();
    let out = run_disposition(
        &fake,
        dir.path(),
        &[
            "--supporting",
            sup.to_str().unwrap(),
            "--expected-profile",
            "nightshift-readonly-continuity",
        ],
    );
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let rec: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rec["schema"], "nightshift.readonly_disposition.v1");
    assert_eq!(rec["disposition"], "continue_observing");
    assert_eq!(rec["expected_consumer_profile"], "nightshift-readonly-continuity");
    assert!(rec["disposition_id"].as_str().unwrap().starts_with("sha256:"));
    assert_eq!(
        rec["source"]["supporting_receipts"][0]["claim"],
        "continuity_rely_eligible"
    );
}

#[test]
fn disposition_cli_refusal_is_a_product_not_a_process_failure() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_nq(
        dir.path(),
        &canned_receipt("nightshift-readonly-continuity", "coverage_insufficient", false),
        0,
    );
    let out = run_disposition(
        &fake,
        dir.path(),
        &["--expected-profile", "nightshift-readonly-continuity"],
    );
    assert!(out.status.success(), "refusals still exit 0");
    let rec: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rec["disposition"], "human_judgment_required");
    assert_eq!(rec["human_judgment_required"], true);
}

#[test]
fn disposition_cli_default_expectation_is_the_base_profile() {
    let dir = tempfile::tempdir().unwrap();
    // NQ answers with a continuity-profile receipt; the CLI, given no
    // --expected-profile, expects the base profile → integrity stop.
    let fake = fake_nq(
        dir.path(),
        &canned_receipt("nightshift-readonly-continuity", "authorized_reliance", true),
        0,
    );
    let out = run_disposition(&fake, dir.path(), &[]);
    assert!(out.status.success());
    let rec: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rec["expected_consumer_profile"], "nightshift-readonly");
    assert_eq!(rec["disposition"], "stop");
    assert_eq!(rec["source_state"]["kind"], "malformed");
}

#[test]
fn disposition_cli_no_decision_is_evidence_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_nq(
        dir.path(),
        &canned_receipt("nightshift-readonly", "authorized_reliance", false),
        1, // NQ reached no decision
    );
    let out = run_disposition(&fake, dir.path(), &[]);
    assert!(out.status.success(), "an honest observation still exits 0");
    let rec: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(rec["disposition"], "evidence_unavailable");
    assert_eq!(rec["source_state"]["kind"], "transport_unavailable");
    assert!(rec.get("source").is_none() || rec["source"].is_null());
}

#[test]
fn disposition_cli_missing_required_argument_is_a_usage_error() {
    let out = Command::new(nightshift_bin())
        .arg("nq")
        .arg("disposition")
        .output()
        .expect("nightshift invocation");
    assert_eq!(out.status.code(), Some(2), "clap usage errors exit 2");
}
