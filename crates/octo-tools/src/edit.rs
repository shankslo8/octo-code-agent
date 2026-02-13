use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::permission::{PermissionDecision, PermissionRequest, PermissionService};
use octo_core::tool::*;
use std::collections::HashMap;
use std::sync::Arc;

pub struct EditTool {
    permission_service: Arc<dyn PermissionService>,
}

impl EditTool {
    pub fn new(permission_service: Arc<dyn PermissionService>) -> Self {
        Self { permission_service }
    }
}

#[async_trait]
impl Tool for EditTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "File path to edit".into(),
                enum_values: None,
            },
        );
        params.insert(
            "old_string".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Exact string to search for and replace".into(),
                enum_values: None,
            },
        );
        params.insert(
            "new_string".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Replacement string".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "edit".into(),
            description: "Edit a file by replacing an exact string match with new content. \
                The old_string must uniquely match one location in the file."
                .into(),
            parameters: params,
            required: vec!["path".into(), "old_string".into(), "new_string".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'path'".into()))?;
        let old_string = params["old_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'old_string'".into()))?;
        let new_string = params["new_string"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'new_string'".into()))?;

        let path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            ctx.working_dir.join(path_str)
        };

        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", path.display())));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let matches: Vec<_> = content.match_indices(old_string).collect();

        if matches.is_empty() {
            return Ok(ToolResult::error(
                "old_string not found in file. Make sure it matches exactly.".into(),
            ));
        }

        if matches.len() > 1 {
            return Ok(ToolResult::error(format!(
                "old_string found {} times. It must be unique. Add more context.",
                matches.len()
            )));
        }

        let req = PermissionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: ctx.session_id.clone(),
            tool_name: "edit".into(),
            action: "edit".into(),
            description: format!("Edit: {}", path.display()),
            path: Some(path.to_string_lossy().to_string()),
        };
        match self.permission_service.request(req).await {
            PermissionDecision::Allow | PermissionDecision::AllowPersistent => {}
            PermissionDecision::Deny => {
                return Err(ToolError::PermissionDenied {
                    tool: "edit".into(),
                    action: path_str.into(),
                });
            }
        }

        let new_content = content.replacen(old_string, new_string, 1);
        tokio::fs::write(&path, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success(format!(
            "Edited {}. Replaced {} chars with {} chars.",
            path.display(),
            old_string.len(),
            new_string.len()
        )))
    }
}
