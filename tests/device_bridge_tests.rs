use mobie::device::{DeviceBridge, CommandRunner};
use std::sync::Arc;
use tokio;
use anyhow::Result;

#[derive(Debug)]
struct MockRunner {
    expected_cmd: String,
    mock_stdout: String,
}

impl CommandRunner for MockRunner {
    fn run(&self, cmd: &str, _args: &[String]) -> Result<std::process::Output> {
        if cmd == self.expected_cmd {
            Ok(std::process::Output {
                status: unsafe { std::mem::zeroed() }, // Success status
                stdout: self.mock_stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            })
        } else {
            Err(anyhow::anyhow!("Unexpected command: {}", cmd))
        }
    }

    fn spawn(&self, cmd: &str, _args: &[String]) -> Result<()> {
        if cmd == self.expected_cmd {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Unexpected spawn command: {}", cmd))
        }
    }
}

#[tokio::test]
async fn test_list_avds_mocked() {
    let mock_stdout = "Pixel_3a_API_34\nPixel_7_Pro_API_33\n".to_string();
    let runner = Arc::new(MockRunner {
        expected_cmd: "emulator".to_string(),
        mock_stdout,
    });
    
    let bridge = DeviceBridge::with_runner(runner);
    let avds = bridge.list_avds().await.expect("Failed to list AVDs");
    
    assert_eq!(avds.len(), 2);
    assert_eq!(avds[0], "Pixel_3a_API_34");
    assert_eq!(avds[1], "Pixel_7_Pro_API_33");
}

#[tokio::test]
async fn test_launch_emulator_mocked() {
    let runner = Arc::new(MockRunner {
        expected_cmd: "emulator".to_string(),
        mock_stdout: "".to_string(),
    });
    
    let bridge = DeviceBridge::with_runner(runner);
    bridge.launch_emulator("Pixel_7_Pro_API_33").await.expect("Failed to launch emulator");
}

#[tokio::test]
async fn test_stop_emulator_mocked() {
    let runner = Arc::new(MockRunner {
        expected_cmd: "adb".to_string(),
        mock_stdout: "".to_string(),
    });
    
    let mut bridge = DeviceBridge::with_runner(runner);
    bridge.select_device("emulator-5554".to_string());
    bridge.stop_emulator().await.expect("Failed to stop emulator");
}
