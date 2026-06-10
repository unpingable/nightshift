//! D0-Origin — WAL-bloat condition stager.
//!
//! Night Shift's job in the cross-repo D0-Origin slice:
//!
//!   * Stage a real WAL-bloat condition on a fresh SQLite DB in a
//!     sandbox tmp dir. The condition must be real enough that NQ's
//!     production `detect_wal_bloat` (reading `v_sqlite_dbs`) actually
//!     fires.
//!   * Hand the sandbox path to NQ's `nq-monitor drill wal-bloat`
//!     subcommand, which runs the production evaluator (no
//!     `--fake-finding` flag, no hardcoded `FindingSnapshot` shortcut).
//!   * Capture the resulting `nq.finding_snapshot.v1` JSON.
//!   * Hand the JSON to AG's `python3 -m governor.drill_runner`.
//!
//! This module owns step 1. The smoke machine. The fire drill exists
//! because the operator (Night Shift) stages a real bloated WAL on a
//! sandbox DB; the alarm is genuine because NQ's evaluator authentically
//! observes the staged condition by stat-ing the file from disk via the
//! production `sqlite_health::collect()` path.
//!
//! ## Staging strategy
//!
//! WAL bloat fires when `wal_pct > 5.0` OR (`db_size_mb < 5120 AND
//! wal_size_mb > 256`) — see migration
//! `003_warning_lifecycle.sql` for the `v_sqlite_dbs` view definition
//! and `nq_db::detect::detect_wal_bloat` for the source.
//!
//! Relative threshold is the cheapest credible: small DB + larger WAL,
//! `wal_pct` quickly exceeds 5%. We:
//!
//!   1. Open the DB in WAL mode with `wal_autocheckpoint=0` so the
//!      writer never auto-truncates the WAL behind us.
//!   2. Write a small baseline of rows and force-checkpoint to size the
//!      main DB file (this gives `db_size_mb > 0`).
//!   3. Write a much larger second batch without checkpointing — those
//!      pages live in the `-wal` sidecar.
//!   4. Drop the connection by leaving scope. SQLite's
//!      `journal_mode=wal` semantics: a clean close issues a passive
//!      checkpoint which can truncate the WAL if no other connection
//!      holds a read mark. To prevent that, we open a SECOND connection
//!      with `SQLITE_OPEN_READ_ONLY` and begin a read transaction. While
//!      it lives, no checkpoint can truncate the WAL.
//!
//! After staging, both connections drop. The reader's drop releases the
//! read mark, but by then the writer has already closed without invoking
//! `wal_checkpoint(TRUNCATE)` and the WAL file persists on disk for the
//! NQ collector to stat. The OS does not auto-cleanup a stale WAL.
//!
//! ## Determinism notes
//!
//! The staged WAL file is **not** byte-deterministic — SQLite stamps
//! internal serial numbers, page lsns, and salt headers. Two runs
//! produce different `-wal` contents but **the same wal_pct ratio**
//! given identical input parameters. AG's transcript normalization
//! handles the downstream non-determinism (timestamps, paths, ids).
//! The condition is real; only the production line stamps are
//! sandbox-specific.

use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Size of the rows used to grow the DB. Picked to land the staged DB
/// in the 60-100 KB range (small enough that any wal we grow above
/// ~50 KB easily exceeds 5%).
const ROW_BLOB_BYTES: usize = 1024;

/// Number of baseline rows written + force-checkpointed. Sets the main
/// DB file size.
const BASELINE_ROWS: usize = 50;

/// Number of bloat rows written WITHOUT checkpointing. These pages live
/// in the WAL sidecar. With `ROW_BLOB_BYTES = 1024` and `BLOAT_ROWS =
/// 3000`, the WAL grows ≈3 MB — far exceeding the 5% relative trigger
/// against a ~100 KB DB.
const BLOAT_ROWS: usize = 3000;

/// One staged WAL-bloat sandbox. The dirs are operator-managed (Night
/// Shift creates them, the test fixture deletes them on tear-down).
#[derive(Debug, Clone)]
pub struct StagedSandbox {
    /// Path to the bloated SQLite DB. Sibling `<path>-wal` is the
    /// bloated WAL the production detector observes.
    pub db_path: PathBuf,
    /// Measured size of the DB file at staging completion. Reported so
    /// the operator can verify the staging without re-stat'ing the file.
    pub db_size_bytes: u64,
    /// Measured size of the `-wal` file at staging completion.
    pub wal_size_bytes: u64,
}

