#[cfg(test)]
mod tests {
    use crate::Database;
    use octo_core::config::AppConfig;
    use octo_core::message::*;
    use octo_core::model::ModelId;
    use octo_core::session::Session;
    use std::path::PathBuf;

    async fn test_db() -> (Database, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let config = AppConfig {
            working_dir: tmp.path().to_path_buf(),
            data_dir: "data".into(),
            ..Default::default()
        };
        let db = Database::open(&config).await.unwrap();
        db.run_migrations().await.unwrap();
        (db, tmp)
    }

    #[tokio::test]
    async fn test_session_crud() {
        let (db, _tmp) = test_db().await;

        // Create
        let session = Session::new("Test session".into());
        db.sessions().create(&session).await.unwrap();

        // Read
        let fetched = db.sessions().get(&session.id).await.unwrap();
        assert_eq!(fetched.title, "Test session");
        assert_eq!(fetched.message_count, 0);

        // List
        let all = db.sessions().list().await.unwrap();
        assert_eq!(all.len(), 1);

        // Update
        let mut updated = fetched;
        updated.title = "Updated title".into();
        updated.message_count = 5;
        updated.cost = 0.01;
        db.sessions().update(&updated).await.unwrap();

        let fetched2 = db.sessions().get(&session.id).await.unwrap();
        assert_eq!(fetched2.title, "Updated title");
        assert_eq!(fetched2.message_count, 5);

        // Delete
        db.sessions().delete(&session.id).await.unwrap();
        let all = db.sessions().list().await.unwrap();
        assert_eq!(all.len(), 0);
    }

    #[tokio::test]
    async fn test_message_crud() {
        let (db, _tmp) = test_db().await;

        let session = Session::new("Msg test".into());
        db.sessions().create(&session).await.unwrap();

        // Create user message
        let user_msg = Message::new_user(session.id.clone(), "Hello".into());
        db.messages().create(&user_msg).await.unwrap();

        // Create assistant message with tool call
        let mut assistant_msg =
            Message::new_assistant(session.id.clone(), ModelId("test-model".into()));
        assistant_msg.parts.push(ContentPart::Text {
            text: "Let me check.".into(),
        });
        assistant_msg.parts.push(ContentPart::ToolCall {
            id: "call-1".into(),
            name: "bash".into(),
            input: r#"{"command":"ls"}"#.into(),
        });
        assistant_msg.add_finish(FinishReason::ToolUse);
        db.messages().create(&assistant_msg).await.unwrap();

        // Create tool result message
        let tool_msg = Message::new_tool_result(
            session.id.clone(),
            vec![ContentPart::ToolResult {
                tool_call_id: "call-1".into(),
                content: "file1.rs\nfile2.rs".into(),
                is_error: false,
            }],
        );
        db.messages().create(&tool_msg).await.unwrap();

        // List all messages
        let messages = db.messages().list(&session.id).await.unwrap();
        assert_eq!(messages.len(), 3);

        // Verify message roles
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert_eq!(messages[2].role, MessageRole::Tool);

        // Verify deserialized content
        assert_eq!(messages[0].text_content(), "Hello");
        let calls = messages[1].tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "bash");
        assert_eq!(
            messages[1].finish_reason(),
            Some(FinishReason::ToolUse)
        );

        // Verify tool result
        match &messages[2].parts[0] {
            ContentPart::ToolResult {
                tool_call_id,
                content,
                is_error,
            } => {
                assert_eq!(tool_call_id, "call-1");
                assert!(content.contains("file1.rs"));
                assert!(!is_error);
            }
            _ => panic!("Expected ToolResult"),
        }

        // Delete session messages
        db.messages()
            .delete_session_messages(&session.id)
            .await
            .unwrap();
        let messages = db.messages().list(&session.id).await.unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_message_update() {
        let (db, _tmp) = test_db().await;

        let session = Session::new("Update test".into());
        db.sessions().create(&session).await.unwrap();

        let mut msg =
            Message::new_assistant(session.id.clone(), ModelId("test".into()));
        msg.parts.push(ContentPart::Text {
            text: "partial".into(),
        });
        db.messages().create(&msg).await.unwrap();

        // Update with more content
        msg.parts.clear();
        msg.parts.push(ContentPart::Text {
            text: "partial response complete".into(),
        });
        msg.add_finish(FinishReason::EndTurn);
        msg.token_usage = Some(TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        });
        db.messages().update(&msg).await.unwrap();

        let fetched = db.messages().list(&session.id).await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].text_content(), "partial response complete");
        assert_eq!(
            fetched[0].finish_reason(),
            Some(FinishReason::EndTurn)
        );
        assert_eq!(
            fetched[0].token_usage.as_ref().unwrap().input_tokens,
            100
        );
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let (db, _tmp) = test_db().await;

        for i in 0..5 {
            let s = Session::new(format!("Session {i}"));
            db.sessions().create(&s).await.unwrap();
        }

        let all = db.sessions().list().await.unwrap();
        assert_eq!(all.len(), 5);
    }
}
