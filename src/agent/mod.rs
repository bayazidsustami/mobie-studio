pub mod action;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::agent::action::Action;
use crate::device::{compress_xml, DeviceBridge};
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

/// Maximum iterations per goal to prevent infinite loops.
const MAX_ITERATIONS: usize = 20;

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
    ///
    /// For each goal the loop executes:
    ///   1. **Observe** — dump the UI tree via ADB and compress it.
    ///   2. **Think**   — send the compressed XML + goal to the LLM.
    ///   3. **Act**     — execute the decided action via ADB.
    ///   4. **Repeat**  — unless the action is `Done` or max iterations hit.
    pub async fn run_loop(
        mut cmd_rx: mpsc::Receiver<AgentMessage>,
        update_tx: mpsc::Sender<AgentUpdate>,
    ) {
        let device = DeviceBridge::new();
        let llm = LlmClient::new(LlmConfig::default());

        info!("Agent Engine started. Waiting for goals...");
        let _ = update_tx
            .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
            .await;

        while let Some(msg) = cmd_rx.recv().await {
            match msg {
                AgentMessage::StartGoal(goal) => {
                    info!("Received new goal: {}", goal);
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(format!(
                            "🎯 Starting goal: \"{}\"",
                            goal
                        )))
                        .await;

                    let mut iteration = 0usize;

                    loop {
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

                        // ============================================
                        // 1. OBSERVE — dump and compress the UI tree
                        // ============================================
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

                        let compressed = compress_xml(&raw_xml);
                        info!("Compressed UI: {} chars", compressed.len());

                        // ============================================
                        // 2. THINK — ask the LLM for the next action
                        // ============================================
                        let _ = update_tx
                            .send(AgentUpdate::StatusChanged(AgentStatus::Thinking))
                            .await;

                        let action = match llm.think(&compressed, &goal).await {
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

                        // ============================================
                        // 3. CHECK FOR DONE
                        // ============================================
                        if let Action::Done { success, reason } = &action {
                            let emoji = if *success { "✅" } else { "❌" };
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!(
                                    "{} Goal completed: {}",
                                    emoji, reason
                                )))
                                .await;
                            break;
                        }

                        // ============================================
                        // 4. ACT — execute the action via ADB
                        // ============================================
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
                            // Don't break — let the agent observe again and try something else
                        }

                        // Small delay to let the device UI settle
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                    }

                    // Return to idle after goal completes
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    info!("Goal processing complete. Returned to Idle.");
                }
                AgentMessage::Stop => {
                    info!("Stopping Agent Engine.");
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    break;
                }
            }
        }
    }
}
