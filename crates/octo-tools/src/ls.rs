use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::tool::*;
use std::collections::HashMap;

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Directory path to list (default: working directory)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "ls".into(),
            description: "List directory contents with file types and sizes.".into(),
            parameters: params,
            required: vec![],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).unwrap_or(serde_json::json!({}));

        let path_str = params["path"].as_str().unwrap_or(".");
        let path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            ctx.working_dir.join(path_str)
        };

        if !path.exists() {
            return Ok(ToolResult::error(format!("Directory not found: {}", path.display())));
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        {
            let meta = entry.metadata().await.ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = meta.as_ref().map_or(false, |m| m.is_dir());
            let size = meta.as_ref().map_or(0, |m| m.len());

            let indicator = if is_dir { "/" } else { "" };
            if is_dir {
                entries.push(format!("  {name}{indicator}"));
            } else {
                entries.push(format!("  {name}{indicator}  ({size} bytes)"));
            }
        }

        entries.sort();
        let mut result = format!("{}:\n", path.display());
        result.push_str(&entries.join("\n"));

        Ok(ToolResult::success(result))
    }
}
