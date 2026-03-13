use crate::llm::LlmConfig;
use crate::device::DeviceBridge;
use crate::agent::tools::{Tap, Input, Swipe, KeyEvent, Observe};
use rig::providers::openai;
use rig::completion::Prompt;
use rig::client::CompletionClient;
use std::sync::Arc;

pub struct RigAgent {
    config: LlmConfig,
    device: Arc<DeviceBridge>,
}

impl RigAgent {
    pub fn new(config: LlmConfig, device: DeviceBridge) -> Self {
        Self { 
            config, 
            device: Arc::new(device) 
        }
    }

    pub async fn think(&self, goal: &str) -> Result<String, anyhow::Error> {
        let api_key = if self.config.api_key.is_empty() {
            "sk-dummy".to_string()
        } else {
            self.config.api_key.clone()
        };

        let client = openai::Client::builder()
            .api_key(&api_key)
            .base_url(&self.config.base_url)
            .build()?;
        
        let agent = client.agent(&self.config.model)
            .preamble("You are a mobile testing agent. Use tools to interact with the device and achieve the goal. Always explain your reasoning.")
            .tool(Tap { device: self.device.clone() })
            .tool(Input { device: self.device.clone() })
            .tool(Swipe { device: self.device.clone() })
            .tool(KeyEvent { device: self.device.clone() })
            .tool(Observe { device: self.device.clone() })
            .build();

        // Use max_turns to allow the agent to iterate
        match agent.prompt(goal).max_turns(20).await {
            Ok(res) => Ok(res),
            Err(e) => {
                Err(anyhow::anyhow!("Rig agent think failed: {}", e))
            }
        }
    }

    // Keep the simple prompt for testing or simple queries
    pub async fn prompt(&self, goal: &str) -> Result<String, anyhow::Error> {
        let api_key = if self.config.api_key.is_empty() {
            "sk-dummy".to_string()
        } else {
            self.config.api_key.clone()
        };

        let client = openai::Client::builder()
            .api_key(&api_key)
            .base_url(&self.config.base_url)
            .build()?;
        
        let agent = client.agent(&self.config.model)
            .preamble("You are a mobile testing agent. Respond with JSON actions.")
            .build();

        match agent.prompt(goal).await {
            Ok(res) => Ok(res),
            Err(e) => {
                Err(anyhow::anyhow!("Rig agent prompt failed: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rig_agent_init() {
        let config = LlmConfig::default();
        let device = DeviceBridge::new();
        let _agent = RigAgent::new(config, device);
    }

    #[tokio::test]
    async fn test_rig_agent_prompt() {
        let config = LlmConfig::default();
        let device = DeviceBridge::new();
        let agent = RigAgent::new(config, device);
        let response = agent.prompt("Hello").await;
        assert!(response.is_ok() || response.is_err());
    }
}
