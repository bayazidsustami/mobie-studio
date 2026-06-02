use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Session {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub goal: String,
    pub status: String,
    pub summary: Option<String>,
    pub chat_log_path: Option<String>,
    pub yaml_path: Option<String>,
    /// Serialized `Vec<rig::completion::Message>` storing the full LLM
    /// conversation history (user prompts, assistant replies, tool calls,
    /// tool results) so multi-turn sessions can be resumed across prompts.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub rig_history_json: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ChatMessage {
    pub id: Option<i64>,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

pub struct SessionManager {
    conn: Connection,
}

impl SessionManager {
    /// Create a new SessionManager.
    /// This will ensure the parent directory exists and initialize the database schema.
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory {:?}", parent))?;
        }

        let conn = Connection::open(db_path)
            .context("Failed to open SQLite connection")?;
            
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .context("Failed to enable foreign keys")?;

        let manager = Self { conn };
        manager.init_schema()?;
        Ok(manager)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                goal TEXT NOT NULL,
                status TEXT NOT NULL,
                summary TEXT,
                chat_log_path TEXT,
                yaml_path TEXT,
                rig_history_json TEXT
            )",
            [],
        ).context("Failed to initialize database schema")?;

        // Migration: ensure summary column exists (ignore error if already exists)
        let _ = self.conn.execute("ALTER TABLE sessions ADD COLUMN summary TEXT", []);
        // Migration: ensure rig_history_json column exists (added in v0.2 for multi-turn)
        let _ = self.conn.execute("ALTER TABLE sessions ADD COLUMN rig_history_json TEXT", []);

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions (id) ON DELETE CASCADE
            )",
            [],
        ).context("Failed to initialize chat_messages schema")?;

        Ok(())
    }

    pub fn insert_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, timestamp, goal, status, summary, chat_log_path, yaml_path, rig_history_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.id,
                session.timestamp.to_rfc3339(),
                session.goal,
                session.status,
                session.summary,
                session.chat_log_path,
                session.yaml_path,
                session.rig_history_json,
            ],
        ).context("Failed to insert session into database")?;
        Ok(())
    }

    pub fn get_all_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, goal, status, summary, chat_log_path, yaml_path, rig_history_json FROM sessions ORDER BY timestamp DESC"
        ).context("Failed to prepare SELECT statement")?;

        let session_iter = stmt.query_map([], |row| {
            let timestamp_str: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Session {
                id: row.get(0)?,
                timestamp,
                goal: row.get(2)?,
                status: row.get(3)?,
                summary: row.get(4)?,
                chat_log_path: row.get(5)?,
                yaml_path: row.get(6)?,
                rig_history_json: row.get(7)?,
            })
        }).context("Failed to query sessions")?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session.context("Failed to parse session row")?);
        }
        Ok(sessions)
    }

    /// Fetch a single session by ID. Returns `None` if not found.
    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, goal, status, summary, chat_log_path, yaml_path, rig_history_json FROM sessions WHERE id = ?1"
        ).context("Failed to prepare SELECT statement")?;

        let mut iter = stmt.query_map(params![id], |row| {
            let timestamp_str: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Session {
                id: row.get(0)?,
                timestamp,
                goal: row.get(2)?,
                status: row.get(3)?,
                summary: row.get(4)?,
                chat_log_path: row.get(5)?,
                yaml_path: row.get(6)?,
                rig_history_json: row.get(7)?,
            })
        }).context("Failed to query session")?;

        match iter.next() {
            Some(s) => Ok(Some(s.context("Failed to parse session row")?)),
            None => Ok(None),
        }
    }

    /// Update the `rig_history_json` column for a session. Used after each
    /// turn to persist the full LLM conversation history.
    pub fn update_rig_history(&self, id: &str, history_json: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET rig_history_json = ?2 WHERE id = ?1",
            params![id, history_json],
        ).context("Failed to update rig_history_json")?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params![id],
        ).context("Failed to delete session from database")?;
        Ok(())
    }

    pub fn clear_all_sessions(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sessions",
            [],
        ).context("Failed to clear all sessions from database")?;
        Ok(())
    }

    pub fn update_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET timestamp = ?2, goal = ?3, status = ?4, summary = ?5, chat_log_path = ?6, yaml_path = ?7, rig_history_json = ?8 WHERE id = ?1",
            params![
                session.id,
                session.timestamp.to_rfc3339(),
                session.goal,
                session.status,
                session.summary,
                session.chat_log_path,
                session.yaml_path,
                session.rig_history_json,
            ],
        ).context("Failed to update session in database")?;
        Ok(())
    }

    pub fn insert_chat_message(&self, message: &ChatMessage) -> Result<()> {
        self.conn.execute(
            "INSERT INTO chat_messages (session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                message.session_id,
                message.role,
                message.content,
                message.timestamp.to_rfc3339(),
            ],
        ).context("Failed to insert chat message into database")?;
        Ok(())
    }

    pub fn get_chat_messages(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, timestamp FROM chat_messages WHERE session_id = ?1 ORDER BY timestamp ASC"
        ).context("Failed to prepare SELECT statement for chat messages")?;

        let message_iter = stmt.query_map(params![session_id], |row| {
            let timestamp_str: String = row.get(4)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(ChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp,
            })
        }).context("Failed to query chat messages")?;

        let mut messages = Vec::new();
        for message in message_iter {
            messages.push(message.context("Failed to parse chat message row")?);
        }
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_session_crud() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_sessions.db");
        let manager = SessionManager::new(db_path)?;

        let session = Session {
            id: "test-id-123".to_string(),
            timestamp: Utc::now(),
            goal: "Test Goal".to_string(),
            status: "success".to_string(),
            summary: Some("Summary".to_string()),
            chat_log_path: Some("/tmp/chat.log".to_string()),
            yaml_path: Some("/tmp/test.yaml".to_string()),
            rig_history_json: None,
        };

        manager.insert_session(&session)?;
        let sessions = manager.get_all_sessions()?;
        
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
        assert_eq!(sessions[0].goal, session.goal);
        // Compare RFC3339 strings to avoid tiny precision differences in DateTime
        assert_eq!(sessions[0].timestamp.to_rfc3339(), session.timestamp.to_rfc3339());

        manager.delete_session(&session.id)?;
        let sessions = manager.get_all_sessions()?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[test]
    fn test_clear_all_sessions() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_clear_all.db");
        let manager = SessionManager::new(db_path)?;

        for i in 1..=3 {
            manager.insert_session(&Session {
                id: format!("id-{}", i),
                timestamp: Utc::now(),
                goal: format!("Goal {}", i),
                status: "success".to_string(),
                summary: None,
                chat_log_path: None,
                yaml_path: None,
                rig_history_json: None,
            })?;
        }

        let sessions = manager.get_all_sessions()?;
        assert_eq!(sessions.len(), 3);

        manager.clear_all_sessions()?;
        let sessions = manager.get_all_sessions()?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[test]
    fn test_rig_history_round_trip() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_rig_history.db");
        let manager = SessionManager::new(db_path)?;

        let id = "sess_history";
        let json = Some(r#"[{"role":"user","content":[{"type":"text","text":"hi"}]}]"#.to_string());

        manager.insert_session(&Session {
            id: id.to_string(),
            timestamp: Utc::now(),
            goal: "Test".to_string(),
            status: "in_progress".to_string(),
            summary: None,
            chat_log_path: None,
            yaml_path: None,
            rig_history_json: json.clone(),
        })?;

        let loaded = manager.get_session(id)?;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().rig_history_json, json);

        // Update the history (simulating a second turn)
        let updated = Some(r#"[{"role":"user","content":[{"type":"text","text":"hi"}]},{"role":"assistant","content":[{"type":"text","text":"hello"}]}]"#.to_string());
        manager.update_rig_history(id, updated.as_deref())?;

        let reloaded = manager.get_session(id)?.unwrap();
        assert_eq!(reloaded.rig_history_json, updated);

        // Clear
        manager.update_rig_history(id, None)?;
        let cleared = manager.get_session(id)?.unwrap();
        assert_eq!(cleared.rig_history_json, None);

        Ok(())
    }
}
