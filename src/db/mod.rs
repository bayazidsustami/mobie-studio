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
    pub chat_log_path: Option<String>,
    pub yaml_path: Option<String>,
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
                chat_log_path TEXT,
                yaml_path TEXT
            )",
            [],
        ).context("Failed to initialize database schema")?;
        Ok(())
    }

    pub fn insert_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, timestamp, goal, status, chat_log_path, yaml_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.timestamp.to_rfc3339(),
                session.goal,
                session.status,
                session.chat_log_path,
                session.yaml_path,
            ],
        ).context("Failed to insert session into database")?;
        Ok(())
    }

    pub fn get_all_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, goal, status, chat_log_path, yaml_path FROM sessions ORDER BY timestamp DESC"
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
                chat_log_path: row.get(4)?,
                yaml_path: row.get(5)?,
            })
        }).context("Failed to query sessions")?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session.context("Failed to parse session row")?);
        }
        Ok(sessions)
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params![id],
        ).context("Failed to delete session from database")?;
        Ok(())
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
            chat_log_path: Some("/tmp/chat.log".to_string()),
            yaml_path: Some("/tmp/test.yaml".to_string()),
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
}
