use octo_core::permission::{PermissionDecision, PermissionRequest, PermissionService};
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::Mutex;

pub struct CliPermissionService {
    auto_approve_sessions: Mutex<HashSet<String>>,
    persistent_approvals: Mutex<HashSet<String>>,
}

impl CliPermissionService {
    pub fn new() -> Self {
        Self {
            auto_approve_sessions: Mutex::new(HashSet::new()),
            persistent_approvals: Mutex::new(HashSet::new()),
        }
    }

    fn approval_key(req: &PermissionRequest) -> String {
        format!("{}:{}:{}", req.session_id, req.tool_name, req.action)
    }
}

#[async_trait::async_trait]
impl PermissionService for CliPermissionService {
    async fn request(&self, req: PermissionRequest) -> PermissionDecision {
        // Check auto-approve
        {
            let sessions = self.auto_approve_sessions.lock().unwrap();
            if sessions.contains(&req.session_id) {
                return PermissionDecision::Allow;
            }
        }

        // Check persistent approval
        let key = Self::approval_key(&req);
        {
            let persistent = self.persistent_approvals.lock().unwrap();
            if persistent.contains(&key) {
                return PermissionDecision::Allow;
            }
        }

        // Prompt user
        eprintln!();
        eprintln!("\x1b[33m[Permission Required]\x1b[0m {}", req.description);
        if let Some(path) = &req.path {
            eprintln!("  Path: {path}");
        }
        eprint!("  Allow? [y]es / [n]o / [a]lways: ");
        io::stderr().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return PermissionDecision::Deny;
        }

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "" => PermissionDecision::Allow,
            "a" | "always" => {
                let mut persistent = self.persistent_approvals.lock().unwrap();
                persistent.insert(key);
                PermissionDecision::AllowPersistent
            }
            _ => PermissionDecision::Deny,
        }
    }

    fn auto_approve_session(&self, session_id: &str) {
        let mut sessions = self.auto_approve_sessions.lock().unwrap();
        sessions.insert(session_id.to_string());
    }
}
