use anyhow::Result;
use crate::core::config::ProviderType;
use std::io::{self, BufRead, Write};

/// Model choice for the interactive selector
struct ModelChoice {
    id: &'static str,
    name: &'static str,
    vendor: &'static str,
    desc: &'static str,
}

const MODELS: &[ModelChoice] = &[
    ModelChoice {
        id: "zai-org/glm-5",
        name: "GLM 5",
        vendor: "Zhipu AI",
        desc: "745B MoE, strong coding ($0.80/M)",
    },
    ModelChoice {
        id: "zai-org/glm-4.7",
        name: "GLM 4.7",
        vendor: "Zhipu AI",
        desc: "Efficient MoE, 202K ctx ($0.52/M)",
    },
    ModelChoice {
        id: "deepseek-ai/deepseek-v3.2-speciale",
        name: "DeepSeek V3.2 Speciale",
        vendor: "DeepSeek",
        desc: "685B MoE, 128K ctx ($0.40/M)",
    },
    ModelChoice {
        id: "qwen/qwen3-max-2026-01-23",
        name: "Qwen3 Max",
        vendor: "Alibaba",
        desc: "Flagship, 131K ctx ($1.20/M)",
    },
    ModelChoice {
        id: "Qwen/Qwen3-Coder",
        name: "Qwen3 Coder",
        vendor: "Alibaba",
        desc: "480B MoE, code-optimized ($0.78/M)",
    },
    ModelChoice {
        id: "moonshotai/kimi-k2.5",
        name: "Kimi K2.5",
        vendor: "Moonshot",
        desc: "Deep reasoning, multimodal ($0.50/M)",
    },
];

/// Show provider selection menu
fn select_provider(current: ProviderType) -> Result<ProviderType> {
    let default_idx = match current {
        ProviderType::AtlasCloud => 0,
        ProviderType::OpenRouter => 1,
    };

    eprintln!("\x1b[1;36m  Select API provider:\x1b[0m\n");

    let providers = [
        ("Atlas Cloud", "api.atlascloud.ai", "GLM, Kimi, Qwen, DeepSeek"),
        ("OpenRouter", "openrouter.ai", "GLM, Kimi, Qwen, DeepSeek"),
    ];

    for (i, (name, url, models)) in providers.iter().enumerate() {
        let default_marker = if i == default_idx {
            " \x1b[33m\u{2190} default\x1b[0m"
        } else {
            ""
        };
        eprintln!(
            "    \x1b[1;33m[{}]\x1b[0m \x1b[1m{:<16}\x1b[0m \x1b[90m({})\x1b[0m  {}{}",
            i + 1,
            name,
            url,
            models,
            default_marker,
        );
    }
    eprintln!();

    eprint!(
        "  \x1b[1mProvider \x1b[33m[{}]\x1b[0m\x1b[1m:\x1b[0m ",
        default_idx + 1
    );
    io::stderr().flush().ok();

    let input = read_line_lossy()?;
    let input = input.trim();

    let idx = if input.is_empty() {
        default_idx
    } else {
        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= providers.len() => n - 1,
            _ => {
                eprintln!("  \x1b[33mInvalid choice, using default.\x1b[0m");
                default_idx
            }
        }
    };

    let chosen = match idx {
        0 => ProviderType::AtlasCloud,
        _ => ProviderType::OpenRouter,
    };

    let (name, url, _) = providers[idx];
    eprintln!(
        "\n  \x1b[32m\u{2713}\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
        name, url
    );

    Ok(chosen)
}

