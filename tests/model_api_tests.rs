use mobie::llm::{fetch_models, ModelListResponse};
use mockito::Server;

#[test]
fn test_deserialize_model_list() {
    let json = r#"{
        "data": [
            {
                "id": "openai/gpt-4o",
                "name": "OpenAI: GPT-4o"
            },
            {
                "id": "anthropic/claude-3.5-sonnet",
                "name": "Anthropic: Claude 3.5 Sonnet"
            }
        ]
    }"#;

    let response: ModelListResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.data.len(), 2);
    assert_eq!(response.data[0].id, "openai/gpt-4o");
    assert_eq!(response.data[1].name.as_deref(), Some("Anthropic: Claude 3.5 Sonnet"));
}

#[tokio::test]
async fn test_fetch_models_success() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _m = server.mock("GET", "/models")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "data": [
                { "id": "m1", "name": "Model 1" },
                { "id": "m2", "name": "Model 2" }
            ]
        }"#)
        .create_async()
        .await;

    let models = fetch_models(&url, "fake_key").await.unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "m1");
    assert_eq!(models[1].name.as_deref(), Some("Model 2"));
}

#[tokio::test]
async fn test_fetch_models_failure() {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _m = server.mock("GET", "/models")
        .with_status(500)
        .with_body("Internal Server Error")
        .create_async()
        .await;

    let result = fetch_models(&url, "fake_key").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("500"));
}
