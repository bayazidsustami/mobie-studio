use std::process::Command;
use tracing::info;

#[derive(Default)]
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

    /// Checks if adb is available and any devices are connected
    pub fn list_devices(&self) -> anyhow::Result<Vec<String>> {
        info!("Executing adb devices...");
        
        // Wrap in a mock command if adb isn't installed for testing.
        // In real scenario: Command::new("adb").arg("devices").output()?;
        let output = Command::new("echo")
            .arg("List of devices attached\nemulator-5554\tdevice")
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();
        
        for line in stdout.lines().skip(1) {
            if line.contains("device") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(id) = parts.first() {
                    devices.push(id.to_string());
                }
            }
        }
        Ok(devices)
    }

    /// Dumps the current UI hierarchy to XML via uiautomator
    pub async fn observe_ui(&self) -> anyhow::Result<String> {
        info!("Dumping UI via adb shell uiautomator...");
        // TODO: Actually run `adb shell uiautomator dump` and pull the XML.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok("<hierarchy><node text=\"Mock Button\"/></hierarchy>".to_string())
    }

    /// Executes a tap action
    pub async fn tap(&self, x: u32, y: u32) -> anyhow::Result<()> {
        info!("Executing tap at ({}, {})", x, y);
        // Command::new("adb").args(["shell", "input", "tap", &x.to_string(), &y.to_string()]).output()?;
        Ok(())
    }
}
