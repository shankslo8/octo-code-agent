use async_trait::async_trait;
use crate::core::error::ToolError;
use crate::core::tool::*;
use std::collections::HashMap;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "pattern".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Regex pattern to search for in file contents".into(),
                enum_values: None,
            },
        );
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Directory or file to search in (default: working directory)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "include".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Glob pattern to filter files (e.g. '*.rs', '*.py')".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "grep".into(),
            description: "Search file contents using regex patterns. Returns matching lines \
                with file paths and line numbers."
                .into(),
            parameters: params,
            required: vec!["pattern".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'pattern'".into()))?;

        let search_path = params["path"]
            .as_str()
            .map(|p| {
                if std::path::Path::new(p).is_absolute() {
                    std::path::PathBuf::from(p)
                } else {
                    ctx.working_dir.join(p)
                }
            })
            .unwrap_or_else(|| ctx.working_dir.clone());

        let include = params["include"].as_str().unwrap_or("");

        // Use `grep -rn` via process for simplicity and performance
        let mut cmd = tokio::process::Command::new("grep");
        cmd.arg("-rn")
            .arg("--color=never")
            .arg("-E")
            .arg(pattern);

        if !include.is_empty() {
            cmd.arg("--include").arg(include);
        }

        cmd.arg(search_path.to_string_lossy().as_ref());

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            cmd.output(),
        )
        .await
        .map_err(|_| ToolError::Timeout(30))?
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.is_empty() {
            Ok(ToolResult::success("No matches found.".into()))
        } else {
            let lines: Vec<&str> = stdout.lines().take(200).collect();
            let total = stdout.lines().count();
            let mut result = lines.join("\n");
            if total > 200 {
                result.push_str(&format!("\n... ({total} total matches, showing first 200)"));
            }
            Ok(ToolResult::success(result))
        }
    }
}
