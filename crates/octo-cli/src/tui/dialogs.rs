use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use octo_core::model::{builtin_models, ModelId};
use octo_core::permission::PermissionRequest;
use octo_core::session::Session;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

// ─── Helpers ─────────────────────────────────────────

const BG: Color = Color::Rgb(20, 20, 35);
const BORDER: Color = Color::Rgb(100, 60, 200);
const DIM: Color = Color::Rgb(120, 120, 140);
const HIGHLIGHT: Color = Color::Rgb(140, 80, 255);
const TEXT: Color = Color::Rgb(220, 220, 230);

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn render_dimmed_bg(f: &mut Frame, area: Rect) {
    f.render_widget(Clear, area);
    let bg = Paragraph::new("").style(Style::default().bg(Color::Rgb(10, 10, 18)));
    f.render_widget(bg, area);
}

fn dialog_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD),
        ))
}

// ─── Model Dialog ────────────────────────────────────

pub struct ModelDialog {
    pub selected: usize,
    pub models: Vec<(String, String, String, String)>, // id, name, vendor, info
}

pub enum ModelDialogAction {
    None,
    Close,
    Select(ModelId),
}

impl ModelDialog {
    pub fn new(current_model_id: &str) -> Self {
        let registry = builtin_models();
        let mut models: Vec<(String, String, String, String)> = registry
            .iter()
            .map(|(id, m)| {
                (
                    id.0.clone(),
                    m.display_name.clone(),
                    m.vendor.to_string(),
                    format!(
                        "{}K ctx  ${:.2}/M in  ${:.2}/M out",
                        m.context_window / 1000,
                        m.pricing.cost_per_1m_input,
                        m.pricing.cost_per_1m_output
                    ),
                )
            })
            .collect();
        models.sort_by(|a, b| a.1.cmp(&b.1));

        let selected = models
            .iter()
            .position(|m| m.0 == current_model_id)
            .unwrap_or(0);

        Self { selected, models }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ModelDialogAction {
        match key.code {
            KeyCode::Esc => ModelDialogAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                ModelDialogAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.models.len() {
                    self.selected += 1;
                }
                ModelDialogAction::None
            }
            KeyCode::Enter => {
                if let Some(m) = self.models.get(self.selected) {
                    ModelDialogAction::Select(ModelId(m.0.clone()))
                } else {
                    ModelDialogAction::Close
                }
            }
            _ => ModelDialogAction::None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        render_dimmed_bg(f, area);

        let width = 60.min(area.width.saturating_sub(4));
        let height = (self.models.len() as u16 + 5).min(area.height.saturating_sub(4));
        let dialog_area = centered_rect(width, height, area);

        let block = dialog_block("Select Model");
        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let mut lines = Vec::new();
        lines.push(Line::from(""));
        for (i, (_, name, vendor, info)) in self.models.iter().enumerate() {
            let marker = if i == self.selected { "> " } else { "  " };
            let style = if i == self.selected {
                Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{marker}{name:<16}"), style),
                Span::styled(format!("{vendor:<12}"), Style::default().fg(DIM)),
                Span::styled(info, Style::default().fg(DIM)),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Up/Down: navigate  Enter: select  Esc: close",
            Style::default().fg(DIM),
        )));

        f.render_widget(Paragraph::new(lines), inner);
    }
}

// ─── Session Dialog ──────────────────────────────────

pub struct SessionDialog {
    pub selected: usize,
    pub sessions: Vec<Session>,
    pub current_id: String,
}

pub enum SessionDialogAction {
    None,
    Close,
    Select(String),
    New,
    Delete(String),
}

impl SessionDialog {
    pub fn new(sessions: Vec<Session>, current_id: &str) -> Self {
        let selected = sessions
            .iter()
            .position(|s| s.id == current_id)
            .unwrap_or(0);
        Self {
            selected,
            sessions,
            current_id: current_id.to_string(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SessionDialogAction {
        match key.code {
            KeyCode::Esc => SessionDialogAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                SessionDialogAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.sessions.len() {
                    self.selected += 1;
                }
                SessionDialogAction::None
            }
            KeyCode::Enter => {
                if let Some(s) = self.sessions.get(self.selected) {
                    SessionDialogAction::Select(s.id.clone())
                } else {
                    SessionDialogAction::Close
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => SessionDialogAction::New,
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if let Some(s) = self.sessions.get(self.selected) {
                    if s.id != self.current_id {
                        SessionDialogAction::Delete(s.id.clone())
                    } else {
                        SessionDialogAction::None
                    }
                } else {
                    SessionDialogAction::None
                }
            }
            _ => SessionDialogAction::None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        render_dimmed_bg(f, area);

        let width = 65.min(area.width.saturating_sub(4));
        let height = (self.sessions.len() as u16 + 6).min(area.height.saturating_sub(4)).max(8);
        let dialog_area = centered_rect(width, height, area);

        let block = dialog_block("Sessions");
        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let mut lines = Vec::new();
        lines.push(Line::from(""));

        if self.sessions.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No sessions yet.",
                Style::default().fg(DIM),
            )));
        } else {
            for (i, s) in self.sessions.iter().enumerate() {
                let marker = if i == self.selected { "> " } else { "  " };
                let current = if s.id == self.current_id {
                    " *"
                } else {
                    ""
                };
                let style = if i == self.selected {
                    Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(TEXT)
                };
                let id_short = &s.id[..8.min(s.id.len())];
                lines.push(Line::from(vec![
                    Span::styled(format!("{marker}{id_short}{current}"), style),
                    Span::styled(
                        format!("  {:<20}", truncate_str(&s.title, 20)),
                        style,
                    ),
                    Span::styled(
                        format!("  {} msgs  ${:.4}", s.message_count, s.cost),
                        Style::default().fg(DIM),
                    ),
                ]));
            }
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Up/Down: nav  Enter: switch  N: new  D: delete  Esc: close",
            Style::default().fg(DIM),
        )));

        f.render_widget(Paragraph::new(lines), inner);
    }
}

