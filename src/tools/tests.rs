use crate::core::team::TeamState;
use crate::core::tool::{Tool, ToolCall, ToolContext};
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

fn test_context(dir: &std::path::Path) -> ToolContext {
    ToolContext {
        session_id: "test-session".into(),
        working_dir: dir.to_path_buf(),
        cancel_token: CancellationToken::new(),
        team_state: Arc::new(RwLock::new(None)),
    }
}

fn test_team_state(base_dir: &std::path::Path) -> Arc<RwLock<Option<TeamState>>> {
    Arc::new(RwLock::new(Some(
        TeamState::new("test-team".into(), "test-lead".into(), true)
            .with_base_dir(base_dir.to_path_buf()),
    )))
}

fn test_context_with_team(
    dir: &std::path::Path,
    team_state: Arc<RwLock<Option<TeamState>>>,
) -> ToolContext {
    ToolContext {
        session_id: "test-session".into(),
        working_dir: dir.to_path_buf(),
        cancel_token: CancellationToken::new(),
        team_state,
    }
}

#[tokio::test]
async fn test_tool_definitions() {
    use super::*;
    use crate::core::permission::PermissionService;
    use std::sync::Arc;

    struct MockPerm;
    #[async_trait::async_trait]
    impl PermissionService for MockPerm {
        async fn request(
            &self,
            _req: crate::core::permission::PermissionRequest,
        ) -> crate::core::permission::PermissionDecision {
            crate::core::permission::PermissionDecision::Allow
        }
        fn auto_approve_session(&self, _session_id: &str) {}
    }

    let perm: Arc<dyn PermissionService> = Arc::new(MockPerm);
    let team_state = Arc::new(RwLock::new(None));
    let tools = create_all_tools(perm, "http://127.0.0.1:19999".into(), team_state).await;

    // 16 tools without CodeRLM, 17 with CodeRLM server running
    assert!(
        tools.len() >= 16,
        "Expected at least 16 tools, got {}",
        tools.len()
    );

    let names: Vec<String> = tools.iter().map(|t| t.definition().name).collect();
    assert!(names.contains(&"bash".to_string()));
    assert!(names.contains(&"view".to_string()));
    assert!(names.contains(&"write".to_string()));
    assert!(names.contains(&"edit".to_string()));
    assert!(names.contains(&"ls".to_string()));
    assert!(names.contains(&"glob".to_string()));
    assert!(names.contains(&"grep".to_string()));
    // coderlm is optional — only present if server is reachable
    // Team tools
    assert!(names.contains(&"team_create".to_string()));
    assert!(names.contains(&"team_delete".to_string()));
    assert!(names.contains(&"spawn_agent".to_string()));
    assert!(names.contains(&"task_create".to_string()));
    assert!(names.contains(&"task_get".to_string()));
    assert!(names.contains(&"task_update".to_string()));
    assert!(names.contains(&"task_list".to_string()));
    assert!(names.contains(&"send_message".to_string()));
    assert!(names.contains(&"check_inbox".to_string()));

    // Check that all tools have non-empty descriptions
    for tool in &tools {
        let def = tool.definition();
        assert!(
            !def.description.is_empty(),
            "Tool {} has empty description",
            def.name
        );
        assert!(!def.name.is_empty());
    }
}

#[test]
fn test_coderlm_tool_definition() {
    let tool = super::CoderlmTool::new("http://127.0.0.1:9999".into());
    let def = tool.definition();

    assert_eq!(def.name, "coderlm");
    assert!(!def.description.is_empty());
    assert!(def.parameters.contains_key("operation"));
    assert!(def.required.contains(&"operation".to_string()));

    // Verify operation enum values
    let op_param = &def.parameters["operation"];
    let enum_vals = op_param.enum_values.as_ref().unwrap();
    assert!(enum_vals.contains(&"health".to_string()));
    assert!(enum_vals.contains(&"search".to_string()));
    assert!(enum_vals.contains(&"implementation".to_string()));
    assert!(enum_vals.contains(&"callers".to_string()));
    assert!(enum_vals.contains(&"tests".to_string()));
    assert!(enum_vals.contains(&"structure".to_string()));
    assert!(enum_vals.contains(&"symbols".to_string()));
    assert!(enum_vals.contains(&"variables".to_string()));
    assert!(enum_vals.contains(&"peek".to_string()));
    assert!(enum_vals.contains(&"grep".to_string()));
    assert_eq!(enum_vals.len(), 10);
}

