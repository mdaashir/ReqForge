//! SQLite-backed document store.
//!
//! Each document is a single row keyed by `doc_id` (UUIDv4). The
//! `state` blob holds the Yrs-encoded state vector — we store the
//! full update log, not just a snapshot, so a new client can hydrate
//! from scratch.

use crate::error::ServerResult;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Db {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl Db {
    pub async fn open(url: &str) -> ServerResult<Self> {
        // url is `sqlite://path?mode=rwc`; we just want `path`
        let path = url
            .strip_prefix("sqlite://")
            .unwrap_or(url)
            .split('?')
            .next()
            .unwrap_or(url);

        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        let conn = Connection::open(path)?;
        Ok(Self {
            conn: std::sync::Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn migrate_async(&self) -> ServerResult<()> {
        // Tiny async wrapper so the binary startup path stays non-blocking.
        // Migration itself is sync (rusqlite); we just dispatch onto a
        // blocking thread to keep the runtime happy.
        let conn = self.clone();
        tokio::task::spawn_blocking(move || conn.migrate())
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("migration join: {e}")))?
    }

    pub async fn upsert_doc_state_async(
        &self,
        doc_id: &str,
        owner: &str,
        state: &[u8],
    ) -> ServerResult<()> {
        let conn = self.clone();
        let doc_id = doc_id.to_string();
        let owner = owner.to_string();
        let state = state.to_vec();
        tokio::task::spawn_blocking(move || conn.upsert_doc_state(&doc_id, &owner, &state))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub async fn load_doc_state_async(&self, doc_id: &str) -> ServerResult<Option<Vec<u8>>> {
        let conn = self.clone();
        let doc_id = doc_id.to_string();
        tokio::task::spawn_blocking(move || conn.load_doc_state(&doc_id))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub async fn list_docs_for_owner_async(&self, owner: &str) -> ServerResult<Vec<DocMeta>> {
        let conn = self.clone();
        let owner = owner.to_string();
        tokio::task::spawn_blocking(move || conn.list_docs_for_owner(&owner))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub async fn get_doc_meta_async(&self, doc_id: &str) -> ServerResult<Option<DocMeta>> {
        let conn = self.clone();
        let doc_id = doc_id.to_string();
        tokio::task::spawn_blocking(move || conn.get_doc_meta(&doc_id))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub async fn delete_doc_async(&self, doc_id: &str) -> ServerResult<bool> {
        let conn = self.clone();
        let doc_id = doc_id.to_string();
        tokio::task::spawn_blocking(move || conn.delete_doc(&doc_id))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub async fn upsert_user_async(&self, user_id: &str, email: &str) -> ServerResult<()> {
        let conn = self.clone();
        let user_id = user_id.to_string();
        let email = email.to_string();
        tokio::task::spawn_blocking(move || conn.upsert_user(&user_id, &email))
            .await
            .map_err(|e| crate::error::ServerError::Internal(format!("db join: {e}")))?
    }

    pub fn migrate(&self) -> ServerResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS docs (
                doc_id       TEXT PRIMARY KEY,
                owner        TEXT NOT NULL,
                state        BLOB NOT NULL,
                created_ms   INTEGER NOT NULL,
                updated_ms   INTEGER NOT NULL,
                byte_size    INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_docs_owner ON docs(owner);

            CREATE TABLE IF NOT EXISTS users (
                user_id      TEXT PRIMARY KEY,
                email        TEXT NOT NULL UNIQUE,
                created_ms   INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_doc_state(
        &self,
        doc_id: &str,
        owner: &str,
        state: &[u8],
    ) -> ServerResult<()> {
        let now = now_ms();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO docs(doc_id, owner, state, created_ms, updated_ms, byte_size)
               VALUES (?1, ?2, ?3, ?4, ?4, ?5)
               ON CONFLICT(doc_id) DO UPDATE SET
                 state = excluded.state,
                 updated_ms = excluded.updated_ms,
                 byte_size = excluded.byte_size"#,
            params![doc_id, owner, state, now, state.len() as i64],
        )?;
        Ok(())
    }

    pub fn load_doc_state(&self, doc_id: &str) -> ServerResult<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        let row: Option<Vec<u8>> = conn
            .query_row(
                "SELECT state FROM docs WHERE doc_id = ?1",
                params![doc_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_docs_for_owner(&self, owner: &str) -> ServerResult<Vec<DocMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT doc_id, owner, created_ms, updated_ms, byte_size
             FROM docs WHERE owner = ?1 ORDER BY updated_ms DESC",
        )?;
        let rows = stmt
            .query_map(params![owner], |r| {
                Ok(DocMeta {
                    doc_id: r.get(0)?,
                    owner: r.get(1)?,
                    created_ms: r.get(2)?,
                    updated_ms: r.get(3)?,
                    byte_size: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_doc_meta(&self, doc_id: &str) -> ServerResult<Option<DocMeta>> {
        let conn = self.conn.lock().unwrap();
        let row: Option<DocMeta> = conn
            .query_row(
                "SELECT doc_id, owner, created_ms, updated_ms, byte_size
                 FROM docs WHERE doc_id = ?1",
                params![doc_id],
                |r| {
                    Ok(DocMeta {
                        doc_id: r.get(0)?,
                        owner: r.get(1)?,
                        created_ms: r.get(2)?,
                        updated_ms: r.get(3)?,
                        byte_size: r.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn delete_doc(&self, doc_id: &str) -> ServerResult<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM docs WHERE doc_id = ?1", params![doc_id])?;
        Ok(n > 0)
    }

    pub fn upsert_user(&self, user_id: &str, email: &str) -> ServerResult<()> {
        let now = now_ms();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO users(user_id, email, created_ms) VALUES (?1, ?2, ?3)
               ON CONFLICT(email) DO UPDATE SET user_id = excluded.user_id"#,
            params![user_id, email, now],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocMeta {
    pub doc_id: String,
    pub owner: String,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub byte_size: i64,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