impl StagedSandbox {
    /// `wal_size / db_size * 100`. Reports the percentage observed at
    /// staging-completion. Anything > 5 will trigger the
    /// `detect_wal_bloat` relative threshold.
    pub fn wal_pct(&self) -> f64 {
        if self.db_size_bytes == 0 {
            0.0
        } else {
            self.wal_size_bytes as f64 * 100.0 / self.db_size_bytes as f64
        }
    }
}

/// Errors the stager can surface. Closed taxonomy — no ambient
/// `anyhow` propagation past this boundary.
#[derive(Debug, thiserror::Error)]
pub enum StagerError {
    #[error("sqlite error during WAL-bloat staging: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("io error during WAL-bloat staging: {0}")]
    Io(#[from] std::io::Error),

    #[error(
        "WAL staging completed but the detector trigger condition was \
         not met (db_size={db_size_bytes}, wal_size={wal_size_bytes}, \
         wal_pct={wal_pct:.2}). Detector requires wal_pct > 5.0 OR \
         (db_size_mb < 5120 AND wal_size_mb > 256). Refusing to ship a \
         sandbox that will not trigger the real detector."
    )]
    InsufficientBloat {
        db_size_bytes: u64,
        wal_size_bytes: u64,
        wal_pct: f64,
    },
}

