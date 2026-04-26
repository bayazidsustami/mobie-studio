use mobie::agent::tools::{Tap, TapArgs};
use mobie::device::{CommandRunner, DeviceBridge};
use mobie::yaml_exporter::TestStep;
use rig::tool::Tool;
use std::sync::{Arc, Mutex};
use anyhow::Result;

#[derive(Debug)]
struct MockRunner;

impl CommandRunner for MockRunner {
    fn run(&self, _cmd: &str, _args: &[String]) -> Result<std::process::Output> {
        Ok(std::process::Output {
            status: std::os::unix::process::ExitStatusExt::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn spawn(&self, _cmd: &str, _args: &[String]) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_tap_tool_records_history() {
    let runner = Arc::new(MockRunner);
    let device = Arc::new(DeviceBridge::with_runner(runner));
    let history = Arc::new(Mutex::new(Vec::<TestStep>::new()));

    let tap_tool = Tap {
        device,
        history: history.clone(),
        screenshots: false,
    };

    let args = TapArgs {
        x: 100,
        y: 200,
        reasoning: "Test tap".to_string(),
    };

    // Execute the tool
    let _ = tap_tool.call(args).await;

    // Verify the history was updated
    let history_lock = history.lock().unwrap();
    assert_eq!(history_lock.len(), 1);
    
    let step = &history_lock[0];
    assert_eq!(step.action, "tap");
    assert_eq!(step.reasoning, "Test tap");
    assert_eq!(step.params.get("x").unwrap().as_u64().unwrap(), 100);
    assert_eq!(step.params.get("y").unwrap().as_u64().unwrap(), 200);
}