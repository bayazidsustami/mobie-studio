//! End-to-end test for the multi-turn session lifecycle.
//!
//! Simulates a two-turn session without hitting a real LLM:
//! 1. Create a session in the DB (turn 1)
//! 2. Persist a rig history JSON blob (the LLM's prior turn)
//! 3. Reload the session and append a new turn
//! 4. Verify the rig history grows monotonically and contains both turns

use anyhow::Result;
use chrono::Utc;
use mobie::db::{Session, SessionManager};
use rig::completion::Message;
use rig::message::{AssistantContent, UserContent};
use rig::one_or_many::OneOrMany;
use tempfile::tempdir;

fn make_user_msg(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    }
}

fn make_assistant_msg(text: &str) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text(text)),
    }
}

#[test]
fn test_multi_turn_session_history_grows() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_multiturn.db");
    let manager = SessionManager::new(db_path)?;

    let session_id = "sess_multi_turn";
    manager.insert_session(&Session {
        id: session_id.to_string(),
        timestamp: Utc::now(),
        goal: "open settings then change theme".to_string(),
        status: "in_progress".to_string(),
        summary: None,
        chat_log_path: None,
        yaml_path: None,
        rig_history_json: None,
    })?;

    // Simulate turn 1 finishing: build a small rig history vec and persist.
    let mut turn1_history: Vec<Message> = Vec::new();
    turn1_history.push(make_user_msg("[Auto-observed] home screen\n\nUser request: open settings"));
    turn1_history.push(make_assistant_msg("Tapping the Settings icon now."));
    let turn1_json = serde_json::to_string(&turn1_history)?;
    manager.update_rig_history(session_id, Some(&turn1_json))?;

    // Reload and verify turn 1 is present.
    let loaded = manager.get_session(session_id)?.expect("session exists");
    let mut history: Vec<Message> = loaded
        .rig_history_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    assert_eq!(history.len(), 2);
    assert!(matches!(history[0], Message::User { .. }));
    assert!(matches!(history[1], Message::Assistant { .. }));

    // Simulate turn 2: append the new user message and assistant reply.
    history.push(make_user_msg("[Auto-observed] settings screen\n\nUser request: change theme to dark"));
    history.push(make_assistant_msg("Tapping the Theme option."));
    let turn2_json = serde_json::to_string(&history)?;
    manager.update_rig_history(session_id, Some(&turn2_json))?;

    // Reload and verify both turns are present (4 messages).
    let reloaded = manager.get_session(session_id)?.expect("session exists");
    let history2: Vec<Message> = reloaded
        .rig_history_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    assert_eq!(history2.len(), 4);

    // Verify the messages are in the correct order.
    if let Message::User { content } = &history2[2] {
        let first = content.first();
        if let UserContent::Text(t) = first {
            assert!(t.text.contains("change theme"));
        } else {
            panic!("expected text content");
        }
    } else {
        panic!("expected user message at index 2");
    }

    Ok(())
}

#[test]
fn test_history_json_none_is_valid() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_none.db");
    let manager = SessionManager::new(db_path)?;

    manager.insert_session(&Session {
        id: "s".into(),
        timestamp: Utc::now(),
        goal: "g".into(),
        status: "in_progress".into(),
        summary: None,
        chat_log_path: None,
        yaml_path: None,
        rig_history_json: None,
    })?;

    let s = manager.get_session("s")?.unwrap();
    assert!(s.rig_history_json.is_none());

    // An empty history is also valid (brand-new session).
    let empty_json = serde_json::to_string(&Vec::<Message>::new())?;
    manager.update_rig_history("s", Some(&empty_json))?;
    let s2 = manager.get_session("s")?.unwrap();
    assert_eq!(s2.rig_history_json.as_deref(), Some(empty_json.as_str()));

    Ok(())
}