#[tokio::test]
async fn test_coderlm_server_unavailable() {
    // Use a port that is almost certainly not running CodeRLM
    let tool = super::CoderlmTool::new("http://127.0.0.1:19999".into());
    let ctx = test_context(std::path::Path::new("/tmp"));

    let call = ToolCall {
        id: "1".into(),
        name: "coderlm".into(),
        input: serde_json::json!({"operation": "health"}).to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    // Should return error result, not Err (graceful degradation)
    assert!(result.is_error);
    assert!(result.content.contains("not reachable") || result.content.contains("CodeRLM"));
}

#[tokio::test]
async fn test_coderlm_invalid_operation() {
    let tool = super::CoderlmTool::new("http://127.0.0.1:19999".into());
    let ctx = test_context(std::path::Path::new("/tmp"));

    let call = ToolCall {
        id: "1".into(),
        name: "coderlm".into(),
        input: serde_json::json!({"operation": "nonexistent"}).to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("Unknown operation"));
}

#[tokio::test]
async fn test_coderlm_missing_required_params() {
    let tool = super::CoderlmTool::new("http://127.0.0.1:19999".into());
    let ctx = test_context(std::path::Path::new("/tmp"));

    // 'search' requires 'query' parameter
    let call = ToolCall {
        id: "1".into(),
        name: "coderlm".into(),
        input: serde_json::json!({"operation": "search"}).to_string(),
    };

    let result = tool.run(&call, &ctx).await;
    assert!(result.is_err()); // Should be InvalidParams error
}

#[tokio::test]
async fn test_view_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let test_file = tmp.path().join("test.txt");
    std::fs::write(&test_file, "line 1\nline 2\nline 3\nline 4\nline 5\n").unwrap();

    let tool = super::ViewTool;
    let ctx = test_context(tmp.path());

    // Test basic view
    let call = ToolCall {
        id: "1".into(),
        name: "view".into(),
        input: serde_json::json!({"path": test_file.to_str().unwrap()}).to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("line 1"));
    assert!(result.content.contains("line 5"));

    // Test with offset and limit
    let call = ToolCall {
        id: "2".into(),
        name: "view".into(),
        input: serde_json::json!({
            "path": test_file.to_str().unwrap(),
            "offset": 2,
            "limit": 2
        })
        .to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("line 2"));
    assert!(result.content.contains("line 3"));
}

#[tokio::test]
async fn test_view_tool_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let tool = super::ViewTool;
    let ctx = test_context(tmp.path());

    let call = ToolCall {
        id: "1".into(),
        name: "view".into(),
        input: serde_json::json!({"path": "/nonexistent/file.txt"}).to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("not found"));
}

#[tokio::test]
async fn test_ls_tool() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("file1.rs"), "").unwrap();
    std::fs::write(tmp.path().join("file2.txt"), "hello").unwrap();
    std::fs::create_dir(tmp.path().join("subdir")).unwrap();

    let tool = super::LsTool;
    let ctx = test_context(tmp.path());

    let call = ToolCall {
        id: "1".into(),
        name: "ls".into(),
        input: "{}".into(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("file1.rs"));
    assert!(result.content.contains("file2.txt"));
    assert!(result.content.contains("subdir/"));
}

#[tokio::test]
async fn test_glob_tool() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a.rs"), "").unwrap();
    std::fs::write(tmp.path().join("b.rs"), "").unwrap();
    std::fs::write(tmp.path().join("c.txt"), "").unwrap();

    let tool = super::GlobTool;
    let ctx = test_context(tmp.path());

    let call = ToolCall {
        id: "1".into(),
        name: "glob".into(),
        input: serde_json::json!({"pattern": "*.rs"}).to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("a.rs"));
    assert!(result.content.contains("b.rs"));
    assert!(!result.content.contains("c.txt"));
    assert!(result.content.contains("2 files found"));
}

#[tokio::test]
async fn test_write_and_edit_tools() {
    use crate::core::permission::{PermissionDecision, PermissionService};
    use std::sync::Arc;

    struct AutoApprove;
    #[async_trait::async_trait]
    impl PermissionService for AutoApprove {
        async fn request(
            &self,
            _req: crate::core::permission::PermissionRequest,
        ) -> PermissionDecision {
            PermissionDecision::Allow
        }
        fn auto_approve_session(&self, _session_id: &str) {}
    }

    let tmp = tempfile::tempdir().unwrap();
    let perm: Arc<dyn PermissionService> = Arc::new(AutoApprove);
    let ctx = test_context(tmp.path());

    // Test write
    let write_tool = super::WriteTool::new(perm.clone());
    let file_path = tmp.path().join("new_file.rs");
    let call = ToolCall {
        id: "1".into(),
        name: "write".into(),
        input: serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "fn main() {\n    println!(\"hello\");\n}\n"
        })
        .to_string(),
    };

    let result = write_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("3 lines"));
    assert!(file_path.exists());

    // Test edit
    let edit_tool = super::EditTool::new(perm.clone());
    let call = ToolCall {
        id: "2".into(),
        name: "edit".into(),
        input: serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "println!(\"hello\")",
            "new_string": "println!(\"world\")"
        })
        .to_string(),
    };

    let result = edit_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);

    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("world"));
    assert!(!content.contains("hello"));
}

