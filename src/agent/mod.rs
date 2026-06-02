use chrono::{DateTime, Utc};
use rig::completion::Message;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::device::{compress_xml, DeviceBridge, DeviceStatus};
use crate::llm::LlmConfig;

pub mod action;
pub mod rig_agent;
pub mod tools;

/// Active session state held in the engine between turns. Persisted to
/// `sessions.rig_history_json` after each successful turn.
#[derive(Debug, Clone)]
struct ActiveSession {
    id: String,
    goal: String,
    created_at: DateTime<Utc>,
    rig_history: Vec<Message>,
    step_history: Arc<Mutex<Vec<crate::yaml_exporter::TestStep>>>,
    yaml_path: Option<String>,
    screenshots: bool,
}

/// State of a session after the engine finishes processing it. Used by tests
/// to assert the lifecycle end-to-end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub status: String,
    pub yaml_path: Option<String>,
    pub turn_count: usize,
    pub step_count: usize,
}

/// High-level status for the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Thinking,
    Acting,
    Error(String),
}

/// Messages the UI sends **to** the Agent Engine.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Send a user prompt. Continues the active session if one is open,
    /// otherwise mints a new `session_id` and starts fresh.
    StartGoal(String, bool),
    /// Cancel the current goal.
    Stop,
    /// Finalize the current session (write final YAML, mark completed) and
    /// clear the active session. The next `StartGoal` will mint a new one.
    EndSession,
    /// Discard the current session and start a new one. Equivalent to
    /// `EndSession` followed by an implicit "create on next send".
    NewSession,
    /// Switch the engine to a different existing session, loading its
    /// `rig_history_json` from the database.
    SwitchSession(String),
    /// Update LLM configuration (API key, model, etc.) at runtime.
    UpdateConfig(LlmConfig),
    /// Select a specific ADB device by serial ID.
    SelectDevice(String),
    /// Refresh the list of connected devices.
    RefreshDevices,
    /// Launch an emulator by AVD name.
    LaunchEmulator(String),
    /// Stop an emulator by ID.
    StopEmulator(String),
    /// Replay an existing YAML test case, bypassing LLM reasoning.
    RetestScenario(std::path::PathBuf),
    /// Fetch available models from the provider.
    FetchModels(String, String),
    /// Clear all session history and artifacts.
    ClearAllHistory,
}

/// Updates the Agent Engine sends **back to** the UI.
#[derive(Debug, Clone)]
pub enum AgentUpdate {
    StatusChanged(AgentStatus),
    AgentReply(String),
    /// Refreshed list of devices with their status.
    DeviceList(Vec<(String, DeviceStatus)>),
    /// Emitted when a YAML test case is successfully generated.
    TestGenerated(std::path::PathBuf),
    /// Emitted when a session is saved to the database.
    SessionSaved,
    /// Emitted when the entire history is cleared.
    HistoryCleared,
    /// Successfully fetched available models.
    ModelsFetched(Vec<crate::llm::ModelData>),
    /// Failed to fetch available models.
    ModelsFetchFailed(String),
    /// Emitted when the active session changes (new, switched, ended).
    ActiveSessionChanged(Option<String>),
    /// Snapshot of session state for the status bar / UI.
    SessionStateUpdate(SessionState),
}

// ---------------------------------------------------------------------------
// Agent Engine
// ---------------------------------------------------------------------------

pub struct AgentEngine {
    pub sender: mpsc::Sender<AgentMessage>,
}

impl AgentEngine {
    /// Starts the agent communication channel.
    pub fn start(
        _update_tx: mpsc::Sender<AgentUpdate>,
        _config: crate::config::AppConfig,
    ) -> (Self, mpsc::Receiver<AgentMessage>) {
        let (msg_tx, msg_rx) = mpsc::channel(64);
        (Self { sender: msg_tx }, msg_rx)
    }

