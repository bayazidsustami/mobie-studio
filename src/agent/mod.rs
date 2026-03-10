use tokio::sync::mpsc;
use tracing::info;

use crate::device::DeviceBridge;
use crate::llm::{LlmClient, LlmConfig};

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
}

/// Updates the Agent Engine sends **back to** the UI.
#[derive(Debug, Clone)]
pub enum AgentUpdate {
    StatusChanged(AgentStatus),
    AgentReply(String),
}

// ---------------------------------------------------------------------------
// Agent Engine
// ---------------------------------------------------------------------------

pub struct AgentEngine {
    pub sender: mpsc::Sender<AgentMessage>,
}

impl AgentEngine {
    /// Spawn the agent loop on the GPUI background executor.
    ///
    /// Returns the `AgentEngine` (for sending commands) and an
    /// `mpsc::Receiver<AgentUpdate>` the UI should poll for updates.
    pub fn start(
        _update_tx: mpsc::Sender<AgentUpdate>,
    ) -> (Self, mpsc::Receiver<AgentMessage>) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        (Self { sender: cmd_tx }, cmd_rx)
    }

    /// The core agent loop.  Runs inside a background task.
    pub async fn run_loop(
        mut cmd_rx: mpsc::Receiver<AgentMessage>,
        update_tx: mpsc::Sender<AgentUpdate>,
    ) {
        let device = DeviceBridge::new();
        let llm = LlmClient::new(LlmConfig::default());

        let mut status = AgentStatus::Idle;
        info!("Agent Engine started. Status: {:?}", status);

        while let Some(msg) = cmd_rx.recv().await {
            match msg {
                AgentMessage::StartGoal(goal) => {
                    info!("Received new goal: {}", goal);

                    // --- Observe ---
                    status = AgentStatus::Observing;
                    let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;

                    let ui_dump = match device.observe_ui().await {
                        Ok(xml) => xml,
                        Err(e) => {
                            status = AgentStatus::Error(e.to_string());
                            let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;
                            continue;
                        }
                    };

                    // --- Think ---
                    status = AgentStatus::Thinking;
                    let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;

                    let response = match llm.think(&ui_dump, &goal).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            status = AgentStatus::Error(e.to_string());
                            let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;
                            continue;
                        }
                    };

                    // --- Act (stub) ---
                    status = AgentStatus::Acting;
                    let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;
                    info!("Agent decided: {}", response);

                    // Report response back to UI
                    let _ = update_tx.send(AgentUpdate::AgentReply(response)).await;

                    // --- Return to Idle ---
                    status = AgentStatus::Idle;
                    let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;
                    info!("Goal processing complete. Returned to {:?}", status);
                }
                AgentMessage::Stop => {
                    info!("Stopping Agent Engine.");
                    status = AgentStatus::Idle;
                    let _ = update_tx.send(AgentUpdate::StatusChanged(status.clone())).await;
                    break;
                }
            }
        }
    }
}
