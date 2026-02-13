use anyhow::Result;
use std::io::{self, Write};

pub async fn run(app: super::App, resume_session: Option<String>) -> Result<()> {
    println!("\x1b[1mocto-code\x1b[0m v{}", env!("CARGO_PKG_VERSION"));
    println!("Model: \x1b[36m{}\x1b[0m", app.agent.model_name());
    println!("Type \x1b[33m/help\x1b[0m for commands, \x1b[33mCtrl-D\x1b[0m to exit.\n");

    let session = match resume_session {
        Some(id) => app
            .db
            .sessions()
            .get(&id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => {
            let s = crate::core::session::Session::new("New session".into());
            app.db
                .sessions()
                .create(&s)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            s
        }
    };

    loop {
        eprint!("\x1b[32;1mocto>\x1b[0m ");
        io::stderr().flush().ok();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // EOF (Ctrl-D)
                println!("\nGoodbye!");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Input error: {e}");
                break;
            }
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        // Handle slash commands
        if input.starts_with('/') {
            match handle_command(&input, &app, &session).await {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    eprintln!("\x1b[31mCommand error: {e}\x1b[0m");
                    continue;
                }
            }
        }

        // Load conversation history
        let messages = app
            .db
            .messages()
            .list(&session.id)
            .await
            .unwrap_or_default();

        // Run the agent
        let (mut rx, _cancel) = app.agent.run(session.id.clone(), messages, input);

        // Render streaming output
        super::output::render_stream(&mut rx, false, None).await?;
    }

    Ok(())
}

async fn handle_command(
    input: &str,
    app: &super::App,
    session: &crate::core::session::Session,
) -> Result<bool> {
    match input {
        "/help" | "/h" => {
            println!("\x1b[1mCommands:\x1b[0m");
            println!("  /help       Show this help");
            println!("  /sessions   List sessions");
            println!("  /clear      Clear current session messages");
            println!("  /model      Show current model");
            println!("  /cost       Show token usage & cost");
            println!("  /exit       Exit");
            Ok(true)
        }
        "/exit" | "/quit" | "/q" => {
            println!("Goodbye!");
            Ok(false)
        }
        "/sessions" | "/s" => {
            let sessions = app
                .db
                .sessions()
                .list()
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if sessions.is_empty() {
                println!("No sessions.");
            } else {
                for s in sessions {
                    let marker = if s.id == session.id { " *" } else { "" };
                    println!(
                        "  \x1b[90m{}\x1b[0m  {}{}  ({} msgs, ${:.4})",
                        &s.id[..8],
                        s.title,
                        marker,
                        s.message_count,
                        s.cost
                    );
                }
            }
            Ok(true)
        }
        "/model" => {
            println!("Model: {} ({})", app.agent.model_name(), app.agent.model_id());
            Ok(true)
        }
        "/cost" => {
            let s = app
                .db
                .sessions()
                .get(&session.id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "Tokens: {} in / {} out | Cost: ${:.4}",
                s.prompt_tokens, s.completion_tokens, s.cost
            );
            Ok(true)
        }
        "/clear" => {
            app.db
                .messages()
                .delete_session_messages(&session.id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Session cleared.");
            Ok(true)
        }
        _ => {
            eprintln!("Unknown command: {input}. Type /help for available commands.");
            Ok(true)
        }
    }
}
