use serde::{Deserialize, Serialize};

pub mod prompt;

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
