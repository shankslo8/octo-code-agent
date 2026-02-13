use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

struct SetupState {
    api_key: String,
    cursor: usize,
    error_message: Option<String>,
    show_key: bool,
    selected_field: usize, // 0 = key input, 1 = save button
}

/// Show API key setup screen. Returns the entered API key or None if cancelled.
pub fn run_setup() -> Result<Option<String>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = SetupState {
        api_key: String::new(),
        cursor: 0,
        error_message: None,
        show_key: false,
        selected_field: 0,
    };

    let result = setup_loop(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn setup_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut SetupState,
) -> Result<Option<String>> {
    loop {
        terminal.draw(|f| setup_ui(f, state))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                // Quit
                (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Esc, _) => {
                    return Ok(None);
                }
                // Toggle show/hide key
                (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                    state.show_key = !state.show_key;
                }
                // Tab to switch fields
                (KeyCode::Tab, _) | (KeyCode::BackTab, _) => {
                    state.selected_field = if state.selected_field == 0 { 1 } else { 0 };
                }
                // Enter
                (KeyCode::Enter, _) => {
                    if state.selected_field == 1 || !state.api_key.is_empty() {
                        // Validate
                        let key = state.api_key.trim().to_string();
                        if key.is_empty() {
                            state.error_message = Some("API key cannot be empty.".into());
                        } else {
                            // Save config and return
                            if let Err(e) = save_api_key(&key) {
                                state.error_message = Some(format!("Failed to save: {e}"));
                            } else {
                                return Ok(Some(key));
                            }
                        }
                    }
                }
                // Text input (only when on key field)
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) if state.selected_field == 0 => {
                    state.api_key.insert(state.cursor, c);
                    state.cursor += c.len_utf8();
                    state.error_message = None;
                }
                (KeyCode::Backspace, _) if state.selected_field == 0 => {
                    if state.cursor > 0 {
                        let prev = state.api_key[..state.cursor]
                            .chars()
                            .last()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        state.cursor -= prev;
                        state.api_key.remove(state.cursor);
                        state.error_message = None;
                    }
                }
                (KeyCode::Left, _) if state.selected_field == 0 => {
                    if state.cursor > 0 {
                        let prev = state.api_key[..state.cursor]
                            .chars()
                            .last()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        state.cursor -= prev;
                    }
                }
                (KeyCode::Right, _) if state.selected_field == 0 => {
                    if state.cursor < state.api_key.len() {
                        let next = state.api_key[state.cursor..]
                            .chars()
                            .next()
                            .map(|c| c.len_utf8())
                            .unwrap_or(1);
                        state.cursor += next;
                    }
                }
                (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    state.cursor = 0;
                }
                (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    state.cursor = state.api_key.len();
                }
                _ => {}
            }
        }
    }
}

