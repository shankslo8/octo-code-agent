use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::core::error::{OctoError, ToolError};
use crate::core::message::*;
use crate::core::model::ModelId;
use crate::core::permission::PermissionService;
use crate::core::provider::*;
use crate::core::team::TeamState;
use crate::core::tool::*;

use crate::agent::event::AgentEvent;

pub struct Agent {
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    permission_service: Arc<dyn PermissionService>,
    system_prompt: String,
    working_dir: std::path::PathBuf,
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl Agent {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        permission_service: Arc<dyn PermissionService>,
        system_prompt: String,
        working_dir: std::path::PathBuf,
        team_state: Arc<RwLock<Option<TeamState>>>,
    ) -> Self {
        Self {
            provider,
            tools,
            permission_service,
            system_prompt,
            working_dir,
            team_state,
        }
    }

    pub fn model_name(&self) -> &str {
        &self.provider.model().display_name
    }

    pub fn model_id(&self) -> &ModelId {
        &self.provider.model().id
    }

    pub fn switch_provider(&mut self, provider: Arc<dyn Provider>) {
        self.provider = provider;
    }

    pub fn run(
        &self,
        session_id: String,
        messages: Vec<Message>,
        user_input: String,
    ) -> (mpsc::Receiver<AgentEvent>, CancellationToken) {
        let (tx, rx) = mpsc::channel(256);
        let cancel = CancellationToken::new();

        let provider = Arc::clone(&self.provider);
        let tools = self.tools.clone();
        let perm_service = Arc::clone(&self.permission_service);
        let system_prompt = self.system_prompt.clone();
        let working_dir = self.working_dir.clone();
        let cancel_clone = cancel.clone();
        let team_state = self.team_state.clone();

        tokio::spawn(async move {
            let result = agent_loop(
                provider,
                tools,
                perm_service,
                system_prompt,
                working_dir,
                session_id,
                messages,
                user_input,
                tx.clone(),
                cancel_clone,
                team_state,
            )
            .await;

            if let Err(e) = result {
                let _ = tx
                    .send(AgentEvent::Error {
                        error: e.to_string(),
                    })
                    .await;
            }
        });

        (rx, cancel)
    }
}

/// Truncate tool result to avoid sending huge content to the API
fn truncate_tool_result(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }
    let boundary = content.floor_char_boundary(max_chars);
    format!(
        "{}\n\n... [truncated: {} total chars, showing first {}]",
        &content[..boundary],
        content.len(),
        max_chars,
    )
}

/// Estimate token count from text (rough: ~4 chars per token)
fn estimate_tokens(text: &str) -> u64 {
    (text.len() as u64) / 4
}

/// Estimate tokens for a single message
fn estimate_message_tokens(msg: &Message) -> u64 {
    let mut total = 0u64;
    for part in &msg.parts {
        match part {
            ContentPart::Text { text } => total += estimate_tokens(text),
            ContentPart::Reasoning { text } => total += estimate_tokens(text),
            ContentPart::ToolCall { input, .. } => total += estimate_tokens(input) + 20,
            ContentPart::ToolResult { content, .. } => total += estimate_tokens(content) + 10,
            ContentPart::Image { .. } => total += 1000,
            ContentPart::ImageUrl { .. } => total += 1000,
            ContentPart::Finish { .. } => {}
        }
    }
    total.max(1)
}

/// Trim messages to fit within context window, keeping system prompt + last user + recent history
fn trim_messages_to_fit(messages: &mut Vec<Message>, context_window: u64, system_prompt: &str) {
    let system_tokens = estimate_tokens(system_prompt) + 200; // overhead
    let max_message_tokens = context_window.saturating_sub(system_tokens) * 3 / 4; // 75% for messages, 25% for output

    let total: u64 = messages.iter().map(|m| estimate_message_tokens(m)).sum();
    if total <= max_message_tokens {
        return;
    }

    // Always keep first user message and last 4 messages
    let keep_tail = 4.min(messages.len());
    let keep_head = 1.min(messages.len());

    if messages.len() <= keep_head + keep_tail {
        return;
    }

    // Remove from the middle (oldest conversation turns)
    let mut current_tokens: u64 = messages[..keep_head]
        .iter()
        .chain(messages[messages.len() - keep_tail..].iter())
        .map(|m| estimate_message_tokens(m))
        .sum();

    let mut remove_end = keep_head;
    for i in keep_head..messages.len() - keep_tail {
        let msg_tokens = estimate_message_tokens(&messages[i]);
        if current_tokens + msg_tokens > max_message_tokens {
            remove_end = i;
            break;
        }
        current_tokens += msg_tokens;
        remove_end = i + 1;
    }

    if remove_end < messages.len() - keep_tail {
        messages.drain(remove_end..messages.len() - keep_tail);
    }
}

