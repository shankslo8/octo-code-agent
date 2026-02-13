use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::tool::*;
use std::collections::HashMap;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "pattern".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Glob pattern to match files (e.g. '**/*.rs', 'src/**/*.ts')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "path".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Base directory to search in (default: working directory)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "glob".into(),
            description: "Find files matching a glob pattern. Returns matching file paths.".into(),
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

        let base_path = params["path"]
            .as_str()
            .map(|p| {
                if std::path::Path::new(p).is_absolute() {
                    std::path::PathBuf::from(p)
                } else {
                    ctx.working_dir.join(p)
                }
            })
            .unwrap_or_else(|| ctx.working_dir.clone());

        let full_pattern = format!("{}/{}", base_path.display(), pattern);

        let paths: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| ToolError::InvalidParams(format!("Invalid pattern: {e}")))?
            .filter_map(|entry| entry.ok())
            .map(|p| p.to_string_lossy().to_string())
            .take(1000)
            .collect();

        if paths.is_empty() {
            Ok(ToolResult::success("No files found matching pattern.".into()))
        } else {
            let count = paths.len();
            let mut result = paths.join("\n");
            if count >= 1000 {
                result.push_str("\n... (results limited to 1000)");
            }
            result.push_str(&format!("\n\n{count} files found."));
            Ok(ToolResult::success(result))
        }
    }
}