#[tokio::test]
async fn test_edit_tool_not_unique() {
    use crate::core::permission::{PermissionDecision, PermissionService};
    use std::sync::Arc;

    struct AutoApprove;
    #[async_trait::async_trait]
    impl PermissionService for AutoApprove {
        async fn request(
            &self,
            _req: crate::core::permission::PermissionRequest,
        ) -> PermissionDecision {
            PermissionDecision::Allow
        }
        fn auto_approve_session(&self, _session_id: &str) {}
    }

    let tmp = tempfile::tempdir().unwrap();
    let perm: Arc<dyn PermissionService> = Arc::new(AutoApprove);
    let ctx = test_context(tmp.path());

    let file_path = tmp.path().join("dup.txt");
    std::fs::write(&file_path, "hello\nhello\nhello\n").unwrap();

    let edit_tool = super::EditTool::new(perm);
    let call = ToolCall {
        id: "1".into(),
        name: "edit".into(),
        input: serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "hello",
            "new_string": "world"
        })
        .to_string(),
    };

    let result = edit_tool.run(&call, &ctx).await.unwrap();
    assert!(result.is_error);
    assert!(result.content.contains("3 times"));
}

#[tokio::test]
async fn test_bash_safe_command() {
    use crate::core::permission::{PermissionDecision, PermissionService};
    use std::sync::Arc;

    struct NeverApprove;
    #[async_trait::async_trait]
    impl PermissionService for NeverApprove {
        async fn request(
            &self,
            _req: crate::core::permission::PermissionRequest,
        ) -> PermissionDecision {
            PermissionDecision::Deny
        }
        fn auto_approve_session(&self, _session_id: &str) {}
    }

    let tmp = tempfile::tempdir().unwrap();
    let perm: Arc<dyn PermissionService> = Arc::new(NeverApprove);
    let ctx = test_context(tmp.path());

    // "echo hello" is a safe command, should work even with NeverApprove
    let bash_tool = super::BashTool::new(perm);
    let call = ToolCall {
        id: "1".into(),
        name: "bash".into(),
        input: serde_json::json!({"command": "echo hello world"}).to_string(),
    };

    let result = bash_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("hello world"));
}

#[tokio::test]
async fn test_bash_unsafe_command_denied() {
    use crate::core::permission::{PermissionDecision, PermissionService};
    use std::sync::Arc;

    struct NeverApprove;
    #[async_trait::async_trait]
    impl PermissionService for NeverApprove {
        async fn request(
            &self,
            _req: crate::core::permission::PermissionRequest,
        ) -> PermissionDecision {
            PermissionDecision::Deny
        }
        fn auto_approve_session(&self, _session_id: &str) {}
    }

    let tmp = tempfile::tempdir().unwrap();
    let perm: Arc<dyn PermissionService> = Arc::new(NeverApprove);
    let ctx = test_context(tmp.path());

    // "rm -rf /" is unsafe, should be denied
    let bash_tool = super::BashTool::new(perm);
    let call = ToolCall {
        id: "1".into(),
        name: "bash".into(),
        input: serde_json::json!({"command": "rm -rf /"}).to_string(),
    };

    let result = bash_tool.run(&call, &ctx).await;
    assert!(result.is_err());
}

// -----------------------------------------------------------------------
// Team tool tests
// -----------------------------------------------------------------------

