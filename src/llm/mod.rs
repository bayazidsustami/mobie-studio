use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::agent::action::Action;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// System Prompt
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = r#"You are a mobile QA agent that controls an Android device to achieve user goals.

## Input
You receive the user's GOAL and a COMPRESSED UI DUMP — a flat, indexed list of visible UI elements:
```
[0] Button "Settings" bounds=[100,200][300,400] clickable
[1] TextView "Battery 85%" id=battery_text bounds=[50,500][400,550]
```

Each element shows: [index] ClassName "text" desc="content-desc" id=resource-id bounds=[x1,y1][x2,y2] flags

## Your Task
Decide the SINGLE NEXT ACTION to take. Respond ONLY with a JSON object — no markdown, no explanation outside JSON.

## Available Actions
- **tap**: Tap a UI element. Use center of its bounds.
  `{"action": "tap", "x": 200, "y": 300, "reasoning": "Tapping Settings"}`
- **input**: Type text into the focused field.
  `{"action": "input", "text": "hello", "reasoning": "Typing search query"}`
- **swipe**: Scroll the screen.
  `{"action": "swipe", "direction": "up"|"down"|"left"|"right", "x": 540, "y": 1200, "reasoning": "Scrolling down"}`
- **key_event**: Press a key. Common codes: Back=4, Home=3, Enter=66.
  `{"action": "key_event", "code": 4, "reasoning": "Going back"}`
- **done**: Goal is achieved or impossible.
  `{"action": "done", "success": true, "reason": "Settings page is now open"}`

## Rules
1. Output ONLY valid JSON — no markdown fences, no extra text.
2. Always include a "reasoning" field explaining your decision.
3. Calculate tap coordinates from element bounds (use center point).
4. If you cannot find the target, try scrolling or going back.
5. If the goal seems achieved, respond with "done" and success=true.
6. If the goal is impossible after multiple attempts, respond with "done" and success=false.
"#;

// ---------------------------------------------------------------------------
// OpenAI-compatible request/response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

// ---------------------------------------------------------------------------
// LlmClient
// ---------------------------------------------------------------------------

pub struct LlmClient {
    config: LlmConfig,
    http: Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }

    /// Returns a reference to the current config (for UI display).
    pub fn config(&self) -> &LlmConfig {
        &self.config
    }

    /// Send the compressed UI dump and goal to the LLM and parse an Action.
    ///
    /// Uses the OpenAI-compatible `/v1/chat/completions` endpoint.
    /// If no API key is configured, returns a simulated action for development.
    pub async fn think(&self, compressed_ui: &str, goal: &str) -> Result<Action> {
        info!(
            "LLM Client [{}]: Thinking about goal '{}'",
            self.config.model, goal
        );

        // --- Dev mode: no API key → return simulated action ---
        if self.config.api_key.is_empty() {
            warn!("No API key configured — returning simulated Done action");
            return Ok(Action::Done {
                success: false,
                reason: "No LLM API key configured. Set your API key in Settings.".to_string(),
            });
        }

        let user_message = format!(
            "GOAL: {}\n\nCURRENT SCREEN:\n{}",
            goal, compressed_ui
        );

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: SYSTEM_PROMPT.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message,
                },
            ],
            temperature: 0.1,
        };

        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to LLM API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "LLM API returned error {}: {}",
                status,
                body
            ));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse LLM API response")?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        info!("LLM raw response: {}", content);

        // Strip potential markdown code fences
        let cleaned = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let action: Action = serde_json::from_str(cleaned).context(format!(
            "Failed to parse LLM response as Action JSON: {}",
            cleaned
        ))?;

        Ok(action)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_llm_client_request_formatting() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let mock = server.mock("POST", "/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(r#"{
                "choices": [
                    {
                        "message": {
                            "content": "{\"action\": \"done\", \"success\": true, \"reason\": \"test\"}"
                        }
                    }
                ]
            }"#)
            .create_async()
            .await;

        let config = LlmConfig {
            api_key: "test-key".to_string(),
            base_url: url,
            ..LlmConfig::default()
        };

        let client = LlmClient::new(config);
        let result = client.think("compressed xml", "test goal").await;

        assert!(result.is_ok());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_llm_client_error_handling() {
        let mut server = Server::new_async().await;
        let url = server.url();

        let _mock = server.mock("POST", "/chat/completions")
            .with_status(500)
            .with_body("Internal Server Error")
            .create_async()
            .await;

        let config = LlmConfig {
            api_key: "test-key".to_string(),
            base_url: url,
            ..LlmConfig::default()
        };

        let client = LlmClient::new(config);
        let result = client.think("xml", "goal").await;

        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("LLM API returned error 500"));
    }
}
