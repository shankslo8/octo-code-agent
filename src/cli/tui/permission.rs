use crate::core::permission::{PermissionDecision, PermissionRequest, PermissionService};
use std::collections::HashSet;
use std::sync::Mutex;
use tokio::sync::{mpsc, oneshot};

pub struct PermissionReq {
    pub request: PermissionRequest,
    pub responder: oneshot::Sender<PermissionDecision>,
}

pub struct TuiPermissionService {
    tx: mpsc::Sender<PermissionReq>,
    auto_approve_sessions: Mutex<HashSet<String>>,
    persistent_approvals: Mutex<HashSet<String>>,
}

impl TuiPermissionService {
    pub fn new() -> (Self, mpsc::Receiver<PermissionReq>) {
        let (tx, rx) = mpsc::channel(32);
        (
            Self {
                tx,
                auto_approve_sessions: Mutex::new(HashSet::new()),
                persistent_approvals: Mutex::new(HashSet::new()),
            },
            rx,
        )
    }
}

#[async_trait::async_trait]
impl PermissionService for TuiPermissionService {
    async fn request(&self, req: PermissionRequest) -> PermissionDecision {
        // Check auto-approve
        {
            let sessions = self.auto_approve_sessions.lock().unwrap();
            if sessions.contains(&req.session_id) {
                return PermissionDecision::Allow;
            }
        }

        // Check persistent approval
        let key = format!("{}:{}:{}", req.session_id, req.tool_name, req.action);
        {
            let persistent = self.persistent_approvals.lock().unwrap();
            if persistent.contains(&key) {
                return PermissionDecision::Allow;
            }
        }

        // Send to TUI for user decision
        let (resp_tx, resp_rx) = oneshot::channel();
        let perm_req = PermissionReq {
            request: req,
            responder: resp_tx,
        };

        if self.tx.send(perm_req).await.is_err() {
            return PermissionDecision::Deny;
        }

        match resp_rx.await {
            Ok(decision) => {
                if decision == PermissionDecision::AllowPersistent {
                    let mut persistent = self.persistent_approvals.lock().unwrap();
                    persistent.insert(key);
                }
                decision
            }
            Err(_) => PermissionDecision::Deny,
        }
    }

    fn auto_approve_session(&self, session_id: &str) {
        let mut sessions = self.auto_approve_sessions.lock().unwrap();
        sessions.insert(session_id.to_string());
    }
}
