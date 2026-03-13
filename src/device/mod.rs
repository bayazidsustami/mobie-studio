pub mod xml_parser;

pub use xml_parser::compress_xml;

use anyhow::{Context, Result};
use std::process::Command;
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

    /// Helper to build adb base args (selects device if set)
    fn adb_base_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(id) = &self.device_id {
            args.push("-s".to_string());
            args.push(id.clone());
        }
        args
    }

    /// Execute an ADB command with a timeout.
    async fn run_adb_timeout(
        &self,
        args: Vec<String>,
        timeout: std::time::Duration,
    ) -> Result<std::process::Output> {
        let output = tokio::task::spawn_blocking(move || {
            Command::new("adb").args(&args).output()
        });

        match tokio::time::timeout(timeout, output).await {
            Ok(Ok(Ok(out))) => Ok(out),
            Ok(Ok(Err(e))) => Err(anyhow::anyhow!("ADB execution failed: {}", e)),
            Ok(Err(e)) => Err(anyhow::anyhow!("ADB task panicked: {}", e)),
            Err(_) => Err(anyhow::anyhow!("ADB command timed out after {:?}", timeout)),
        }
    }

    /// Checks if adb is available and any devices are connected
    pub async fn list_devices(&self) -> Result<Vec<String>> {
        info!("Executing adb devices...");

        let output = self
            .run_adb_timeout(vec!["devices".to_string()], std::time::Duration::from_secs(5))
            .await?;

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
        let mut base = self.adb_base_args();
        base.extend(["shell".to_string(), "uiautomator".to_string(), "dump".to_string()]);

        let dump_cmd = self.run_adb_timeout(base, std::time::Duration::from_secs(10)).await?;

        if !dump_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&dump_cmd.stderr);
            warn!("uiautomator dump failed: {}", stderr);
            return Err(anyhow::anyhow!("uiautomator dump failed: {}", stderr));
        }

        let mut base2 = self.adb_base_args();
        base2.extend(["shell".to_string(), "cat".to_string(), "/sdcard/window_dump.xml".to_string()]);
        
        let cat_cmd = self.run_adb_timeout(base2, std::time::Duration::from_secs(5)).await?;

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
        let mut args = self.adb_base_args();
        args.extend([
            "shell".to_string(),
            "input".to_string(),
            "tap".to_string(),
            x.to_string(),
            y.to_string(),
        ]);
        
        let output = self.run_adb_timeout(args, std::time::Duration::from_secs(5)).await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Tap failed"));
        }
        Ok(())
    }

    pub async fn swipe(&self, x1: u32, y1: u32, x2: u32, y2: u32, duration_ms: u32) -> Result<()> {
        info!(
            "Executing swipe from ({}, {}) to ({}, {}) over {}ms",
            x1, y1, x2, y2, duration_ms
        );
        let mut args = self.adb_base_args();
        args.extend([
            "shell".to_string(),
            "input".to_string(),
            "swipe".to_string(),
            x1.to_string(),
            y1.to_string(),
            x2.to_string(),
            y2.to_string(),
            duration_ms.to_string(),
        ]);

        let output = self.run_adb_timeout(args, std::time::Duration::from_secs(5)).await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Swipe failed"));
        }
        Ok(())
    }

    pub async fn input_text(&self, text: &str) -> Result<()> {
        info!("Executing input text: {}", text);
        let text = text.replace(' ', "%s");
        let mut args = self.adb_base_args();
        args.extend(["shell".to_string(), "input".to_string(), "text".to_string(), text]);

        let output = self.run_adb_timeout(args, std::time::Duration::from_secs(5)).await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Input text failed"));
        }
        Ok(())
    }

    pub async fn keyevent(&self, code: u32) -> Result<()> {
        info!("Executing keyevent {}", code);
        let mut args = self.adb_base_args();
        args.extend([
            "shell".to_string(),
            "input".to_string(),
            "keyevent".to_string(),
            code.to_string(),
        ]);

        let output = self.run_adb_timeout(args, std::time::Duration::from_secs(5)).await?;
        if !output.status.success() {
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
                direction,
                x,
                y,
                distance,
                ..
            } => {
                let (w, h) = self.get_screen_size().await.unwrap_or((1080, 2400));
                let dist = distance.unwrap_or_else(|| match direction {
                    SwipeDirection::Up | SwipeDirection::Down => h / 3,
                    SwipeDirection::Left | SwipeDirection::Right => w / 2,
                });

                let (x2, y2) = match direction {
                    SwipeDirection::Up => (*x, y.saturating_sub(dist)),
                    SwipeDirection::Down => (*x, (*y + dist).min(h - 1)),
                    SwipeDirection::Left => (x.saturating_sub(dist), *y),
                    SwipeDirection::Right => ((*x + dist).min(w - 1), *y),
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

    /// Fetches the screen dimensions via `adb shell wm size`.
    pub async fn get_screen_size(&self) -> Result<(u32, u32)> {
        info!("Fetching screen size via wm size...");
        let base = self.adb_base_args();
        let output = tokio::task::spawn_blocking(move || {
            Command::new("adb")
                .args(&base)
                .args(["shell", "wm", "size"])
                .output()
        })
        .await
        .context("spawn_blocking for wm size panicked")?
        .context("Failed to run wm size")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Typical output: "Physical size: 1080x2400"
        if let Some(line) = stdout.lines().next() {
            if let Some(size_str) = line.split(':').last() {
                let parts: Vec<&str> = size_str.trim().split('x').collect();
                if parts.len() == 2 {
                    let w = parts[0].parse::<u32>()?;
                    let h = parts[1].parse::<u32>()?;
                    return Ok((w, h));
                }
            }
        }
        Err(anyhow::anyhow!(
            "Failed to parse screen size from: {}",
            stdout
        ))
    }
}
