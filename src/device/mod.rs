pub mod xml_parser;

pub use xml_parser::compress_xml;

use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{info, warn};

use crate::agent::action::{Action, SwipeDirection};

#[derive(Default, Clone, Debug)]
pub struct DeviceBridge {
    device_id: Option<String>,
}

impl DeviceBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the currently selected device ID, if any.
    pub fn selected_device(&self) -> Option<&str> {
        self.device_id.as_deref()
    }

    /// Sets the active device ID.
    pub fn select_device(&mut self, id: String) {
        self.device_id = Some(id);
    }

    /// Helper to build an adb command with the selected device if present
    fn adb_cmd(&self) -> Command {
        let mut cmd = Command::new("adb");
        if let Some(id) = &self.device_id {
            cmd.arg("-s").arg(id);
        }
        cmd
    }

    /// Checks if adb is available and any devices are connected
    pub async fn list_devices(&self) -> Result<Vec<String>> {
        info!("Executing adb devices...");

        let output = Command::new("adb")
            .arg("devices")
            .output()
            .await
            .context("Failed to execute adb devices")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) {
            if line.contains("device") && !line.contains("offline") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(id) = parts.first() {
                    if *id != "List" {
                        devices.push(id.to_string());
                    }
                }
            }
        }
        Ok(devices)
    }

    /// Dumps the current UI hierarchy to XML via uiautomator
    pub async fn observe_ui(&self) -> Result<String> {
        info!("Dumping UI via adb shell uiautomator...");
        let dump_cmd = self
            .adb_cmd()
            .args(["shell", "uiautomator", "dump"])
            .output()
            .await
            .context("Failed to run uiautomator dump")?;

        if !dump_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&dump_cmd.stderr);
            warn!("uiautomator dump failed: {}", stderr);
            return Err(anyhow::anyhow!("uiautomator dump failed: {}", stderr));
        }

        let cat_cmd = self
            .adb_cmd()
            .args(["shell", "cat", "/sdcard/window_dump.xml"])
            .output()
            .await
            .context("Failed to read window_dump.xml")?;

        if !cat_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&cat_cmd.stderr);
            return Err(anyhow::anyhow!("cat window_dump.xml failed: {}", stderr));
        }

        let xml = String::from_utf8_lossy(&cat_cmd.stdout).to_string();
        Ok(xml)
    }

    /// Executes a tap action
    pub async fn tap(&self, x: u32, y: u32) -> Result<()> {
        info!("Executing tap at ({}, {})", x, y);
        let status = self
            .adb_cmd()
            .args(["shell", "input", "tap", &x.to_string(), &y.to_string()])
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Tap failed"));
        }
        Ok(())
    }

    pub async fn swipe(&self, x1: u32, y1: u32, x2: u32, y2: u32, duration_ms: u32) -> Result<()> {
        info!(
            "Executing swipe from ({}, {}) to ({}, {}) over {}ms",
            x1, y1, x2, y2, duration_ms
        );
        let status = self
            .adb_cmd()
            .args([
                "shell",
                "input",
                "swipe",
                &x1.to_string(),
                &y1.to_string(),
                &x2.to_string(),
                &y2.to_string(),
                &duration_ms.to_string(),
            ])
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Swipe failed"));
        }
        Ok(())
    }

    pub async fn input_text(&self, text: &str) -> Result<()> {
        info!("Executing input text: {}", text);
        // Replace spaces with %s for adb input text
        let text = text.replace(" ", "%s");
        let status = self
            .adb_cmd()
            .args(["shell", "input", "text", &text])
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Input text failed"));
        }
        Ok(())
    }

    pub async fn keyevent(&self, code: u32) -> Result<()> {
        info!("Executing keyevent {}", code);
        let status = self
            .adb_cmd()
            .args(["shell", "input", "keyevent", &code.to_string()])
            .status()
            .await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Keyevent {} failed", code));
        }
        Ok(())
    }

    /// Press the Android Back button (keyevent 4).
    pub async fn back(&self) -> Result<()> {
        self.keyevent(4).await
    }

    /// Press the Android Home button (keyevent 3).
    pub async fn home(&self) -> Result<()> {
        self.keyevent(3).await
    }

    /// Dispatch an `Action` (from the LLM) to the appropriate ADB command.
    pub async fn execute_action(&self, action: &Action) -> Result<()> {
        match action {
            Action::Tap { x, y, .. } => self.tap(*x, *y).await,
            Action::Input { text, .. } => self.input_text(text).await,
            Action::Swipe {
                direction, x, y, ..
            } => {
                let (x2, y2) = match direction {
                    SwipeDirection::Up => (*x, y.saturating_sub(600)),
                    SwipeDirection::Down => (*x, y + 600),
                    SwipeDirection::Left => (x.saturating_sub(400), *y),
                    SwipeDirection::Right => (x + 400, *y),
                };
                self.swipe(*x, *y, x2, y2, 300).await
            }
            Action::KeyEvent { code, .. } => self.keyevent(*code).await,
            Action::Done { .. } => {
                // Nothing to execute; the agent loop handles termination.
                Ok(())
            }
        }
    }
}
