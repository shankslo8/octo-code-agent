use async_trait::async_trait;
use crate::core::error::ToolError;
use crate::core::tool::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const MAX_OUTPUT: usize = 30_000;

const OPERATIONS: &[&str] = &[
    "health",
    "structure",
    "symbols",
    "search",
    "implementation",
    "callers",
    "tests",
    "variables",
    "peek",
    "grep",
];

#[derive(Debug, Clone)]
struct SessionState {
    session_id: String,
}

pub struct CoderlmTool {
    client: reqwest::Client,
    server_url: String,
    session: Arc<RwLock<Option<SessionState>>>,
}

#[derive(Debug, Deserialize)]
struct SessionResponse {
    session_id: String,
}

impl CoderlmTool {
    pub fn new(server_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            server_url,
            session: Arc::new(RwLock::new(None)),
        }
    }

    async fn ensure_session(&self, working_dir: &str) -> Result<String, String> {
        // Check cached session
        {
            let guard = self.session.read().await;
            if let Some(ref state) = *guard {
                return Ok(state.session_id.clone());
            }
        }

        // Create new session
        self.create_session(working_dir).await
    }

    async fn create_session(&self, working_dir: &str) -> Result<String, String> {
        let url = format!("{}/sessions", self.server_url);
        let body = serde_json::json!({ "cwd": working_dir });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("CodeRLM server not reachable at {}: {e}", self.server_url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Failed to create CodeRLM session: HTTP {status} - {text}"
            ));
        }

        let session_resp: SessionResponse = resp
            .json()
            .await
            .map_err(|e| format!("Invalid session response: {e}"))?;

        let session_id = session_resp.session_id.clone();
        let mut guard = self.session.write().await;
        *guard = Some(SessionState {
            session_id: session_id.clone(),
        });

        Ok(session_id)
    }

    async fn invalidate_session(&self) {
        let mut guard = self.session.write().await;
        *guard = None;
    }

    async fn api_get(
        &self,
        path: &str,
        query: &[(String, String)],
        working_dir: &str,
    ) -> Result<String, String> {
        let session_id = self.ensure_session(working_dir).await?;
        let url = format!("{}{}", self.server_url, path);

        let resp = self
            .client
            .get(&url)
            .header("X-Session-Id", &session_id)
            .query(query)
            .send()
            .await
            .map_err(|e| format!("CodeRLM request failed: {e}"))?;

        let status = resp.status();

        // Session expired or unauthorized: retry once with a new session
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::GONE {
            self.invalidate_session().await;
            let new_session_id = self.create_session(working_dir).await?;

            let resp = self
                .client
                .get(&url)
                .header("X-Session-Id", &new_session_id)
                .query(query)
                .send()
                .await
                .map_err(|e| format!("CodeRLM request failed after session refresh: {e}"))?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(format!("CodeRLM HTTP {status}: {text}"));
            }
            return resp
                .text()
                .await
                .map_err(|e| format!("Failed to read response: {e}"));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("CodeRLM HTTP {status}: {text}"));
        }

        resp.text()
            .await
            .map_err(|e| format!("Failed to read response: {e}"))
    }

    fn truncate_output(text: String) -> String {
        if text.len() > MAX_OUTPUT {
            let mut truncated = text;
            truncated.truncate(MAX_OUTPUT);
            truncated.push_str("\n... (output truncated)");
            truncated
        } else {
            text
        }
    }
}

