use mobie::agent::rig_agent::RigAgent;
use mobie::device::DeviceBridge;
use mobie::llm::LlmConfig;
use mockito::Server;

#[tokio::test]
async fn test_openrouter_headers_presence() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("HTTP-Referer", "https://mobie.studio")
        .match_header("X-Title", "Mobie Studio")
        .with_status(200)
        .with_body(r#"{"choices":[{"message":{"content":"Hello!","role":"assistant"}}]}"#)
        .create_async()
        .await;

    let config = LlmConfig {
        provider: "openai".to_string(),
        api_key: "sk-test".to_string(),
        model: "gpt-4o".to_string(),
        base_url: url.clone(),
    };

    let agent = RigAgent::new(config, DeviceBridge::new());

    // We expect this to fail or succeed depending on implementation.
    // Currently, it should FAIL because headers are not implemented yet.
    let _ = agent.prompt("Hello").await;

    mock.assert_async().await;
}
