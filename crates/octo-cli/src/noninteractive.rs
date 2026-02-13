use anyhow::Result;
use octo_agent::AgentEvent;

pub async fn run(
    app: super::App,
    prompt: String,
    output_format: super::OutputFormat,
    quiet: bool,
) -> Result<()> {
    // Create session
    let truncated: String = prompt.chars().take(80).collect();
    let title = if truncated.len() < prompt.len() {
        format!("{truncated}...")
    } else {
        truncated
    };
    let session = octo_core::session::Session::new(title);
    app.db
        .sessions()
        .create(&session)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Auto-approve all permissions for non-interactive mode
    app.permission_service.auto_approve_session(&session.id);

    let (mut rx, _cancel) = app.agent.run(session.id.clone(), vec![], prompt);

    match output_format {
        super::OutputFormat::Text => {
            super::output::render_stream(&mut rx, quiet, None).await?;
        }
        super::OutputFormat::Json => {
            let mut full_content = String::new();
            let mut total_usage = octo_core::message::TokenUsage::default();

            while let Some(event) = rx.recv().await {
                match event {
                    AgentEvent::ContentDelta { text } => {
                        full_content.push_str(&text);
                    }
                    AgentEvent::Complete { usage, .. } => {
                        total_usage = usage;
                    }
                    AgentEvent::Error { error } => {
                        let output = serde_json::json!({
                            "error": error,
                        });
                        println!("{}", serde_json::to_string_pretty(&output)?);
                        return Ok(());
                    }
                    _ => {}
                }
            }

            let output = serde_json::json!({
                "content": full_content,
                "usage": {
                    "input_tokens": total_usage.input_tokens,
                    "output_tokens": total_usage.output_tokens,
                },
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}
