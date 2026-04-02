use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai;

#[tokio::test]
async fn test_rig_initialization() {
    std::env::set_var("OPENAI_API_KEY", "sk-dummy");

    let client = openai::Client::from_env();

    let _agent = client
        .agent("gpt-4o")
        .preamble("You are a test agent.")
        .build();

    // Verification: If we reached here without panicking, initialization worked.
    assert!(true);
}
