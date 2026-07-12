//! SQLite-backed storage for telemetry data.

use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Db {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl Db {
    pub async fn open(path: &str) -> anyhow::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let conn = Connection::open(path)?;
        let db = Self {
            conn: std::sync::Arc::new(Mutex::new(conn)),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS usage_events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                app_version TEXT NOT NULL,
                os          TEXT NOT NULL,
                feature     TEXT NOT NULL,
                count       INTEGER NOT NULL,
                window_ts   INTEGER NOT NULL,
                ingested_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS crash_reports (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                app_version TEXT NOT NULL,
                os          TEXT NOT NULL,
                message     TEXT NOT NULL,
                email       TEXT,
                stack       TEXT NOT NULL,
                crashed_at  INTEGER NOT NULL,
                ingested_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE INDEX IF NOT EXISTS idx_usage_feature ON usage_events(feature);
            CREATE INDEX IF NOT EXISTS idx_crash_app_version ON crash_reports(app_version);
            "#,
        )?;
        Ok(())
    }

    pub fn insert_usage_events(
        &self,
        events: &[crate::routes::UsageEvent],
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        for ev in events {
            conn.execute(
                "INSERT INTO usage_events (app_version, os, feature, count, window_ts) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![ev.app_version, ev.os, ev.feature, ev.count, ev.window_ts],
            )?;
        }
        Ok(())
    }

    pub fn insert_crash_report(&self, report: &crate::routes::CrashReport) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO crash_reports (app_version, os, message, email, stack, crashed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                report.app_version,
                report.os,
                report.message,
                report.email,
                serde_json::to_string(&report.stack).unwrap_or_default(),
                report.crashed_at
            ],
        )?;
        Ok(())
    }
}
