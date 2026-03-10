use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
        }
    }
}

pub struct LlmClient {
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Returns a reference to the current config (for UI display).
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Simulates sending the UI Dump and User Goal to the LLM.
    /// TODO: Replace with actual HTTP request via reqwest to the BYOK provider.
    pub async fn think(&self, _ui_dump: &str, goal: &str) -> anyhow::Result<String> {
        info!(
            "LLM Client [{}]: Thinking about goal '{}' with current UI state...",
            self.config.model, goal
        );

        // TODO: Actual HTTP request to the BYOK provider using reqwest
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(format!("Simulated Action for goal: {}", goal))
    }
}
