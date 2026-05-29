//! Forbidden-cycle structural-absence sentinel test.
//!
//! Pins the NS-side structural absence of the forbidden cycle named in
//! `docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md`:
//!
//! ```text
//! NS posture / closure verdict / `SilenceShape`  →  NQ substrate truth
//! ```
//!
//! The bilateral spike at
//! `~/git/cartography/coordination/NQ-NS-CHANNEL-SPLIT.md` and both the
//! NS-side and NQ-side channel-split GAPs commit each component to
//! **structural** absence (not by guard, not by flag, not by lint) of
//! this back-edge. Every other channel between NS and NQ is a forward
//! DAG edge:
//!
//! - NQ truth → NS posture (forward; the normal consumption path).
//! - NS posture → Governor / operator CLI / Continuity / NS-internal
//!   SQLite (forward; the outbound surfaces).
//!
//! Only the NS → NQ-truth edge is forbidden. Once that edge closes,
//! `NS asserts SilenceShape` becomes `NQ substrate truth: silence`
//! becomes input to NS's next posture — the ouroboros the bilateral
//! spike exists to refuse.
//!
//! ## What this test guards
//!
//! Today NS has zero code paths that write to NQ. Its only NQ-touching
//! calls are read-only `nq … export` invocations through
//! `crates/nightshiftd/src/nq.rs::CliNqSource` (`findings export`) and
//! `crates/nightshiftd/src/liveness.rs::CliLivenessSource` (`liveness
//! export`). NS does not open NQ's SQLite directly; the only
//! `rusqlite::Connection::open` call site in NS is against NS's own
//! store. This test pins those structural properties.
//!
//! ## How updating this test signals doctrine has moved
//!
//! Same sentinel shape as
//! `b2_stale_imported_basis_sentinel_ok_to_proceed_is_not_authorization`:
//! the test is the doctrine surface, not just a regression guard. If a
//! forcing case ever justifies an NS → NQ write path (no current
//! forcing case exists), this test must be updated deliberately. The
//! deliberate update is the operator signal that NS forbidden-cycle
//! doctrine has moved.
//!
//! ## Coverage scope and limits
//!
//! This test covers the **subprocess-invocation** and **direct-DB**
//! outbound surfaces to NQ. The NS-side channel-split GAP names two
//! other outbound surfaces that this test does **not** verify:
//!
//! - **Continuity MCP emit of NS posture as NQ-readable truth.** NS
//!   does not currently have Continuity MCP wiring. When wiring lands,
//!   the test scope must expand.
//! - **Operator CLI output.** NS prints packets / `runs show` to
//!   stdout, never to NQ. Structural absence by output direction, not
//!   by file inspection.
//!
//! Expanding the coverage when those surfaces materialize is downstream
//! work for whichever slice adds them.
//!
//! See `docs/working/decisions/AUDIT-BACKLOG.md`
//! § "Self-subject-collapse: NS forbidden-cycle structural-absence
//! audit owed" — this test closes the audit-owed entry.

use std::fs;
use std::path::PathBuf;

