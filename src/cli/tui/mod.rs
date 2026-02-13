pub mod dialogs;
pub mod markdown;
pub mod permission;

use anyhow::Result;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::StreamExt;

use crate::agent::AgentEvent;
use crate::core::permission::PermissionDecision;

use dialogs::*;
use permission::PermissionReq;

// ─── Colors ──────────────────────────────────────────

const BG: Color = Color::Rgb(15, 15, 25);
const SURFACE: Color = Color::Rgb(22, 22, 36);
const BORDER: Color = Color::Rgb(50, 50, 70);
const BORDER_ACTIVE: Color = Color::Rgb(100, 60, 200);
const TEXT: Color = Color::Rgb(210, 210, 225);
const DIM: Color = Color::Rgb(90, 90, 110);
const ACCENT: Color = Color::Rgb(140, 80, 255);
const GREEN: Color = Color::Rgb(80, 200, 120);
const YELLOW: Color = Color::Rgb(230, 190, 60);
const CYAN: Color = Color::Rgb(80, 200, 220);
const RED: Color = Color::Rgb(230, 80, 80);
const TOOL_FG: Color = Color::Rgb(200, 160, 60);

// ─── Types ───────────────────────────────────────────

#[derive(Clone)]
struct ChatMessage {
    role: ChatRole,
    content: String,
}

#[derive(Clone)]
enum ChatRole {
    User,
    Assistant,
    Tool(String),
    Error,
    System,
}

enum ActiveDialog {
    Model(ModelDialog),
    Session(SessionDialog),
    Command(CommandDialog),
    Help,
}

struct PendingPermission {
    dialog: PermissionDialog,
    responder: oneshot::Sender<PermissionDecision>,
}

// ─── App State ───────────────────────────────────────

struct TuiApp {
    app: super::App,
    session: crate::core::session::Session,
    messages: Vec<ChatMessage>,
    input: String,
    input_cursor: usize,
    scroll_offset: u16,
    total_content_height: u16,
    is_streaming: bool,
    current_stream_text: String,
    model_name: String,
    model_id: String,
    total_tokens: (u64, u64),
    total_cost: f64,
    should_quit: bool,
    status_message: String,
    active_dialog: Option<ActiveDialog>,
    pending_permission: Option<PendingPermission>,
    perm_rx: mpsc::Receiver<PermissionReq>,
    agent_rx: Option<mpsc::Receiver<AgentEvent>>,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
    show_sidebar: bool,
    changed_files: Vec<String>,
    tick: u64,
}

