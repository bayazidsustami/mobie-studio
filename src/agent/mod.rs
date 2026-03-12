pub mod action;

pub use action::Action;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::device::DeviceBridge;
use crate::llm::{LlmClient, LlmConfig};
use crate::yaml_exporter::{export, TestCase, TestStep};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Observing,
    Thinking,
    Acting,
    Error(String),
}

/// Messages the UI sends **to** the Agent Engine.
pub enum AgentMessage {
    StartGoal(String),
    Stop,
    /// Update LLM configuration (API key, model, etc.) at runtime.
    UpdateConfig(LlmConfig),
    /// Select a specific ADB device by serial ID.
    SelectDevice(String),
    /// Refresh the list of connected devices.
    RefreshDevices,
}

/// Updates the Agent Engine sends **back to** the UI.
#[derive(Debug, Clone)]
pub enum AgentUpdate {
    StatusChanged(AgentStatus),
    AgentReply(String),
    /// Refreshed list of connected ADB device IDs.
    DeviceList(Vec<String>),
}

/// Maximum iterations per goal to prevent infinite loops.
const MAX_ITERATIONS: usize = 20;

// ---------------------------------------------------------------------------
// Session History
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SessionHistory {
    actions: Vec<Action>,
    limit: usize,
}

impl SessionHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            actions: Vec::new(),
            limit,
        }
    }

    pub fn push(&mut self, action: Action) {
        if self.actions.len() >= self.limit {
            self.actions.remove(0);
        }
        self.actions.push(action);
    }

    pub fn get_recent(&self, count: usize) -> &[Action] {
        let start = self.actions.len().saturating_sub(count);
        &self.actions[start..]
    }

    /// Simple loop detection: check if the last action is the same as the one before it.
    /// This can be expanded to check longer patterns.
    pub fn is_looping(&self) -> bool {
        if self.actions.len() < 2 {
            return false;
        }
        let last = &self.actions[self.actions.len() - 1];
        let prev = &self.actions[self.actions.len() - 2];

        // For now, compare Display output as a proxy for identity
        last.to_string() == prev.to_string()
    }
}

// ---------------------------------------------------------------------------
// Agent Engine
// ---------------------------------------------------------------------------

pub struct AgentEngine {
    pub sender: mpsc::Sender<AgentMessage>,
}

impl AgentEngine {
    /// Spawn the agent loop on the GPUI background executor.
    pub fn start(
        _update_tx: mpsc::Sender<AgentUpdate>,
        _initial_config: AppConfig,
    ) -> (Self, mpsc::Receiver<AgentMessage>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        (Self { sender: cmd_tx }, cmd_rx)
    }

