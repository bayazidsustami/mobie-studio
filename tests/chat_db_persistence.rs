use anyhow::Result;
use chrono::Utc;
use mobie::db::{ChatMessage, SessionManager};
use tempfile::tempdir;

#[test]
fn test_chat_message_persistence() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_chat.db");
    let manager = SessionManager::new(db_path)?;

    let session_id = "session-1";
    let session = mobie::db::Session {
        id: session_id.to_string(),
        timestamp: Utc::now(),
        goal: "Test Goal".to_string(),
        status: "in_progress".to_string(),
        summary: None,
        chat_log_path: None,
        yaml_path: None,
    };
    manager.insert_session(&session)?;

    let message = ChatMessage {
        id: None,
        session_id: session_id.to_string(),
        role: "user".to_string(),
        content: "Hello, Agent!".to_string(),
        timestamp: Utc::now(),
    };

    manager.insert_chat_message(&message)?;
    let messages = manager.get_chat_messages(session_id)?;
    
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, message.content);
    assert_eq!(messages[0].role, message.role);
    assert_eq!(messages[0].session_id, message.session_id);

    // Test cascading deletion
    manager.delete_session(session_id)?;
    let messages_after_delete = manager.get_chat_messages(session_id)?;
    assert_eq!(messages_after_delete.len(), 0);

    Ok(())
}