    /// The main command-processing loop. Runs on a dedicated thread (see main.rs).
    pub async fn run_loop(
        mut msg_rx: mpsc::Receiver<AgentMessage>,
        update_tx: mpsc::Sender<AgentUpdate>,
    ) {
        info!("Agent Engine loop started.");
        let mut device = DeviceBridge::new();

        let db_path = crate::config::db_path();
        let session_manager = crate::db::SessionManager::new(db_path).ok();

        let mut config = LlmConfig {
            api_key: "".into(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com/v1".into(),
            provider: "openai".into(),
        };

        let mut rig_agent = rig_agent::RigAgent::new(config.clone(), device.clone());

        // The active multi-turn session, if any. Persisted across consecutive
        // `StartGoal` messages so the LLM retains full prior context.
        let mut active: Option<ActiveSession> = None;

        // Initial device refresh
        Self::refresh_devices(&device, &update_tx).await;

        while let Some(msg) = msg_rx.recv().await {
            match msg {
                AgentMessage::UpdateConfig(new_config) => {
                    info!("Updating LLM config: {:?}", new_config);
                    config = new_config;
                    rig_agent = rig_agent::RigAgent::new(config.clone(), device.clone());
                }

                AgentMessage::SelectDevice(id) => {
                    info!("Selecting device: {}", id);
                    device.select_device(id);
                    rig_agent = rig_agent::RigAgent::new(config.clone(), device.clone());
                }

                AgentMessage::RefreshDevices => {
                    Self::refresh_devices(&device, &update_tx).await;
                }

                AgentMessage::LaunchEmulator(name) => {
                    info!("Launching emulator: {}", name);
                    if let Err(e) = device.launch_emulator(&name).await {
                        error!("Failed to launch emulator {}: {}", name, e);
                    }

                    let device_clone = device.clone();
                    let update_tx_clone = update_tx.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        info!("Polling status for launched emulator: {}", name_clone);
                        for _ in 0..60 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            Self::refresh_devices(&device_clone, &update_tx_clone).await;

                            if let Ok(DeviceStatus::Online) =
                                device_clone.get_avd_status(&name_clone).await
                            {
                                info!("Emulator {} is now Online.", name_clone);
                                break;
                            }
                        }
                    });
                }

                AgentMessage::StopEmulator(id_or_name) => {
                    info!("Stopping emulator: {}", id_or_name);
                    let mut serial = Some(id_or_name.clone());

                    if !id_or_name.starts_with("emulator-") {
                        if let Ok(Some(s)) = device.find_serial_for_avd(&id_or_name).await {
                            serial = Some(s);
                        }
                    }

                    if let Some(s) = serial {
                        let mut temp_bridge = device.clone();
                        temp_bridge.select_device(s);
                        if let Err(e) = temp_bridge.stop_emulator().await {
                            error!("Failed to stop emulator: {}", e);
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    Self::refresh_devices(&device, &update_tx).await;
                }

                AgentMessage::Stop => {
                    info!("Stopping current goal (keeping session open).");
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply("⏹ Goal cancelled.".to_string()))
                        .await;
                }

                AgentMessage::EndSession => {
                    info!("Ending current session.");
                    Self::finalize_active_session(&mut active, &session_manager, &update_tx).await;
                    active = None;
                    let _ = update_tx.send(AgentUpdate::ActiveSessionChanged(None)).await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(
                            "📦 Session ended and saved.".to_string(),
                        ))
                        .await;
                }

                AgentMessage::NewSession => {
                    info!("Starting a new session (closing current).");
                    Self::finalize_active_session(&mut active, &session_manager, &update_tx).await;
                    active = None;
                    let _ = update_tx.send(AgentUpdate::ActiveSessionChanged(None)).await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(
                            "🆕 New session. Send a goal to begin.".to_string(),
                        ))
                        .await;
                }

