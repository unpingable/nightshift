//! Closed declaration of Night Shift's subprocess operations.
//!
//! Every `Command::new` site in a declared-diagnostic module must appear here,
//! naming the exact executable and verb it invokes. `scripts/check_no_actuation_surface.sh`
//! enforces that correspondence; an undeclared site fails the build.
//!
//! # Why a declaration and not an allowlist
//!
//! The actuation boundary's read/actuation binary had no row for an operation
//! that *executes* something bounded while remaining unable to mutate any
//! governed substrate. Cartography added one on 2026-07-26 — **bounded
//! diagnostic execution** — under eleven conjunctive constraints, and pinned
//! two properties this module exists to satisfy:
//!
//! > Classification attaches to declared *operations*, never to a file, a
//! > directory, or a module name.
//!
//! > The category is closed and explicitly enumerated. An undeclared subprocess
//! > site is not in the category by resemblance.
//!
//! So `drill.rs` is **not** trusted because of its name. Its subprocess sites
//! are counted against the table below. Adding one without declaring it here
//! is a build failure, which is the operator signal that the invocation shape
//! changed.
//!
//! # This is not a read
//!
//! Per the doctrine, do not call these reads merely because they are
//! non-actuating. A read observes without executing; a bounded diagnostic
//! executes a named operation in order to observe.
//!
//! See `docs/working/decisions/ACTUATION_BOUNDARY.md`.

/// What an operation is permitted to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticWrites {
    /// Writes nothing. Pure observation.
    Nothing,
    /// Writes only into a disposable root supplied by the caller. The path is
    /// an argument, never a compiled-in or user-configuration location.
    DeclaredDisposable,
}

/// Where an operation sits in the actuation boundary's taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCategory {
    /// Observes without executing anything of consequence — `--help`,
    /// `import`, a chain render. Still a subprocess, hence still declared.
    PureRead,
    /// Executes a named operation in order to observe. Not a read.
    BoundedDiagnosticExecution,
}

/// One declared subprocess operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticOperation {
    /// Stable operation identity, for receipts and review.
    pub name: &'static str,
    /// The Rust function that owns the call site.
    pub owner: &'static str,
    /// Exact program identity. A logical name (`nq-monitor`, `python3`,
    /// `governor`) resolved by the caller; never a shell string.
    pub executable: &'static str,
    /// The fixed argv token that selects the operation.
    pub verb: &'static str,
    pub writes: DiagnosticWrites,
    pub category: DiagnosticCategory,
    /// Why this operation satisfies the eleven constraints.
    pub rationale: &'static str,
}

/// The closed set. Enumerated, not inferred.
///
/// Ordered by the call site's line in `drill.rs` so a reviewer can walk the
/// file against this table.
pub const DECLARED_DIAGNOSTIC_OPERATIONS: &[DiagnosticOperation] = &[
    DiagnosticOperation {
        name: "nq.drill.wal_bloat",
        owner: "drill::capture_genuine_nq_finding",
        executable: "nq-monitor",
        verb: "drill",
        writes: DiagnosticWrites::DeclaredDisposable,
        category: DiagnosticCategory::BoundedDiagnosticExecution,
        rationale: "Drives NQ's own evaluator over a staged sandbox database to \
                    produce nq.finding_snapshot.v1. Writes only inside the \
                    caller-supplied sandbox directory. Mints no claim and holds \
                    no NQ standing.",
    },
    DiagnosticOperation {
        name: "ag.drill_runner",
        owner: "drill::run_drill",
        executable: "python3",
        verb: "-m",
        writes: DiagnosticWrites::DeclaredDisposable,
        category: DiagnosticCategory::BoundedDiagnosticExecution,
        rationale: "Runs governor.drill_runner, whose --root is required and \
                    opens a fresh receipt store there. Consumes a module-constant \
                    fixture capacity token, not a credential; emits a \
                    deterministic proposal stub. No global AG state is touched.",
    },
    DiagnosticOperation {
        name: "ag.governor_why",
        owner: "drill::shell_governor_why",
        executable: "governor",
        verb: "why",
        writes: DiagnosticWrites::Nothing,
        category: DiagnosticCategory::PureRead,
        rationale: "Renders an existing receipt chain. Reads only.",
    },
    DiagnosticOperation {
        name: "ag.governor_reachable",
        owner: "drill::governor_cli_reachable",
        executable: "governor",
        verb: "--help",
        writes: DiagnosticWrites::Nothing,
        category: DiagnosticCategory::PureRead,
        rationale: "Reachability probe.",
    },
    DiagnosticOperation {
        name: "nq.bin_reachable",
        owner: "drill::nq_bin_reachable",
        executable: "nq-monitor",
        verb: "--help",
        writes: DiagnosticWrites::Nothing,
        category: DiagnosticCategory::PureRead,
        rationale: "Reachability probe for the hop that runs first.",
    },
    DiagnosticOperation {
        name: "ag.drill_runner_importable",
        owner: "drill::drill_runner_module_importable",
        executable: "python3",
        verb: "-c",
        writes: DiagnosticWrites::Nothing,
        category: DiagnosticCategory::PureRead,
        rationale: "Import probe. The -c argument is a fixed literal import \
                    statement, not caller input.",
    },
    DiagnosticOperation {
        name: "ag.drill_poster",
        owner: "drill::run_demo_command",
        executable: "python3",
        verb: "-m",
        writes: DiagnosticWrites::DeclaredDisposable,
        category: DiagnosticCategory::BoundedDiagnosticExecution,
        rationale: "Runs governor.drill_poster, whose declared scope is \
                    display-only plus invariant assertions, into a --root the \
                    caller supplies.",
    },
    DiagnosticOperation {
        name: "ag.drill_poster_importable",
        owner: "drill::drill_poster_module_importable",
        executable: "python3",
        verb: "-c",
        writes: DiagnosticWrites::Nothing,
        category: DiagnosticCategory::PureRead,
        rationale: "Import probe. Fixed literal argument.",
    },
];