async fn agent_loop(
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    _perm_service: Arc<dyn PermissionService>,
    system_prompt: String,
    working_dir: std::path::PathBuf,
    session_id: String,
    mut messages: Vec<Message>,
    user_input: String,
    tx: mpsc::Sender<AgentEvent>,
    cancel: CancellationToken,
    team_state: Arc<RwLock<Option<TeamState>>>,
) -> Result<(), OctoError> {
    let tool_defs: Vec<ToolDefinition> = tools.iter().map(|t| t.definition()).collect();
    let context_window = provider.model().context_window;

    let user_msg = Message::new_user(session_id.clone(), user_input);
    messages.push(user_msg);

    let _ = tx
        .send(AgentEvent::Started {
            session_id: session_id.clone(),
        })
        .await;

    loop {
        if cancel.is_cancelled() {
            return Err(OctoError::Cancelled);
        }

        // Trim messages to fit context window
        trim_messages_to_fit(&mut messages, context_window, &system_prompt);

        let mut event_stream = 'retry: {
            let mut last_err = None;
            for agent_attempt in 0..3u32 {
                match provider
                    .stream_response(&messages, &system_prompt, &tool_defs)
                    .await
                {
                    Ok(stream) => break 'retry stream,
                    Err(crate::core::error::ProviderError::RateLimited { retry_after_ms }) => {
                        let wait = retry_after_ms.max(5_000) * (agent_attempt as u64 + 1);
                        let _ = tx
                            .send(AgentEvent::Error {
                                error: format!(
                                    "Rate limited. Waiting {:.0}s... (attempt {}/3)",
                                    wait as f64 / 1000.0,
                                    agent_attempt + 1,
                                ),
                            })
                            .await;
                        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                        last_err = Some(crate::core::error::ProviderError::RateLimited {
                            retry_after_ms: wait,
                        });
                    }
                    Err(e) => return Err(OctoError::Provider(e)),
                }
            }
            return Err(OctoError::Provider(last_err.unwrap()));
        };

        let (assistant_msg, finish_reason, usage) =
            process_stream(&mut event_stream, &session_id, &provider, &tx, &cancel).await?;

        messages.push(assistant_msg.clone());

        match finish_reason {
            FinishReason::EndTurn | FinishReason::MaxTokens => {
                let _ = tx
                    .send(AgentEvent::Complete {
                        message: assistant_msg,
                        finish_reason,
                        usage,
                    })
                    .await;
                return Ok(());
            }
            FinishReason::ToolUse => {
                let tool_calls = assistant_msg.tool_calls();
                let mut tool_results = Vec::new();

                for (call_id, call_name, call_input) in tool_calls {
                    if cancel.is_cancelled() {
                        return Err(OctoError::Cancelled);
                    }

                    let tool = tools
                        .iter()
                        .find(|t| t.definition().name == call_name)
                        .ok_or_else(|| OctoError::Tool(ToolError::NotFound(call_name.to_string())))?;

                    let _ = tx
                        .send(AgentEvent::ToolCallStart {
                            id: call_id.to_string(),
                            name: call_name.to_string(),
                        })
                        .await;

                    let tool_ctx = ToolContext {
                        session_id: session_id.clone(),
                        working_dir: working_dir.clone(),
                        cancel_token: cancel.clone(),
                        team_state: team_state.clone(),
                    };

                    let call = ToolCall {
                        id: call_id.to_string(),
                        name: call_name.to_string(),
                        input: call_input.to_string(),
                    };

                    let result = match tool.run(&call, &tool_ctx).await {
                        Ok(r) => r,
                        Err(e) => {
                            let err_msg = e.to_string();
                            let _ = tx
                                .send(AgentEvent::ToolResult {
                                    tool_call_id: call_id.to_string(),
                                    tool_name: call_name.to_string(),
                                    result: err_msg.clone(),
                                    is_error: true,
                                })
                                .await;
                            crate::core::tool::ToolResult::error(err_msg)
                        }
                    };

                    if !result.is_error {
                        let _ = tx
                            .send(AgentEvent::ToolResult {
                                tool_call_id: call_id.to_string(),
                                tool_name: call_name.to_string(),
                                result: result.content.clone(),
                                is_error: false,
                            })
                            .await;
                    }

                    // Truncate large tool results to avoid blowing up token usage
                    let truncated_content = truncate_tool_result(&result.content, 30_000);

                    // Wrap tool result with markers for prompt injection defense
                    let wrapped_content = format!(
                        "<tool_output tool=\"{}\">\n{}\n</tool_output>",
                        call_name, truncated_content
                    );

                    tool_results.push(ContentPart::ToolResult {
                        tool_call_id: call_id.to_string(),
                        content: wrapped_content,
                        is_error: result.is_error,
                    });
                }

                let tool_msg = Message::new_tool_result(session_id.clone(), tool_results);
                messages.push(tool_msg);
            }
            _ => {
                let _ = tx
                    .send(AgentEvent::Complete {
                        message: assistant_msg,
                        finish_reason,
                        usage,
                    })
                    .await;
                return Ok(());
            }
        }
    }
}