#[test]
fn test_team_tool_definitions() {
    let ts = Arc::new(RwLock::new(None));

    let team_create = super::TeamCreateTool::new(ts.clone());
    let def = team_create.definition();
    assert_eq!(def.name, "team_create");
    assert!(def.required.contains(&"team_name".to_string()));

    let team_delete = super::TeamDeleteTool::new(ts.clone());
    assert_eq!(team_delete.definition().name, "team_delete");

    let spawn = super::SpawnAgentTool::new(ts.clone());
    let def = spawn.definition();
    assert_eq!(def.name, "spawn_agent");
    assert!(def.required.contains(&"name".to_string()));
    assert!(def.required.contains(&"prompt".to_string()));

    let tc = super::TaskCreateTool::new(ts.clone());
    let def = tc.definition();
    assert_eq!(def.name, "task_create");
    assert!(def.required.contains(&"subject".to_string()));

    let tg = super::TaskGetTool::new(ts.clone());
    assert_eq!(tg.definition().name, "task_get");

    let tu = super::TaskUpdateTool::new(ts.clone());
    let def = tu.definition();
    assert_eq!(def.name, "task_update");
    assert!(def.parameters.contains_key("status"));

    let tl = super::TaskListTool::new(ts.clone());
    assert_eq!(tl.definition().name, "task_list");

    let sm = super::SendMessageTool::new(ts.clone());
    let def = sm.definition();
    assert_eq!(def.name, "send_message");
    assert!(def.required.contains(&"type".to_string()));

    let ci = super::CheckInboxTool::new(ts.clone());
    let def = ci.definition();
    assert_eq!(def.name, "check_inbox");
    assert!(def.parameters.contains_key("wait_seconds"));
}

#[tokio::test]
async fn test_team_create_and_delete_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let ts: Arc<RwLock<Option<TeamState>>> = Arc::new(RwLock::new(Some(
        TeamState::new("_unused".into(), "lead".into(), true)
            .with_base_dir(tmp.path().to_path_buf()),
    )));
    let ctx = test_context_with_team(tmp.path(), ts.clone());

    // Create team
    let tool = super::TeamCreateTool::new(ts.clone());
    let call = ToolCall {
        id: "1".into(),
        name: "team_create".into(),
        input: serde_json::json!({
            "team_name": "my-team",
            "description": "Test team"
        })
        .to_string(),
    };

    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("my-team"));

    // Verify config file exists
    let config_path = tmp.path().join("teams/my-team/config.json");
    assert!(config_path.exists());

    // Verify team state was updated
    {
        let state = ts.read().unwrap();
        let st = state.as_ref().unwrap();
        assert_eq!(st.team_name, "my-team");
        assert!(st.is_lead);
    }

    // Delete team
    let del_tool = super::TeamDeleteTool::new(ts.clone());
    let call = ToolCall {
        id: "2".into(),
        name: "team_delete".into(),
        input: "{}".into(),
    };
    let result = del_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("deleted"));

    // Verify directories removed
    assert!(!config_path.exists());

    // Verify team state cleared
    assert!(ts.read().unwrap().is_none());
}

#[tokio::test]
async fn test_task_crud_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let ts = test_team_state(tmp.path());
    let ctx = test_context_with_team(tmp.path(), ts.clone());

    // Ensure tasks dir exists
    let tasks_dir = tmp.path().join("tasks/test-team");
    std::fs::create_dir_all(&tasks_dir).unwrap();

    // Create task
    let create_tool = super::TaskCreateTool::new(ts.clone());
    let call = ToolCall {
        id: "1".into(),
        name: "task_create".into(),
        input: serde_json::json!({
            "subject": "Implement feature X",
            "description": "Build the new feature",
            "active_form": "Implementing feature X"
        })
        .to_string(),
    };
    let result = create_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("\"id\": \"1\""));

    // Get task
    let get_tool = super::TaskGetTool::new(ts.clone());
    let call = ToolCall {
        id: "2".into(),
        name: "task_get".into(),
        input: serde_json::json!({"task_id": "1"}).to_string(),
    };
    let result = get_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("Implement feature X"));
    assert!(result.content.contains("pending"));

    // Update task
    let update_tool = super::TaskUpdateTool::new(ts.clone());
    let call = ToolCall {
        id: "3".into(),
        name: "task_update".into(),
        input: serde_json::json!({
            "task_id": "1",
            "status": "in_progress",
            "owner": "worker-1"
        })
        .to_string(),
    };
    let result = update_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("in_progress"));
    assert!(result.content.contains("worker-1"));

    // List tasks
    let list_tool = super::TaskListTool::new(ts.clone());
    let call = ToolCall {
        id: "4".into(),
        name: "task_list".into(),
        input: "{}".into(),
    };
    let result = list_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("Implement feature X"));

    // Create second task, blocked by first
    let call = ToolCall {
        id: "5".into(),
        name: "task_create".into(),
        input: serde_json::json!({
            "subject": "Deploy feature X",
            "description": "Deploy after implementation"
        })
        .to_string(),
    };
    create_tool.run(&call, &ctx).await.unwrap();

    let call = ToolCall {
        id: "6".into(),
        name: "task_update".into(),
        input: serde_json::json!({
            "task_id": "2",
            "add_blocked_by": ["1"]
        })
        .to_string(),
    };
    update_tool.run(&call, &ctx).await.unwrap();

    // Complete first task — should auto-unblock second
    let call = ToolCall {
        id: "7".into(),
        name: "task_update".into(),
        input: serde_json::json!({
            "task_id": "1",
            "status": "completed"
        })
        .to_string(),
    };
    update_tool.run(&call, &ctx).await.unwrap();

    // Verify task 2 is unblocked
    let call = ToolCall {
        id: "8".into(),
        name: "task_get".into(),
        input: serde_json::json!({"task_id": "2"}).to_string(),
    };
    let result = get_tool.run(&call, &ctx).await.unwrap();
    let task2: serde_json::Value = serde_json::from_str(&result.content).unwrap();
    assert!(task2["blocked_by"].as_array().unwrap().is_empty());

    // Delete task
    let call = ToolCall {
        id: "9".into(),
        name: "task_update".into(),
        input: serde_json::json!({
            "task_id": "2",
            "status": "deleted"
        })
        .to_string(),
    };
    let result = update_tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("deleted"));
}