    /// The core agent loop. Runs inside a background task.
    pub async fn run_loop(
        mut cmd_rx: mpsc::Receiver<AgentMessage>,
        update_tx: mpsc::Sender<AgentUpdate>,
    ) {
        // Load persisted config on startup (or could receive via channel from main)
        let app_config = crate::config::load_config();
        let mut device = DeviceBridge::new();
        let mut llm = LlmClient::new(app_config.llm.clone());

        info!("Agent Engine started. Waiting for goals...");
        let _ = update_tx
            .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
            .await;

        // Perform an initial device list refresh
        Self::refresh_devices(&device, &update_tx).await;

        while let Some(msg) = cmd_rx.recv().await {
            match msg {
                // ----------------------------------------------------------------
                // Config / device management messages
                // ----------------------------------------------------------------
                AgentMessage::UpdateConfig(new_cfg) => {
                    info!("Config updated: model={}", new_cfg.model);
                    llm = LlmClient::new(new_cfg);
                }
                AgentMessage::SelectDevice(id) => {
                    info!("Device selected: {}", id);
                    device.select_device(id.clone());
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(format!(
                            "📱 Device selected: {}",
                            id
                        )))
                        .await;
                }
                AgentMessage::RefreshDevices => {
                    info!("Refreshing device list...");
                    Self::refresh_devices(&device, &update_tx).await;
                }

                // ----------------------------------------------------------------
                // Stop
                // ----------------------------------------------------------------
                AgentMessage::Stop => {
                    info!("Stopping Agent Engine.");
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(
                            "⏹ Goal cancelled.".to_string(),
                        ))
                        .await;
                }

                // ----------------------------------------------------------------
                // StartGoal — the main agent loop
                // ----------------------------------------------------------------
                AgentMessage::StartGoal(goal) => {
                    info!("Received new goal: {}", goal);
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(format!(
                            "🎯 Starting goal: \"{}\"",
                            goal
                        )))
                        .await;

                    // Accumulate steps for YAML export
                    let mut recorded_steps: Vec<TestStep> = Vec::new();
                    let mut iteration = 0usize;
                    let mut goal_success = false;

                    'agent_loop: loop {
                        iteration += 1;
                        if iteration > MAX_ITERATIONS {
                            warn!("Max iterations ({}) reached for goal", MAX_ITERATIONS);
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "⚠️ Stopped after {} iterations — goal may not be achievable.",
                                    MAX_ITERATIONS
                                )))
                                .await;
                            break;
                        }

                        info!("--- Iteration {}/{} ---", iteration, MAX_ITERATIONS);

                        // 1. OBSERVE
                        let _ = update_tx
                            .send(AgentUpdate::StatusChanged(AgentStatus::Observing))
                            .await;

                        let raw_xml = match device.observe_ui().await {
                            Ok(xml) => xml,
                            Err(e) => {
                                error!("Observe failed: {}", e);
                                let _ = update_tx
                                    .send(AgentUpdate::StatusChanged(AgentStatus::Error(
                                        e.to_string(),
                                    )))
                                    .await;
                                let _ = update_tx
                                    .send(AgentUpdate::AgentReply(format!(
                                        "❌ Failed to observe UI: {}",
                                        e
                                    )))
                                    .await;
                                break;
                            }
                        };

                        // Check if a Stop was sent during observe
                        if let Ok(AgentMessage::Stop) = cmd_rx.try_recv() {
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply("⏹ Goal cancelled.".to_string()))
                                .await;
                            break 'agent_loop;
                        }

                        // 2. THINK
                        let _ = update_tx
                            .send(AgentUpdate::StatusChanged(AgentStatus::Thinking))
                            .await;

                        let action = match llm.think(&raw_xml, &goal).await {
                            Ok(a) => a,
                            Err(e) => {
                                error!("LLM think failed: {}", e);
                                let _ = update_tx
                                    .send(AgentUpdate::StatusChanged(AgentStatus::Error(
                                        e.to_string(),
                                    )))
                                    .await;
                                let _ = update_tx
                                    .send(AgentUpdate::AgentReply(format!(
                                        "❌ LLM error: {}",
                                        e
                                    )))
                                    .await;
                                break;
                            }
                        };

                        info!("Action decided: {}", action);
                        let _ = update_tx
                            .send(AgentUpdate::AgentReply(format!(
                                "🤖 Step {}: {}",
                                iteration, action
                            )))
                            .await;

                        // 3. DONE check
                        if let Action::Done { success, reason } = &action {
                            let emoji = if *success { "✅" } else { "❌" };
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "{} Goal completed: {}",
                                    emoji, reason
                                )))
                                .await;
                            goal_success = *success;
                            break;
                        }

                        // Record this step
                        let step = action_to_test_step(&action);
                        recorded_steps.push(step);

                        // 4. ACT
                        let _ = update_tx
                            .send(AgentUpdate::StatusChanged(AgentStatus::Acting))
                            .await;

                        if let Err(e) = device.execute_action(&action).await {
                            error!("Action execution failed: {}", e);
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "⚠️ Action failed: {} — retrying...",
                                    e
                                )))
                                .await;
                        }

                        // Wait for device UI to settle
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                    }

                    // YAML export on success
                    if goal_success && !recorded_steps.is_empty() {
                        let test_case = TestCase {
                            goal: goal.clone(),
                            steps: recorded_steps,
                            success: true,
                        };
                        match export(&test_case) {
                            Ok(path) => {
                                let _ = update_tx
                                    .send(AgentUpdate::AgentReply(format!(
                                        "📄 Test case exported: {}",
                                        path.display()
                                    )))
                                    .await;
                            }
                            Err(e) => {
                                warn!("YAML export failed: {}", e);
                                let _ = update_tx
                                    .send(AgentUpdate::AgentReply(format!(
                                        "⚠️ YAML export failed: {}",
                                        e
                                    )))
                                    .await;
                            }
                        }
                    }

                    // Return to idle
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    info!("Goal processing complete. Returned to Idle.");
                }
            }
        }
    }

    /// Refresh the ADB device list and push it to the UI.
    async fn refresh_devices(device: &DeviceBridge, update_tx: &mpsc::Sender<AgentUpdate>) {
        match device.list_devices().await {
            Ok(devices) => {
                info!("Found {} device(s)", devices.len());
                let _ = update_tx.send(AgentUpdate::DeviceList(devices)).await;
            }
            Err(e) => {
                warn!("Failed to list ADB devices: {}", e);
                let _ = update_tx.send(AgentUpdate::DeviceList(vec![])).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert an `Action` into a `TestStep` for YAML recording.
fn action_to_test_step(action: &Action) -> TestStep {
    use serde_json::json;
    let (action_name, params, reasoning) = match action {
        Action::Tap { x, y, reasoning } => {
            let mut p = std::collections::HashMap::new();
            p.insert("x".to_string(), json!(x));
            p.insert("y".to_string(), json!(y));
            ("tap", p, reasoning.clone())
        }
        Action::Input { text, reasoning } => {
            let mut p = std::collections::HashMap::new();
            p.insert("text".to_string(), json!(text));
            ("input", p, reasoning.clone())
        }
        Action::Swipe {
            direction,
            x,
            y,
            reasoning,
        } => {
            let mut p = std::collections::HashMap::new();
            p.insert(
                "direction".to_string(),
                json!(format!("{:?}", direction).to_lowercase()),
            );
            p.insert("x".to_string(), json!(x));
            p.insert("y".to_string(), json!(y));
            ("swipe", p, reasoning.clone())
        }
        Action::KeyEvent { code, reasoning } => {
            let mut p = std::collections::HashMap::new();
            p.insert("code".to_string(), json!(code));
            ("key_event", p, reasoning.clone())
        }
        Action::Done { .. } => (
            "done",
            std::collections::HashMap::new(),
            String::new(),
        ),
    };

    TestStep {
        action: action_name.to_string(),
        params,
        reasoning,
    }
}