/// Programs any declared operation is permitted to invoke. A closed set, so a
/// new executable cannot enter by editing one table row's string alone without
/// this list agreeing.
pub const DECLARED_DIAGNOSTIC_EXECUTABLES: &[&str] = &["nq-monitor", "python3", "governor"];

/// Number of declared operations. The gate compares this to the count of
/// subprocess call sites in the declared-diagnostic module.
#[must_use]
pub fn declared_operation_count() -> usize {
    DECLARED_DIAGNOSTIC_OPERATIONS.len()
}

/// Look up a declared operation by name.
#[must_use]
pub fn declared(name: &str) -> Option<&'static DiagnosticOperation> {
    DECLARED_DIAGNOSTIC_OPERATIONS
        .iter()
        .find(|op| op.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_operation_names_a_declared_executable() {
        for op in DECLARED_DIAGNOSTIC_OPERATIONS {
            assert!(
                DECLARED_DIAGNOSTIC_EXECUTABLES.contains(&op.executable),
                "{} invokes undeclared executable {:?}",
                op.name,
                op.executable
            );
        }
    }

    #[test]
    fn no_declared_executable_is_a_substrate_actuator_or_shell() {
        // The doctrine's "May never do" column, mechanically.
        for forbidden in [
            "ssh", "salt", "ansible", "systemctl", "kubectl", "docker", "sh", "bash", "git",
            "sudo", "curl", "wget",
        ] {
            assert!(
                !DECLARED_DIAGNOSTIC_EXECUTABLES.contains(&forbidden),
                "{forbidden} must never be a declared diagnostic executable"
            );
        }
    }

    #[test]
    fn operation_names_and_owners_are_unique() {
        for (i, op) in DECLARED_DIAGNOSTIC_OPERATIONS.iter().enumerate() {
            assert!(
                DECLARED_DIAGNOSTIC_OPERATIONS[..i]
                    .iter()
                    .all(|p| p.name != op.name),
                "duplicate operation name {:?}",
                op.name
            );
        }
    }

    #[test]
    fn every_operation_carries_a_rationale() {
        for op in DECLARED_DIAGNOSTIC_OPERATIONS {
            assert!(
                op.rationale.len() > 40,
                "{} must state why it satisfies the constraints",
                op.name
            );
        }
    }

    /// A pure read may not write. That is what makes it a read rather than a
    /// bounded diagnostic execution.
    #[test]
    fn pure_reads_write_nothing() {
        for op in DECLARED_DIAGNOSTIC_OPERATIONS {
            if op.category == DiagnosticCategory::PureRead {
                assert_eq!(
                    op.writes,
                    DiagnosticWrites::Nothing,
                    "{} is declared a pure read but declares writes",
                    op.name
                );
            }
        }
    }

    #[test]
    fn the_table_is_closed_and_addressable() {
        assert_eq!(declared_operation_count(), 8);
        assert!(declared("nq.drill.wal_bloat").is_some());
        assert!(declared("no.such.operation").is_none());
    }
}