/// Show model selection menu and return the chosen ModelId
fn select_model(_provider: ProviderType) -> Result<crate::core::model::ModelId> {
    let models = MODELS;

    eprintln!("\x1b[1;36m  Select a model:\x1b[0m\n");
    for (i, m) in models.iter().enumerate() {
        let default_marker = if i == 0 {
            " \x1b[33m\u{2190} default\x1b[0m"
        } else {
            ""
        };
        eprintln!(
            "    \x1b[1;33m[{}]\x1b[0m \x1b[1m{:<20}\x1b[0m \x1b[90m({})\x1b[0m  {}{}",
            i + 1,
            m.name,
            m.vendor,
            m.desc,
            default_marker,
        );
    }

    // Custom model option
    eprintln!(
        "    \x1b[1;33m[{}]\x1b[0m \x1b[90mCustom model ID...\x1b[0m",
        models.len() + 1
    );
    eprintln!();

    eprint!("  \x1b[1mModel \x1b[33m[1]\x1b[0m\x1b[1m:\x1b[0m ");
    io::stderr().flush().ok();

    let input = read_line_lossy()?;
    let input = input.trim();

    if input.is_empty() {
        let chosen = &models[0];
        eprintln!(
            "\n  \x1b[32m\u{2713}\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
            chosen.name, chosen.vendor
        );
        return Ok(crate::core::model::ModelId(chosen.id.to_string()));
    }

    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= models.len() => {
            let chosen = &models[n - 1];
            eprintln!(
                "\n  \x1b[32m\u{2713}\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
                chosen.name, chosen.vendor
            );
            Ok(crate::core::model::ModelId(chosen.id.to_string()))
        }
        Ok(n) if n == models.len() + 1 => {
            // Custom model ID
            eprint!("  \x1b[1mModel ID:\x1b[0m ");
            io::stderr().flush().ok();
            let custom = read_line_lossy()?;
            let custom = custom.trim().to_string();
            if custom.is_empty() {
                eprintln!("  \x1b[33mEmpty input, using default.\x1b[0m");
                let chosen = &models[0];
                eprintln!(
                    "\n  \x1b[32m\u{2713}\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
                    chosen.name, chosen.vendor
                );
                Ok(crate::core::model::ModelId(chosen.id.to_string()))
            } else {
                eprintln!(
                    "\n  \x1b[32m\u{2713}\x1b[0m Using custom model \x1b[1;36m{}\x1b[0m\n",
                    custom
                );
                Ok(crate::core::model::ModelId(custom))
            }
        }
        _ => {
            // Try as direct model ID string
            if input.contains('/') {
                eprintln!(
                    "\n  \x1b[32m\u{2713}\x1b[0m Using custom model \x1b[1;36m{}\x1b[0m\n",
                    input
                );
                Ok(crate::core::model::ModelId(input.to_string()))
            } else {
                eprintln!("  \x1b[33mInvalid choice, using default.\x1b[0m");
                let chosen = &models[0];
                eprintln!(
                    "\n  \x1b[32m\u{2713}\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
                    chosen.name, chosen.vendor
                );
                Ok(crate::core::model::ModelId(chosen.id.to_string()))
            }
        }
    }
}

/// Show API key status and allow input/change.
/// If a key already exists, just show it and continue without prompting.
fn prompt_api_key(config: &mut crate::core::config::AppConfig) -> Result<()> {
    let (provider_name, key_env) = match config.provider_type {
        ProviderType::AtlasCloud => ("Atlas Cloud", "ATLAS_API_KEY"),
        ProviderType::OpenRouter => ("OpenRouter", "OPENROUTER_API_KEY"),
    };

    let active_keys = config.get_active_api_keys();

    if active_keys.is_empty() {
        // No key — must input
        eprintln!(
            "  \x1b[33mNo API key for {}.\x1b[0m",
            provider_name,
        );
        eprintln!(
            "  \x1b[90mEnv: {} | Config: octo-code.json\x1b[0m",
            key_env,
        );
        eprint!("\n  \x1b[1mAPI Key:\x1b[0m ");
        io::stderr().flush().ok();

        let key = read_line_lossy()?.trim().to_string();
        if key.is_empty() {
            anyhow::bail!("API key is required. Set {} or enter it here.", key_env);
        }

        apply_api_key(config, &key);
        eprintln!("  \x1b[32m\u{2713}\x1b[0m Key saved.\n");
    } else {
        // Key exists — show masked version and continue
        let current = &active_keys[0];
        let masked = if current.len() > 8 {
            format!("{}...{}", &current[..4], &current[current.len() - 4..])
        } else {
            "****".to_string()
        };

        eprintln!(
            "  \x1b[90mAPI Key: \x1b[36m{}\x1b[0m \x1b[90m({})\x1b[0m",
            masked,
            provider_name,
        );
    }

    Ok(())
}

/// Apply an API key to the config based on current provider type
fn apply_api_key(config: &mut crate::core::config::AppConfig, key: &str) {
    match config.provider_type {
        ProviderType::OpenRouter => {
            config.openrouter_api_key = Some(key.to_string());
        }
        ProviderType::AtlasCloud => {
            config.api_key = Some(key.to_string());
            config.api_keys = vec![key.to_string()];
        }
    }

    // Also try to save to config file for persistence
    if let Err(e) = save_api_key_to_config(config) {
        eprintln!("  \x1b[33mWarning: Could not save to config: {}\x1b[0m", e);
    }
}

