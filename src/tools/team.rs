use async_trait::async_trait;
use crate::core::error::ToolError;
use crate::core::team::{self, TeamConfig, TeamMember, TeamState};
use crate::core::tool::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

// ---------------------------------------------------------------------------
// Helpers shared by all team tools
// ---------------------------------------------------------------------------

fn get_base_dir(team_state: &Arc<RwLock<Option<TeamState>>>) -> PathBuf {
    let state = team_state.read().unwrap();
    state
        .as_ref()
        .map(|s| s.base_dir.clone())
        .unwrap_or_else(team::default_base_dir)
}

fn get_team(team_state: &Arc<RwLock<Option<TeamState>>>) -> Result<(PathBuf, String), ToolError> {
    let state = team_state.read().unwrap();
    let st = state
        .as_ref()
        .ok_or_else(|| ToolError::ExecutionFailed("No active team".into()))?;
    Ok((st.base_dir.clone(), st.team_name.clone()))
}

// ===========================================================================
// TeamCreateTool
// ===========================================================================

pub struct TeamCreateTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TeamCreateTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TeamCreateTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "team_name".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Name for the new team".into(),
                enum_values: None,
            },
        );
        params.insert(
            "description".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Team description/purpose".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "team_create".into(),
            description: "Create a new team for coordinating multiple agents working in parallel."
                .into(),
            parameters: params,
            required: vec!["team_name".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let team_name = params["team_name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'team_name'".into()))?;
        let description = params["description"].as_str().unwrap_or("").to_string();

        let base = get_base_dir(&self.team_state);

        // Create directories
        let team_dir = team::teams_dir(&base).join(team_name);
        let td = team::tasks_dir(&base, team_name);
        let inbox_dir = team::inboxes_dir(&base, team_name);
        std::fs::create_dir_all(&team_dir)
            .map_err(|e| ToolError::ExecutionFailed(format!("create team dir: {e}")))?;
        std::fs::create_dir_all(&td)
            .map_err(|e| ToolError::ExecutionFailed(format!("create tasks dir: {e}")))?;
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|e| ToolError::ExecutionFailed(format!("create inboxes dir: {e}")))?;

        // Agent name — use existing state or default to "team-lead"
        let agent_name = {
            let state = self.team_state.read().unwrap();
            state
                .as_ref()
                .map(|s| s.agent_name.clone())
                .unwrap_or_else(|| "team-lead".to_string())
        };

        let agent_id = format!("{}@{}", agent_name, team_name);

        let config = TeamConfig {
            name: team_name.to_string(),
            description: description.clone(),
            created_at: chrono::Utc::now(),
            lead_agent_id: agent_id.clone(),
            members: vec![TeamMember {
                agent_id: agent_id.clone(),
                name: agent_name.clone(),
                agent_type: "team-lead".to_string(),
                model: None,
                cwd: ctx.working_dir.to_string_lossy().to_string(),
                joined_at: chrono::Utc::now(),
            }],
        };

        team::write_team_config(&base, team_name, &config)
            .map_err(|e| ToolError::ExecutionFailed(format!("write config: {e}")))?;

        // Update shared team state
        {
            let mut state = self.team_state.write().unwrap();
            *state = Some(
                TeamState::new(team_name.to_string(), agent_name, true).with_base_dir(base.clone()),
            );
        }

        let result = serde_json::json!({
            "team_name": team_name,
            "description": description,
            "agent_id": agent_id,
            "team_dir": team_dir.to_string_lossy(),
            "tasks_dir": td.to_string_lossy(),
        });

        Ok(ToolResult::success(
            serde_json::to_string_pretty(&result).unwrap(),
        ))
    }
}

// ===========================================================================
// TeamDeleteTool
// ===========================================================================

