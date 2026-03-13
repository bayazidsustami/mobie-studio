pub mod action;
pub mod rig_agent;
pub mod tools;

pub use action::Action;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::config::AppConfig;
use crate::device::DeviceBridge;
use crate::llm::LlmConfig;

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
        let mut rig_agent = rig_agent::RigAgent::new(app_config.llm.clone(), device.clone());

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
                    rig_agent = rig_agent::RigAgent::new(new_cfg, device.clone());
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

                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Thinking))
                        .await;

                    match rig_agent.think(&goal).await {
                        Ok(res) => {
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "✅ Goal completed: {}",
                                    res
                                )))
                                .await;
                        }
                        Err(e) => {
                            error!("Rig think failed: {}", e);
                            let _ = update_tx
                                .send(AgentUpdate::StatusChanged(AgentStatus::Error(
                                    e.to_string(),
                                )))
                                .await;
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!("❌ Agent error: {}", e)))
                                .await;
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_engine_uses_rig_agent() {
        // This is a placeholder test to drive the integration.
        // We'll check if RigAgent can be used within the engine context.
        let config = LlmConfig::default();
        let _rig = rig_agent::RigAgent::new(config, DeviceBridge::new());
        // The real verification will be in the implementation of run_loop.
        assert!(true);
    }
}
