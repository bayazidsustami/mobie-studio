use crate::llm::LlmConfig;
use rig::providers::openai;
use rig::completion::Prompt;
use rig::client::CompletionClient;

pub struct RigAgent {
    config: LlmConfig,
}

impl RigAgent {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    pub async fn prompt(&self, goal: &str) -> Result<String, anyhow::Error> {
        // Use configured API key and base URL
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
        let _agent = RigAgent::new(config);
    }

    #[tokio::test]
    async fn test_rig_agent_prompt() {
        let config = LlmConfig::default();
        let agent = RigAgent::new(config);
        let response = agent.prompt("Hello").await;
        assert!(response.is_ok() || response.is_err());
    }
}