pub struct TeamDeleteTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TeamDeleteTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TeamDeleteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "team_delete".into(),
            description: "Delete the current team and clean up all team resources.".into(),
            parameters: HashMap::new(),
            required: vec![],
        }
    }

    async fn run(&self, _call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let (base, team_name) = get_team(&self.team_state)?;

        // Check active members
        if let Ok(config) = team::read_team_config(&base, &team_name) {
            if config.members.len() > 1 {
                return Ok(ToolResult::error(format!(
                    "Team still has {} active members. Send shutdown requests first.",
                    config.members.len() - 1
                )));
            }
        }

        // Remove directories
        let team_dir = team::teams_dir(&base).join(&team_name);
        if team_dir.exists() {
            std::fs::remove_dir_all(&team_dir)
                .map_err(|e| ToolError::ExecutionFailed(format!("remove team dir: {e}")))?;
        }
        let td = team::tasks_dir(&base, &team_name);
        if td.exists() {
            std::fs::remove_dir_all(&td)
                .map_err(|e| ToolError::ExecutionFailed(format!("remove tasks dir: {e}")))?;
        }

        // Clear state
        {
            let mut state = self.team_state.write().unwrap();
            *state = None;
        }

        Ok(ToolResult::success(format!(
            "Team '{}' deleted successfully.",
            team_name
        )))
    }
}

// ===========================================================================
// SpawnAgentTool
// ===========================================================================

pub struct SpawnAgentTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl SpawnAgentTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for SpawnAgentTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "name".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Name for the new agent".into(),
                enum_values: None,
            },
        );
        params.insert(
            "prompt".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "The task/prompt for the agent".into(),
                enum_values: None,
            },
        );
        params.insert(
            "agent_type".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Type of agent (e.g. 'general-purpose', 'researcher')".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "spawn_agent".into(),
            description:
                "Spawn a new agent teammate that runs as a background subprocess.".into(),
            parameters: params,
            required: vec!["name".into(), "prompt".into()],
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let name = params["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'name'".into()))?;
        let raw_prompt = params["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'prompt'".into()))?;
        let agent_type = params["agent_type"].as_str().unwrap_or("general-purpose");

        let (base, team_name) = get_team(&self.team_state)?;
        let agent_id = format!("{}@{}", name, team_name);

        // Wrap the prompt with team context so the spawned agent reports back
        let lead_name = {
            let st = self.team_state.read().unwrap();
            st.as_ref()
                .map(|s| s.agent_name.clone())
                .unwrap_or_else(|| "team-lead".to_string())
        };
        let prompt = format!(
            "You are team member '{}' in team '{}'. Complete the task below.\n\
             When DONE, you MUST:\n\
             1. Use `send_message` with type=\"message\", recipient=\"{}\" to report a summary of what you did and which files you changed.\n\
             2. If you had any errors or blockers, include them in the message.\n\n\
             ## Task\n{}",
            name, team_name, lead_name, raw_prompt
        );

        // Register member in config
        let mut config = team::read_team_config(&base, &team_name)
            .map_err(|e| ToolError::ExecutionFailed(format!("read team config: {e}")))?;

        if config.members.iter().any(|m| m.name == name) {
            return Ok(ToolResult::error(format!(
                "Agent '{}' already exists in team.",
                name
            )));
        }

        config.members.push(TeamMember {
            agent_id: agent_id.clone(),
            name: name.to_string(),
            agent_type: agent_type.to_string(),
            model: None,
            cwd: ctx.working_dir.to_string_lossy().to_string(),
            joined_at: chrono::Utc::now(),
        });

        team::write_team_config(&base, &team_name, &config)
            .map_err(|e| ToolError::ExecutionFailed(format!("update team config: {e}")))?;

        // Stagger spawns to avoid rate limiting with a single API key.
        // Each agent gets a delay based on its position among members.
        let member_count = config.members.len(); // includes the one we just added
        if member_count > 2 {
            // First spawned agent (member #2) starts immediately, subsequent ones stagger
            let delay_secs = (member_count - 2) as u64 * 8;
            if delay_secs > 0 {
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            }
        }

        // Spawn subprocess
        let exe = std::env::current_exe()
            .map_err(|e| ToolError::ExecutionFailed(format!("get current exe: {e}")))?;

        let _child = tokio::process::Command::new(exe)
            .arg("-p")
            .arg(&prompt)
            .arg("--team-name")
            .arg(&team_name)
            .arg("--agent-name")
            .arg(name)
            .current_dir(&ctx.working_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("spawn agent: {e}")))?;

        let result = serde_json::json!({
            "agent_id": agent_id,
            "name": name,
            "agent_type": agent_type,
            "status": "spawned",
        });

        Ok(ToolResult::success(
            serde_json::to_string_pretty(&result).unwrap(),
        ))
    }
}
