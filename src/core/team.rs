use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Shared team state (in-process, behind Arc<RwLock<Option<TeamState>>>)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TeamState {
    pub team_name: String,
    pub agent_name: String,
    pub agent_id: String,
    pub is_lead: bool,
    pub base_dir: PathBuf,
}

impl TeamState {
    pub fn new(team_name: String, agent_name: String, is_lead: bool) -> Self {
        let agent_id = format!("{}@{}", agent_name, team_name);
        let base_dir = default_base_dir();
        Self {
            team_name,
            agent_name,
            agent_id,
            is_lead,
            base_dir,
        }
    }

    pub fn with_base_dir(mut self, base_dir: PathBuf) -> Self {
        self.base_dir = base_dir;
        self
    }
}

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

/// Default base directory: `~/.octo-code/`
pub fn default_base_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".octo-code")
}

pub fn teams_dir(base: &Path) -> PathBuf {
    base.join("teams")
}

pub fn tasks_dir(base: &Path, team: &str) -> PathBuf {
    base.join("tasks").join(team)
}

pub fn team_config_path(base: &Path, team: &str) -> PathBuf {
    teams_dir(base).join(team).join("config.json")
}

pub fn inboxes_dir(base: &Path, team: &str) -> PathBuf {
    teams_dir(base).join(team).join("inboxes")
}

pub fn inbox_path(base: &Path, team: &str, agent: &str) -> PathBuf {
    inboxes_dir(base, team).join(format!("{}.json", agent))
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub lead_agent_id: String,
    pub members: Vec<TeamMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub agent_id: String,
    pub name: String,
    pub agent_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub cwd: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub from: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub read: bool,
}

// ---------------------------------------------------------------------------
// File I/O  (all functions take `base_dir` for testability)
// ---------------------------------------------------------------------------

pub fn read_team_config(base: &Path, team: &str) -> io::Result<TeamConfig> {
    let path = team_config_path(base, team);
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn write_team_config(base: &Path, team: &str, config: &TeamConfig) -> io::Result<()> {
    let path = team_config_path(base, team);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)
}

pub fn read_task(base: &Path, team: &str, id: &str) -> io::Result<TaskItem> {
    let path = tasks_dir(base, team).join(format!("{}.json", id));
    let data = fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn write_task(base: &Path, team: &str, task: &TaskItem) -> io::Result<()> {
    let dir = tasks_dir(base, team);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", task.id));
    let data = serde_json::to_string_pretty(task)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)
}

pub fn delete_task(base: &Path, team: &str, id: &str) -> io::Result<()> {
    let path = tasks_dir(base, team).join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

pub fn list_tasks(base: &Path, team: &str) -> io::Result<Vec<TaskItem>> {
    let dir = tasks_dir(base, team);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut tasks = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            let filename = path.file_stem().unwrap_or_default().to_string_lossy();
            if filename == "counter" {
                continue;
            }
            if let Ok(task) = read_task_from_path(&path) {
                tasks.push(task);
            }
        }
    }
    tasks.sort_by(|a, b| {
        let a_num: u64 = a.id.parse().unwrap_or(0);
        let b_num: u64 = b.id.parse().unwrap_or(0);
        a_num.cmp(&b_num)
    });
    Ok(tasks)
}

fn read_task_from_path(path: &Path) -> io::Result<TaskItem> {
    let data = fs::read_to_string(path)?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Increment and return the next task ID for a team.
pub fn next_task_id(base: &Path, team: &str) -> io::Result<String> {
    let dir = tasks_dir(base, team);
    fs::create_dir_all(&dir)?;
    let counter_path = dir.join("counter.json");

    #[derive(Serialize, Deserialize)]
    struct Counter {
        next_id: u64,
    }

    let mut counter = if counter_path.exists() {
        let data = fs::read_to_string(&counter_path)?;
        serde_json::from_str::<Counter>(&data).unwrap_or(Counter { next_id: 1 })
    } else {
        Counter { next_id: 1 }
    };

    let id = counter.next_id;
    counter.next_id += 1;

    let data = serde_json::to_string(&counter)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&counter_path, data)?;

    Ok(id.to_string())
}

pub fn read_inbox(base: &Path, team: &str, agent: &str) -> io::Result<Vec<InboxMessage>> {
    let path = inbox_path(base, team, agent);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn append_inbox(base: &Path, team: &str, agent: &str, msg: InboxMessage) -> io::Result<()> {
    let dir = inboxes_dir(base, team);
    fs::create_dir_all(&dir)?;

    let mut messages = read_inbox(base, team, agent).unwrap_or_default();
    messages.push(msg);

    let path = inbox_path(base, team, agent);
    let data = serde_json::to_string_pretty(&messages)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&path, data)
}