/// Save API key to global config file
fn save_api_key_to_config(config: &crate::core::config::AppConfig) -> Result<()> {
    let config_dir = if cfg!(target_os = "macos") {
        dirs::home_dir().map(|h| h.join("Library/Application Support/octo-code"))
    } else {
        dirs::config_dir().map(|c| c.join("octo-code"))
    };

    if let Some(dir) = config_dir {
        std::fs::create_dir_all(&dir)?;
        let config_path = dir.join("config.json");

        let mut file_config: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        match config.provider_type {
            ProviderType::OpenRouter => {
                if let Some(ref key) = config.openrouter_api_key {
                    file_config["openrouter_api_key"] = serde_json::Value::String(key.clone());
                    file_config["provider_type"] = serde_json::Value::String("open_router".into());
                }
            }
            ProviderType::AtlasCloud => {
                if let Some(ref key) = config.api_key {
                    file_config["api_key"] = serde_json::Value::String(key.clone());
                    file_config["base_url"] =
                        serde_json::Value::String("https://api.atlascloud.ai".into());
                }
            }
        }

        std::fs::write(&config_path, serde_json::to_string_pretty(&file_config)?)?;
    }

    Ok(())
}

/// Read a line from stdin, handling non-UTF-8 bytes gracefully
fn read_line_lossy() -> Result<String> {
    let stdin = io::stdin();
    let mut buf = Vec::new();
    match stdin.lock().read_until(b'\n', &mut buf) {
        Ok(0) => Ok(String::new()),
        Ok(_) => Ok(String::from_utf8_lossy(&buf).into_owned()),
        Err(e) => Err(anyhow::anyhow!("Input error: {e}")),
    }
}

/// Read the user's task/requirements
fn read_prompt() -> Result<Option<String>> {
    eprint!("  \x1b[1;32mocto>\x1b[0m ");
    io::stderr().flush().ok();

    let stdin = io::stdin();
    let mut buf = Vec::new();
    match stdin.lock().read_until(b'\n', &mut buf) {
        Ok(0) => Ok(None), // EOF
        Ok(_) => {
            let trimmed = String::from_utf8_lossy(&buf).trim().to_string();
            if trimmed.is_empty() {
                Ok(Some(String::new()))
            } else {
                Ok(Some(trimmed))
            }
        }
        Err(e) => Err(anyhow::anyhow!("Input error: {e}")),
    }
}

