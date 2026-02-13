use anyhow::Result;
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
        desc: "745B MoE, frontier agentic, MIT",
    },
    ModelChoice {
        id: "zai-org/glm-4.7",
        name: "GLM 4.7",
        vendor: "Zhipu AI",
        desc: "358B MoE, agent-optimized, 128K out",
    },
    ModelChoice {
        id: "moonshotai/kimi-k2.5",
        name: "Kimi K2.5",
        vendor: "Moonshot",
        desc: "Ultra-long context, multimodal",
    },
    ModelChoice {
        id: "qwen/qwen3-max-2026-01-23",
        name: "Qwen3 Max",
        vendor: "Alibaba",
        desc: "Flagship reasoning, code gen",
    },
    ModelChoice {
        id: "minimaxai/minimax-m2.1",
        name: "MiniMax M2.1",
        vendor: "MiniMax",
        desc: "230B MoE, cheapest ($0.30/M)",
    },
    ModelChoice {
        id: "deepseek-ai/deepseek-v3.2",
        name: "DeepSeek V3.2",
        vendor: "DeepSeek",
        desc: "685B MoE, IOI gold medal",
    },
];

/// Show model selection menu and return the chosen ModelId
fn select_model() -> Result<octo_core::model::ModelId> {
    eprintln!("\x1b[1;36m  Select a model:\x1b[0m\n");
    for (i, m) in MODELS.iter().enumerate() {
        let default_marker = if i == 0 { " \x1b[33m← default\x1b[0m" } else { "" };
        eprintln!(
            "    \x1b[1;33m[{}]\x1b[0m \x1b[1m{:<16}\x1b[0m \x1b[90m({})\x1b[0m  {}{}",
            i + 1,
            m.name,
            m.vendor,
            m.desc,
            default_marker,
        );
    }
    eprintln!();

    eprint!("  \x1b[1mModel \x1b[33m[1]\x1b[0m\x1b[1m:\x1b[0m ");
    io::stderr().flush().ok();

    let input = read_line_lossy()?;
    let input = input.trim();

    let idx = if input.is_empty() {
        0
    } else {
        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= MODELS.len() => n - 1,
            _ => {
                eprintln!("  \x1b[33mInvalid choice, using default.\x1b[0m");
                0
            }
        }
    };

    let chosen = &MODELS[idx];
    eprintln!(
        "\n  \x1b[32m✓\x1b[0m Using \x1b[1;36m{}\x1b[0m \x1b[90m({})\x1b[0m\n",
        chosen.name, chosen.vendor
    );

    Ok(octo_core::model::ModelId(chosen.id.to_string()))
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
    config: octo_core::config::AppConfig,
    db: octo_storage::Database,
    permission_service: std::sync::Arc<dyn octo_core::permission::PermissionService>,
    team_state: std::sync::Arc<std::sync::RwLock<Option<octo_core::team::TeamState>>>,
    resume_session: Option<String>,
    preset_model: Option<octo_core::model::ModelId>,
) -> Result<()> {
    eprintln!();

    // Model selection (skip if preset via --model flag)
    let model_id = match preset_model {
        Some(id) => id,
        None => select_model()?,
    };

    // Resolve display name
    let model_display = octo_core::model::get_model(&model_id)
        .map(|m| m.display_name.clone())
        .unwrap_or_else(|| model_id.0.clone());

    // Banner with model name
    eprintln!(
        "  \x1b[1;35m🐙 OctoCode Agent\x1b[0m v{} \x1b[90m(\x1b[1;36m{}\x1b[90m)\x1b[0m",
        env!("CARGO_PKG_VERSION"),
        model_display,
    );
    eprintln!("  \x1b[90mType your task, /help for commands, Ctrl-D to exit\x1b[0m");
    eprintln!();

    // Get pricing for cost display
    let pricing = octo_core::model::get_model(&model_id).map(|m| {
        super::output::Pricing {
            cost_per_1m_input: m.pricing.cost_per_1m_input,
            cost_per_1m_output: m.pricing.cost_per_1m_output,
        }
    });

    // Build provider + agent
    let provider = octo_providers::create_provider(&config, Some(&model_id))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let tools = octo_tools::create_all_tools(
        permission_service.clone(),
        config.coderlm.server_url.clone(),
        team_state.clone(),
    )
    .await;

    let system_prompt = octo_agent::prompt::build_system_prompt(
        &config.working_dir,
        &config.context_paths,
    );

    let agent = octo_agent::Agent::new(
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
            let s = octo_core::session::Session::new("Interactive session".into());
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
                        let marker = if s.id == session.id { " ←" } else { "" };
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
                "/clear" => {
                    db.messages()
                        .delete_session_messages(&session.id)
                        .await
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    eprintln!("  \x1b[32m✓\x1b[0m Session cleared.\n");
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