#[tokio::test]
async fn test_send_message_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let ts = test_team_state(tmp.path());
    let ctx = test_context_with_team(tmp.path(), ts.clone());

    // Create team config so broadcast can read members
    let config = crate::core::team::TeamConfig {
        name: "test-team".into(),
        description: "".into(),
        created_at: chrono::Utc::now(),
        lead_agent_id: "test-lead@test-team".into(),
        members: vec![
            crate::core::team::TeamMember {
                agent_id: "test-lead@test-team".into(),
                name: "test-lead".into(),
                agent_type: "team-lead".into(),
                model: None,
                cwd: "/tmp".into(),
                joined_at: chrono::Utc::now(),
            },
            crate::core::team::TeamMember {
                agent_id: "worker@test-team".into(),
                name: "worker".into(),
                agent_type: "general-purpose".into(),
                model: None,
                cwd: "/tmp".into(),
                joined_at: chrono::Utc::now(),
            },
        ],
    };
    crate::core::team::write_team_config(tmp.path(), "test-team", &config).unwrap();

    // Send direct message
    let tool = super::SendMessageTool::new(ts.clone());
    let call = ToolCall {
        id: "1".into(),
        name: "send_message".into(),
        input: serde_json::json!({
            "type": "message",
            "recipient": "worker",
            "content": "Hello from lead!",
            "summary": "Greeting"
        })
        .to_string(),
    };
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("Message sent"));

    // Verify inbox file
    let inbox = crate::core::team::read_inbox(tmp.path(), "test-team", "worker").unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].from, "test-lead");
    assert_eq!(inbox[0].text, "Hello from lead!");

    // Test broadcast
    let call = ToolCall {
        id: "2".into(),
        name: "send_message".into(),
        input: serde_json::json!({
            "type": "broadcast",
            "content": "Team update!",
            "summary": "Update"
        })
        .to_string(),
    };
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("1 members"));

    // Verify worker got the broadcast (now has 2 messages)
    let inbox = crate::core::team::read_inbox(tmp.path(), "test-team", "worker").unwrap();
    assert_eq!(inbox.len(), 2);
    assert_eq!(inbox[1].text, "Team update!");
}

#[tokio::test]
async fn test_check_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let ts = test_team_state(tmp.path());
    let ctx = test_context_with_team(tmp.path(), ts.clone());

    // No messages yet — wait_seconds=0 should return immediately
    let tool = super::CheckInboxTool::new(ts.clone());
    let call = ToolCall {
        id: "1".into(),
        name: "check_inbox".into(),
        input: serde_json::json!({"wait_seconds": 0}).to_string(),
    };
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("No new messages"));

    // Manually add a message to lead's inbox
    let msg = crate::core::team::InboxMessage {
        from: "worker".into(),
        text: "Task done!".into(),
        timestamp: chrono::Utc::now(),
        read: false,
    };
    crate::core::team::append_inbox(tmp.path(), "test-team", "test-lead", msg).unwrap();

    // Now check_inbox should find the message
    let call = ToolCall {
        id: "2".into(),
        name: "check_inbox".into(),
        input: serde_json::json!({"wait_seconds": 0}).to_string(),
    };
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
    assert!(result.content.contains("Task done!"));
    assert!(result.content.contains("worker"));

    // After reading, messages should be marked read — check again
    let call = ToolCall {
        id: "3".into(),
        name: "check_inbox".into(),
        input: serde_json::json!({"wait_seconds": 0}).to_string(),
    };
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(result.content.contains("No new messages"));
}