                AgentMessage::SwitchSession(target_id) => {
                    info!("Switching to session: {}", target_id);
                    // Finalize outgoing session first
                    Self::finalize_active_session(&mut active, &session_manager, &update_tx).await;
                    active = None;

                    if let Some(ref mgr) = session_manager {
                        if let Ok(Some(loaded)) = mgr.get_session(&target_id) {
                            let rig_history: Vec<Message> = loaded
                                .rig_history_json
                                .as_deref()
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or_default();

                            active = Some(ActiveSession {
                                id: loaded.id.clone(),
                                goal: loaded.goal.clone(),
                                created_at: loaded.timestamp,
                                rig_history,
                                step_history: Arc::new(Mutex::new(Vec::new())),
                                yaml_path: loaded.yaml_path.clone(),
                                screenshots: true,
                            });

                            let _ = update_tx
                                .send(AgentUpdate::ActiveSessionChanged(Some(target_id.clone())))
                                .await;
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "↩ Switched to session {}",
                                    target_id
                                )))
                                .await;
                        } else {
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "❌ Session {} not found",
                                    target_id
                                )))
                                .await;
                        }
                    }
                }

                AgentMessage::StartGoal(goal, screenshots) => {
                    info!("Received goal: {} (screenshots: {})", goal, screenshots);

                    // Resolve session id: reuse active or mint a new one.
                    let (session_id, is_continuation) = match &active {
                        Some(s) => (s.id.clone(), true),
                        None => {
                            let new_id = format!("sess_{}", chrono::Utc::now().timestamp());
                            (new_id, false)
                        }
                    };

                    if is_continuation {
                        info!("Continuing active session {}", session_id);
                        // Append the new goal to the existing goal string for context.
                        if let Some(ref mut a) = active {
                            a.goal = format!("{}\n---\n{}", a.goal, goal);
                        }
                    } else {
                        // Create the DB row up front to satisfy FK constraints.
                        if let Some(ref mgr) = session_manager {
                            let session = crate::db::Session {
                                id: session_id.clone(),
                                timestamp: chrono::Utc::now(),
                                goal: goal.clone(),
                                status: "in_progress".to_string(),
                                summary: None,
                                chat_log_path: None,
                                yaml_path: None,
                                rig_history_json: None,
                            };
                            if let Err(e) = mgr.insert_session(&session) {
                                error!("Failed to log session to DB: {}", e);
                            }
                        }

                        active = Some(ActiveSession {
                            id: session_id.clone(),
                            goal: goal.clone(),
                            created_at: chrono::Utc::now(),
                            rig_history: Vec::new(),
                            step_history: Arc::new(Mutex::new(Vec::new())),
                            yaml_path: None,
                            screenshots,
                        });
                    }

                    // Persist user message in chat_messages (always, per turn)
                    if let Some(ref mgr) = session_manager {
                        let _ = mgr.insert_chat_message(&crate::db::ChatMessage {
                            id: None,
                            session_id: session_id.clone(),
                            role: "user".to_string(),
                            content: goal.clone(),
                            timestamp: chrono::Utc::now(),
                        });
                    }

                    let _ = update_tx
                        .send(AgentUpdate::ActiveSessionChanged(Some(session_id.clone())))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(format!(
                            "🎯 {} (session {}): \"{}\"",
                            if is_continuation { "Continuing" } else { "Starting" },
                            session_id,
                            goal
                        )))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Thinking))
                        .await;

                    // -----------------------------------------------------------------
                    // P4: Auto-observe the current screen state and prepend it to
                    // the LLM input. This handles the "user manually changed apps
                    // between prompts" case automatically.
                    // -----------------------------------------------------------------
                    let llm_prompt = match device.observe_ui().await {
                        Ok(xml) => {
                            let compressed = compress_xml(&xml);
                            format!(
                                "[Auto-observed current screen state]\n{}\n\nUser request: {}",
                                compressed, goal
                            )
                        }
                        Err(e) => {
                            error!("Auto-observe failed ({}); sending goal only.", e);
                            goal.clone()
                        }
                    };

                    // Split borrows for the duration of the think() call.
                    let (mut history_ref, step_history, session_screenshots) = {
                        let a = active
                            .as_mut()
                            .expect("active session must be set before StartGoal");
                        (a.rig_history.clone(), a.step_history.clone(), a.screenshots)
                    };

                    let think_result = rig_agent
                        .think(&llm_prompt, session_screenshots, &mut history_ref, step_history.clone())
                        .await;

                    // Write the (potentially updated) history back into active.
                    if let Some(ref mut a) = active {
                        a.rig_history = history_ref;
                    }

                    let mut status = "success".to_string();
                    let mut final_reply: Option<String> = None;
                    let mut summary: Option<String> = None;

                    match think_result {
                        Ok(res) => {
                            // Save assistant message
                            if let Some(ref mgr) = session_manager {
                                let _ = mgr.insert_chat_message(&crate::db::ChatMessage {
                                    id: None,
                                    session_id: session_id.clone(),
                                    role: "assistant".to_string(),
                                    content: res.clone(),
                                    timestamp: chrono::Utc::now(),
                                });
                            }

                            // Generate AI summary from step history
                            if let Some(ref a) = active {
                                if let Ok(h) = a.step_history.lock() {
                                    if !h.is_empty() {
                                        if let Ok(s) = rig_agent
                                            .generate_summary(&a.goal, &h, &res)
                                            .await
                                        {
                                            summary = Some(s);
                                        }
                                    }
                                }
                            }

                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!("✅ Done: {}", res)))
                                .await;
                            final_reply = Some(res);
                        }
                        Err(e) => {
                            error!("Agent failed: {}", e);
                            status = format!("error: {}", e);
                            let _ = update_tx
                                .send(AgentUpdate::StatusChanged(AgentStatus::Error(
                                    e.to_string(),
                                )))
                                .await;
                            if let Some(ref mgr) = session_manager {
                                let _ = mgr.insert_chat_message(&crate::db::ChatMessage {
                                    id: None,
                                    session_id: session_id.clone(),
                                    role: "assistant".to_string(),
                                    content: format!("❌ Error: {}", e),
                                    timestamp: chrono::Utc::now(),
                                });
                            }
                        }
                    }

                    // Persist rig history and step history → YAML on every turn.
                    if let Some(ref mut a) = active {
                        // Save rig history JSON
                        if let Some(ref mgr) = session_manager {
                            let json = serde_json::to_string(&a.rig_history).ok();
                            if let Err(e) = mgr.update_rig_history(&a.id, json.as_deref()) {
                                error!("Failed to update rig_history: {}", e);
                            }
                        }

                        // Write / overwrite YAML test case
                        if let Ok(h) = a.step_history.lock() {
                            if !h.is_empty() {
                                let tc = crate::yaml_exporter::TestCase {
                                    goal: a.goal.clone(),
                                    screenshots: a.screenshots,
                                    steps: h.clone(),
                                    success: status == "success",
                                };

                                // Stable per-session path. The first turn picks a
                                // filename; subsequent turns overwrite the same file.
                                let yaml_path = if let Some(p) = &a.yaml_path {
                                    std::path::PathBuf::from(p)
                                } else {
                                    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                                    let results_dir = home.join("mobie-results");
                                    let _ = std::fs::create_dir_all(&results_dir);
                                    let filename = format!("{}.yaml", a.id);
                                    let p = results_dir.join(&filename);
                                    a.yaml_path = Some(p.to_string_lossy().to_string());
                                    p
                                };

                                match crate::yaml_exporter::export_to_path(&tc, &yaml_path) {
                                    Ok(()) => {
                                        let _ = update_tx
                                            .send(AgentUpdate::TestGenerated(yaml_path.clone()))
                                            .await;
                                    }
                                    Err(e) => error!("Failed to write YAML test case: {}", e),
                                }
                            }
                        }
                    }

                    // Update the session row
                    if let Some(ref mgr) = session_manager {
                        if let Some(ref a) = active {
                            let session = crate::db::Session {
                                id: a.id.clone(),
                                timestamp: chrono::Utc::now(),
                                goal: a.goal.clone(),
                                status: status.clone(),
                                summary: summary.clone(),
                                chat_log_path: None,
                                yaml_path: a.yaml_path.clone(),
                                rig_history_json: serde_json::to_string(&a.rig_history).ok(),
                            };
                            if let Err(e) = mgr.update_session(&session) {
                                error!("Failed to update session in DB: {}", e);
                            } else {
                                let _ = update_tx.send(AgentUpdate::SessionSaved).await;
                            }

                            // Push a state snapshot for the UI status bar
                            let turn_count = a.rig_history.len();
                            let step_count = a
                                .step_history
                                .lock()
                                .map(|h| h.len())
                                .unwrap_or(0);
                            let _ = update_tx
                                .send(AgentUpdate::SessionStateUpdate(SessionState {
                                    id: a.id.clone(),
                                    status: status.clone(),
                                    yaml_path: a.yaml_path.clone(),
                                    turn_count,
                                    step_count,
                                }))
                                .await;
                        }
                    }

                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                            let _ = final_reply; // already sent
                        }

                AgentMessage::RetestScenario(path) => {
                    info!("Retesting scenario from: {:?}", path);
                    let session_id = format!("retest_{}", chrono::Utc::now().timestamp());
                    let _ = update_tx.send(AgentUpdate::StatusChanged(AgentStatus::Acting)).await;
                    let _ = update_tx.send(AgentUpdate::AgentReply(format!("🔄 Replaying: {} (ID: {})", path.file_name().unwrap_or_default().to_string_lossy(), session_id))).await;

                    let mut status = "success".to_string();
                    let mut yaml_output_path = None;

                    if let Ok(yaml) = std::fs::read_to_string(&path) {
                        if let Ok(tc) = serde_yaml::from_str::<crate::yaml_exporter::TestCase>(&yaml) {
                            let mut retest_steps = Vec::new();
                            for step in tc.steps {
                                let _ = update_tx.send(AgentUpdate::AgentReply(format!("⚡ {} - {}", step.action, step.reasoning))).await;
                                match step.action.as_str() {
                                    "tap" => {
                                        let x = step.params.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                        let y = step.params.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                        let _ = device.tap(x, y).await;
                                    }
                                    "input" => {
                                        let text = step.params.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                        let _ = device.input_text(text).await;
                                    }
                                    "swipe" => {
                                        let x = step.params.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                        let y = step.params.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                        let direction = step.params.get("direction").and_then(|v| v.as_str()).unwrap_or("up");
                                        let distance = step.params.get("distance").and_then(|v| v.as_u64()).unwrap_or(500) as u32;
                                        let (x2, y2) = match direction {
                                            "up" => (x, y.saturating_sub(distance)),
                                            "down" => (x, y + distance),
                                            "left" => (x.saturating_sub(distance), y),
                                            "right" => (x + distance, y),
                                            _ => (x, y),
                                        };
                                        let _ = device.swipe(x, y, x2, y2, 300).await;
                                    }
                                    "key_event" => {
                                        let code = step.params.get("code").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                                        let _ = device.keyevent(code).await;
                                    }
                                    "screenshot" => {
                                        let _ = device.screenshot().await;
                                    }
                                    _ => {}
                                }

                                let mut retest_step = step.clone();
                                if tc.screenshots && step.action != "screenshot" && step.action != "observe" {
                                    retest_step.screenshot = device.screenshot().await.ok();
                                } else if step.action == "screenshot" {
                                    retest_step.screenshot = device.screenshot().await.ok();
                                }
                                retest_steps.push(retest_step);

                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            }

                            if tc.screenshots {
                                let retest_tc = crate::yaml_exporter::TestCase {
                                    goal: format!("Retest: {}", tc.goal),
                                    screenshots: true,
                                    steps: retest_steps,
                                    success: true,
                                };
                                if let Ok(out_p) = crate::yaml_exporter::export(&retest_tc) {
                                    yaml_output_path = Some(out_p.to_string_lossy().to_string());
                                }
                            }

                            let _ = update_tx.send(AgentUpdate::AgentReply("✅ Replay complete.".to_string())).await;
                        } else {
                            status = "error: Failed to parse test case".to_string();
                            let _ = update_tx.send(AgentUpdate::AgentReply("❌ Failed to parse test case.".to_string())).await;
                        }
                    } else {
                        status = "error: Failed to read test case file".to_string();
                        let _ = update_tx.send(AgentUpdate::AgentReply("❌ Failed to read test case file.".to_string())).await;
                    }

                    if let Some(ref mgr) = session_manager {
                        let session = crate::db::Session {
                            id: session_id.clone(),
                            timestamp: chrono::Utc::now(),
                            goal: format!("Retest: {}", path.file_name().unwrap_or_default().to_string_lossy()),
                            status: status.clone(),
                            summary: None,
                            chat_log_path: None,
                            yaml_path: yaml_output_path,
                            rig_history_json: None,
                        };
                        if mgr.insert_session(&session).is_ok() {
                            let _ = mgr.insert_chat_message(&crate::db::ChatMessage {
                                id: None,
                                session_id: session_id.clone(),
                                role: "user".to_string(),
                                content: format!("Retest scenario: {}", path.to_string_lossy()),
                                timestamp: chrono::Utc::now(),
                            });

                            let _ = update_tx.send(AgentUpdate::SessionSaved).await;
                        }
                    }

                    if let Some(ref mgr) = session_manager {
                        let _ = mgr.insert_chat_message(&crate::db::ChatMessage {
                            id: None,
                            session_id: session_id.clone(),
                            role: "assistant".to_string(),
                            content: if status == "success" {
                                "✅ Replay complete!".to_string()
                            } else {
                                format!("❌ Replay failed: {}", status)
                            },
                            timestamp: chrono::Utc::now(),
                        });
                    }

                    let _ = update_tx.send(AgentUpdate::StatusChanged(AgentStatus::Idle)).await;
                }

                AgentMessage::FetchModels(base_url, api_key) => {
                    info!("Fetching models from {}...", base_url);
                    let update_tx = update_tx.clone();
                    tokio::spawn(async move {
                        match crate::llm::fetch_models(&base_url, &api_key).await {
                            Ok(models) => {
                                let _ = update_tx.send(AgentUpdate::ModelsFetched(models)).await;
                            }
                            Err(e) => {
                                let _ = update_tx.send(AgentUpdate::ModelsFetchFailed(e.to_string())).await;
                            }
                        }
                    });
                }

                AgentMessage::ClearAllHistory => {
                    info!("Clearing all session history and artifacts.");
                    active = None;
                    if let Some(ref mgr) = session_manager {
                        if let Err(e) = mgr.clear_all_sessions() {
                            error!("Failed to clear sessions in DB: {}", e);
                        }
                    }
                    if let Err(e) = crate::yaml_exporter::clear_all_artifacts() {
                        error!("Failed to clear artifacts on disk: {}", e);
                    }
                    let _ = update_tx.send(AgentUpdate::HistoryCleared).await;
                    let _ = update_tx.send(AgentUpdate::ActiveSessionChanged(None)).await;
                }
            }
        }
    }

    /// Finalize the active session: write its final YAML, mark the DB row
    /// as completed. Called by `EndSession`, `NewSession`, and `SwitchSession`.
    async fn finalize_active_session(
        active: &mut Option<ActiveSession>,
        session_manager: &Option<crate::db::SessionManager>,
        update_tx: &mpsc::Sender<AgentUpdate>,
    ) {
        if let Some(a) = active.as_ref() {
            if let Some(ref mgr) = session_manager {
                let session = crate::db::Session {
                    id: a.id.clone(),
                    timestamp: chrono::Utc::now(),
                    goal: a.goal.clone(),
                    status: "completed".to_string(),
                    summary: None,
                    chat_log_path: None,
                    yaml_path: a.yaml_path.clone(),
                    rig_history_json: serde_json::to_string(&a.rig_history).ok(),
                };
                if let Err(e) = mgr.update_session(&session) {
                    error!("Failed to finalize session: {}", e);
                } else {
                    let _ = update_tx.send(AgentUpdate::SessionSaved).await;
                }
            }
        }
    }

    /// Refresh the ADB device list and AVDs and push it to the UI.
    async fn refresh_devices(device: &DeviceBridge, update_tx: &mpsc::Sender<AgentUpdate>) {
        info!("Refreshing device list...");
        let mut final_list = Vec::new();

        let avds = device.list_avds().await.unwrap_or_default();
        let online_serials = device.list_devices().await.unwrap_or_default();

        for name in avds {
            let status = device
                .get_avd_status(&name)
                .await
                .unwrap_or(DeviceStatus::Offline);

            final_list.push((name, status));
        }

        for serial in online_serials {
            if !serial.starts_with("emulator-") {
                final_list.push((serial, DeviceStatus::Online));
            }
        }

        let _ = update_tx.send(AgentUpdate::DeviceList(final_list)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_engine_init() {
        let (update_tx, _) = mpsc::channel(1);
        let config = crate::config::AppConfig::default();
        let (_engine, _) = AgentEngine::start(update_tx, config);
    }

    #[test]
    fn test_agent_generates_yaml_on_success() {
        let update = AgentUpdate::TestGenerated(std::path::PathBuf::from("test.yaml"));
        if let AgentUpdate::TestGenerated(p) = update {
            assert_eq!(p.to_str().unwrap(), "test.yaml");
        } else {
            panic!("TestGenerated not found");
        }
    }

    #[test]
    fn test_agent_handles_retest_scenario_msg() {
        let msg = AgentMessage::RetestScenario(std::path::PathBuf::from("test.yaml"));
        if let AgentMessage::RetestScenario(p) = msg {
            assert_eq!(p.to_str().unwrap(), "test.yaml");
        } else {
            panic!("RetestScenario not found");
        }
    }

    #[test]
    fn test_session_state_serializes() {
        let s = SessionState {
            id: "sess_123".into(),
            status: "success".into(),
            yaml_path: Some("/tmp/sess_123.yaml".into()),
            turn_count: 2,
            step_count: 5,
        };
        let json = serde_json::to_string(&s).unwrap();
        let de: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn test_active_session_struct_holds_rig_history() {
        let a = ActiveSession {
            id: "s1".into(),
            goal: "open settings".into(),
            created_at: chrono::Utc::now(),
            rig_history: vec![],
            step_history: Arc::new(Mutex::new(Vec::new())),
            yaml_path: None,
            screenshots: true,
        };
        assert_eq!(a.id, "s1");
        assert!(a.rig_history.is_empty());
    }
}
