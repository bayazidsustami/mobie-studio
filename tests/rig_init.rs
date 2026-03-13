use rig::providers::openai;
use rig::client::{ProviderClient, CompletionClient};

#[tokio::test]
async fn test_rig_initialization() {
    std::env::set_var("OPENAI_API_KEY", "sk-dummy");
    
    let client = openai::Client::from_env();
    
    let agent = client.agent("gpt-4o")
        .preamble("You are a test agent.")
        .build();
        
    // Verification: If we reached here without panicking, initialization worked.
    assert!(true);
}
