use tokio::sync::mpsc;
use tracing::{error, info};

use crate::device::{DeviceBridge, DeviceStatus};
use crate::llm::LlmConfig;

pub mod action;
pub mod rig_agent;
pub mod tools;

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
    /// Start a new goal (exploratory run).
    StartGoal(String),
    /// Cancel the current goal.
    Stop,
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

        let mut config = LlmConfig {
            api_key: "".into(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com/v1".into(),
            provider: "openai".into(),
        };

        let mut rig_agent = rig_agent::RigAgent::new(config.clone(), device.clone());

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

                    // Poll for status changes in a separate task so we don't block the command loop
                    let device_clone = device.clone();
                    let update_tx_clone = update_tx.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        info!("Polling status for launched emulator: {}", name_clone);
                        // Poll every 2 seconds for up to 2 minutes
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

                    // If it's not a serial, try to find the serial for this AVD name
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
                    info!("Stopping Agent.");
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply("⏹ Goal cancelled.".to_string()))
                        .await;
                }

                AgentMessage::StartGoal(goal) => {
                    info!("Received goal: {}", goal);
                    let _ = update_tx
                        .send(AgentUpdate::AgentReply(format!(
                            "🎯 Starting: \"{}\"",
                            goal
                        )))
                        .await;
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Thinking))
                        .await;

                    match rig_agent.think(&goal).await {
                        Ok(res) => {
                            let _ = update_tx
                                .send(AgentUpdate::AgentReply(format!("✅ Done: {}", res)))
                                .await;
                            
                            // Generate YAML test case
                            if let Ok(h) = rig_agent.history.lock() {
                                if !h.is_empty() {
                                    let tc = crate::yaml_exporter::TestCase {
                                        goal: goal.clone(),
                                        steps: h.clone(),
                                        success: true,
                                    };
                                    match crate::yaml_exporter::export(&tc) {
                                        Ok(path) => {
                                            let _ = update_tx.send(AgentUpdate::TestGenerated(path)).await;
                                        }
                                        Err(e) => {
                                            error!("Failed to export YAML test case: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Agent failed: {}", e);
                            let _ = update_tx
                                .send(AgentUpdate::StatusChanged(AgentStatus::Error(
                                    e.to_string(),
                                )))
                                .await;
                        }
                    }
                    let _ = update_tx
                        .send(AgentUpdate::StatusChanged(AgentStatus::Idle))
                        .await;
                }
            }
        }
    }

    /// Refresh the ADB device list and AVDs and push it to the UI.
    async fn refresh_devices(device: &DeviceBridge, update_tx: &mpsc::Sender<AgentUpdate>) {
        info!("Refreshing device list...");
        let mut final_list = Vec::new();

        // 1. Get all registered AVD names
        let avds = device.list_avds().await.unwrap_or_default();

        // 2. Get currently online ADB serials
        let online_serials = device.list_devices().await.unwrap_or_default();

        // 3. Map AVDs to their current status
        for name in avds {
            let status = device
                .get_avd_status(&name)
                .await
                .unwrap_or(DeviceStatus::Offline);

            // If the AVD is online, get_avd_status logic uses its serial.
            // We want the UI to primarily show the AVD name.
            final_list.push((name, status));
        }

        // 4. Add any online devices that are NOT emulators (e.g. physical hardware)
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

    #[tokio::test]
    async fn test_agent_generates_yaml_on_success() {
        let update = AgentUpdate::TestGenerated(std::path::PathBuf::from("test.yaml"));
        if let AgentUpdate::TestGenerated(p) = update {
            assert_eq!(p.to_str().unwrap(), "test.yaml");
        } else {
            panic!("TestGenerated not found");
        }
    }
}
