//! D0d-a integration test — `nightshift watchbill run wal-bloat-review
//! --drill --scenario=all-green` end-to-end.
//!
//! Asserts:
//!
//!   1. The command exits 0 and emits a deterministic transcript on
//!      stdout — a receipt render of the chain, not LLM narration.
//!   2. The transcript contains the DRILL prefix from the embedded
//!      `governor why` walk.
//!   3. Two invocations against fresh receipt roots produce
//!      byte-identical transcripts (determinism).
//!
//! Skip protocol: if AG's `python3 -m governor.drill_runner` is not
//! importable (the agent_gov repo is not installed in the Python
//! environment this test runs against), the test is skipped with a
//! note rather than failing. This keeps the scheduler crate's test
//! suite green on stock dev machines while still exercising the
//! cross-repo wiring when both sides are present.

use std::path::PathBuf;
use std::process::Command;

use nightshiftd::drill;

/// Resolve the nightshiftd binary built by cargo for this test.
/// `CARGO_BIN_EXE_nightshift` is populated by cargo when there is a
/// binary target named `nightshift`. The crate's binary is set up
/// that way.
fn nightshift_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nightshift"))
}

/// Skip if the AG-side drill runner module isn't importable. Without
/// it, the cross-repo subprocess will fail; that failure is not a
/// regression in scheduler.
fn skip_if_drill_runner_missing() -> bool {
    if !drill::drill_runner_module_importable(None) {
        eprintln!(
            "[drill_runner_all_green skip] python3 -m \
             governor.drill_runner is not importable; the agent_gov \
             Python package is not installed in this test \
             environment. Install via `pip install -e \
             ~/git/agent_gov` to exercise this test."
        );
        return true;
    }
    false
}

#[test]
fn drill_all_green_command_emits_deterministic_transcript_with_drill_prefix() {
    if skip_if_drill_runner_missing() {
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let receipt_root_a = tmpdir.path().join("run_a");
    let receipt_root_b = tmpdir.path().join("run_b");

    // First invocation — fresh receipt root A.
    let out_a = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "wal-bloat-review",
            "--drill",
            "--scenario",
            "all-green",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root_a)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        out_a.status.success(),
        "nightshift watchbill run --drill exited non-zero (code={:?}); \
         stderr:\n{}",
        out_a.status.code(),
        String::from_utf8_lossy(&out_a.stderr),
    );

    let transcript_a = String::from_utf8(out_a.stdout)
        .expect("transcript must be UTF-8");

    // Loadbearing: the transcript is a receipt render of the chain.
    assert!(
        transcript_a.contains("nightshift watchbill: wal-bloat-review --drill --scenario=all-green"),
        "transcript missing header:\n{transcript_a}",
    );
    assert!(
        transcript_a.contains("origin_mode: drill"),
        "transcript missing origin_mode: drill line:\n{transcript_a}",
    );
    assert!(
        transcript_a.contains("DRILL"),
        "transcript missing DRILL prefix (from embedded `governor why` \
         walk):\n{transcript_a}",
    );
    assert!(
        transcript_a.contains("proposal_packet:"),
        "transcript missing proposal_packet section:\n{transcript_a}",
    );
    assert!(
        transcript_a.contains("Diagnose WAL bloat"),
        "transcript missing deterministic proposal text:\n{transcript_a}",
    );

    // Second invocation — fresh receipt root B. Same inputs except
    // root path; receipt ids are content-addressed and the transcript
    // strips timestamps and root-path noise, so the bytes must match.
    let out_b = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "wal-bloat-review",
            "--drill",
            "--scenario",
            "all-green",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root_b)
        .output()
        .expect("invoking nightshift must not fail (second time)");
    assert!(out_b.status.success());
    let transcript_b = String::from_utf8(out_b.stdout)
        .expect("transcript must be UTF-8");

    // Load-bearing determinism assertion.
    assert_eq!(
        transcript_a, transcript_b,
        "transcripts must be byte-identical across invocations \
         (determinism contract per D0d-a slice spec)\n--- A ---\n{transcript_a}\n--- B ---\n{transcript_b}"
    );
}

#[test]
fn drill_rejects_unsupported_scenario() {
    if skip_if_drill_runner_missing() {
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let receipt_root = tmpdir.path().join("run");
    // Legacy D0d-a era name — still rejected under the D0d-1 closed
    // six-scenario set (operator-load-bearing: no silent aliasing
    // from legacy names to canonical names).
    let out = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "wal-bloat-review",
            "--drill",
            "--scenario",
            "1_no_standing",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "drill should reject legacy D0d-a scenario name '1_no_standing' \
         (D0d-1 admits the six-scenario gauntlet canonical names); \
         stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
}

#[test]
fn drill_accepts_the_six_canonical_scenarios() {
    if skip_if_drill_runner_missing() {
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    // D0d-1 closed set — every scenario must be accepted by the CLI gate.
    // We do not assert the deterministic transcript shape here (each
    // scenario's transcript shape is exercised in the AG-side
    // ``test_drill_runner_d0d1_scenarios.py``); we only assert that the
    // CLI gate admits each scenario name without refusal.
    for scenario in nightshiftd::drill::SUPPORTED_SCENARIOS {
        let receipt_root = tmpdir.path().join(format!("run_{}", scenario));
        let out = Command::new(&bin)
            .args([
                "watchbill",
                "run",
                "wal-bloat-review",
                "--drill",
                "--scenario",
                scenario,
                "--drill-receipt-root",
            ])
            .arg(&receipt_root)
            .output()
            .expect("invoking nightshift must not fail");
        assert!(
            out.status.success(),
            "scenario {:?} must be accepted by the D0d-1 CLI gate; \
             stderr={}",
            scenario,
            String::from_utf8_lossy(&out.stderr),
        );
    }
}

#[test]
fn drill_accepts_already_consumed_alias_for_replay_budget() {
    if skip_if_drill_runner_missing() {
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
            nightshiftd::drill::SCENARIO_ALIAS_ALREADY_CONSUMED,
            "--drill-receipt-root",
        ])
        .arg(&receipt_root)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        out.status.success(),
        "operator-ratified alias 'already-consumed' must resolve to \
         'replay-budget' at the CLI gate; stderr={}",
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn drill_rejects_unsupported_workload() {
    if skip_if_drill_runner_missing() {
        return;
    }
    let bin = nightshift_bin();
    let tmpdir = tempfile::tempdir().expect("tmpdir");
    let receipt_root = tmpdir.path().join("run");
    let out = Command::new(&bin)
        .args([
            "watchbill",
            "run",
            "some-other-workload",
            "--drill",
            "--scenario",
            "all-green",
            "--drill-receipt-root",
        ])
        .arg(&receipt_root)
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "drill should reject unsupported workload 'some-other-workload' \
         (D0d-a wires only 'wal-bloat-review'); stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
}

#[test]
fn drill_requires_scenario_flag_when_drill_is_set() {
    if skip_if_drill_runner_missing() {
        return;
    }
    let bin = nightshift_bin();
    let out = Command::new(&bin)
        .args(["watchbill", "run", "wal-bloat-review", "--drill"])
        .output()
        .expect("invoking nightshift must not fail");
    assert!(
        !out.status.success(),
        "drill mode must require --scenario; stdout: {}",
        String::from_utf8_lossy(&out.stdout),
    );
}
