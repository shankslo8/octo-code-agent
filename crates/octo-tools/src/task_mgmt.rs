use async_trait::async_trait;
use octo_core::error::ToolError;
use octo_core::team::{self, TaskItem, TaskStatus, TeamState};
use octo_core::tool::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

fn get_team(team_state: &Arc<RwLock<Option<TeamState>>>) -> Result<(PathBuf, String), ToolError> {
    let state = team_state.read().unwrap();
    let st = state
        .as_ref()
        .ok_or_else(|| ToolError::ExecutionFailed("No active team".into()))?;
    Ok((st.base_dir.clone(), st.team_name.clone()))
}

// ===========================================================================
// TaskCreateTool
// ===========================================================================

pub struct TaskCreateTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TaskCreateTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TaskCreateTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "subject".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Brief title for the task".into(),
                enum_values: None,
            },
        );
        params.insert(
            "description".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Detailed description of what needs to be done".into(),
                enum_values: None,
            },
        );
        params.insert(
            "active_form".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Present continuous form for spinner (e.g. 'Running tests')".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "task_create".into(),
            description: "Create a new task in the team's shared task list.".into(),
            parameters: params,
            required: vec!["subject".into(), "description".into()],
        }
    }

    async fn run(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let (base, team_name) = get_team(&self.team_state)?;
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let subject = params["subject"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'subject'".into()))?;
        let description = params["description"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'description'".into()))?;
        let active_form = params["active_form"].as_str().map(|s| s.to_string());

        let id = team::next_task_id(&base, &team_name)
            .map_err(|e| ToolError::ExecutionFailed(format!("get task id: {e}")))?;

        let task = TaskItem {
            id: id.clone(),
            subject: subject.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            active_form,
            owner: None,
            blocks: vec![],
            blocked_by: vec![],
            metadata: None,
        };

        team::write_task(&base, &team_name, &task)
            .map_err(|e| ToolError::ExecutionFailed(format!("write task: {e}")))?;

        let result = serde_json::json!({
            "id": id,
            "subject": subject,
            "status": "pending",
        });
        Ok(ToolResult::success(
            serde_json::to_string_pretty(&result).unwrap(),
        ))
    }
}

// ===========================================================================
// TaskGetTool
// ===========================================================================

pub struct TaskGetTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TaskGetTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TaskGetTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "task_id".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "The ID of the task to retrieve".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "task_get".into(),
            description: "Get full details of a task by its ID.".into(),
            parameters: params,
            required: vec!["task_id".into()],
        }
    }

    async fn run(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let (base, team_name) = get_team(&self.team_state)?;
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let task_id = params["task_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'task_id'".into()))?;

        let task = team::read_task(&base, &team_name, task_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("task not found: {e}")))?;

        let result = serde_json::to_string_pretty(&task)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolResult::success(result))
    }
}

// ===========================================================================
// TaskUpdateTool
// ===========================================================================