impl TuiApp {
    fn new(
        app: super::App,
        session: crate::core::session::Session,
        perm_rx: mpsc::Receiver<PermissionReq>,
    ) -> Self {
        let model_name = app.agent.model_name().to_string();
        let model_id = app.agent.model_id().to_string();
        Self {
            app,
            session,
            messages: vec![],
            input: String::new(),
            input_cursor: 0,
            scroll_offset: 0,
            total_content_height: 0,
            is_streaming: false,
            current_stream_text: String::new(),
            model_name,
            model_id,
            total_tokens: (0, 0),
            total_cost: 0.0,
            should_quit: false,
            status_message: "Ready".into(),
            active_dialog: None,
            pending_permission: None,
            perm_rx,
            agent_rx: None,
            cancel_token: None,
            show_sidebar: false,
            changed_files: Vec::new(),
            tick: 0,
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.total_content_height;
    }
}

// ─── Entry Point ─────────────────────────────────────

pub async fn run(
    app: super::App,
    resume_session: Option<String>,
    perm_rx: mpsc::Receiver<PermissionReq>,
) -> Result<()> {
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

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut tui_app = TuiApp::new(app, session, perm_rx);
    let result = run_event_loop(&mut terminal, &mut tui_app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

// ─── Event Loop ──────────────────────────────────────

async fn recv_agent(rx: &mut Option<mpsc::Receiver<AgentEvent>>) -> Option<AgentEvent> {
    match rx {
        Some(rx) => rx.recv().await,
        None => std::future::pending().await,
    }
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
) -> Result<()> {
    let mut event_reader = EventStream::new();
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(80));

    loop {
        terminal.draw(|f| ui(f, app))?;
        if app.should_quit {
            return Ok(());
        }

        tokio::select! {
            biased;

            Some(event) = recv_agent(&mut app.agent_rx) => {
                handle_agent_event(app, event).await;
            }

            Some(perm) = app.perm_rx.recv(), if app.pending_permission.is_none() => {
                app.pending_permission = Some(PendingPermission {
                    dialog: PermissionDialog::new(&perm.request),
                    responder: perm.responder,
                });
            }

            Some(Ok(event)) = event_reader.next() => {
                if let Event::Key(key) = event {
                    handle_key_event(app, key).await;
                }
            }

            _ = tick_interval.tick() => {
                app.tick += 1;
            }
        }
    }
}

// ─── Agent Events ────────────────────────────────────

async fn handle_agent_event(app: &mut TuiApp, event: AgentEvent) {
    match event {
        AgentEvent::ContentDelta { text } => {
            app.current_stream_text.push_str(&text);
            app.status_message = "Generating...".into();
            app.scroll_to_bottom();
        }
        AgentEvent::ThinkingDelta { .. } => {
            app.status_message = "Reasoning...".into();
        }
        AgentEvent::ToolCallStart { name, .. } => {
            if !app.current_stream_text.is_empty() {
                app.messages.push(ChatMessage {
                    role: ChatRole::Assistant,
                    content: std::mem::take(&mut app.current_stream_text),
                });
            }
            app.status_message = format!("Running {name}...");
            app.scroll_to_bottom();
        }
        AgentEvent::ToolResult {
            tool_name,
            result,
            is_error,
            ..
        } => {
            if !is_error {
                if let Some(path) = extract_file_path(&tool_name, &result) {
                    if !app.changed_files.contains(&path) {
                        app.changed_files.push(path);
                        if !app.show_sidebar {
                            app.show_sidebar = true;
                        }
                    }
                }
            }
            let truncated = if result.chars().count() > 500 {
                let s: String = result.chars().take(500).collect();
                format!("{s}... ({} chars)", result.len())
            } else {
                result
            };
            let content = if is_error {
                format!("ERROR: {truncated}")
            } else {
                truncated
            };
            app.messages.push(ChatMessage {
                role: ChatRole::Tool(tool_name),
                content,
            });
            app.scroll_to_bottom();
        }
        AgentEvent::Complete { usage, .. } => {
            if !app.current_stream_text.is_empty() {
                app.messages.push(ChatMessage {
                    role: ChatRole::Assistant,
                    content: std::mem::take(&mut app.current_stream_text),
                });
            }
            app.total_tokens.0 += usage.input_tokens;
            app.total_tokens.1 += usage.output_tokens;
            if let Some(m) = crate::core::model::get_model(app.app.agent.model_id()) {
                app.total_cost += m.calculate_cost(usage.input_tokens, usage.output_tokens);
            }
            app.status_message = "Ready".into();
            app.is_streaming = false;
            app.agent_rx = None;
            app.cancel_token = None;
            app.scroll_to_bottom();
            let _ = persist_session(&app.app, &app.session, &app.total_tokens, app.total_cost).await;
        }
        AgentEvent::Error { error } => {
            app.messages.push(ChatMessage {
                role: ChatRole::Error,
                content: error,
            });
            app.status_message = "Error".into();
            app.is_streaming = false;
            app.agent_rx = None;
            app.cancel_token = None;
            app.scroll_to_bottom();
        }
        _ => {}
    }
}

// ─── Key Handling ────────────────────────────────────

async fn handle_key_event(app: &mut TuiApp, key: crossterm::event::KeyEvent) {
    // Permission dialog
    if let Some(perm) = &mut app.pending_permission {
        match perm.dialog.handle_key(key) {
            PermissionDialogAction::Allow => {
                let p = app.pending_permission.take().unwrap();
                let _ = p.responder.send(PermissionDecision::Allow);
            }
            PermissionDialogAction::AlwaysAllow => {
                let p = app.pending_permission.take().unwrap();
                let _ = p.responder.send(PermissionDecision::AllowPersistent);
            }
            PermissionDialogAction::Deny => {
                let p = app.pending_permission.take().unwrap();
                let _ = p.responder.send(PermissionDecision::Deny);
            }
            PermissionDialogAction::None => {}
        }
        return;
    }

    // Active dialog
    if let Some(dialog) = &mut app.active_dialog {
        match dialog {
            ActiveDialog::Model(d) => match d.handle_key(key) {
                ModelDialogAction::Close => app.active_dialog = None,
                ModelDialogAction::Select(id) => {
                    switch_model(app, &id);
                    app.active_dialog = None;
                }
                _ => {}
            },
            ActiveDialog::Session(d) => match d.handle_key(key) {
                SessionDialogAction::Close => app.active_dialog = None,
                SessionDialogAction::Select(id) => {
                    switch_session(app, &id).await;
                    app.active_dialog = None;
                }
                SessionDialogAction::New => {
                    create_new_session(app).await;
                    app.active_dialog = None;
                }
                SessionDialogAction::Delete(id) => {
                    let _ = app.app.db.sessions().delete(&id).await;
                    let _ = app.app.db.messages().delete_session_messages(&id).await;
                    if let Some(ActiveDialog::Session(d)) = &mut app.active_dialog {
                        d.sessions.retain(|s| s.id != id);
                        d.selected = d.selected.min(d.sessions.len().saturating_sub(1));
                    }
                }
                _ => {}
            },
            ActiveDialog::Command(d) => match d.handle_key(key) {
                CommandDialogAction::Close => app.active_dialog = None,
                CommandDialogAction::Execute(cmd) => {
                    app.active_dialog = None;
                    handle_command(app, &cmd).await;
                }
                _ => {}
            },
            ActiveDialog::Help => {
                app.active_dialog = None;
            }
        }
        return;
    }

    // Global keys
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            if app.is_streaming {
                if let Some(c) = &app.cancel_token { c.cancel(); }
                app.is_streaming = false;
                app.agent_rx = None;
                app.cancel_token = None;
                app.status_message = "Cancelled".into();
                app.messages.push(ChatMessage { role: ChatRole::System, content: "(cancelled)".into() });
                app.scroll_to_bottom();
            } else {
                app.should_quit = true;
            }
        }
        (KeyCode::Char('o'), KeyModifiers::CONTROL) if !app.is_streaming => {
            app.active_dialog = Some(ActiveDialog::Model(ModelDialog::new(&app.model_id)));
        }
        (KeyCode::Char('s'), KeyModifiers::CONTROL) if !app.is_streaming => {
            let sessions = app.app.db.sessions().list().await.unwrap_or_default();
            app.active_dialog = Some(ActiveDialog::Session(SessionDialog::new(sessions, &app.session.id)));
        }
        (KeyCode::Char('k'), KeyModifiers::CONTROL) if !app.is_streaming => {
            app.active_dialog = Some(ActiveDialog::Command(CommandDialog::new()));
        }
        (KeyCode::F(1), _) if !app.is_streaming => {
            app.active_dialog = Some(ActiveDialog::Help);
        }
        (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
            app.show_sidebar = !app.show_sidebar;
        }
        (KeyCode::Char('l'), KeyModifiers::CONTROL) if !app.is_streaming => {
            compact_conversation(app).await;
        }
        // Submit
        (KeyCode::Enter, KeyModifiers::NONE) if !app.is_streaming => {
            if !app.input.trim().is_empty() {
                let input = app.input.trim().to_string();
                app.input.clear();
                app.input_cursor = 0;
                if input.starts_with('/') {
                    handle_command(app, &input).await;
                } else {
                    submit_message(app, input).await;
                }
            }
        }
        // Text editing
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) if !app.is_streaming => {
            app.input.insert(app.input_cursor, c);
            app.input_cursor += c.len_utf8();
        }
        (KeyCode::Backspace, _) if !app.is_streaming && app.input_cursor > 0 => {
            let prev = app.input[..app.input_cursor].chars().last().map(|c| c.len_utf8()).unwrap_or(1);
            app.input_cursor -= prev;
            app.input.remove(app.input_cursor);
        }
        (KeyCode::Delete, _) if !app.is_streaming && app.input_cursor < app.input.len() => {
            app.input.remove(app.input_cursor);
        }
        (KeyCode::Left, _) if !app.is_streaming && app.input_cursor > 0 => {
            let prev = app.input[..app.input_cursor].chars().last().map(|c| c.len_utf8()).unwrap_or(1);
            app.input_cursor -= prev;
        }
        (KeyCode::Right, _) if !app.is_streaming && app.input_cursor < app.input.len() => {
            let next = app.input[app.input_cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            app.input_cursor += next;
        }
        (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => { app.input_cursor = 0; }
        (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => { app.input_cursor = app.input.len(); }
        (KeyCode::Up, _) => { app.scroll_offset = app.scroll_offset.saturating_sub(3); }
        (KeyCode::Down, _) => { app.scroll_offset = app.scroll_offset.saturating_add(3); }
        (KeyCode::PageUp, _) => { app.scroll_offset = app.scroll_offset.saturating_sub(20); }
        (KeyCode::PageDown, _) => { app.scroll_offset = app.scroll_offset.saturating_add(20); }
        _ => {}
    }
}

// ─── Commands ────────────────────────────────────────

async fn handle_command(app: &mut TuiApp, input: &str) {
    let cmd = input.split_whitespace().next().unwrap_or(input);
    match cmd {
        "/help" | "/h" => { app.active_dialog = Some(ActiveDialog::Help); }
        "/exit" | "/quit" | "/q" => { app.should_quit = true; }
        "/model" | "/m" => {
            app.active_dialog = Some(ActiveDialog::Model(ModelDialog::new(&app.model_id)));
        }
        "/session" | "/s" => {
            let sessions = app.app.db.sessions().list().await.unwrap_or_default();
            app.active_dialog = Some(ActiveDialog::Session(SessionDialog::new(sessions, &app.session.id)));
        }
        "/cost" => {
            app.messages.push(ChatMessage {
                role: ChatRole::System,
                content: format!("Tokens: {} in / {} out | Cost: ${:.4}", app.total_tokens.0, app.total_tokens.1, app.total_cost),
            });
            app.scroll_to_bottom();
        }
        "/clear" => {
            app.messages.clear();
            app.messages.push(ChatMessage { role: ChatRole::System, content: "Chat cleared.".into() });
            let _ = app.app.db.messages().delete_session_messages(&app.session.id).await;
        }
        "/compact" => { compact_conversation(app).await; }
        "/sidebar" => { app.show_sidebar = !app.show_sidebar; }
        _ => {
            app.messages.push(ChatMessage {
                role: ChatRole::System,
                content: format!("Unknown: {cmd}. Ctrl+K for commands."),
            });
            app.scroll_to_bottom();
        }
    }
}

// ─── Actions ─────────────────────────────────────────

async fn submit_message(app: &mut TuiApp, input: String) {
    app.messages.push(ChatMessage { role: ChatRole::User, content: input.clone() });
    app.scroll_to_bottom();
    app.is_streaming = true;
    app.current_stream_text.clear();
    app.status_message = "Thinking...".into();
    let messages = app.app.db.messages().list(&app.session.id).await.unwrap_or_default();
    let (rx, cancel) = app.app.agent.run(app.session.id.clone(), messages, input);
    app.agent_rx = Some(rx);
    app.cancel_token = Some(cancel);
}

fn switch_model(app: &mut TuiApp, model_id: &crate::core::model::ModelId) {
    match crate::providers::create_provider(&app.app.config, Some(model_id)) {
        Ok(p) => {
            app.app.agent.switch_provider(p);
            app.model_name = app.app.agent.model_name().to_string();
            app.model_id = app.app.agent.model_id().to_string();
            app.messages.push(ChatMessage {
                role: ChatRole::System,
                content: format!("Model: {}", app.model_name),
            });
        }
        Err(e) => {
            app.messages.push(ChatMessage { role: ChatRole::Error, content: format!("{e}") });
        }
    }
    app.scroll_to_bottom();
}

async fn switch_session(app: &mut TuiApp, session_id: &str) {
    if session_id == app.session.id { return; }
    match app.app.db.sessions().get(session_id).await {
        Ok(session) => {
            app.session = session;
            app.messages.clear();
            app.changed_files.clear();
            if let Ok(db_msgs) = app.app.db.messages().list(session_id).await {
                for msg in &db_msgs {
                    let text = msg.text_content();
                    if text.is_empty() { continue; }
                    let role = match msg.role {
                        crate::core::message::MessageRole::User => ChatRole::User,
                        crate::core::message::MessageRole::Assistant => ChatRole::Assistant,
                        crate::core::message::MessageRole::System => ChatRole::System,
                        crate::core::message::MessageRole::Tool => ChatRole::Tool("tool".into()),
                    };
                    app.messages.push(ChatMessage { role, content: text.to_string() });
                }
            }
            app.scroll_to_bottom();
        }
        Err(e) => {
            app.messages.push(ChatMessage { role: ChatRole::Error, content: format!("{e}") });
        }
    }
}

async fn create_new_session(app: &mut TuiApp) {
    let s = crate::core::session::Session::new("New session".into());
    if let Err(e) = app.app.db.sessions().create(&s).await {
        app.messages.push(ChatMessage { role: ChatRole::Error, content: format!("{e}") });
        return;
    }
    app.session = s;
    app.messages.clear();
    app.changed_files.clear();
    app.total_tokens = (0, 0);
    app.total_cost = 0.0;
}

async fn compact_conversation(app: &mut TuiApp) {
    if app.messages.len() <= 5 {
        app.messages.push(ChatMessage { role: ChatRole::System, content: "Not enough to compact.".into() });
        app.scroll_to_bottom();
        return;
    }
    let keep = 4;
    let to_compact = app.messages.len().saturating_sub(keep);
    let mut parts = Vec::new();
    for msg in app.messages.drain(..to_compact) {
        let prefix = match &msg.role {
            ChatRole::User => "User: ",
            ChatRole::Assistant => "Assistant: ",
            ChatRole::Tool(n) => { parts.push(format!("Tool [{n}]")); continue; }
            _ => continue,
        };
        let short: String = msg.content.chars().take(150).collect();
        parts.push(format!("{prefix}{short}"));
    }
    let summary = format!("[Compacted {} messages]\n{}", to_compact, parts.join("\n"));
    app.messages.insert(0, ChatMessage { role: ChatRole::System, content: summary });
    let _ = app.app.db.messages().delete_session_messages(&app.session.id).await;
    app.status_message = format!("Compacted {to_compact} messages");
    app.scroll_to_bottom();
}

async fn persist_session(
    inner: &super::App,
    session: &crate::core::session::Session,
    tokens: &(u64, u64),
    cost: f64,
) -> Result<()> {
    let mut s = session.clone();
    s.prompt_tokens = tokens.0;
    s.completion_tokens = tokens.1;
    s.cost = cost;
    s.message_count += 1;
    inner.db.sessions().update(&s).await.map_err(|e| anyhow::anyhow!("{e}"))
}

fn extract_file_path(tool_name: &str, result: &str) -> Option<String> {
    match tool_name {
        "write" => result.split(" to ").nth(1).map(|s| s.trim().to_string()),
        "edit" => result.strip_prefix("Edited ").and_then(|s| s.split('.').next()).map(|s| s.trim().to_string()),
        _ => None,
    }
}

// ─── UI Rendering ────────────────────────────────────

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let area = f.area();
    f.render_widget(Paragraph::new("").style(Style::default().bg(BG)), area);

    let (main_area, sidebar_area) = if app.show_sidebar && area.width > 60 {
        let c = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(40), Constraint::Length(24)])
            .split(area);
        (c[0], Some(c[1]))
    } else {
        (area, None)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header
            Constraint::Min(5),    // Chat
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status
        ])
        .split(main_area);

    render_header(f, app, chunks[0]);
    render_chat(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_status(f, app, chunks[3]);
    if let Some(sb) = sidebar_area { render_sidebar(f, app, sb); }

    // Overlays
    if let Some(perm) = &app.pending_permission {
        perm.dialog.render(f, area);
    } else if let Some(d) = &app.active_dialog {
        match d {
            ActiveDialog::Model(d) => d.render(f, area),
            ActiveDialog::Session(d) => d.render(f, area),
            ActiveDialog::Command(d) => d.render(f, area),
            ActiveDialog::Help => HelpDialog::render(f, area),
        }
    }
}

