use super::message::*;
use super::model::*;
use super::session::*;

#[test]
fn test_message_creation() {
    let msg = Message::new_user("session-1".into(), "Hello world".into());
    assert_eq!(msg.role, MessageRole::User);
    assert_eq!(msg.session_id, "session-1");
    assert_eq!(msg.text_content(), "Hello world");
    assert!(!msg.id.is_empty());
}

#[test]
fn test_assistant_message() {
    let mut msg =
        Message::new_assistant("session-1".into(), ModelId("test-model".into()));
    assert_eq!(msg.role, MessageRole::Assistant);
    assert!(msg.text_content().is_empty());

    msg.append_text("Hello ");
    msg.append_text("world!");
    assert_eq!(msg.text_content(), "Hello world!");
}

#[test]
fn test_tool_call_message() {
    let mut msg =
        Message::new_assistant("session-1".into(), ModelId("test".into()));
    msg.add_tool_call(
        "call-1".into(),
        "bash".into(),
        r#"{"command":"ls"}"#.into(),
    );

    let calls = msg.tool_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "call-1");
    assert_eq!(calls[0].1, "bash");
}

#[test]
fn test_tool_result_message() {
    let results = vec![ContentPart::ToolResult {
        tool_call_id: "call-1".into(),
        content: "file1.rs\nfile2.rs".into(),
        is_error: false,
    }];
    let msg = Message::new_tool_result("session-1".into(), results);
    assert_eq!(msg.role, MessageRole::Tool);
    assert_eq!(msg.parts.len(), 1);
}

#[test]
fn test_finish_reason() {
    let mut msg =
        Message::new_assistant("s1".into(), ModelId("m".into()));
    assert!(msg.finish_reason().is_none());

    msg.add_finish(FinishReason::EndTurn);
    assert_eq!(msg.finish_reason(), Some(FinishReason::EndTurn));
}

#[test]
fn test_content_part_serialization() {
    let parts = vec![
        ContentPart::Text {
            text: "hello".into(),
        },
        ContentPart::ToolCall {
            id: "1".into(),
            name: "bash".into(),
            input: "{}".into(),
        },
        ContentPart::ToolResult {
            tool_call_id: "1".into(),
            content: "ok".into(),
            is_error: false,
        },
    ];

    let json = serde_json::to_string(&parts).unwrap();
    let deserialized: Vec<ContentPart> = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.len(), 3);

    match &deserialized[0] {
        ContentPart::Text { text } => assert_eq!(text, "hello"),
        _ => panic!("Expected Text"),
    }
    match &deserialized[1] {
        ContentPart::ToolCall { name, .. } => assert_eq!(name, "bash"),
        _ => panic!("Expected ToolCall"),
    }
}

#[test]
fn test_session_creation() {
    let session = Session::new("Test session".into());
    assert!(!session.id.is_empty());
    assert_eq!(session.title, "Test session");
    assert_eq!(session.message_count, 0);
    assert_eq!(session.cost, 0.0);
}

#[test]
fn test_model_registry() {
    let models = builtin_models();
    assert_eq!(models.len(), 6);

    let glm = models.get(&ModelId("zai-org/glm-5".into()));
    assert!(glm.is_some());
    let glm = glm.unwrap();
    assert_eq!(glm.vendor, ModelVendor::Zhipu);
    assert!(glm.capabilities.supports_tool_use);
    assert!(glm.capabilities.supports_streaming);
    assert_eq!(glm.context_window, 202_752);
}

#[test]
fn test_model_cost_calculation() {
    let model = get_model(&ModelId("zai-org/glm-5".into())).unwrap();
    // 1000 input tokens, 500 output tokens
    let cost = model.calculate_cost(1000, 500);
    // (1000/1M * 0.80) + (500/1M * 2.56) = 0.0008 + 0.00128 = 0.00208
    assert!((cost - 0.00208).abs() < 0.0001);
}

#[test]
fn test_all_six_models_exist() {
    let ids = [
        "zai-org/glm-5",
        "zai-org/glm-4.7",
        "deepseek-ai/deepseek-v3.2-speciale",
        "qwen/qwen3-max-2026-01-23",
        "Qwen/Qwen3-Coder",
        "moonshotai/kimi-k2.5",
    ];
    for id in &ids {
        assert!(get_model(&ModelId(id.to_string())).is_some(), "Model {id} not found");
    }
}

#[test]
fn test_default_model() {
    let model = get_default_model();
    assert_eq!(model.id.0, "zai-org/glm-5");
}

#[test]
fn test_config_defaults() {
    let config = crate::core::config::AppConfig::default();
    assert_eq!(config.data_dir, ".octo-code");
    assert!(!config.debug);
    assert!(config.api_key.is_none());
    assert_eq!(config.base_url, "https://api.atlascloud.ai");
    assert_eq!(config.agent.coder_model.0, "zai-org/glm-5");
    assert_eq!(config.agent.max_tokens, 16_384);
}

#[test]
fn test_config_has_api_key() {
    let mut config = crate::core::config::AppConfig::default();
    assert!(!config.has_api_key());

    config.api_key = Some("test-key".into());
    assert!(config.has_api_key());

    config.api_key = Some("".into());
    assert!(!config.has_api_key());
}

#[test]
fn test_message_role_serialization() {
    let role = MessageRole::Assistant;
    let json = serde_json::to_string(&role).unwrap();
    assert_eq!(json, "\"assistant\"");

    let deserialized: MessageRole = serde_json::from_str("\"user\"").unwrap();
    assert_eq!(deserialized, MessageRole::User);
}

#[test]
fn test_finish_reason_serialization() {
    let reason = FinishReason::ToolUse;
    let json = serde_json::to_string(&reason).unwrap();
    assert_eq!(json, "\"tool_use\"");

    let deserialized: FinishReason = serde_json::from_str("\"end_turn\"").unwrap();
    assert_eq!(deserialized, FinishReason::EndTurn);
}