pub struct TaskUpdateTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TaskUpdateTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TaskUpdateTool {
    fn definition(&self) -> ToolDefinition {
        let mut params = HashMap::new();
        params.insert(
            "task_id".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "The ID of the task to update".into(),
                enum_values: None,
            },
        );
        params.insert(
            "status".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "New status: pending, in_progress, completed, or deleted".into(),
                enum_values: Some(vec![
                    "pending".into(),
                    "in_progress".into(),
                    "completed".into(),
                    "deleted".into(),
                ]),
            },
        );
        params.insert(
            "subject".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "New subject for the task".into(),
                enum_values: None,
            },
        );
        params.insert(
            "description".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "New description for the task".into(),
                enum_values: None,
            },
        );
        params.insert(
            "owner".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Agent name to assign as owner".into(),
                enum_values: None,
            },
        );
        params.insert(
            "active_form".into(),
            ParamSchema {
                param_type: "string".into(),
                description: "Present continuous form for spinner".into(),
                enum_values: None,
            },
        );
        params.insert(
            "add_blocks".into(),
            ParamSchema {
                param_type: "array".into(),
                description: "Task IDs that this task blocks".into(),
                enum_values: None,
            },
        );
        params.insert(
            "add_blocked_by".into(),
            ParamSchema {
                param_type: "array".into(),
                description: "Task IDs that block this task".into(),
                enum_values: None,
            },
        );

        ToolDefinition {
            name: "task_update".into(),
            description: "Update a task's status, owner, or other fields.".into(),
            parameters: params,
            required: vec!["task_id".into()],
        }
    }

    async fn run(&self, call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let (base, team_name) = get_team(&self.team_state)?;
        let params: serde_json::Value = serde_json::from_str(&call.input)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let task_id = params["task_id"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidParams("missing 'task_id'".into()))?;

        // Handle "deleted" status — just remove the file
        if let Some("deleted") = params["status"].as_str() {
            team::delete_task(&base, &team_name, task_id)
                .map_err(|e| ToolError::ExecutionFailed(format!("delete task: {e}")))?;
            return Ok(ToolResult::success(format!("Task {} deleted.", task_id)));
        }

        let mut task = team::read_task(&base, &team_name, task_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("task not found: {e}")))?;

        // Apply updates
        if let Some(status) = params["status"].as_str() {
            task.status = match status {
                "pending" => TaskStatus::Pending,
                "in_progress" => TaskStatus::InProgress,
                "completed" => TaskStatus::Completed,
                other => {
                    return Err(ToolError::InvalidParams(format!(
                        "invalid status: {}",
                        other
                    )))
                }
            };
        }
        if let Some(s) = params["subject"].as_str() {
            task.subject = s.to_string();
        }
        if let Some(d) = params["description"].as_str() {
            task.description = d.to_string();
        }
        if let Some(o) = params["owner"].as_str() {
            task.owner = Some(o.to_string());
        }
        if let Some(af) = params["active_form"].as_str() {
            task.active_form = Some(af.to_string());
        }

        if let Some(arr) = params["add_blocks"].as_array() {
            for v in arr {
                if let Some(id) = v.as_str() {
                    if !task.blocks.contains(&id.to_string()) {
                        task.blocks.push(id.to_string());
                    }
                }
            }
        }
        if let Some(arr) = params["add_blocked_by"].as_array() {
            for v in arr {
                if let Some(id) = v.as_str() {
                    if !task.blocked_by.contains(&id.to_string()) {
                        task.blocked_by.push(id.to_string());
                    }
                }
            }
        }

        team::write_task(&base, &team_name, &task)
            .map_err(|e| ToolError::ExecutionFailed(format!("write task: {e}")))?;

        // Auto-unblock: when a task completes, remove it from others' blocked_by
        if task.status == TaskStatus::Completed {
            if let Ok(all_tasks) = team::list_tasks(&base, &team_name) {
                for mut other in all_tasks {
                    if other.id != task.id && other.blocked_by.contains(&task.id) {
                        other.blocked_by.retain(|id| id != &task.id);
                        let _ = team::write_task(&base, &team_name, &other);
                    }
                }
            }
        }

        let result = serde_json::to_string_pretty(&task)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolResult::success(result))
    }
}

// ===========================================================================
// TaskListTool
// ===========================================================================

pub struct TaskListTool {
    team_state: Arc<RwLock<Option<TeamState>>>,
}

impl TaskListTool {
    pub fn new(team_state: Arc<RwLock<Option<TeamState>>>) -> Self {
        Self { team_state }
    }
}

#[async_trait]
impl Tool for TaskListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "task_list".into(),
            description: "List all tasks in the team's shared task list.".into(),
            parameters: HashMap::new(),
            required: vec![],
        }
    }

    async fn run(&self, _call: &ToolCall, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let (base, team_name) = get_team(&self.team_state)?;

        let tasks = team::list_tasks(&base, &team_name)
            .map_err(|e| ToolError::ExecutionFailed(format!("list tasks: {e}")))?;

        if tasks.is_empty() {
            return Ok(ToolResult::success("No tasks found.".into()));
        }

        let summary: Vec<serde_json::Value> = tasks
            .iter()
            .map(|t| {
                let open_blockers: Vec<&String> = t
                    .blocked_by
                    .iter()
                    .filter(|id| {
                        tasks
                            .iter()
                            .any(|o| &o.id == *id && o.status != TaskStatus::Completed)
                    })
                    .collect();
                serde_json::json!({
                    "id": t.id,
                    "subject": t.subject,
                    "status": t.status,
                    "owner": t.owner,
                    "blocked_by": open_blockers,
                })
            })
            .collect();

        let result = serde_json::to_string_pretty(&summary)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        Ok(ToolResult::success(result))
    }
}