fn render_header(f: &mut Frame, app: &TuiApp, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" \u{1F419} ", Style::default().fg(ACCENT)),
        Span::styled(
            "OctoCode",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  \u{2502}  ", Style::default().fg(BORDER)),
        Span::styled(&app.model_name, Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("  \u{2502}  {}", &app.session.id[..8.min(app.session.id.len())]),
            Style::default().fg(DIM),
        ),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(SURFACE)),
        area,
    );
}

fn render_chat(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        match &msg.role {
            ChatRole::User => {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  \u{25B6} ", Style::default().fg(GREEN)),
                    Span::styled("You", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
                ]));
                for l in msg.content.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("    {l}"),
                        Style::default().fg(TEXT),
                    )));
                }
            }
            ChatRole::Assistant => {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("  \u{2728} ", Style::default().fg(ACCENT)),
                    Span::styled(
                        "Assistant",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                ]));
                let md_lines = markdown::render_markdown(&msg.content, "    ");
                lines.extend(md_lines);
            }
            ChatRole::Tool(name) => {
                lines.push(Line::from(vec![
                    Span::styled("    \u{2502} ", Style::default().fg(BORDER)),
                    Span::styled(
                        format!("\u{2699} {name}"),
                        Style::default().fg(TOOL_FG),
                    ),
                ]));
                for l in msg.content.lines().take(8) {
                    lines.push(Line::from(Span::styled(
                        format!("    \u{2502}   {l}"),
                        Style::default().fg(DIM),
                    )));
                }
                if msg.content.lines().count() > 8 {
                    lines.push(Line::from(Span::styled(
                        format!("    \u{2502}   ... ({} lines)", msg.content.lines().count()),
                        Style::default().fg(DIM),
                    )));
                }
            }
            ChatRole::Error => {
                lines.push(Line::from(Span::styled(
                    format!("  \u{2716} {}", msg.content),
                    Style::default().fg(RED).add_modifier(Modifier::BOLD),
                )));
            }
            ChatRole::System => {
                for l in msg.content.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  \u{2500} {l}"),
                        Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
                    )));
                }
            }
        }
    }

    // Streaming
    if app.is_streaming && !app.current_stream_text.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  \u{2728} ", Style::default().fg(ACCENT)),
            Span::styled("Assistant", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        ]));
        let md_lines = markdown::render_markdown(&app.current_stream_text, "    ");
        lines.extend(md_lines);
        lines.push(Line::from(Span::styled(
            "    \u{2588}",
            Style::default().fg(ACCENT),
        )));
    } else if app.is_streaming {
        let spin = markdown::spinner(app.tick);
        lines.push(Line::from(Span::styled(
            format!("  {spin} {}", app.status_message),
            Style::default().fg(YELLOW),
        )));
    }

    let total = lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    app.total_content_height = total.saturating_sub(visible);
    if app.is_streaming { app.scroll_offset = app.total_content_height; }
    app.scroll_offset = app.scroll_offset.min(app.total_content_height);

    let chat = Paragraph::new(Text::from(lines))
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(BORDER)));
    f.render_widget(chat, area);

    if app.total_content_height > 0 {
        let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(Style::default().fg(ACCENT))
            .track_style(Style::default().fg(BORDER));
        let mut state = ScrollbarState::new(app.total_content_height as usize)
            .position(app.scroll_offset as usize);
        f.render_stateful_widget(
            sb,
            area.inner(ratatui::layout::Margin { vertical: 1, horizontal: 0 }),
            &mut state,
        );
    }
}