#[async_trait]
impl Tool for CoderlmTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();

        params.insert(
            "operation".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "The operation to perform. One of: health, structure, symbols, search, implementation, callers, tests, variables, peek, grep".into(),
                enum_values: Some(OPERATIONS.iter().map(|s| s.to_string()).collect()),
            },
        );
        params.insert(
            "query".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Search query string (required for 'search')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "symbol".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Symbol name (required for 'implementation', 'callers', 'tests')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "function".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Function name (required for 'variables')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "file".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "File path filter (optional for symbols, implementation, callers, tests, variables; required for 'peek')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "pattern".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Grep pattern (required for 'grep')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "kind".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Symbol kind filter for 'symbols' (e.g., function, class, struct)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "start".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Start line number (required for 'peek')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "end".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "End line number (required for 'peek')".into(),
                enum_values: None,
            },
        );
        params.insert(
            "limit".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Maximum number of results (optional, for symbols, search, callers, tests, grep)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "depth".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Directory depth for 'structure' (optional)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "max_matches".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Max matches for 'grep' (optional)".into(),
                enum_values: None,
            },
        );
        params.insert(
            "context_lines".into(),
            ParamSchema {
                param_type: "integer".into(),
                description: "Context lines for 'grep' (optional)".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "coderlm".into(),
            description: "Code intelligence tool powered by CodeRLM (tree-sitter based). \
                Provides precise semantic code navigation: symbol search, implementation lookup, \
                caller tracking, test discovery, and project structure. \
                Use this instead of grep/glob when you need accurate code understanding. \
                Falls back gracefully if the CodeRLM server is not running."
                .into(),
            parameters: params,
            required: vec!["operation".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let operation = params["operation"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'operation'".into()))?;

        if !OPERATIONS.contains(&operation) {
            return Ok(ToolResult::error(format!(
                "Unknown operation '{operation}'. Valid operations: {}",
                OPERATIONS.join(", ")
            )));
        }

        let working_dir = ctx.working_dir.to_string_lossy().to_string();

        let result = match operation {
            "health" => {
                self.api_get("/health", &[], &working_dir).await
            }
            "structure" => {
                let mut query = vec![];
                if let Some(d) = params["depth"].as_u64() {
                    query.push(("depth".to_string(), d.to_string()));
                }
                self.api_get("/structure", &query, &working_dir).await
            }
            "symbols" => {
                let mut query = vec![];
                if let Some(kind) = params["kind"].as_str() {
                    query.push(("kind".to_string(), kind.to_string()));
                }
                if let Some(file) = params["file"].as_str() {
                    query.push(("file".to_string(), file.to_string()));
                }
                if let Some(limit) = params["limit"].as_u64() {
                    query.push(("limit".to_string(), limit.to_string()));
                }
                self.api_get("/symbols", &query, &working_dir).await
            }
            "search" => {
                let q = params["query"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'search' requires 'query'".into()))?;
                let mut query = vec![("query".to_string(), q.to_string())];
                if let Some(limit) = params["limit"].as_u64() {
                    query.push(("limit".to_string(), limit.to_string()));
                }
                self.api_get("/symbols/search", &query, &working_dir).await
            }
            "implementation" => {
                let symbol = params["symbol"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'implementation' requires 'symbol'".into()))?;
                let mut query = vec![("symbol".to_string(), symbol.to_string())];
                if let Some(file) = params["file"].as_str() {
                    query.push(("file".to_string(), file.to_string()));
                }
                self.api_get("/symbols/implementation", &query, &working_dir).await
            }
            "callers" => {
                let symbol = params["symbol"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'callers' requires 'symbol'".into()))?;
                let mut query = vec![("symbol".to_string(), symbol.to_string())];
                if let Some(file) = params["file"].as_str() {
                    query.push(("file".to_string(), file.to_string()));
                }
                if let Some(limit) = params["limit"].as_u64() {
                    query.push(("limit".to_string(), limit.to_string()));
                }
                self.api_get("/symbols/callers", &query, &working_dir).await
            }
            "tests" => {
                let symbol = params["symbol"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'tests' requires 'symbol'".into()))?;
                let mut query = vec![("symbol".to_string(), symbol.to_string())];
                if let Some(file) = params["file"].as_str() {
                    query.push(("file".to_string(), file.to_string()));
                }
                if let Some(limit) = params["limit"].as_u64() {
                    query.push(("limit".to_string(), limit.to_string()));
                }
                self.api_get("/symbols/tests", &query, &working_dir).await
            }
            "variables" => {
                let function = params["function"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'variables' requires 'function'".into()))?;
                let mut query = vec![("function".to_string(), function.to_string())];
                if let Some(file) = params["file"].as_str() {
                    query.push(("file".to_string(), file.to_string()));
                }
                self.api_get("/symbols/variables", &query, &working_dir).await
            }
            "peek" => {
                let file = params["file"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'peek' requires 'file'".into()))?;
                let start = params["start"]
                    .as_u64()
                    .ok_or_else(|| ToolError::InvalidParams("'peek' requires 'start'".into()))?;
                let end = params["end"]
                    .as_u64()
                    .ok_or_else(|| ToolError::InvalidParams("'peek' requires 'end'".into()))?;
                let query = vec![
                    ("file".to_string(), file.to_string()),
                    ("start".to_string(), start.to_string()),
                    ("end".to_string(), end.to_string()),
                ];
                self.api_get("/peek", &query, &working_dir).await
            }
            "grep" => {
                let pattern = params["pattern"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidParams("'grep' requires 'pattern'".into()))?;
                let mut query = vec![("pattern".to_string(), pattern.to_string())];
                if let Some(max) = params["max_matches"].as_u64() {
                    query.push(("max_matches".to_string(), max.to_string()));
                }
                if let Some(ctx_lines) = params["context_lines"].as_u64() {
                    query.push(("context_lines".to_string(), ctx_lines.to_string()));
                }
                self.api_get("/grep", &query, &working_dir).await
            }
            _ => unreachable!(),
        };

        match result {
            Ok(body) => Ok(ToolResult::success(Self::truncate_output(body))),
            Err(msg) => Ok(ToolResult::error(msg)),
        }
    }
}
