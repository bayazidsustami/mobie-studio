use anyhow::Result;
use mobie::device::{CommandRunner, DeviceBridge};
use std::sync::Arc;
use tokio;

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
    bridge
        .launch_emulator("Pixel_7_Pro_API_33")
        .await
        .expect("Failed to launch emulator");
}

#[tokio::test]
async fn test_stop_emulator_mocked() {
    let runner = Arc::new(MockRunner {
        expected_cmd: "adb".to_string(),
        mock_stdout: "".to_string(),
    });

    let mut bridge = DeviceBridge::with_runner(runner);
    bridge.select_device("emulator-5554".to_string());
    bridge
        .stop_emulator()
        .await
        .expect("Failed to stop emulator");
}

#[tokio::test]
async fn test_get_avd_status_mocked() {
    use mobie::device::DeviceStatus;

    // Scenario:
    // - adb devices: emulator-5554
    // - adb -s emulator-5554 emu avd name: Pixel_7_Pro_API_33
    // - adb -s emulator-5554 shell getprop sys.boot_completed: 1

    #[derive(Debug)]
    struct StatusMockRunner;
    impl CommandRunner for StatusMockRunner {
        fn run(&self, cmd: &str, args: &[String]) -> Result<std::process::Output> {
            let stdout = match (
                cmd,
                args.get(0).map(|s| s.as_str()),
                args.get(1).map(|s| s.as_str()),
            ) {
                ("adb", Some("devices"), _) => "List of devices attached\nemulator-5554\tdevice\n",
                ("adb", Some("-s"), Some("emulator-5554")) => {
                    if args.contains(&"emu".to_string()) {
                        "Pixel_7_Pro_API_33\nOK\n"
                    } else if args.contains(&"shell".to_string()) {
                        "1\n"
                    } else {
                        ""
                    }
                }
                _ => "",
            };
            Ok(std::process::Output {
                status: unsafe { std::mem::zeroed() },
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            })
        }
        fn spawn(&self, _cmd: &str, _args: &[String]) -> Result<()> {
            Ok(())
        }
    }

    let bridge = DeviceBridge::with_runner(Arc::new(StatusMockRunner));
    // Should be Online
    assert_eq!(
        bridge.get_avd_status("Pixel_7_Pro_API_33").await.unwrap(),
        DeviceStatus::Online
    );
    // Should be Offline
    assert_eq!(
        bridge.get_avd_status("Unknown_AVD").await.unwrap(),
        DeviceStatus::Offline
    );
}
