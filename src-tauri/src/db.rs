//! SQLite persistence: conversations, messages, licence acceptances, settings.
//! Schema changes go in `MIGRATIONS` (append only; never edit an applied one).

use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE conversations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        model TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
     );
     CREATE TABLE messages (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
        role TEXT NOT NULL CHECK (role IN ('system','user','assistant')),
        content TEXT NOT NULL,
        created_at INTEGER NOT NULL
     );
     CREATE INDEX idx_messages_conv ON messages(conversation_id, id);
     CREATE TABLE license_acceptances (
        license_key TEXT PRIMARY KEY,
        model_tag TEXT NOT NULL,
        accepted_at INTEGER NOT NULL
     );
     CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
];

#[derive(Debug, Clone, Serialize)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

pub struct Db(Mutex<Connection>);

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| crate::error::AppError::Database(format!("create {}: {e}", dir.display())))?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> AppResult<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(&format!("BEGIN; {sql}; PRAGMA user_version = {}; COMMIT;", i + 1))?;
        }
        Ok(Db(Mutex::new(conn)))
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn create_conversation(&self, model: &str, title: &str) -> AppResult<Conversation> {
        let c = self.conn();
        let t = now();
        c.execute(
            "INSERT INTO conversations (title, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![title, model, t],
        )?;
        Ok(Conversation { id: c.last_insert_rowid(), title: title.into(), model: model.into(), created_at: t, updated_at: t })
    }

    pub fn list_conversations(&self) -> AppResult<Vec<Conversation>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT id, title, model, created_at, updated_at FROM conversations ORDER BY updated_at DESC")?;
        let rows = st.query_map([], |r| {
            Ok(Conversation { id: r.get(0)?, title: r.get(1)?, model: r.get(2)?, created_at: r.get(3)?, updated_at: r.get(4)? })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get_conversation(&self, id: i64) -> AppResult<Option<Conversation>> {
        let c = self.conn();
        Ok(c.query_row(
            "SELECT id, title, model, created_at, updated_at FROM conversations WHERE id = ?1",
            [id],
            |r| Ok(Conversation { id: r.get(0)?, title: r.get(1)?, model: r.get(2)?, created_at: r.get(3)?, updated_at: r.get(4)? }),
        )
        .optional()?)
    }

    pub fn rename_conversation(&self, id: i64, title: &str) -> AppResult<()> {
        self.conn().execute("UPDATE conversations SET title = ?1 WHERE id = ?2", params![title, id])?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: i64) -> AppResult<()> {
        self.conn().execute("DELETE FROM conversations WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn add_message(&self, conversation_id: i64, role: &str, content: &str) -> AppResult<Message> {
        let c = self.conn();
        let t = now();
        c.execute(
            "INSERT INTO messages (conversation_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![conversation_id, role, content, t],
        )?;
        let id = c.last_insert_rowid();
        c.execute("UPDATE conversations SET updated_at = ?1 WHERE id = ?2", params![t, conversation_id])?;
        Ok(Message { id, conversation_id, role: role.into(), content: content.into(), created_at: t })
    }

    pub fn messages(&self, conversation_id: i64) -> AppResult<Vec<Message>> {
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT id, conversation_id, role, content, created_at FROM messages WHERE conversation_id = ?1 ORDER BY id",
        )?;
        let rows = st.query_map([conversation_id], |r| {
            Ok(Message { id: r.get(0)?, conversation_id: r.get(1)?, role: r.get(2)?, content: r.get(3)?, created_at: r.get(4)? })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn accept_license(&self, license_key: &str, model_tag: &str) -> AppResult<()> {
        self.conn().execute(
            "INSERT OR REPLACE INTO license_acceptances (license_key, model_tag, accepted_at) VALUES (?1, ?2, ?3)",
            params![license_key, model_tag, now()],
        )?;
        Ok(())
    }

    pub fn license_accepted(&self, license_key: &str) -> AppResult<bool> {
        Ok(self
            .conn()
            .query_row("SELECT 1 FROM license_acceptances WHERE license_key = ?1", [license_key], |_| Ok(()))
            .optional()?
            .is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_roundtrip_and_cascade() {
        let db = Db::open_in_memory().unwrap();
        let c = db.create_conversation("llama3.2:3b", "Hello").unwrap();
        db.add_message(c.id, "user", "hi").unwrap();
        db.add_message(c.id, "assistant", "hello!").unwrap();
        assert_eq!(db.messages(c.id).unwrap().len(), 2);
        assert_eq!(db.list_conversations().unwrap().len(), 1);
        db.delete_conversation(c.id).unwrap();
        assert!(db.messages(c.id).unwrap().is_empty());
        assert!(db.get_conversation(c.id).unwrap().is_none());
    }

    #[test]
    fn rejects_bad_role() {
        let db = Db::open_in_memory().unwrap();
        let c = db.create_conversation("m", "t").unwrap();
        assert!(db.add_message(c.id, "tool; DROP", "x").is_err());
    }

    #[test]
    fn license_acceptance() {
        let db = Db::open_in_memory().unwrap();
        assert!(!db.license_accepted("Gemma Terms of Use").unwrap());
        db.accept_license("Gemma Terms of Use", "gemma3:4b").unwrap();
        assert!(db.license_accepted("Gemma Terms of Use").unwrap());
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("baseio-test-{}", std::process::id()));
        let path = dir.join("t.db");
        drop(Db::open(&path).unwrap());
        let db = Db::open(&path).unwrap();
        db.create_conversation("m", "t").unwrap();
        drop(db);
        let _ = std::fs::remove_dir_all(dir);
    }
}
