use async_trait::async_trait;
use std::pin::Pin;

use crate::core::error::ProviderError;
use crate::core::message::{ContentPart, FinishReason, Message, TokenUsage};
use crate::core::model::Model;
use crate::core::tool::ToolDefinition;

#[derive(Debug, Clone)]
pub enum ProviderEvent {
    ContentStart,
    ContentDelta { text: String },
    ContentStop,
    ThinkingDelta { text: String },
    ToolUseStart { id: String, name: String },
    ToolUseDelta { input_json_chunk: String },
    ToolUseStop,
    Complete { finish_reason: FinishReason, usage: TokenUsage },
    Error { error: ProviderError },
}

pub struct ProviderResponse {
    pub content: Vec<ContentPart>,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}

pub type ProviderEventStream =
    Pin<Box<dyn futures_core::Stream<Item = ProviderEvent> + Send>>;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn send_messages(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse, ProviderError>;

    async fn stream_response(
        &self,
        messages: &[Message],
        system_prompt: &str,
        tools: &[ToolDefinition],
    ) -> Result<ProviderEventStream, ProviderError>;

    fn model(&self) -> &Model;
}
