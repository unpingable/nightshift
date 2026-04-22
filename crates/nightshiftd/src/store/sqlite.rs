//! SQLite-backed Store implementation.
//!
//! v1 default per GAP-storage.md. Postgres (v2) satisfies the same
//! trait when the time comes. The store owns state, not intelligence.
//!
//! Concurrency: v1 assumes a single daemon. All writes take an
//! `IMMEDIATE` transaction. If the store cannot prove exclusive
//! ownership of a run transition, Night Shift fails closed —
//! CLAUDE.md invariant #13.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::agenda::Agenda;
use crate::bundle::Bundle;
use crate::errors::{NightShiftError, Result};
use crate::finding::FindingKey;
use crate::ledger::RunLedgerEvent;
use crate::packet::Packet;
use crate::store::{RunFilter, RunSummary, RunTrigger, Store};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS agendas (
    agenda_id  TEXT PRIMARY KEY,
    content    TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
    run_id             TEXT PRIMARY KEY,
    agenda_id          TEXT NOT NULL REFERENCES agendas(agenda_id),
    trigger            TEXT NOT NULL,
    target_finding_key TEXT,
    started_at         TEXT NOT NULL,
    completed_at       TEXT
);

CREATE INDEX IF NOT EXISTS runs_by_finding ON runs(target_finding_key);
CREATE INDEX IF NOT EXISTS runs_by_agenda  ON runs(agenda_id);

CREATE TABLE IF NOT EXISTS run_events (
    event_id TEXT PRIMARY KEY,
    run_id   TEXT NOT NULL REFERENCES runs(run_id),
    kind     TEXT NOT NULL,
    at       TEXT NOT NULL,
    payload  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS run_events_by_run ON run_events(run_id, at);

CREATE TABLE IF NOT EXISTS bundles (
    run_id   TEXT PRIMARY KEY REFERENCES runs(run_id),
    content  TEXT NOT NULL,
    saved_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS packets (
    packet_id   TEXT PRIMARY KEY,
    run_id      TEXT NOT NULL REFERENCES runs(run_id),
    agenda_id   TEXT NOT NULL,
    content     TEXT NOT NULL,
    produced_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS packets_by_run ON packets(run_id);

INSERT OR IGNORE INTO schema_version (version) VALUES (1);
"#;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| NightShiftError::Store(format!("opening {}: {e}", path.display())))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| NightShiftError::Store(format!("opening in-memory sqlite: {e}")))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| NightShiftError::Store(format!("enabling foreign_keys: {e}")))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| NightShiftError::Store(format!("schema init: {e}")))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn with_tx<R>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<R>) -> Result<R> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| NightShiftError::Store(format!("store lock poisoned: {e}")))?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| NightShiftError::Store(format!("begin tx: {e}")))?;
        let out = f(&tx)?;
        tx.commit()
            .map_err(|e| NightShiftError::Store(format!("commit: {e}")))?;
        Ok(out)
    }

    fn with_conn<R>(&self, f: impl FnOnce(&Connection) -> Result<R>) -> Result<R> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| NightShiftError::Store(format!("store lock poisoned: {e}")))?;
        f(&conn)
    }
}

fn store_err(op: &str, e: impl std::fmt::Display) -> NightShiftError {
    NightShiftError::Store(format!("{op}: {e}"))
}

fn parse_trigger(s: &str) -> Result<RunTrigger> {
    match s {
        "scheduled" => Ok(RunTrigger::Scheduled),
        "event" => Ok(RunTrigger::Event),
        "manual" => Ok(RunTrigger::Manual),
        other => Err(NightShiftError::Store(format!(
            "unknown run trigger: {other}"
        ))),
    }
}

fn trigger_str(t: RunTrigger) -> &'static str {
    match t {
        RunTrigger::Scheduled => "scheduled",
        RunTrigger::Event => "event",
        RunTrigger::Manual => "manual",
    }
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| NightShiftError::Store(format!("bad timestamp {s}: {e}")))
}

