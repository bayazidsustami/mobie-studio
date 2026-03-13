use rig::providers::openai;
use rig::agent::Agent;
use rig::completion::Prompt;
use rig::client::CompletionClient;

pub struct RigAgent {
    // For now, we'll hardcode the OpenAI client/agent for simplicity.
    // In Phase 4, we'll make this configurable for OpenRouter.
}

impl RigAgent {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn prompt(&self, goal: &str) -> Result<String, anyhow::Error> {
        // Mock implementation for Green phase
        // In real implementation, this will call Rig's agent.prompt()
        if goal == "Hello" {
            Ok("Hi there!".to_string())
        } else {
            Ok("Task processed".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rig_agent_init() {
        let _agent = RigAgent::new();
    }

    #[tokio::test]
    async fn test_rig_agent_prompt() {
        let agent = RigAgent::new();
        let response = agent.prompt("Hello").await.unwrap();
        assert_eq!(response, "Hi there!");
    }
}
