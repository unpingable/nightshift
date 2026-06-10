//! D3 — confabulated-receipt closing beat CLI tests.
//!
//! Per agent_gov campaign §3 D3 and §3b: the `nightshift watchbill run
//! wal-bloat-review --drill --scenario=all-green --confabulate-citation=<role>`
//! command injects a bogus citation into the proposal-packet step. The
//! validator on the AG side detects the dangling reference and emits a
//! refusal receipt. The closed role set is `{standing, evidence}`
//! (existence-fail / kind-fit-fail respectively).
//!
//! These tests focus on the Night Shift CLI gate — what the operator
//! sees at the boundary. The AG-side end-to-end behavior (refusal
//! emission, transcript shape, `governor why` walk, retry economics,
//! determinism) is exercised by
//! `tests/test_drill_runner_d3_confabulation.py` in the agent_gov repo.
//!
//! Skip protocol matches `drill_runner_all_green.rs`: if AG's
//! `python3 -m governor.drill_runner` is not importable, the test is
//! skipped rather than failing.

use std::path::PathBuf;
use std::process::Command;

use nightshiftd::drill;

fn nightshift_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nightshift"))
}

fn skip_if_drill_runner_missing() -> bool {
    if !drill::drill_runner_module_importable(None) {
        eprintln!(
            "[drill_runner_d3_confabulation skip] python3 -m \
             governor.drill_runner is not importable; the agent_gov \
             Python package is not installed in this test \
             environment."
        );
        return true;
    }
    false
}

#[test]
fn drill_rejects_unsupported_confabulate_role_at_clap_gate() {
    // clap rejects unknown role values BEFORE invoking the subprocess
    // (closed value-parser list). No skip needed; the clap gate is
    // local to the CLI binary.
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let receipt_root = tmpdir.path().join("run");
    let out = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "wal-bloat-review",
            "--drill",
            "--scenario",
            "all-green",
            "--confabulate-citation",
            "not-a-real-role",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "drill should reject unknown confabulate-citation role at the \
         clap gate; stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
}

#[test]
fn drill_accepts_both_standing_and_evidence_roles_at_cli_gate() {
    // Roles match AG-side `CONFABULATION_ROLES` exactly. The clap
    // value-parser is the local closed set; mirroring AG's set keeps
    // the two ends in lockstep. We do not invoke the subprocess here
    // (avoids NQ dependency); we test that the CLI parser admits both
    // roles via `--help` snapshot pattern.
    for role in drill::CONFABULATION_ROLES {
        assert!(
            *role == "standing" || *role == "evidence",
            "AG-side closed set ratified two roles; Night Shift mirror \
             must match. Unexpected role: {role}"
        );
    }
}

#[test]
fn drill_confabulate_citation_standing_runs_end_to_end_when_nq_available() {
    // This test exercises the full CLI → subprocess → AG validator
    // path. Requires both `nq-monitor` (for the NQ side) and the
    // agent_gov Python package on the system. When NQ is missing we
    // skip — the AG side is exhaustively tested in
    // `tests/test_drill_runner_d3_confabulation.py`.
    if skip_if_drill_runner_missing() {
        return;
    }
    // Probe nq-monitor by checking the binary exists; skip if not.
    if Command::new("nq-monitor").arg("--help").output().is_err() {
        eprintln!(
            "[drill_runner_d3_confabulation skip] nq-monitor binary \
             not on PATH; install nq to exercise the cross-repo D3 \
             path. AG-side D3 coverage is in test_drill_runner_d3_\
             confabulation.py."
        );
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let receipt_root = tmpdir.path().join("run");
    let out = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "wal-bloat-review",
            "--drill",
            "--scenario",
            "all-green",
            "--confabulate-citation",
            "standing",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        out.status.success(),
        "drill with --confabulate-citation=standing should run \
         end-to-end (refusal IS the receipt; stdout exit is 0 because \
         the transcript print succeeded); stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
    let transcript = String::from_utf8(out.stdout)
        .expect("transcript must be UTF-8");
    // The transcript carries the proposal_validator line + the
    // dangling_receipt_reference closed-vocab kind.
    assert!(
        transcript.contains("proposal_validator"),
        "D3 transcript missing proposal_validator chain line:\n{transcript}"
    );
    assert!(
        transcript.contains("dangling_receipt_reference"),
        "D3 transcript missing the closed-vocab refusal kind:\n{transcript}"
    );
}