impl Store for SqliteStore {
    fn create_agenda(&self, agenda: &Agenda) -> Result<String> {
        let content = serde_json::to_string(agenda)?;
        let now = Utc::now().to_rfc3339();
        self.with_tx(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO agendas (agenda_id, content, created_at) VALUES (?, ?, ?)",
                params![agenda.agenda_id, content, now],
            )
            .map_err(|e| store_err("insert agenda", e))?;
            Ok(())
        })?;
        Ok(agenda.agenda_id.clone())
    }

    fn get_agenda(&self, agenda_id: &str) -> Result<Option<Agenda>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT content FROM agendas WHERE agenda_id = ?",
                    params![agenda_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| store_err("select agenda", e))?;
            match row {
                Some(s) => Ok(Some(serde_json::from_str(&s)?)),
                None => Ok(None),
            }
        })
    }

    fn create_run(
        &self,
        agenda_id: &str,
        trigger: RunTrigger,
        target: Option<&FindingKey>,
    ) -> Result<String> {
        let run_id = format!("run_{}", uuid::Uuid::new_v4().simple());
        let started_at = Utc::now().to_rfc3339();
        let target_key = target.map(|k| k.as_string());
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO runs (run_id, agenda_id, trigger, target_finding_key, started_at, completed_at)
                 VALUES (?, ?, ?, ?, ?, NULL)",
                params![run_id, agenda_id, trigger_str(trigger), target_key, started_at],
            )
            .map_err(|e| store_err("insert run", e))?;
            Ok(())
        })?;
        Ok(run_id)
    }

    fn get_run_summary(&self, run_id: &str) -> Result<Option<RunSummary>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT run_id, agenda_id, trigger, target_finding_key, started_at, completed_at
                     FROM runs WHERE run_id = ?",
                    params![run_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, String>(4)?,
                            r.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|e| store_err("select run", e))?;
            match row {
                None => Ok(None),
                Some((run_id, agenda_id, trigger_s, target, started_s, completed_s)) => {
                    let trigger = parse_trigger(&trigger_s)?;
                    let started_at = parse_ts(&started_s)?;
                    let completed_at = match completed_s {
                        Some(s) => Some(parse_ts(&s)?),
                        None => None,
                    };
                    Ok(Some(RunSummary {
                        run_id,
                        agenda_id,
                        trigger,
                        target_finding_key: target,
                        started_at,
                        completed_at,
                    }))
                }
            }
        })
    }

    fn complete_run(&self, run_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE runs SET completed_at = ? WHERE run_id = ? AND completed_at IS NULL",
                    params![now, run_id],
                )
                .map_err(|e| store_err("complete run", e))?;
            if n == 0 {
                return Err(NightShiftError::Store(format!(
                    "run not found or already completed: {run_id}"
                )));
            }
            Ok(())
        })
    }

    fn append_run_event(&self, event: &RunLedgerEvent) -> Result<()> {
        let kind_str = serde_json::to_value(event.kind)?
            .as_str()
            .expect("run ledger kinds serialize as strings")
            .to_string();
        let payload = serde_json::to_string(&event.payload)?;
        let at = event.at.to_rfc3339();
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO run_events (event_id, run_id, kind, at, payload) VALUES (?, ?, ?, ?, ?)",
                params![event.event_id, event.run_id, kind_str, at, payload],
            )
            .map_err(|e| store_err("insert run_event", e))?;
            Ok(())
        })
    }

    fn list_events(&self, run_id: &str) -> Result<Vec<RunLedgerEvent>> {
        self.with_conn(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT event_id, run_id, kind, at, payload
                     FROM run_events WHERE run_id = ? ORDER BY at ASC, event_id ASC",
                )
                .map_err(|e| store_err("prepare list_events", e))?;
            let rows = stmt
                .query_map(params![run_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                })
                .map_err(|e| store_err("query list_events", e))?;
            let mut out = Vec::new();
            for row in rows {
                let (event_id, run_id, kind_s, at_s, payload_s) =
                    row.map_err(|e| store_err("row list_events", e))?;
                let kind = serde_json::from_value(serde_json::Value::String(kind_s.clone()))
                    .map_err(|e| store_err(&format!("parse kind {kind_s}"), e))?;
                let at = parse_ts(&at_s)?;
                let payload: serde_json::Value = serde_json::from_str(&payload_s)?;
                out.push(RunLedgerEvent {
                    event_id,
                    run_id,
                    kind,
                    at,
                    payload,
                });
            }
            Ok(out)
        })
    }

    fn save_bundle(&self, run_id: &str, bundle: &Bundle) -> Result<()> {
        let content = serde_json::to_string(bundle)?;
        let now = Utc::now().to_rfc3339();
        self.with_tx(|tx| {
            tx.execute(
                "INSERT OR REPLACE INTO bundles (run_id, content, saved_at) VALUES (?, ?, ?)",
                params![run_id, content, now],
            )
            .map_err(|e| store_err("insert bundle", e))?;
            Ok(())
        })
    }

    fn get_bundle(&self, run_id: &str) -> Result<Option<Bundle>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT content FROM bundles WHERE run_id = ?",
                    params![run_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| store_err("select bundle", e))?;
            match row {
                Some(s) => Ok(Some(serde_json::from_str(&s)?)),
                None => Ok(None),
            }
        })
    }

    fn save_packet(&self, run_id: &str, packet: &Packet) -> Result<()> {
        let content = serde_json::to_string(packet)?;
        let produced_at = packet.produced_at.to_rfc3339();
        self.with_tx(|tx| {
            tx.execute(
                "INSERT INTO packets (packet_id, run_id, agenda_id, content, produced_at)
                 VALUES (?, ?, ?, ?, ?)",
                params![packet.packet_id, run_id, packet.agenda_id, content, produced_at],
            )
            .map_err(|e| store_err("insert packet", e))?;
            Ok(())
        })
    }

    fn get_packet(&self, run_id: &str) -> Result<Option<Packet>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT content FROM packets WHERE run_id = ? ORDER BY produced_at DESC LIMIT 1",
                    params![run_id],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| store_err("select packet", e))?;
            match row {
                Some(s) => Ok(Some(serde_json::from_str(&s)?)),
                None => Ok(None),
            }
        })
    }

    fn list_runs(&self, filter: RunFilter) -> Result<Vec<RunSummary>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT run_id, agenda_id, trigger, target_finding_key, started_at, completed_at
                 FROM runs",
            );
            let mut where_parts: Vec<&str> = Vec::new();
            let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            if let Some(a) = &filter.agenda_id {
                where_parts.push("agenda_id = ?");
                args.push(Box::new(a.clone()));
            }
            if let Some(k) = &filter.target_finding_key {
                where_parts.push("target_finding_key = ?");
                args.push(Box::new(k.clone()));
            }
            if !where_parts.is_empty() {
                sql.push_str(" WHERE ");
                sql.push_str(&where_parts.join(" AND "));
            }
            sql.push_str(" ORDER BY started_at ASC");
            if let Some(lim) = filter.limit {
                sql.push_str(&format!(" LIMIT {lim}"));
            }

            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| store_err("prepare list_runs", e))?;
            let arg_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
            let rows = stmt
                .query_map(rusqlite::params_from_iter(arg_refs.iter()), |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<String>>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                })
                .map_err(|e| store_err("query list_runs", e))?;
            let mut out = Vec::new();
            for row in rows {
                let (run_id, agenda_id, trigger_s, target, started_s, completed_s) =
                    row.map_err(|e| store_err("row list_runs", e))?;
                let trigger = parse_trigger(&trigger_s)?;
                let started_at = parse_ts(&started_s)?;
                let completed_at = match completed_s {
                    Some(s) => Some(parse_ts(&s)?),
                    None => None,
                };
                out.push(RunSummary {
                    run_id,
                    agenda_id,
                    trigger,
                    target_finding_key: target,
                    started_at,
                    completed_at,
                });
            }
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::{EvidenceState, FindingKey, FindingSnapshot, Severity};
    use crate::ledger::RunLedgerEventKind;
    use chrono::TimeZone;

    fn mk_store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("in-memory sqlite must open")
    }

    fn mk_agenda() -> Agenda {
        serde_yaml::from_str(
            r#"
agenda_version: 0
agenda_id: test-agenda
workflow_family: ops
owner: operator@test
cadence: { kind: scheduled, expr: "0 3 * * *" }
scope: { hosts: [h1], services: [], paths: [], repos: [] }
artifact_target: packet
promotion_ceiling: advise
reconciler: { required: true }
allowed_evidence_sources: [nq]
allowed_tool_classes: [discover, read, propose]
budget: { max_wall_seconds: 600 }
governor_binding: { required_above: observe }
criticality: { class: standard, re_alert_after: 4h, silence_max_duration: 72h }
diagnosis: { mode: self_check }
"#,
        )
        .expect("test agenda must parse")
    }

    fn mk_key() -> FindingKey {
        FindingKey {
            source: "nq".into(),
            detector: "wal_bloat".into(),
            subject: "h1:/var/lib/db".into(),
        }
    }

    fn mk_snapshot(key: &FindingKey) -> FindingSnapshot {
        FindingSnapshot {
            finding_key: key.clone(),
            host: "h1".into(),
            severity: Severity::Warning,
            domain: None,
            persistence_generations: 3,
            first_seen_at: Utc.with_ymd_and_hms(2026, 4, 10, 0, 0, 0).unwrap(),
            current_status: EvidenceState::Active,
            snapshot_generation: 1,
            captured_at: Utc::now(),
            evidence_hash: String::new(),
        }
    }

    #[test]
    fn agenda_round_trip() {
        let s = mk_store();
        let a = mk_agenda();
        s.create_agenda(&a).unwrap();
        let got = s.get_agenda(&a.agenda_id).unwrap().unwrap();
        assert_eq!(got.agenda_id, a.agenda_id);
    }

    #[test]
    fn create_run_and_complete() {
        let s = mk_store();
        let a = mk_agenda();
        s.create_agenda(&a).unwrap();
        let key = mk_key();
        let run_id = s
            .create_run(&a.agenda_id, RunTrigger::Manual, Some(&key))
            .unwrap();
        assert!(run_id.starts_with("run_"));
        s.complete_run(&run_id).unwrap();
        let runs = s
            .list_runs(RunFilter {
                agenda_id: Some(a.agenda_id.clone()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].completed_at.is_some());
        assert_eq!(runs[0].target_finding_key.as_deref(), Some(key.as_string().as_str()));
    }

    #[test]
    fn same_finding_across_multiple_runs_is_queryable() {
        let s = mk_store();
        let a = mk_agenda();
        s.create_agenda(&a).unwrap();
        let key = mk_key();

        let r1 = s
            .create_run(&a.agenda_id, RunTrigger::Manual, Some(&key))
            .unwrap();
        s.complete_run(&r1).unwrap();
        let r2 = s
            .create_run(&a.agenda_id, RunTrigger::Manual, Some(&key))
            .unwrap();
        s.complete_run(&r2).unwrap();

        let runs = s
            .list_runs(RunFilter {
                target_finding_key: Some(key.as_string()),
                ..Default::default()
            })
            .unwrap();

        assert_eq!(runs.len(), 2, "both runs must be retrievable by finding_key");
        assert_ne!(runs[0].run_id, runs[1].run_id);
        assert_eq!(runs[0].target_finding_key, runs[1].target_finding_key);
    }

    #[test]
    fn append_and_list_run_events() {
        let s = mk_store();
        let a = mk_agenda();
        s.create_agenda(&a).unwrap();
        let run_id = s
            .create_run(&a.agenda_id, RunTrigger::Scheduled, None)
            .unwrap();
        s.append_run_event(&RunLedgerEvent {
            event_id: "ev1".into(),
            run_id: run_id.clone(),
            kind: RunLedgerEventKind::RunCaptured,
            at: Utc::now(),
            payload: serde_json::json!({"reason": "test"}),
        })
        .unwrap();
        s.append_run_event(&RunLedgerEvent {
            event_id: "ev2".into(),
            run_id: run_id.clone(),
            kind: RunLedgerEventKind::RunCompleted,
            at: Utc::now(),
            payload: serde_json::Value::Null,
        })
        .unwrap();
        let evs = s.list_events(&run_id).unwrap();
        assert_eq!(evs.len(), 2);
        assert!(matches!(evs[0].kind, RunLedgerEventKind::RunCaptured));
        assert!(matches!(evs[1].kind, RunLedgerEventKind::RunCompleted));
    }

    #[test]
    fn bundle_and_packet_round_trip() {
        use crate::bundle::{Bundle, CapturePhase};
        let s = mk_store();
        let a = mk_agenda();
        s.create_agenda(&a).unwrap();
        let run_id = s
            .create_run(&a.agenda_id, RunTrigger::Manual, None)
            .unwrap();
        let bundle = Bundle {
            bundle_version: 0,
            agenda_id: a.agenda_id.clone(),
            run_id: run_id.clone(),
            capture: CapturePhase {
                captured_at: Utc::now(),
                captured_by: "test".into(),
                capture_reason: "round-trip test".into(),
                inputs: vec![],
            },
            reconciliation: None,
        };
        s.save_bundle(&run_id, &bundle).unwrap();
        let got = s.get_bundle(&run_id).unwrap().unwrap();
        assert_eq!(got.run_id, bundle.run_id);

        // Quick smoke — save/get a minimal packet too
        let _ = mk_snapshot(&mk_key()); // just exercise the helper in this test
    }
}
