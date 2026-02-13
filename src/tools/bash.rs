use async_trait::async_trait;
use crate::core::error::ToolError;
use crate::core::permission::{PermissionDecision, PermissionRequest, PermissionService};
use crate::core::tool::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

const MAX_OUTPUT: usize = 30_000;
const DEFAULT_TIMEOUT: u64 = 120;
const MAX_TIMEOUT: u64 = 600;

const SAFE_PREFIXES: &[&str] = &[
    "ls", "cat", "head", "tail", "wc", "find", "which", "whoami", "pwd", "date", "echo",
    "git status", "git log", "git diff", "git branch", "git show",
    "cargo check", "cargo test", "cargo clippy", "cargo fmt --check",
    "rustc --version", "cargo --version", "node --version", "python --version",
];

/// Dangerous commands that should always be blocked
const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "dd if=/dev",
    "mkfs.",
    "> /dev/sda",
    "chmod 777 /",
    "chmod -R 777 /",
    ":(){ :|:& };:", // fork bomb
    "wget -O- | sh",
    "curl | sh",
    "curl | bash",
    "wget -O- | bash",
    "shutdown",
    "reboot",
    "init 0",
    "init 6",
    "kill -9 1",
    "killall",
    "pkill -9",
];

pub struct BashTool {
    permission_service: Arc<dyn PermissionService>,
}

impl BashTool {
    pub fn new(permission_service: Arc<dyn PermissionService>) -> Self {
        Self { permission_service }
    }

    fn is_safe_command(command: &str) -> bool {
        let trimmed = command.trim();
        SAFE_PREFIXES
            .iter()
            .any(|safe| trimmed.starts_with(safe))
    }

    fn is_dangerous_command(command: &str) -> bool {
        let lower = command.to_lowercase();
        DANGEROUS_PATTERNS.iter().any(|pat| lower.contains(pat))
    }
}

#[async_trait]
impl Tool for BashTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "command".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "The bash command to execute".into(),
                enum_values: None,
            },
        );
        params.insert(
            "timeout".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Timeout in seconds (max 600, default 120)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "bash".into(),
            description: "Execute a bash command in the project directory. \
                Use for running shell commands, build tools, tests, git operations, etc. \
                Commands that modify files or state require user permission."
                .into(),
            parameters: params,
            required: vec!["command".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value =
            serde_json::from_str(&call.input).map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'command'".into()))?;

        let timeout = params["timeout"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT)
            .min(MAX_TIMEOUT);

        // Block dangerous commands unconditionally
        if Self::is_dangerous_command(command) {
            return Err(ToolError::PermissionDenied {
                tool: "bash".into(),
                action: format!("BLOCKED dangerous command: {command}"),
            });
        }

        if !Self::is_safe_command(command) {
            let req = PermissionRequest {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: ctx.session_id.clone(),
                tool_name: "bash".into(),
                action: "execute".into(),
                description: format!("Run: {command}"),
                path: Some(ctx.working_dir.to_string_lossy().to_string()),
            };
            match self.permission_service.request(req).await {
                PermissionDecision::Allow | PermissionDecision::AllowPersistent => {}
                PermissionDecision::Deny => {
                    return Err(ToolError::PermissionDenied {
                        tool: "bash".into(),
                        action: command.into(),
                    });
                }
            }
        }

        let output = tokio::time::timeout(
            Duration::from_secs(timeout),
            Command::new("bash")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.working_dir)
                .output(),
        )
        .await
        .map_err(|_| ToolError::Timeout(timeout))?
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("STDERR:\n");
            result.push_str(&stderr);
        }

        if result.len() > MAX_OUTPUT {
            result.truncate(MAX_OUTPUT);
            result.push_str("\n... (output truncated)");
        }

        if result.is_empty() {
            result = "(no output)".into();
        }

        if output.status.success() {
            Ok(ToolResult::success(result))
        } else {
            let code = output.status.code().unwrap_or(-1);
            Ok(ToolResult::error(format!(
                "Exit code {code}\n{result}"
            )))
        }
    }
}