pub async fn run(
    mut config: crate::core::config::AppConfig,
    db: crate::storage::Database,
    permission_service: std::sync::Arc<dyn crate::core::permission::PermissionService>,
    team_state: std::sync::Arc<std::sync::RwLock<Option<crate::core::team::TeamState>>>,
    resume_session: Option<String>,
    preset_model: Option<crate::core::model::ModelId>,
) -> Result<()> {
    eprintln!();

    // Provider selection (skip if preset via --model flag)
    if preset_model.is_none() {
        let provider_type = select_provider(config.provider_type)?;
        config.provider_type = provider_type;
    }

    // Show current API key status and allow changing
    prompt_api_key(&mut config)?;

    // Model selection (skip if preset via --model flag)
    let model_id = match preset_model {
        Some(id) => id,
        None => select_model(config.provider_type)?,
    };

    // For custom/OpenRouter models not in registry, create a fallback
    if crate::core::model::get_model(&model_id).is_none() {
        // Register as a generic model with reasonable defaults
        eprintln!(
            "  \x1b[33mNote: Model '{}' not in registry, using default parameters.\x1b[0m\n",
            model_id
        );
    }

    // Resolve display name
    let model_display = crate::core::model::get_model(&model_id)
        .map(|m| m.display_name.clone())
        .unwrap_or_else(|| model_id.0.clone());

    // Banner with model name
    eprintln!(
        "  \x1b[1;35m\u{1F419} OctoCode Agent\x1b[0m v{} \x1b[90m(\x1b[1;36m{}\x1b[90m)\x1b[0m",
        env!("CARGO_PKG_VERSION"),
        model_display,
    );
    eprintln!("  \x1b[90mType your task, /help for commands, Ctrl-D to exit\x1b[0m");
    eprintln!();

    // Get pricing for cost display
    let pricing = crate::core::model::get_model(&model_id).map(|m| {
        super::output::Pricing {
            cost_per_1m_input: m.pricing.cost_per_1m_input,
            cost_per_1m_output: m.pricing.cost_per_1m_output,
        }
    });

    // Build provider + agent
    let provider = crate::providers::create_provider(&config, Some(&model_id))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let tools = crate::tools::create_all_tools(
        permission_service.clone(),
        config.coderlm.server_url.clone(),
        team_state.clone(),
    )
    .await;

    let system_prompt = crate::agent::prompt::build_system_prompt(
        &config.working_dir,
        &config.context_paths,
    );

    let agent = crate::agent::Agent::new(
        provider,
        tools,
        permission_service.clone(),
        system_prompt,
        config.working_dir.clone(),
        team_state,
    );

    // Session
    let session = match resume_session {
        Some(id) => db
            .sessions()
            .get(&id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => {
            let s = crate::core::session::Session::new("Interactive session".into());
            db.sessions()
                .create(&s)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            s
        }
    };

    // Auto-approve all permissions in interactive mode
    permission_service.auto_approve_session(&session.id);

    eprintln!("  \x1b[90mSession: {}\x1b[0m", &session.id[..8]);
    eprintln!();

    // Main loop
    loop {
        let prompt = match read_prompt()? {
            None => {
                eprintln!("\n  \x1b[90mGoodbye!\x1b[0m");
                break;
            }
            Some(p) if p.is_empty() => continue,
            Some(p) => p,
        };

        // Handle slash commands
        if prompt.starts_with('/') {
            match prompt.as_str() {
                "/help" | "/h" => {
                    eprintln!("\n  \x1b[1mCommands:\x1b[0m");
                    eprintln!("    /model     Show current model");
                    eprintln!("    /key       Change API key");
                    eprintln!("    /cost      Show token usage & cost");
                    eprintln!("    /sessions  List sessions");
                    eprintln!("    /clear     Clear current session");
                    eprintln!("    /exit      Exit\n");
                    continue;
                }
                "/exit" | "/quit" | "/q" => {
                    eprintln!("  \x1b[90mGoodbye!\x1b[0m");
                    break;
                }
                "/model" => {
                    eprintln!(
                        "  Model: \x1b[1;36m{}\x1b[0m ({})\n",
                        agent.model_name(),
                        agent.model_id()
                    );
                    continue;
                }
                "/cost" => {
                    let s = db
                        .sessions()
                        .get(&session.id)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    eprintln!(
                        "  Tokens: {} in / {} out | Cost: \x1b[33m${:.4}\x1b[0m\n",
                        s.prompt_tokens, s.completion_tokens, s.cost
                    );
                    continue;
                }
                "/sessions" | "/s" => {
                    let sessions = db
                        .sessions()
                        .list()
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    for s in sessions {
                        let marker = if s.id == session.id { " \u{2190}" } else { "" };
                        eprintln!(
                            "    \x1b[90m{}\x1b[0m {}{}  ({} msgs)",
                            &s.id[..8],
                            s.title,
                            marker,
                            s.message_count,
                        );
                    }
                    eprintln!();
                    continue;
                }
                "/key" => {
                    let (provider_name, key_env) = match config.provider_type {
                        ProviderType::AtlasCloud => ("Atlas Cloud", "ATLAS_API_KEY"),
                        ProviderType::OpenRouter => ("OpenRouter", "OPENROUTER_API_KEY"),
                    };
                    let active_keys = config.get_active_api_keys();
                    if !active_keys.is_empty() {
                        let current = &active_keys[0];
                        let masked = if current.len() > 8 {
                            format!("{}...{}", &current[..4], &current[current.len() - 4..])
                        } else {
                            "****".to_string()
                        };
                        eprintln!(
                            "  \x1b[90mCurrent: \x1b[36m{}\x1b[0m \x1b[90m({})\x1b[0m",
                            masked, provider_name,
                        );
                    }
                    eprintln!(
                        "  \x1b[90mEnv: {} | Config: octo-code.json\x1b[0m",
                        key_env,
                    );
                    eprint!("  \x1b[1mNew API Key:\x1b[0m ");
                    io::stderr().flush().ok();
                    let input = read_line_lossy()?.trim().to_string();
                    if input.is_empty() {
                        eprintln!("  \x1b[33mCancelled.\x1b[0m\n");
                    } else {
                        apply_api_key(&mut config, &input);
                        eprintln!("  \x1b[32m\u{2713}\x1b[0m Key updated. Restart to apply.\n");
                    }
                    continue;
                }
                "/clear" => {
                    db.messages()
                        .delete_session_messages(&session.id)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    eprintln!("  \x1b[32m\u{2713}\x1b[0m Session cleared.\n");
                    continue;
                }
                _ => {
                    eprintln!("  Unknown command. Type /help\n");
                    continue;
                }
            }
        }

        eprintln!();

        // Load history
        let messages = db
            .messages()
            .list(&session.id)
            .await
            .unwrap_or_default();

        // Run agent
        let (mut rx, _cancel) = agent.run(session.id.clone(), messages, prompt);

        // Render output
        super::output::render_stream(&mut rx, false, pricing).await?;

        eprintln!();
    }

    Ok(())
}
