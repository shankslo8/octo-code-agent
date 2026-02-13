use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::ConfigError;
use crate::model::ModelId;

/// Atlas Cloud base URL (OpenAI-compatible unified gateway)
const DEFAULT_BASE_URL: &str = "https://api.atlascloud.ai";

/// OpenRouter base URL
const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api";

/// API provider type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    AtlasCloud,
    OpenRouter,
}

impl Default for ProviderType {
    fn default() -> Self {
        Self::AtlasCloud
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_working_dir")]
    pub working_dir: PathBuf,

    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    /// Single API key for Atlas Cloud (covers all models)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Multiple API keys for load balancing (randomly rotated per request)
    #[serde(default)]
    pub api_keys: Vec<String>,

    /// API provider type (atlas_cloud or open_router)
    #[serde(default)]
    pub provider_type: ProviderType,

    /// OpenRouter API key (separate from Atlas Cloud key)
    #[serde(default)]
    pub openrouter_api_key: Option<String>,

    /// Base URL for the API gateway (default: Atlas Cloud)
    #[serde(default = "default_base_url")]
    pub base_url: String,

    #[serde(default)]
    pub agent: AgentConfig,

    #[serde(default)]
    pub shell: ShellConfig,

    #[serde(default = "default_context_paths")]
    pub context_paths: Vec<String>,

    #[serde(default)]
    pub debug: bool,

    #[serde(default)]
    pub coderlm: CoderlmConfig,
}

fn default_base_url() -> String {
    DEFAULT_BASE_URL.into()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            working_dir: default_working_dir(),
            data_dir: default_data_dir(),
            api_key: None,
            api_keys: vec![],
            provider_type: ProviderType::default(),
            openrouter_api_key: None,
            base_url: default_base_url(),
            agent: AgentConfig::default(),
            shell: ShellConfig::default(),
            context_paths: default_context_paths(),
            debug: false,
            coderlm: CoderlmConfig::default(),
        }
    }
}

fn default_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn default_data_dir() -> String {
    ".octo-code".into()
}

fn default_context_paths() -> Vec<String> {
    vec![
        "CLAUDE.md".into(),
        "CLAUDE.local.md".into(),
        "octo-code.md".into(),
        ".github/copilot-instructions.md".into(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Primary coding model (default for most tasks)
    #[serde(default = "default_coder_model")]
    pub coder_model: ModelId,

    /// Fast/cheap model for simple tasks (ls, grep, quick questions)
    #[serde(default = "default_fast_model")]
    pub fast_model: ModelId,

    /// Strong model for complex reasoning and architecture
    #[serde(default = "default_reasoning_model")]
    pub reasoning_model: ModelId,

    /// Model for long context tasks (large file analysis)
    #[serde(default = "default_long_context_model")]
    pub long_context_model: ModelId,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: u64,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
}

fn default_coder_model() -> ModelId {
    ModelId("z-ai/glm-5".into())
}

fn default_fast_model() -> ModelId {
    ModelId("minimax/minimax-m2.5".into())
}

fn default_reasoning_model() -> ModelId {
    ModelId("Qwen/Qwen3-Coder".into())
}

fn default_long_context_model() -> ModelId {
    ModelId("moonshotai/kimi-k2-thinking".into())
}

fn default_max_tokens() -> u64 {
    16_384
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            coder_model: default_coder_model(),
            fast_model: default_fast_model(),
            reasoning_model: default_reasoning_model(),
            long_context_model: default_long_context_model(),
            max_tokens: default_max_tokens(),
            reasoning_effort: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default = "default_shell")]
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            path: default_shell(),
            args: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoderlmConfig {
    #[serde(default = "default_coderlm_url")]
    pub server_url: String,

    #[serde(default = "default_coderlm_timeout")]
    pub timeout_secs: u64,
}

fn default_coderlm_url() -> String {
    "http://127.0.0.1:9999".into()
}

fn default_coderlm_timeout() -> u64 {
    10
}

impl Default for CoderlmConfig {
    fn default() -> Self {
        Self {
            server_url: default_coderlm_url(),
            timeout_secs: default_coderlm_timeout(),
        }
    }
}

pub fn load_config(working_dir: Option<PathBuf>) -> Result<AppConfig, ConfigError> {
    let wd = working_dir.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let mut config = AppConfig::default();
    config.working_dir = wd.clone();

    // Try loading global config
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("octo-code").join("config.json");
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)
                .map_err(|e| ConfigError::File(e.to_string()))?;
            let file_config: AppConfig = serde_json::from_str(&content)
                .map_err(|e| ConfigError::Invalid(e.to_string()))?;
            merge_config(&mut config, file_config);
        }
    }

    // Try loading local project config
    let local_path = wd.join("octo-code.json");
    if local_path.exists() {
        let content = std::fs::read_to_string(&local_path)
            .map_err(|e| ConfigError::File(e.to_string()))?;
        let file_config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::Invalid(e.to_string()))?;
        merge_config(&mut config, file_config);
    }

    // Auto-detect API key from environment
    detect_api_key(&mut config);

    // Auto-detect CodeRLM server URL from environment
    detect_coderlm_url(&mut config);

    Ok(config)
}

