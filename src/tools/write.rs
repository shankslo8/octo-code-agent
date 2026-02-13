use async_trait::async_trait;
use crate::core::error::ToolError;
use crate::core::permission::{PermissionDecision, PermissionRequest, PermissionService};
use crate::core::tool::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct WriteTool {
    permission_service: Arc<dyn PermissionService>,
}

impl WriteTool {
    pub fn new(permission_service: Arc<dyn PermissionService>) -> Self {
        Self { permission_service }
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "File path to write to".into(),
                enum_values: None,
            },
        );
        params.insert(
            "content".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Content to write to the file".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "write".into(),
            description: "Write content to a file. Creates the file if it doesn't exist, \
                or overwrites if it does. Creates parent directories as needed."
                .into(),
            parameters: params,
            required: vec!["path".into(), "content".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'path'".into()))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'content'".into()))?;

        let path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            ctx.working_dir.join(path_str)
        };

        let req = PermissionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: ctx.session_id.clone(),
            tool_name: "write".into(),
            action: "write".into(),
            description: format!("Write to: {}", path.display()),
            path: Some(path.to_string_lossy().to_string()),
        };
        match self.permission_service.request(req).await {
            PermissionDecision::Allow | PermissionDecision::AllowPersistent => {}
            PermissionDecision::Deny => {
                return Err(ToolError::PermissionDenied {
                    tool: "write".into(),
                    action: path_str.into(),
                });
            }
        }

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        tokio::fs::write(&path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let lines = content.lines().count();
        Ok(ToolResult::success(format!(
            "Wrote {} lines to {}",
            lines,
            path.display()
        )))
    }
}