fn setup_ui(f: &mut Frame, state: &SetupState) {
    let area = f.area();

    // Dark background (ensures readability on any terminal theme)
    let bg = Paragraph::new("")
        .style(Style::default().bg(Color::Rgb(15, 15, 25)).fg(Color::Rgb(220, 220, 220)));
    f.render_widget(bg, area);

    // Center the setup dialog
    let dialog_width = 70.min(area.width.saturating_sub(4));
    let dialog_height = 20.min(area.height.saturating_sub(2));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Logo + title
            Constraint::Length(3),  // Description
            Constraint::Length(3),  // API key input
            Constraint::Length(2),  // Toggle hint
            Constraint::Length(3),  // Save button
            Constraint::Min(0),    // Error / spacer
        ])
        .split(dialog_area);

    // Logo + Title
    let logo = Text::from(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🐙  OctoCode Setup",
            Style::default()
                .fg(Color::Rgb(140, 80, 255))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Welcome! Enter your Atlas Cloud API key to get started.",
            Style::default().fg(Color::Rgb(180, 180, 180)),
        )),
        Line::from(Span::styled(
            "  Get your key at: https://www.atlascloud.ai",
            Style::default().fg(Color::Cyan),
        )),
    ]);
    f.render_widget(Paragraph::new(logo), chunks[0]);

    // Description
    let desc = Paragraph::new(Line::from(vec![
        Span::styled("  Supports: ", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("GLM-5", Style::default().fg(Color::Yellow)),
        Span::styled(" · ", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("Kimi K2.5", Style::default().fg(Color::Yellow)),
        Span::styled(" · ", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("Qwen3 Max", Style::default().fg(Color::Yellow)),
        Span::styled(" · ", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("MiniMax M2.1", Style::default().fg(Color::Yellow)),
        Span::styled(" · ", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("DeepSeek V3.2", Style::default().fg(Color::Yellow)),
    ]));
    f.render_widget(desc, chunks[1]);

    // API Key input
    let display_key = if state.show_key {
        state.api_key.clone()
    } else if state.api_key.is_empty() {
        String::new()
    } else {
        let visible = 4.min(state.api_key.len());
        let masked = state.api_key.len().saturating_sub(visible);
        format!(
            "{}{}",
            "*".repeat(masked),
            &state.api_key[state.api_key.len() - visible..]
        )
    };

    let input_style = if state.selected_field == 0 {
        Style::default().fg(Color::Rgb(140, 80, 255))
    } else {
        Style::default().fg(Color::Rgb(140, 140, 140))
    };

    let input = Paragraph::new(display_key.as_str())
        .style(Style::default().fg(Color::Rgb(220, 220, 220)).bg(Color::Rgb(30, 30, 45)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(input_style)
                .title(Span::styled(
                    " API Key ",
                    Style::default()
                        .fg(Color::Rgb(140, 80, 255))
                        .add_modifier(Modifier::BOLD),
                )),
        );
    f.render_widget(input, chunks[2]);

    // Cursor
    if state.selected_field == 0 {
        let cursor_x = chunks[2].x + 1 + if state.show_key {
            state.api_key[..state.cursor].chars().count() as u16
        } else {
            display_key.chars().count() as u16
        };
        f.set_cursor_position((cursor_x.min(chunks[2].x + chunks[2].width - 2), chunks[2].y + 1));
    }

    // Toggle hint
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("  Ctrl+V", Style::default().fg(Color::Cyan)),
        Span::styled(
            if state.show_key { ": hide key" } else { ": show key" },
            Style::default().fg(Color::Rgb(140, 140, 140)),
        ),
        Span::styled("  │  Tab", Style::default().fg(Color::Cyan)),
        Span::styled(": switch field", Style::default().fg(Color::Rgb(140, 140, 140))),
        Span::styled("  │  Esc", Style::default().fg(Color::Cyan)),
        Span::styled(": cancel", Style::default().fg(Color::Rgb(140, 140, 140))),
    ]));
    f.render_widget(hint, chunks[3]);

    // Save button
    let button_style = if state.selected_field == 1 {
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(100, 60, 200))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Rgb(140, 80, 255))
            .add_modifier(Modifier::BOLD)
    };

    let button = Paragraph::new(Line::from(Span::styled(
        "  [ Save & Continue ]  ",
        button_style,
    )))
    .block(Block::default().borders(Borders::NONE));
    f.render_widget(button, chunks[4]);

    // Error message
    if let Some(err) = &state.error_message {
        let error = Paragraph::new(Line::from(Span::styled(
            format!("  ⚠ {err}"),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(error, chunks[5]);
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

fn save_api_key(key: &str) -> Result<()> {
    // Save to global config directory
    let config_dir = if cfg!(target_os = "macos") {
        dirs::home_dir().map(|h| h.join("Library/Application Support/octo-code"))
    } else {
        dirs::config_dir().map(|c| c.join("octo-code"))
    };

    if let Some(dir) = config_dir {
        std::fs::create_dir_all(&dir)?;
        let config_path = dir.join("config.json");

        // Read existing config or create new
        let mut config: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        config["api_key"] = serde_json::Value::String(key.to_string());
        config["base_url"] = serde_json::Value::String("https://api.atlascloud.ai".into());

        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;
    }

    Ok(())
}