// ─── Command Palette ─────────────────────────────────

pub struct CommandEntry {
    pub name: String,
    pub description: String,
    pub shortcut: String,
}

pub struct CommandDialog {
    pub selected: usize,
    pub filter: String,
    pub commands: Vec<CommandEntry>,
    pub filtered_indices: Vec<usize>,
}

pub enum CommandDialogAction {
    None,
    Close,
    Execute(String),
}

impl CommandDialog {
    pub fn new() -> Self {
        let commands = vec![
            CommandEntry {
                name: "/help".into(),
                description: "Show help".into(),
                shortcut: "F1".into(),
            },
            CommandEntry {
                name: "/model".into(),
                description: "Select model".into(),
                shortcut: "Ctrl+O".into(),
            },
            CommandEntry {
                name: "/session".into(),
                description: "Switch session".into(),
                shortcut: "Ctrl+S".into(),
            },
            CommandEntry {
                name: "/compact".into(),
                description: "Compact conversation".into(),
                shortcut: "Ctrl+L".into(),
            },
            CommandEntry {
                name: "/clear".into(),
                description: "Clear chat".into(),
                shortcut: "".into(),
            },
            CommandEntry {
                name: "/cost".into(),
                description: "Show token usage".into(),
                shortcut: "".into(),
            },
            CommandEntry {
                name: "/sidebar".into(),
                description: "Toggle file sidebar".into(),
                shortcut: "Ctrl+B".into(),
            },
            CommandEntry {
                name: "/exit".into(),
                description: "Exit OctoCode".into(),
                shortcut: "Ctrl+C".into(),
            },
        ];
        let filtered_indices = (0..commands.len()).collect();
        Self {
            selected: 0,
            filter: String::new(),
            commands,
            filtered_indices,
        }
    }

    fn update_filter(&mut self) {
        let lower = self.filter.to_lowercase();
        self.filtered_indices = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                lower.is_empty()
                    || c.name.to_lowercase().contains(&lower)
                    || c.description.to_lowercase().contains(&lower)
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> CommandDialogAction {
        match key.code {
            KeyCode::Esc => CommandDialogAction::Close,
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                CommandDialogAction::None
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered_indices.len() {
                    self.selected += 1;
                }
                CommandDialogAction::None
            }
            KeyCode::Enter => {
                if let Some(&idx) = self.filtered_indices.get(self.selected) {
                    CommandDialogAction::Execute(self.commands[idx].name.clone())
                } else {
                    CommandDialogAction::Close
                }
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.update_filter();
                CommandDialogAction::None
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE
                    || key.modifiers == KeyModifiers::SHIFT =>
            {
                self.filter.push(c);
                self.update_filter();
                CommandDialogAction::None
            }
            _ => CommandDialogAction::None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        render_dimmed_bg(f, area);

        let width = 55.min(area.width.saturating_sub(4));
        let height = (self.filtered_indices.len() as u16 + 7)
            .min(area.height.saturating_sub(4))
            .max(10);
        let dialog_area = centered_rect(width, height, area);

        let block = dialog_block("Command Palette");
        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        // Filter input
        let filter_text = if self.filter.is_empty() {
            "  Type to filter...".to_string()
        } else {
            format!("  {}", self.filter)
        };
        let filter_style = if self.filter.is_empty() {
            Style::default().fg(DIM)
        } else {
            Style::default().fg(TEXT)
        };
        f.render_widget(Paragraph::new(filter_text).style(filter_style), chunks[0]);

        // Command list
        let mut lines = Vec::new();
        for (i, &idx) in self.filtered_indices.iter().enumerate() {
            let cmd = &self.commands[idx];
            let marker = if i == self.selected { "> " } else { "  " };
            let style = if i == self.selected {
                Style::default().fg(HIGHLIGHT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{marker}{:<12}", cmd.name), style),
                Span::styled(
                    format!("{:<22}", cmd.description),
                    Style::default().fg(DIM),
                ),
                Span::styled(&cmd.shortcut, Style::default().fg(Color::Cyan)),
            ]));
        }
        f.render_widget(Paragraph::new(lines), chunks[1]);

        // Hint
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Up/Down: nav  Enter: run  Esc: close",
                Style::default().fg(DIM),
            )),
            chunks[2],
        );
    }
}

