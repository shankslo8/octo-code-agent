mod interactive;
mod noninteractive;
mod output;
mod permission_ui;
mod repl;
mod setup;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "octo-code", version, about = "AI coding assistant for the terminal")]
struct Cli {
    /// Non-interactive mode: provide a prompt directly
    #[arg(short, long)]
    prompt: Option<String>,

    /// Working directory
    #[arg(short = 'c', long = "cwd")]
    working_dir: Option<PathBuf>,

    /// Output format for non-interactive mode
    #[arg(short = 'f', long, default_value = "text")]
    output_format: OutputFormat,

    /// Suppress progress indicators
    #[arg(short, long)]
    quiet: bool,

    /// Use simple REPL mode instead of default interactive
    #[arg(long)]
    repl: bool,

    /// Use TUI mode (ratatui)
    #[arg(long)]
    tui: bool,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Resume a previous session by ID
    #[arg(long)]
    session: Option<String>,

    /// Model to use (overrides config)
    #[arg(short, long)]
    model: Option<String>,

    /// Team name (for spawned team agents)
    #[arg(long, env = "OCTO_TEAM_NAME")]
    team_name: Option<String>,

    /// Agent name within team
    #[arg(long, env = "OCTO_AGENT_NAME")]
    agent_name: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

pub struct App {
    pub agent: octo_agent::Agent,
    pub db: octo_storage::Database,
    pub config: octo_core::config::AppConfig,
    pub permission_service: Arc<dyn octo_core::permission::PermissionService>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("warn")
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let mut config = octo_core::config::load_config(cli.working_dir.clone())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // If no API key, show setup screen (interactive modes only)
    if !config.has_api_key() && cli.prompt.is_none() {
        match setup::run_setup()? {
            Some(key) => {
                config.api_key = Some(key);
            }
            None => {
                eprintln!("Setup cancelled. Set ATLAS_API_KEY env var or run again to configure.");
                return Ok(());
            }
        }
    }

    if !config.has_api_key() {
        anyhow::bail!(
            "No API key found. Set ATLAS_API_KEY env var, or run octo-code interactively to configure."
        );
    }

    let db = octo_storage::Database::open(&config)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    db.run_migrations()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let model_id = cli.model.map(octo_core::model::ModelId);

    // Build team state from CLI args / env vars
    let team_state: Arc<RwLock<Option<octo_core::team::TeamState>>> = Arc::new(RwLock::new(
        match (&cli.team_name, &cli.agent_name) {
            (Some(team), Some(name)) => Some(octo_core::team::TeamState::new(
                team.clone(),
                name.clone(),
                false,
            )),
            _ => None,
        },
    ));

    if let Some(prompt) = cli.prompt {
        // Non-interactive: use CLI permission service
        let app =
            build_app_with_cli_permissions(config, db, model_id.as_ref(), team_state.clone()).await?;
        noninteractive::run(app, prompt, cli.output_format, cli.quiet).await
    } else if cli.repl {
        // REPL: use CLI permission service
        let app =
            build_app_with_cli_permissions(config, db, model_id.as_ref(), team_state.clone()).await?;
        repl::run(app, cli.session).await
    } else if cli.tui {
        // TUI: use TUI permission service (channel-based overlays)
        let (perm_service, perm_rx) = tui::permission::TuiPermissionService::new();
        let permission_service: Arc<dyn octo_core::permission::PermissionService> =
            Arc::new(perm_service);
        let app = build_app(config, db, model_id.as_ref(), permission_service, team_state).await?;
        tui::run(app, cli.session, perm_rx).await
    } else {
        // Default: simple interactive mode with model selection
        let permission_service: Arc<dyn octo_core::permission::PermissionService> =
            Arc::new(permission_ui::CliPermissionService::new());
        interactive::run(config, db, permission_service, team_state, cli.session, model_id).await
    }
}

async fn build_app_with_cli_permissions(
    config: octo_core::config::AppConfig,
    db: octo_storage::Database,
    model_id: Option<&octo_core::model::ModelId>,
    team_state: Arc<RwLock<Option<octo_core::team::TeamState>>>,
) -> Result<App> {
    let permission_service: Arc<dyn octo_core::permission::PermissionService> =
        Arc::new(permission_ui::CliPermissionService::new());
    build_app(config, db, model_id, permission_service, team_state).await
}

async fn build_app(
    config: octo_core::config::AppConfig,
    db: octo_storage::Database,
    model_id: Option<&octo_core::model::ModelId>,
    permission_service: Arc<dyn octo_core::permission::PermissionService>,
    team_state: Arc<RwLock<Option<octo_core::team::TeamState>>>,
) -> Result<App> {
    let provider = octo_providers::create_provider(&config, model_id)
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

    Ok(App {
        agent,
        db,
        config,
        permission_service,
    })
}
