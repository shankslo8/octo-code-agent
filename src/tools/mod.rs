mod bash;
mod coderlm;
mod edit;
mod glob_tool;
mod grep;
mod ls;
mod send_message;
mod task_mgmt;
mod team;
mod view;
mod write;

pub use bash::BashTool;
pub use coderlm::CoderlmTool;
pub use edit::EditTool;
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use ls::LsTool;
pub use send_message::{CheckInboxTool, SendMessageTool};
pub use task_mgmt::{TaskCreateTool, TaskGetTool, TaskListTool, TaskUpdateTool};
pub use team::{SpawnAgentTool, TeamCreateTool, TeamDeleteTool};
pub use view::ViewTool;
pub use write::WriteTool;

use crate::core::permission::PermissionService;
use crate::core::team::TeamState;
use crate::core::tool::Tool;
use std::sync::{Arc, RwLock};

#[cfg(test)]
mod tests;

pub async fn create_all_tools(
    permission_service: Arc<dyn PermissionService>,
    coderlm_server_url: String,
    team_state: Arc<RwLock<Option<TeamState>>>,
) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(BashTool::new(permission_service.clone())),
        Arc::new(ViewTool),
        Arc::new(WriteTool::new(permission_service.clone())),
        Arc::new(EditTool::new(permission_service.clone())),
        Arc::new(LsTool),
        Arc::new(GlobTool),
        Arc::new(GrepTool),
    ];

    // Only add CodeRLM if server is reachable
    if is_coderlm_available(&coderlm_server_url).await {
        eprintln!("  \x1b[32m✓\x1b[0m CodeRLM connected");
        tools.push(Arc::new(CoderlmTool::new(coderlm_server_url)));
    }

    // Team tools
    tools.push(Arc::new(TeamCreateTool::new(team_state.clone())));
    tools.push(Arc::new(TeamDeleteTool::new(team_state.clone())));
    tools.push(Arc::new(SpawnAgentTool::new(team_state.clone())));
    tools.push(Arc::new(TaskCreateTool::new(team_state.clone())));
    tools.push(Arc::new(TaskGetTool::new(team_state.clone())));
    tools.push(Arc::new(TaskUpdateTool::new(team_state.clone())));
    tools.push(Arc::new(TaskListTool::new(team_state.clone())));
    tools.push(Arc::new(SendMessageTool::new(team_state.clone())));
    tools.push(Arc::new(CheckInboxTool::new(team_state.clone())));

    tools
}

async fn is_coderlm_available(server_url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    client
        .get(format!("{}/health", server_url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
