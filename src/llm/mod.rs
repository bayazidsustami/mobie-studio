use serde::{Deserialize, Serialize};

pub mod prompt;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LlmConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelData {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelListResponse {
    pub data: Vec<ModelData>,
}

pub async fn fetch_models(base_url: &str, api_key: &str) -> anyhow::Result<Vec<ModelData>> {
    let url = if base_url.ends_with('/') {
        format!("{}models", base_url)
    } else {
        format!("{}/models", base_url)
    };

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Failed to fetch models: {} - {}", status, body);
    }

    let model_list: ModelListResponse = response.json().await?;
    Ok(model_list.data)
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
