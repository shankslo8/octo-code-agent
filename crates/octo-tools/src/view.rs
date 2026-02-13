use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::tool::*;
use std::collections::HashMap;

pub struct ViewTool;

#[async_trait]
impl Tool for ViewTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Absolute or relative file path to read".into(),
                enum_values: None,
            },
        );
        params.insert(
            "offset".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Line number to start reading from (1-based)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "limit".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Maximum number of lines to read (default: 2000)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "view".into(),
            description: "Read a file's contents with optional line offset and limit. \
                Returns content with line numbers."
                .into(),
            parameters: params,
            required: vec!["path".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let path_str = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'path'".into()))?;

        let path = if std::path::Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            ctx.working_dir.join(path_str)
        };

        if !path.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", path.display())));
        }

        // Handle directories: list contents instead of reading
        if path.is_dir() {
            let mut entries = Vec::new();
            let mut dir = tokio::fs::read_dir(&path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            while let Some(entry) = dir
                .next_entry()
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            {
                let is_dir = entry
                    .file_type()
                    .await
                    .map(|ft| ft.is_dir())
                    .unwrap_or(false);
                let name = entry.file_name().to_string_lossy().to_string();
                if is_dir {
                    entries.push(format!("  {name}/"));
                } else {
                    entries.push(format!("  {name}"));
                }
            }
            entries.sort();
            return Ok(ToolResult::success(format!(
                "Directory: {}\n{}",
                path.display(),
                entries.join("\n")
            )));
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read {}: {e}", path.display())))?;

        let offset = params["offset"].as_u64().unwrap_or(1).max(1) as usize;
        let limit = params["limit"].as_u64().unwrap_or(2000) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let start = (offset - 1).min(total_lines);
        let end = (start + limit).min(total_lines);

        let mut result = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            let line_num = start + i + 1;
            result.push_str(&format!("{:>6}\t{}\n", line_num, line));
        }

        if end < total_lines {
            result.push_str(&format!(
                "\n... ({} more lines, {} total)",
                total_lines - end,
                total_lines
            ));
        }

        Ok(ToolResult::success(result))
    }
}
