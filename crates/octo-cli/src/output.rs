use anyhow::Result;
use octo_agent::AgentEvent;
use std::io::{self, Write};
use tokio::sync::mpsc;

/// Pricing info for cost display
#[derive(Clone, Copy)]
pub struct Pricing {
    pub cost_per_1m_input: f64,
    pub cost_per_1m_output: f64,
}

impl Default for Pricing {
    fn default() -> Self {
        Self {
            cost_per_1m_input: 0.0,
            cost_per_1m_output: 0.0,
        }
    }
}

const USD_TO_KRW: f64 = 1450.0;

pub async fn render_stream(
    rx: &mut mpsc::Receiver<AgentEvent>,
    quiet: bool,
    pricing: Option<Pricing>,
) -> Result<()> {
    let mut first_content = true;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::Started { .. } => {
                if !quiet {
                    eprint!("\x1b[90mThinking...\x1b[0m");
                    io::stderr().flush().ok();
                }
            }
            AgentEvent::ContentDelta { text } => {
                if first_content {
                    // Clear "Thinking..."
                    if !quiet {
                        eprint!("\r\x1b[K");
                    }
                    first_content = false;
                }
                print!("{text}");
                io::stdout().flush().ok();
            }
            AgentEvent::ThinkingDelta { text } => {
                if !quiet {
                    eprint!("\x1b[3;90m{text}\x1b[0m");
                    io::stderr().flush().ok();
                }
            }
            AgentEvent::ToolCallStart { name, .. } => {
                first_content = true;
                eprintln!("\n\x1b[36;1m[tool: {name}]\x1b[0m");
            }
            AgentEvent::ToolResult {
                tool_name,
                result,
                is_error,
                ..
            } => {
                if is_error {
                    eprintln!("\x1b[31;1m[error: {tool_name}]\x1b[0m {result}");
                } else if !quiet {
                    let display = if result.len() > 500 {
                        let boundary = result.floor_char_boundary(500);
                        format!("{}... ({} chars)", &result[..boundary], result.len())
                    } else {
                        result
                    };
                    eprintln!("\x1b[90m{display}\x1b[0m");
                }
            }
            AgentEvent::Complete { usage, .. } => {
                if !quiet {
                    let input = usage.input_tokens;
                    let output = usage.output_tokens;
                    let total = input + output;

                    if let Some(p) = pricing {
                        let cost_usd = (input as f64 / 1_000_000.0) * p.cost_per_1m_input
                            + (output as f64 / 1_000_000.0) * p.cost_per_1m_output;
                        let cost_krw = cost_usd * USD_TO_KRW;

                        eprintln!(
                            "\n\x1b[90m[토큰] 입력 {} / 출력 {} / 합계 {}\x1b[0m",
                            format_tokens(input),
                            format_tokens(output),
                            format_tokens(total),
                        );
                        eprintln!(
                            "\x1b[90m[비용] ${:.4} (\x1b[33m{}\x1b[90m)\x1b[0m",
                            cost_usd,
                            format_krw(cost_krw),
                        );
                    } else {
                        eprintln!(
                            "\n\x1b[90m[토큰] 입력 {} / 출력 {} / 합계 {}\x1b[0m",
                            format_tokens(input),
                            format_tokens(output),
                            format_tokens(total),
                        );
                    }
                }
            }
            AgentEvent::Error { error } => {
                eprintln!("\n\x1b[31;1m[error]\x1b[0m {error}");
            }
            _ => {}
        }
    }

    println!();
    Ok(())
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

fn format_krw(krw: f64) -> String {
    if krw < 1.0 {
        format!("{:.1}원", krw)
    } else if krw < 1000.0 {
        format!("{:.0}원", krw)
    } else {
        format!("{:.0}원", krw)
    }
}
