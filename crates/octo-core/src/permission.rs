use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub action: String,
    pub description: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    AllowPersistent,
    Deny,
}

#[async_trait]
pub trait PermissionService: Send + Sync {
    async fn request(&self, req: PermissionRequest) -> PermissionDecision;
    fn auto_approve_session(&self, session_id: &str);
}
