use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::ModelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    Cancelled,
    Error,
    PermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Reasoning {
        text: String,
    },
    Image {
        data: String,
        media_type: String,
    },
    ImageUrl {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    ToolCall {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_call_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
    Finish {
        reason: FinishReason,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_creation_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub parts: Vec<ContentPart>,
    pub model_id: Option<ModelId>,
    pub token_usage: Option<TokenUsage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Message {
    pub fn new_user(session_id: String, text: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role: MessageRole::User,
            parts: vec![ContentPart::Text { text }],
            model_id: None,
            token_usage: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_assistant(session_id: String, model_id: ModelId) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role: MessageRole::Assistant,
            parts: Vec::new(),
            model_id: Some(model_id),
            token_usage: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_tool_result(session_id: String, results: Vec<ContentPart>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role: MessageRole::Tool,
            parts: results,
            model_id: None,
            token_usage: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn tool_calls(&self) -> Vec<(&str, &str, &str)> {
        self.parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::ToolCall { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input.as_str()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn text_content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.parts.iter().find_map(|p| match p {
            ContentPart::Finish { reason, .. } => Some(*reason),
            _ => None,
        })
    }

    pub fn append_text(&mut self, delta: &str) {
        if let Some(ContentPart::Text { text }) = self.parts.last_mut() {
            text.push_str(delta);
        } else {
            self.parts.push(ContentPart::Text {
                text: delta.to_string(),
            });
        }
        self.updated_at = Utc::now();
    }

    pub fn add_tool_call(&mut self, id: String, name: String, input: String) {
        self.parts.push(ContentPart::ToolCall { id, name, input });
        self.updated_at = Utc::now();
    }

    pub fn add_finish(&mut self, reason: FinishReason) {
        self.parts.push(ContentPart::Finish {
            reason,
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }
}
