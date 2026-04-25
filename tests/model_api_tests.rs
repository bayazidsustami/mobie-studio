use mobie::llm::ModelListResponse;

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