fn merge_config(base: &mut AppConfig, overlay: AppConfig) {
    if overlay.api_key.is_some() {
        base.api_key = overlay.api_key;
    }
    if !overlay.api_keys.is_empty() {
        base.api_keys = overlay.api_keys;
    }
    if overlay.openrouter_api_key.is_some() {
        base.openrouter_api_key = overlay.openrouter_api_key;
    }
    if overlay.provider_type != ProviderType::default() {
        base.provider_type = overlay.provider_type;
    }
    if overlay.base_url != default_base_url() {
        base.base_url = overlay.base_url;
    }
    if overlay.agent.coder_model.0 != default_coder_model().0 {
        base.agent.coder_model = overlay.agent.coder_model;
    }
    if overlay.agent.fast_model.0 != default_fast_model().0 {
        base.agent.fast_model = overlay.agent.fast_model;
    }
    if overlay.agent.reasoning_model.0 != default_reasoning_model().0 {
        base.agent.reasoning_model = overlay.agent.reasoning_model;
    }
    if overlay.agent.long_context_model.0 != default_long_context_model().0 {
        base.agent.long_context_model = overlay.agent.long_context_model;
    }
    if overlay.agent.max_tokens != default_max_tokens() {
        base.agent.max_tokens = overlay.agent.max_tokens;
    }
    if overlay.agent.reasoning_effort.is_some() {
        base.agent.reasoning_effort = overlay.agent.reasoning_effort;
    }
    if overlay.debug {
        base.debug = true;
    }
    if overlay.coderlm.server_url != default_coderlm_url() {
        base.coderlm.server_url = overlay.coderlm.server_url;
    }
    if overlay.coderlm.timeout_secs != default_coderlm_timeout() {
        base.coderlm.timeout_secs = overlay.coderlm.timeout_secs;
    }
}

fn detect_api_key(config: &mut AppConfig) {
    // Check for OpenRouter API key
    if config.openrouter_api_key.is_none() {
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
            if !key.is_empty() {
                config.openrouter_api_key = Some(key);
            }
        }
    }

    // Check for multiple Atlas keys: ATLAS_API_KEYS=key1,key2,key3
    if config.api_keys.is_empty() {
        if let Ok(keys) = std::env::var("ATLAS_API_KEYS") {
            let parsed: Vec<String> = keys
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parsed.is_empty() {
                config.api_keys = parsed;
            }
        }
    }

    // If we have multiple keys, use the first as the primary
    if config.api_key.is_none() && !config.api_keys.is_empty() {
        config.api_key = Some(config.api_keys[0].clone());
        return;
    }

    if config.api_key.is_some() {
        // Ensure single key is also in the keys list
        if config.api_keys.is_empty() {
            config.api_keys.push(config.api_key.clone().unwrap());
        }
        return;
    }

    // Check ATLAS_API_KEY first, then fall back to common provider keys
    let env_vars = [
        "ATLAS_API_KEY",
        "ATLAS_CLOUD_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
    ];

    for env_var in &env_vars {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                config.api_key = Some(key.clone());
                if config.api_keys.is_empty() {
                    config.api_keys.push(key);
                }
                return;
            }
        }
    }

    // If no Atlas key found but OpenRouter key exists, auto-detect provider
    if config.api_key.is_none() && config.openrouter_api_key.is_some() {
        config.provider_type = ProviderType::OpenRouter;
    }
}

fn detect_coderlm_url(config: &mut AppConfig) {
    if let Ok(url) = std::env::var("CODERLM_URL") {
        if !url.is_empty() {
            config.coderlm.server_url = url;
        }
    }
}

impl AppConfig {
    pub fn get_api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub fn data_path(&self) -> PathBuf {
        self.working_dir.join(&self.data_dir)
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.as_ref().map_or(false, |k| !k.is_empty())
    }

    /// Check if any provider has an API key configured
    pub fn has_any_api_key(&self) -> bool {
        self.has_api_key() || self.openrouter_api_key.is_some()
    }

    pub fn get_api_keys(&self) -> Vec<String> {
        if !self.api_keys.is_empty() {
            self.api_keys.clone()
        } else if let Some(ref key) = self.api_key {
            vec![key.clone()]
        } else {
            vec![]
        }
    }

    /// Get API keys for the currently active provider
    pub fn get_active_api_keys(&self) -> Vec<String> {
        match self.provider_type {
            ProviderType::OpenRouter => {
                if let Some(ref key) = self.openrouter_api_key {
                    vec![key.clone()]
                } else {
                    vec![]
                }
            }
            ProviderType::AtlasCloud => self.get_api_keys(),
        }
    }

    /// Get base URL for the currently active provider
    pub fn get_active_base_url(&self) -> String {
        match self.provider_type {
            ProviderType::OpenRouter => OPENROUTER_BASE_URL.to_string(),
            ProviderType::AtlasCloud => self.base_url.clone(),
        }
    }
}
