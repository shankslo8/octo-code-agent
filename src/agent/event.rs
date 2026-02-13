use crate::core::message::{FinishReason, Message, TokenUsage};

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Started {
        session_id: String,
    },
    ContentDelta {
        text: String,
    },
    ThinkingDelta {
        text: String,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallInputDelta {
        id: String,
        input_chunk: String,
    },
    ToolResult {
        tool_call_id: String,
        tool_name: String,
        result: String,
        is_error: bool,
    },
    Complete {
        message: Message,
        finish_reason: FinishReason,
        usage: TokenUsage,
    },
    Error {
        error: String,
    },
}
