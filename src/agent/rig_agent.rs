use rig::providers::openai;
use rig::agent::Agent;

pub struct RigAgent {
    // For now, we'll just hold a basic agent.
    // Rig's Agent is usually generic over the model.
    // We'll use a dynamic or specific one later.
}

impl RigAgent {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rig_agent_init() {
        let _agent = RigAgent::new();
    }
}