fn render_input(f: &mut Frame, app: &TuiApp, area: Rect) {
    let (border, text_style) = if app.is_streaming {
        (BORDER, Style::default().fg(DIM))
    } else {
        (BORDER_ACTIVE, Style::default().fg(TEXT))
    };

    let title = if app.is_streaming {
        format!(" {} Streaming... Ctrl+C cancel ", markdown::spinner(app.tick))
    } else {
        " Message \u{2502} Enter send \u{2502} Ctrl+K cmds ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(Span::styled(title, Style::default().fg(ACCENT)));

    let input = Paragraph::new(app.input.as_str()).style(text_style).block(block);
    f.render_widget(input, area);

    if !app.is_streaming && app.active_dialog.is_none() && app.pending_permission.is_none() {
        let cx = area.x + 1 + app.input[..app.input_cursor].chars().count() as u16;
        f.set_cursor_position((cx.min(area.x + area.width - 2), area.y + 1));
    }
}

fn render_status(f: &mut Frame, app: &TuiApp, area: Rect) {
    let status_fg = if app.is_streaming { YELLOW } else { GREEN };
    let sep = Span::styled(" \u{2502} ", Style::default().fg(BORDER));

    let line = Line::from(vec![
        Span::styled(format!(" {} ", app.status_message), Style::default().fg(status_fg)),
        sep.clone(),
        Span::styled(format!("{}\u{2191}{}\u{2193}", app.total_tokens.0, app.total_tokens.1), Style::default().fg(DIM)),
        sep.clone(),
        Span::styled(format!("${:.4}", app.total_cost), Style::default().fg(DIM)),
        sep.clone(),
        Span::styled(&app.model_name, Style::default().fg(CYAN)),
        sep,
        Span::styled("^K cmds  ^O model  ^S session  F1 help", Style::default().fg(Color::Rgb(60, 60, 80))),
    ]);
    f.render_widget(Paragraph::new(line).style(Style::default().bg(SURFACE)), area);
}

fn render_sidebar(f: &mut Frame, app: &TuiApp, area: Rect) {
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    if app.changed_files.is_empty() {
        lines.push(Line::from(Span::styled("  No changes", Style::default().fg(DIM))));
    } else {
        for path in &app.changed_files {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            lines.push(Line::from(vec![
                Span::styled("  + ", Style::default().fg(GREEN)),
                Span::styled(name, Style::default().fg(TEXT)),
            ]));
        }
    }
    let block = Block::default()
        .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            format!(" Files ({}) ", app.changed_files.len()),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(Paragraph::new(lines).block(block).style(Style::default().bg(BG)), area);
}
