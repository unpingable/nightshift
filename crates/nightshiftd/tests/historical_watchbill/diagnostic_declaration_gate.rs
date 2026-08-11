//! HISTORICAL DRILL SPECIMEN. Structural tests for the retired bounded-diagnostic-execution classification.
//!
//! The cartography doctrine (amended 2026-07-26) pins two properties this
//! suite exists to enforce mechanically:
//!
//! > Classification attaches to declared *operations*, never to a file, a
//! > directory, or a module name.
//!
//! > The category is closed and explicitly enumerated. An undeclared subprocess
//! > site is not in the category by resemblance.
//!
//! The negative tests run the real guard against a temporary copy of the source
//! tree, so they assert what the gate *does*, not what it is supposed to do.

use std::path::{Path, PathBuf};
use std::process::Command;

use nightshiftd::diagnostic_operations::{
    declared_operation_count, DiagnosticCategory, DiagnosticWrites,
    DECLARED_DIAGNOSTIC_EXECUTABLES, DECLARED_DIAGNOSTIC_OPERATIONS,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn drill_src() -> String {
    std::fs::read_to_string(repo_root().join("crates/nightshiftd/src/drill.rs"))
        .expect("read drill.rs")
}

/// Run the real guard against a scratch copy of the repo.
fn guard_on_copy(mutate: impl FnOnce(&Path)) -> (i32, String) {
    let tmp = tempfile::tempdir().expect("tmpdir");
    let root = repo_root();
    let dst = tmp.path().join("repo");
    // The guard only reads `crates/nightshiftd/src`, `Cargo.toml`s, and its own
    // script, so copy just those.
    std::fs::create_dir_all(dst.join("crates/nightshiftd/src")).unwrap();
    std::fs::create_dir_all(dst.join("scripts")).unwrap();
    for entry in std::fs::read_dir(root.join("crates/nightshiftd/src")).unwrap() {
        let p = entry.unwrap().path();
        if p.is_file() {
            std::fs::copy(
                &p,
                dst.join("crates/nightshiftd/src")
                    .join(p.file_name().unwrap()),
            )
            .unwrap();
        }
    }
    std::fs::copy(
        root.join("scripts/check_no_actuation_surface.sh"),
        dst.join("scripts/check_no_actuation_surface.sh"),
    )
    .unwrap();
    std::fs::copy(root.join("Cargo.toml"), dst.join("Cargo.toml")).unwrap();
    std::fs::copy(
        root.join("crates/nightshiftd/Cargo.toml"),
        dst.join("crates/nightshiftd/Cargo.toml"),
    )
    .unwrap();

    mutate(&dst);

    let out = Command::new("bash")
        .arg(dst.join("scripts/check_no_actuation_surface.sh"))
        .current_dir(&dst)
        .output()
        .expect("run guard");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

fn append_to_drill(dst: &Path, snippet: &str) {
    let p = dst.join("crates/nightshiftd/src/drill.rs");
    let mut s = std::fs::read_to_string(&p).unwrap();
    s.push_str(snippet);
    std::fs::write(&p, s).unwrap();
}

// ---------------------------------------------------------------------------
// Positive: the declaration describes the code
// ---------------------------------------------------------------------------

#[test]
fn every_subprocess_site_in_the_declared_module_is_declared() {
    let sites = drill_src().matches("Command::new").count();
    assert_eq!(
        sites,
        declared_operation_count(),
        "drill.rs has {sites} subprocess call site(s) but {} operation(s) are declared",
        declared_operation_count()
    );
}

#[test]
fn every_declared_owner_exists_in_the_module() {
    let src = drill_src();
    for op in DECLARED_DIAGNOSTIC_OPERATIONS {
        let fn_name = op.owner.rsplit("::").next().unwrap();
        assert!(
            src.contains(&format!("fn {fn_name}")),
            "declared operation {} names owner {} which does not exist",
            op.name,
            op.owner
        );
    }
}

#[test]
fn declared_operations_invoke_only_declared_executables() {
    for op in DECLARED_DIAGNOSTIC_OPERATIONS {
        assert!(
            DECLARED_DIAGNOSTIC_EXECUTABLES.contains(&op.executable),
            "{} invokes undeclared executable {:?}",
            op.name,
            op.executable
        );
    }
}

/// Constraint 5: no standing, authority, lease, capability, credential, or
/// reusable execution token.
#[test]
fn the_declared_module_holds_no_authority_material() {
    let src = drill_src();
    for forbidden in [
        "ProposedAction",
        "Capability",
        "ExecutionLease",
        "ExecutionGrant",
        "credential",
        "Credential",
        "api_key",
        "bearer",
        "private_key",
    ] {
        assert!(
            !src.contains(forbidden),
            "drill.rs must hold no authority material, found {forbidden:?}"
        );
    }
}

/// Constraints 6 and 8: no substrate mutation, no actuator, no repair.
#[test]
fn the_declared_module_cannot_reach_an_actuator_or_git() {
    let src = drill_src();
    for forbidden in [
        "Command::new(\"git\")",
        "Command::new(\"ssh\")",
        "Command::new(\"salt\")",
        "Command::new(\"systemctl\")",
        "Command::new(\"kubectl\")",
        "Command::new(\"docker\")",
        "Command::new(\"sudo\")",
        // Constraint: no shell, so no arbitrary-command expansion.
        "Command::new(\"sh\")",
        "Command::new(\"bash\")",
        "sh -c",
        "bash -c",
    ] {
        assert!(
            !src.contains(forbidden),
            "drill.rs must not reach an actuator or shell, found {forbidden:?}"
        );
    }
}

/// Constraint 7: cannot issue AG authorization or Docket standing.
#[test]
fn the_declared_module_issues_no_ag_or_docket_authority() {
    let src = drill_src();
    // Precise call-shaped tokens. Bare words like "reserve" match inside
    // ordinary prose such as "preserved", which would make this test fail for
    // a reason that has nothing to do with authority.
    for forbidden in [
        "authz-request",
        "docket-issuance",
        "mint_standing",
        "docket_standing",
        "reserve(",
        "reserve_attempt",
        "dispatch(",
        "dispatch_attempt",
        "AuthzRequest",
        "DocketIssuance",
    ] {
        assert!(
            !src.contains(forbidden),
            "drill.rs must issue no AG/Docket authority, found {forbidden:?}"
        );
    }
}

/// Constraint 3: pure reads write nothing; bounded diagnostics write only into
/// a caller-supplied root.
#[test]
fn write_declarations_match_their_category() {
    for op in DECLARED_DIAGNOSTIC_OPERATIONS {
        match op.category {
            DiagnosticCategory::PureRead => assert_eq!(
                op.writes,
                DiagnosticWrites::Nothing,
                "{} is a pure read and must declare no writes",
                op.name
            ),
            DiagnosticCategory::BoundedDiagnosticExecution => {}
        }
    }
    // The category must actually be used — a table of only reads would mean the
    // classification was never needed.
    assert!(DECLARED_DIAGNOSTIC_OPERATIONS
        .iter()
        .any(|o| o.category == DiagnosticCategory::BoundedDiagnosticExecution));
}

// ---------------------------------------------------------------------------
// Negative: the gate refuses. These run the real guard.
// ---------------------------------------------------------------------------

#[test]
fn the_guard_is_green_on_the_unmodified_tree() {
    let (code, out) = guard_on_copy(|_| {});
    assert_eq!(code, 0, "guard must be green on the real tree:\n{out}");
    assert!(out.contains("clean"));
}

#[test]
fn the_guard_refuses_an_undeclared_subprocess_site() {
    let (code, out) = guard_on_copy(|dst| {
        append_to_drill(
            dst,
            "\nfn _undeclared() { let _ = std::process::Command::new(\"python3\").output(); }\n",
        );
    });
    assert_eq!(code, 1, "an undeclared site must fail:\n{out}");
    assert!(
        out.contains("declared-diagnostic drift"),
        "failure must name the drift:\n{out}"
    );
}

#[test]
fn the_guard_refuses_a_new_file_with_a_subprocess() {
    let (code, out) = guard_on_copy(|dst| {
        std::fs::write(
            dst.join("crates/nightshiftd/src/_injected.rs"),
            "pub fn x() { let _ = std::process::Command::new(\"python3\").output(); }\n",
        )
        .unwrap();
    });
    assert_eq!(code, 1, "a new subprocess file must fail:\n{out}");
    assert!(out.contains("outside read-adapter allowlist and declared diagnostics"));
}

#[test]
fn the_guard_refuses_an_actuator_even_inside_the_declared_module() {
    // The load-bearing property: declaring a module does not buy it cover.
    // Checks 1-4 are file-independent, so an actuator fails regardless.
    for actuator in ["systemctl", "ssh", "kubectl", "docker"] {
        let (code, out) = guard_on_copy(|dst| {
            append_to_drill(
                dst,
                &format!(
                    "\nfn _act() {{ let _ = std::process::Command::new(\"{actuator}\").arg(\"restart\").output(); }}\n"
                ),
            );
        });
        assert_eq!(code, 1, "{actuator} inside drill.rs must fail:\n{out}");
    }
}

#[test]
fn the_guard_refuses_git_mutation_from_the_declared_module() {
    let (code, out) = guard_on_copy(|dst| {
        append_to_drill(
            dst,
            "\nfn _push() { let _ = std::process::Command::new(\"git\").arg(\"push\").output(); }\n",
        );
    });
    assert_eq!(code, 1, "git invocation must fail:\n{out}");
}

#[test]
fn the_guard_refuses_a_shell_expansion_site() {
    let (code, out) = guard_on_copy(|dst| {
        append_to_drill(
            dst,
            "\nfn _sh() { let _ = std::process::Command::new(\"sh\").arg(\"-c\").arg(\"x\").output(); }\n",
        );
    });
    assert_eq!(code, 1, "shell expansion must fail:\n{out}");
}

#[test]
fn the_guard_refuses_when_the_declaration_table_is_removed() {
    // A blanket file-level classification is exactly what this prevents:
    // delete the declaration and drill.rs is not silently trusted.
    let (code, out) = guard_on_copy(|dst| {
        std::fs::remove_file(dst.join("crates/nightshiftd/src/diagnostic_operations.rs")).unwrap();
    });
    assert_eq!(code, 1, "a missing declaration table must fail:\n{out}");
    assert!(
        out.contains("no declaration table"),
        "failure must name the missing table:\n{out}"
    );
}