/// Read an NS source file relative to the crate's `src/` directory.
fn ns_src(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// `nq` CLI subcommands whose semantics would let NS push posture /
/// closure / silence-shape into NQ as substrate truth.
///
/// Intentionally broader than what `nq` exposes today — we name the
/// **shape** of forbidden verbs so the test rejects new verbs of that
/// shape, not just verbs that currently exist. If a new read-only verb
/// is added to `nq` (e.g. a new preflight subcommand) and lives in NS
/// via `Command::new`, the verb is fine; the test catches write-shaped
/// verbs by name.
const FORBIDDEN_NQ_SUBCOMMANDS: &[&str] = &[
    // Write verbs that would let NS mint a finding NQ then treats as truth.
    "insert",
    "emit",
    "emit-finding",
    "mint",
    "declare",
    "ingest",
    "create-finding",
    "publish",
    // Posture / state / closure push verbs (hypothetical; none exist in `nq`).
    "push-posture",
    "report-closure",
    "report-state",
    "report-attention",
    "report-silence",
    "ack-finding",
];

#[test]
fn forbidden_cycle_structural_absence_sentinel_ns_does_not_write_to_nq() {
    let nq_rs = ns_src("nq.rs");
    let liveness_rs = ns_src("liveness.rs");

    // 1. No forbidden subcommand may appear as a string-literal CLI arg.
    //    We check both the `.arg("foo")` shape and the bare `"foo"`
    //    shape — the latter catches construction via intermediate
    //    variables.
    for verb in FORBIDDEN_NQ_SUBCOMMANDS {
        let arg_form = format!(".arg(\"{verb}\")");
        let bare_form = format!("\"{verb}\"");
        assert!(
            !nq_rs.contains(&arg_form),
            "crates/nightshiftd/src/nq.rs contains `{arg_form}` — \
             this is an NS → NQ-truth back-edge the channel-split GAP \
             refuses structurally. If this is intentional, NS-side \
             forbidden-cycle doctrine has moved and this test must be \
             updated deliberately. See \
             docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md \
             § \"Forbidden cycle\"."
        );
        assert!(
            !nq_rs.contains(&bare_form),
            "crates/nightshiftd/src/nq.rs contains string literal `{bare_form}` — \
             see commentary on .arg() form above."
        );
        assert!(
            !liveness_rs.contains(&arg_form),
            "crates/nightshiftd/src/liveness.rs contains `{arg_form}` — \
             same discipline as nq.rs above."
        );
        assert!(
            !liveness_rs.contains(&bare_form),
            "crates/nightshiftd/src/liveness.rs contains string literal `{bare_form}` — \
             same discipline as nq.rs above."
        );
    }

    // 2. Every `Command::new(...)` site in nq.rs / liveness.rs must be
    //    paired with an `.arg("export")` somewhere in the same file.
    //    Today both files contain only `findings export` and `liveness
    //    export` subprocess invocations; the counts must match.
    //
    //    If a new read-only NQ verb lands that does not use the `export`
    //    subcommand (e.g. a future `nq peek` invocation from NS code),
    //    this assertion must be updated deliberately. That update is the
    //    operator signal that NS's NQ-CLI invocation shape has changed.
    let nq_command_count = nq_rs.matches("Command::new").count();
    let nq_export_count = nq_rs.matches(".arg(\"export\")").count();
    assert_eq!(
        nq_command_count, nq_export_count,
        "nq.rs has {nq_command_count} `Command::new` site(s) but only \
         {nq_export_count} `.arg(\"export\")` site(s). Every NS → NQ \
         subprocess call must be a read-only `export` verb. If a new \
         read-only NQ verb is required, expand this test deliberately. \
         See docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md."
    );

    let liveness_command_count = liveness_rs.matches("Command::new").count();
    let liveness_export_count = liveness_rs.matches(".arg(\"export\")").count();
    assert_eq!(
        liveness_command_count, liveness_export_count,
        "liveness.rs has {liveness_command_count} `Command::new` site(s) \
         but only {liveness_export_count} `.arg(\"export\")` site(s). \
         Same discipline as nq.rs."
    );

    // 3. NS may not open NQ's SQLite directly. The `--nq-db` path is
    //    passed as a `--db` argument to the `nq` binary; NS code does
    //    not construct a `rusqlite::Connection` against the NQ DB.
    //
    //    Today the only `Connection::open(path)` site in NS is
    //    `crates/nightshiftd/src/store/sqlite.rs:123` — against NS's own
    //    store, which is structurally distinct from NQ's DB.
    //
    //    Forbidden patterns: any use of a variable named like an NQ DB
    //    path as the argument to `Connection::open`.
    for module in &["nq.rs", "liveness.rs", "pipeline.rs", "main.rs", "mvp_a.rs"] {
        let src = ns_src(module);
        for forbidden_var in &["nq_db", "nq_db_path", "nqdb_path"] {
            let needle_owned = format!("Connection::open({forbidden_var}");
            let needle_ref = format!("Connection::open(&{forbidden_var}");
            assert!(
                !src.contains(&needle_owned) && !src.contains(&needle_ref),
                "crates/nightshiftd/src/{module} opens an NQ DB path \
                 (`{forbidden_var}`) via `rusqlite::Connection::open`. \
                 NS must not open NQ's database directly; pass the path \
                 to the `nq` binary instead. See \
                 docs/working/gaps/NQ_NS_CHANNEL_SPLIT_NS_SIDE.md \
                 § \"Forbidden cycle\"."
            );
        }
    }
}
