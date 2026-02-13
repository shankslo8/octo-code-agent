use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::team::{self, InboxMessage, TeamState};
use octo_core::tool::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SendMessageTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl SendMessageTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }

    fn get_state(&self) -> Result<TeamState, ToolError> {
        let state = self.team_state.read().unwrap();
        state
            .clone()
            .ok_or_else(|| ToolError::ExecutionFailed("No active team".into()))
    }
}

#[async_trait]
impl Tool for SendMessageTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "type".into(),
            ParamSchema {
                param_type: "string".into(),
                description:
                    "Message type: message, broadcast, shutdown_request, shutdown_response".into(),
                enum_values: Some(vec![
                    "message".into(),
                    "broadcast".into(),
                    "shutdown_request".into(),
                    "shutdown_response".into(),
                ]),
            },
        );
        params.insert(
            "recipient".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Agent name of the recipient (for message/shutdown_request)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "content".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Message text".into(),
                enum_values: None,
            },
        );
        params.insert(
            "summary".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Short summary of the message (5-10 words)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "approve".into(),
            ParamSchema {
                param_type: "boolean".into(),
                description: "Whether to approve shutdown (for shutdown_response)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "send_message".into(),
            description:
                "Send messages to teammates. Supports direct messages, broadcasts, and shutdown requests/responses."
                    .into(),
            parameters: params,
            required: vec!["type".into()],
        }
    }

    async fn run(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let st = self.get_state()?;
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let msg_type = params["type"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'type'".into()))?;
        let content = params["content"].as_str().unwrap_or("");

        match msg_type {
            "message" => {
                let recipient = params["recipient"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("missing 'recipient'".into()))?;

                let msg = InboxMessage {
                    from: st.agent_name.clone(),
                    text: content.to_string(),
                    timestamp: chrono::Utc::now(),
                    read: false,
                };

                team::append_inbox(&st.base_dir, &st.team_name, recipient, msg)
                    .map_err(|e| ToolError::ExecutionFailed(format!("send message: {e}")))?;

                Ok(ToolResult::success(format!(
                    "Message sent to '{}'.",
                    recipient
                )))
            }

            "broadcast" => {
                let config = team::read_team_config(&st.base_dir, &st.team_name)
                    .map_err(|e| ToolError::ExecutionFailed(format!("read team config: {e}")))?;

                let mut sent = 0;
                for member in &config.members {
                    if member.name != st.agent_name {
                        let msg = InboxMessage {
                            from: st.agent_name.clone(),
                            text: content.to_string(),
                            timestamp: chrono::Utc::now(),
                            read: false,
                        };
                        team::append_inbox(&st.base_dir, &st.team_name, &member.name, msg)
                            .map_err(|e| {
                                ToolError::ExecutionFailed(format!(
                                    "send to {}: {e}",
                                    member.name
                                ))
                            })?;
                        sent += 1;
                    }
                }

                Ok(ToolResult::success(format!(
                    "Broadcast sent to {} members.",
                    sent
                )))
            }

            "shutdown_request" => {
                let recipient = params["recipient"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("missing 'recipient'".into()))?;

                let shutdown_msg = serde_json::json!({
                    "type": "shutdown_request",
                    "from": st.agent_name,
                    "content": content,
                    "request_id": uuid::Uuid::new_v4().to_string(),
                });

                let msg = InboxMessage {
                    from: st.agent_name.clone(),
                    text: shutdown_msg.to_string(),
                    timestamp: chrono::Utc::now(),
                    read: false,
                };

                team::append_inbox(&st.base_dir, &st.team_name, recipient, msg)
                    .map_err(|e| ToolError::ExecutionFailed(format!("send shutdown: {e}")))?;

                Ok(ToolResult::success(format!(
                    "Shutdown request sent to '{}'.",
                    recipient
                )))
            }

            "shutdown_response" => {
                let approve = params["approve"].as_bool().unwrap_or(false);

                let config = team::read_team_config(&st.base_dir, &st.team_name)
                    .map_err(|e| ToolError::ExecutionFailed(format!("read team config: {e}")))?;

                let lead_name = config
                    .members
                    .iter()
                    .find(|m| m.agent_id == config.lead_agent_id)
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| "team-lead".to_string());

                if approve {
                    // Send confirmation
                    let msg = InboxMessage {
                        from: st.agent_name.clone(),
                        text: format!("Shutdown approved by {}. Exiting.", st.agent_name),
                        timestamp: chrono::Utc::now(),
                        read: false,
                    };
                    let _ = team::append_inbox(&st.base_dir, &st.team_name, &lead_name, msg);

                    // Remove self from config
                    let mut config = config;
                    config.members.retain(|m| m.name != st.agent_name);
                    let _ = team::write_team_config(&st.base_dir, &st.team_name, &config);

                    // Exit process
                    std::process::exit(0);
                } else {
                    let msg = InboxMessage {
                        from: st.agent_name.clone(),
                        text: format!("Shutdown rejected by {}: {}", st.agent_name, content),
                        timestamp: chrono::Utc::now(),
                        read: false,
                    };
                    team::append_inbox(&st.base_dir, &st.team_name, &lead_name, msg)
                        .map_err(|e| ToolError::ExecutionFailed(format!("send rejection: {e}")))?;

                    Ok(ToolResult::success("Shutdown rejected.".into()))
                }
            }

            other => Err(ToolError::InvalidParams(format!(
                "unknown message type: {}",
                other
            ))),
        }
    }
}

// ===========================================================================
// CheckInboxTool
// ===========================================================================

pub struct CheckInboxTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl CheckInboxTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }

    fn get_state(&self) -> Result<TeamState, ToolError> {
        let state = self.team_state.read().unwrap();
        state
            .clone()
            .ok_or_else(|| ToolError::ExecutionFailed("No active team".into()))
    }
}

#[async_trait]
impl Tool for CheckInboxTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "wait_seconds".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Seconds to wait for new messages before returning (0 = no wait, default 10, max 30)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "check_inbox".into(),
            description: "Check your inbox for messages from teammates. If no unread messages and wait_seconds > 0, polls every 2s until a message arrives or timeout.".into(),
            parameters: params,
            required: vec![],
        }
    }

    async fn run(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let st = self.get_state()?;
        let params: serde_json::Value = serde_json::from_str(&call.input).unwrap_or_default();
        let wait = params["wait_seconds"].as_u64().unwrap_or(10).min(30);

        // Poll loop
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(wait);
        loop {
            let messages = team::read_inbox(&st.base_dir, &st.team_name, &st.agent_name)
                .map_err(|e| ToolError::ExecutionFailed(format!("read inbox: {e}")))?;

            let unread: Vec<&InboxMessage> = messages.iter().filter(|m| !m.read).collect();

            if !unread.is_empty() {
                // Format unread
                let formatted: Vec<serde_json::Value> = unread
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "from": m.from,
                            "text": m.text,
                            "timestamp": m.timestamp.to_rfc3339(),
                        })
                    })
                    .collect();

                // Mark all as read
                let mut all = messages;
                for msg in &mut all {
                    msg.read = true;
                }
                let inbox_path = team::inbox_path(&st.base_dir, &st.team_name, &st.agent_name);
                let data = serde_json::to_string_pretty(&all)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                std::fs::write(&inbox_path, data)
                    .map_err(|e| ToolError::ExecutionFailed(format!("write inbox: {e}")))?;

                return Ok(ToolResult::success(
                    serde_json::to_string_pretty(&formatted).unwrap(),
                ));
            }

            if std::time::Instant::now() >= deadline {
                return Ok(ToolResult::success("No new messages.".into()));
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}
