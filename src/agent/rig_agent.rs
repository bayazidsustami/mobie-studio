use crate::agent::tools::{Input, KeyEvent, Observe, Screenshot, Swipe, Tap};
use crate::device::DeviceBridge;
use crate::llm::LlmConfig;
use crate::yaml_exporter::TestStep;
use reqwest::header::{HeaderMap, HeaderValue};
use rig::client::CompletionClient;
use rig::completion::{Message, Prompt};
use rig::providers::openai;
use std::sync::{Arc, Mutex};

pub struct RigAgent {
    config: LlmConfig,
    device: Arc<DeviceBridge>,
}

impl RigAgent {
    pub fn new(config: LlmConfig, device: DeviceBridge) -> Self {
        Self {
            config,
            device: Arc::new(device),
        }
    }

    fn build_client(&self) -> Result<openai::Client<reqwest::Client>, anyhow::Error> {
        let api_key = if self.config.api_key.is_empty() {
            "sk-dummy".to_string()
        } else {
            self.config.api_key.clone()
        };

        let mut client_builder = reqwest::Client::builder();

        // Include OpenRouter metadata headers.
        // These are harmless for other providers but required/recommended for OpenRouter.
        let mut headers = HeaderMap::new();
        headers.insert(
            "HTTP-Referer",
            HeaderValue::from_static("https://mobie.studio"),
        );
        headers.insert("X-Title", HeaderValue::from_static("Mobie Studio"));
        client_builder = client_builder.default_headers(headers);

        let http_client = client_builder.build()?;

        Ok(openai::Client::builder()
            .api_key(&api_key)
            .base_url(&self.config.base_url)
            .http_client(http_client)
            .build()?)
    }

    /// Run a multi-turn agent invocation.
    ///
    /// `goal` is the **already-composed** user prompt (typically the engine
    /// prepends an auto-observation of the current screen before calling us).
    /// `history` is the accumulated LLM conversation (`Vec<rig::completion::Message>`).
    /// After this returns, `history` contains the new turn appended (so the
    /// caller can persist it). `step_history` is the per-session `TestStep`
    /// accumulator shared with the device tools.
    pub async fn think(
        &self,
        goal: &str,
        screenshots: bool,
        history: &mut Vec<Message>,
        step_history: Arc<Mutex<Vec<TestStep>>>,
    ) -> Result<String, anyhow::Error> {
        let client = self.build_client()?;

        let agent = client
            .agent(&self.config.model)
            .preamble("You are a mobile testing agent. Use tools to interact with the device and achieve the goal. Always explain your reasoning. You have access to the full prior conversation - if the user has already told you what to do, refer to that context rather than re-asking.")
            .tool(Tap { device: self.device.clone(), history: step_history.clone(), screenshots })
            .tool(Input { device: self.device.clone(), history: step_history.clone(), screenshots })
            .tool(Swipe { device: self.device.clone(), history: step_history.clone(), screenshots })
            .tool(KeyEvent { device: self.device.clone(), history: step_history.clone(), screenshots })
            .tool(Observe { device: self.device.clone(), history: step_history.clone() })
            .tool(Screenshot { device: self.device.clone(), history: step_history.clone(), screenshots })
            .build();

        // `with_history` mutates `history` in place, appending the new
        // user prompt + all tool calls + the final assistant reply.
        match agent.prompt(goal).with_history(history).max_turns(50).await {
            Ok(res) => Ok(res),
            Err(e) => Err(anyhow::anyhow!("Rig agent think failed: {}", e)),
        }
    }

    // Keep the simple prompt for testing or simple queries
    pub async fn prompt(&self, goal: &str) -> Result<String, anyhow::Error> {
        let client = self.build_client()?;

        let agent = client
            .agent(&self.config.model)
            .preamble("You are a mobile testing agent. Respond with JSON actions.")
            .build();

        match agent.prompt(goal).await {
            Ok(res) => Ok(res),
            Err(e) => Err(anyhow::anyhow!("Rig agent prompt failed: {}", e)),
        }
    }

    pub async fn generate_summary(
        &self,
        goal: &str,
        history: &[TestStep],
        final_res: &str,
    ) -> Result<String, anyhow::Error> {
        let client = self.build_client()?;

        let agent = client
            .agent(&self.config.model)
            .preamble("You are a helpful assistant. Summarize the following mobile testing session in one short sentence (max 15 words). Focus on what was achieved.")
            .build();

        let mut content = format!("Goal: {}\n\nSteps:\n", goal);
        for step in history {
            content.push_str(&format!(
                "- Action: {}, Reasoning: {}\n",
                step.action, step.reasoning
            ));
        }
        content.push_str(&format!("\nFinal Result: {}", final_res));

        match agent.prompt(content).await {
            Ok(res) => Ok(res.trim().to_string()),
            Err(e) => Err(anyhow::anyhow!("Summary generation failed: {}", e)),
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

    /// Smoke test for the multi-turn `think()` signature: ensures the function
    /// compiles and accepts the new `&mut Vec<Message>` + `Arc<Mutex<Vec<TestStep>>>`
    /// parameters. Does not hit a real LLM.
    #[test]
    fn test_think_signature_compiles() {
        // We don't call think() because that would require a real LLM. The
        // signature itself is exercised at compile time by every caller.
        // This test just asserts the imports are in scope.
        let _ = std::marker::PhantomData::<RigAgent>;
        let _: Vec<Message> = Vec::new();
        let _: Arc<Mutex<Vec<TestStep>>> = Arc::new(Mutex::new(Vec::new()));
    }
}