// ─── Help Dialog ─────────────────────────────────────

pub struct HelpDialog;

impl HelpDialog {
    pub fn render(f: &mut Frame, area: Rect) {
        render_dimmed_bg(f, area);

        let width = 55.min(area.width.saturating_sub(4));
        let height = 22.min(area.height.saturating_sub(4));
        let dialog_area = centered_rect(width, height, area);

        let block = dialog_block("Help");
        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Key Bindings",
                Style::default()
                    .fg(HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )),
            help_line("Enter", "Send message"),
            help_line("Ctrl+C", "Cancel / Quit"),
            help_line("Ctrl+O", "Select model"),
            help_line("Ctrl+S", "Switch session"),
            help_line("Ctrl+K", "Command palette"),
            help_line("Ctrl+L", "Compact conversation"),
            help_line("Ctrl+B", "Toggle sidebar"),
            help_line("Up/Down", "Scroll chat"),
            help_line("PgUp/PgDn", "Scroll page"),
            help_line("F1", "Show this help"),
            Line::from(""),
            Line::from(Span::styled(
                "  Commands",
                Style::default()
                    .fg(HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "    /help  /model  /session  /compact",
                Style::default().fg(DIM),
            )),
            Line::from(Span::styled(
                "    /clear  /cost  /sidebar  /exit",
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press any key to close",
                Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
            )),
        ];

        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn help_line<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("    {key:<14}"), Style::default().fg(Color::Cyan)),
        Span::styled(desc, Style::default().fg(TEXT)),
    ])
}

// ─── Permission Dialog ───────────────────────────────

pub struct PermissionDialog {
    pub description: String,
    pub tool_name: String,
    pub path: Option<String>,
    pub selected: usize, // 0=Allow, 1=Always, 2=Deny
}

pub enum PermissionDialogAction {
    None,
    Allow,
    AlwaysAllow,
    Deny,
}

impl PermissionDialog {
    pub fn new(req: &PermissionRequest) -> Self {
        Self {
            description: req.description.clone(),
            tool_name: req.tool_name.clone(),
            path: req.path.clone(),
            selected: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PermissionDialogAction {
        match key.code {
            KeyCode::Esc => PermissionDialogAction::Deny,
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected = self.selected.saturating_sub(1);
                PermissionDialogAction::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.selected < 2 {
                    self.selected += 1;
                }
                PermissionDialogAction::None
            }
            KeyCode::Tab => {
                self.selected = (self.selected + 1) % 3;
                PermissionDialogAction::None
            }
            KeyCode::Enter => match self.selected {
                0 => PermissionDialogAction::Allow,
                1 => PermissionDialogAction::AlwaysAllow,
                _ => PermissionDialogAction::Deny,
            },
            KeyCode::Char('y') | KeyCode::Char('Y') => PermissionDialogAction::Allow,
            KeyCode::Char('a') | KeyCode::Char('A') => PermissionDialogAction::AlwaysAllow,
            KeyCode::Char('n') | KeyCode::Char('N') => PermissionDialogAction::Deny,
            _ => PermissionDialogAction::None,
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        render_dimmed_bg(f, area);

        let width = 55.min(area.width.saturating_sub(4));
        let height = 11.min(area.height.saturating_sub(2));
        let dialog_area = centered_rect(width, height, area);

        let block = dialog_block("Permission Required");
        let inner = block.inner(dialog_area);
        f.render_widget(block, dialog_area);

        let mut lines = Vec::new();
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Tool: ", Style::default().fg(DIM)),
            Span::styled(&self.tool_name, Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Action: ", Style::default().fg(DIM)),
            Span::styled(&self.description, Style::default().fg(TEXT)),
        ]));
        if let Some(path) = &self.path {
            let short: String = path.chars().take(40).collect();
            lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().fg(DIM)),
                Span::styled(short, Style::default().fg(TEXT)),
            ]));
        }
        lines.push(Line::from(""));

        // Buttons
        let buttons: Vec<Span> = ["  Allow  ", " Always ", "  Deny  "]
            .iter()
            .enumerate()
            .map(|(i, label)| {
                if i == self.selected {
                    Span::styled(
                        format!("[ {label} ]"),
                        Style::default()
                            .fg(Color::White)
                            .bg(HIGHLIGHT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(
                        format!("[ {label} ]"),
                        Style::default().fg(DIM),
                    )
                }
            })
            .collect();
        let mut button_line = vec![Span::raw("  ")];
        button_line.extend(buttons);
        lines.push(Line::from(button_line));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Tab/Arrows: nav  Enter: confirm  Y/A/N: quick",
            Style::default().fg(DIM),
        )));

        f.render_widget(Paragraph::new(lines), inner);
    }
}

// ─── Utils ───────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!("{}...", chars[..max - 3].iter().collect::<String>())
    }
}