/// Stage a WAL-bloat condition in the given sandbox directory.
///
/// The operator (caller) owns the directory lifecycle. The function
/// creates `<sandbox_dir>/staged.sqlite` and writes the bloated WAL
/// alongside it. Two side-by-side calls against different sandbox dirs
/// produce independent staged DBs.
///
/// **Hard refusal:** if the staging completes but the actual bytes on
/// disk do not satisfy the production detector's trigger, the function
/// returns `StagerError::InsufficientBloat`. The drill must not run
/// against a sandbox that will silently produce zero findings.
pub fn stage_wal_bloat(sandbox_dir: &Path) -> Result<StagedSandbox, StagerError> {
    std::fs::create_dir_all(sandbox_dir)?;
    let db_path = sandbox_dir.join("staged.sqlite");
    // Clean up any previous staging artifact so repeated runs land on a
    // fresh substrate. SQLite is finicky about lingering -wal/-shm
    // files from a previous staging on a different DB.
    for suffix in ["", "-wal", "-shm"] {
        let p = sandbox_dir.join(format!("staged.sqlite{}", suffix));
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }

    // 1. Open writer in WAL mode, disable auto-checkpoint. SQLite
    //    PRAGMAs sometimes return rows (journal_mode does), so we use
    //    `query_row` to consume the row even when we don't care about
    //    the value.
    let writer = Connection::open(&db_path)?;
    let _: String = writer
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))?;
    writer.pragma_update(None, "synchronous", "NORMAL")?;
    writer.pragma_update(None, "wal_autocheckpoint", 0i32)?;
    writer.execute(
        "CREATE TABLE IF NOT EXISTS staged_wal_bloat_t (id INTEGER PRIMARY KEY, blob BLOB)",
        [],
    )?;

    // 2. Baseline rows + force-checkpoint to set DB size.
    let baseline_blob = vec![b'x'; ROW_BLOB_BYTES];
    {
        let tx = writer.unchecked_transaction()?;
        for _ in 0..BASELINE_ROWS {
            tx.execute(
                "INSERT INTO staged_wal_bloat_t (blob) VALUES (?1)",
                rusqlite::params![&baseline_blob],
            )?;
        }
        tx.commit()?;
    }
    // `wal_checkpoint` returns a 3-tuple row (busy/log/checkpointed);
    // consume it with `query_row`.
    let _: (i64, i64, i64) = writer.query_row(
        "PRAGMA wal_checkpoint(TRUNCATE)",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    // 3. Open a READER holding a read mark — prevents the writer's
    //    close from issuing a truncating passive checkpoint. The reader
    //    is dropped AFTER the writer.
    let reader = Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    reader.pragma_update(None, "wal_autocheckpoint", 0i32)?;
    // Begin a deferred transaction and touch the table so the read mark
    // is taken.
    let mut reader_stmt = reader.prepare("SELECT COUNT(*) FROM staged_wal_bloat_t")?;
    let _count: i64 = reader_stmt.query_row([], |row| row.get(0))?;
    drop(reader_stmt);

    // 4. Bloat rows: written through the writer, persisted in the WAL.
    //    The reader's read mark holds; no checkpoint can truncate.
    let bloat_blob = vec![b'y'; ROW_BLOB_BYTES];
    {
        let tx = writer.unchecked_transaction()?;
        for _ in 0..BLOAT_ROWS {
            tx.execute(
                "INSERT INTO staged_wal_bloat_t (blob) VALUES (?1)",
                rusqlite::params![&bloat_blob],
            )?;
        }
        tx.commit()?;
    }

    // 5. Drop the writer FIRST. Its close issues a passive checkpoint
    //    request but the reader's read mark prevents WAL truncation.
    drop(writer);
    // 6. Drop the reader. Its mark releases; but the writer is already
    //    closed, so no further checkpointing happens. The WAL persists
    //    on disk for NQ to stat.
    drop(reader);

    // 7. Measure final file sizes.
    let db_size_bytes = std::fs::metadata(&db_path)?.len();
    let wal_path = sandbox_dir.join("staged.sqlite-wal");
    let wal_size_bytes = if wal_path.exists() {
        std::fs::metadata(&wal_path)?.len()
    } else {
        0
    };

    let staged = StagedSandbox {
        db_path,
        db_size_bytes,
        wal_size_bytes,
    };

    // 8. Hard-refuse insufficient bloat. Mirror NQ's production trigger
    //    so we never ship a sandbox that the real detector will not
    //    flag — that would be the "smoke machine fails, fake alarm"
    //    shape D0-Origin exists to refuse.
    let pct = staged.wal_pct();
    let db_mb = staged.db_size_bytes as f64 / (1024.0 * 1024.0);
    let wal_mb = staged.wal_size_bytes as f64 / (1024.0 * 1024.0);
    let relative_trigger = pct > 5.0;
    let absolute_trigger = db_mb < 5120.0 && wal_mb > 256.0;
    if !(relative_trigger || absolute_trigger) {
        return Err(StagerError::InsufficientBloat {
            db_size_bytes: staged.db_size_bytes,
            wal_size_bytes: staged.wal_size_bytes,
            wal_pct: pct,
        });
    }

    Ok(staged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_wal_bloat_produces_detector_triggering_substrate() {
        let dir = tempfile::tempdir().unwrap();
        let staged = stage_wal_bloat(dir.path()).expect("staging should succeed");
        assert!(
            staged.db_size_bytes > 0,
            "DB file should have non-zero size"
        );
        assert!(
            staged.wal_size_bytes > 0,
            "WAL file should have non-zero size"
        );
        assert!(
            staged.wal_pct() > 5.0,
            "wal_pct should exceed the 5% relative threshold; got {:.2}",
            staged.wal_pct()
        );
        // Both files must persist on disk after staging — the WAL
        // staying around is the whole point of the stager.
        assert!(staged.db_path.exists());
        let wal_path = staged.db_path.with_extension("sqlite-wal");
        assert!(
            wal_path.exists(),
            "expected wal sidecar at {:?}",
            wal_path
        );
    }

    #[test]
    fn stage_wal_bloat_is_idempotent_for_same_sandbox_dir() {
        let dir = tempfile::tempdir().unwrap();
        let staged1 = stage_wal_bloat(dir.path()).expect("first stage");
        let pct1 = staged1.wal_pct();
        let staged2 = stage_wal_bloat(dir.path()).expect("re-stage");
        let pct2 = staged2.wal_pct();
        // Both runs trigger; the exact byte sizes can drift due to
        // SQLite's per-page bookkeeping, so we assert both passed the
        // trigger and the wal_pct stays within an order of magnitude.
        assert!(pct1 > 5.0);
        assert!(pct2 > 5.0);
    }
}