async fn process_stream(
    stream: &mut ProviderEventStream,
    session_id: &str,
    provider: &Arc<dyn Provider>,
    tx: &mpsc::Sender<AgentEvent>,
    cancel: &CancellationToken,
) -> Result<(Message, FinishReason, TokenUsage), OctoError> {
    let model_id = provider.model().id.clone();
    let mut msg = Message::new_assistant(session_id.to_string(), model_id);

    let mut current_text = String::new();
    let mut current_thinking = String::new(); // Buffer thinking tokens instead of pushing each chunk
    let mut current_tool_id = String::new();
    let mut current_tool_name = String::new();
    let mut current_tool_input = String::new();
    let mut finish_reason = FinishReason::EndTurn;
    let mut usage = TokenUsage::default();

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                return Err(OctoError::Cancelled);
            }
            event = stream.next() => {
                match event {
                    None => break,
                    Some(ProviderEvent::ContentDelta { text }) => {
                        // Flush thinking buffer when content starts
                        if !current_thinking.is_empty() {
                            msg.parts.push(ContentPart::Reasoning {
                                text: std::mem::take(&mut current_thinking),
                            });
                        }
                        current_text.push_str(&text);
                        let _ = tx.send(AgentEvent::ContentDelta { text }).await;
                    }
                    Some(ProviderEvent::ContentStop) => {
                        if !current_text.is_empty() {
                            msg.parts.push(ContentPart::Text {
                                text: std::mem::take(&mut current_text),
                            });
                        }
                    }
                    Some(ProviderEvent::ThinkingDelta { text }) => {
                        let _ = tx.send(AgentEvent::ThinkingDelta { text: text.clone() }).await;
                        // Buffer thinking instead of pushing each chunk
                        current_thinking.push_str(&text);
                    }
                    Some(ProviderEvent::ToolUseStart { id, name }) => {
                        // Flush any pending text
                        if !current_text.is_empty() {
                            msg.parts.push(ContentPart::Text {
                                text: std::mem::take(&mut current_text),
                            });
                        }
                        // Flush thinking buffer
                        if !current_thinking.is_empty() {
                            msg.parts.push(ContentPart::Reasoning {
                                text: std::mem::take(&mut current_thinking),
                            });
                        }
                        // Flush previous tool call if any (handles multiple tool calls)
                        if !current_tool_name.is_empty() {
                            msg.parts.push(ContentPart::ToolCall {
                                id: std::mem::take(&mut current_tool_id),
                                name: std::mem::take(&mut current_tool_name),
                                input: std::mem::take(&mut current_tool_input),
                            });
                        }
                        current_tool_id = id.clone();
                        current_tool_name = name.clone();
                        current_tool_input.clear();
                        let _ = tx.send(AgentEvent::ToolCallInputDelta {
                            id,
                            input_chunk: String::new(),
                        }).await;
                    }
                    Some(ProviderEvent::ToolUseDelta { input_json_chunk }) => {
                        current_tool_input.push_str(&input_json_chunk);
                    }
                    Some(ProviderEvent::ToolUseStop) => {
                        // Only push if we have a valid tool call (guard against empty names)
                        if !current_tool_name.is_empty() {
                            msg.parts.push(ContentPart::ToolCall {
                                id: std::mem::take(&mut current_tool_id),
                                name: std::mem::take(&mut current_tool_name),
                                input: std::mem::take(&mut current_tool_input),
                            });
                        }
                    }
                    Some(ProviderEvent::Complete { finish_reason: fr, usage: u }) => {
                        finish_reason = fr;
                        usage = u;
                    }
                    Some(ProviderEvent::Error { error }) => {
                        return Err(OctoError::Provider(error));
                    }
                    Some(ProviderEvent::ContentStart) => {}
                }
            }
        }
    }

    // Flush remaining buffers
    if !current_thinking.is_empty() {
        msg.parts.push(ContentPart::Reasoning {
            text: current_thinking,
        });
    }
    if !current_text.is_empty() {
        msg.parts.push(ContentPart::Text { text: current_text });
    }

    msg.add_finish(finish_reason);
    msg.token_usage = Some(usage.clone());

    Ok((msg, finish_reason, usage))
}
