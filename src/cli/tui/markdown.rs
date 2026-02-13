use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

const CODE_BG: Color = Color::Rgb(30, 30, 50);
const CODE_FG: Color = Color::Rgb(180, 220, 160);
const BOLD_FG: Color = Color::Rgb(240, 240, 255);
const DIM: Color = Color::Rgb(120, 120, 140);
const LINK_FG: Color = Color::Rgb(100, 160, 255);
const H1_FG: Color = Color::Rgb(140, 80, 255);
const H2_FG: Color = Color::Rgb(100, 160, 255);
const LIST_BULLET: Color = Color::Rgb(140, 80, 255);
const TEXT_FG: Color = Color::Rgb(210, 210, 225);
const INLINE_CODE_FG: Color = Color::Rgb(220, 180, 120);
const INLINE_CODE_BG: Color = Color::Rgb(35, 35, 55);

/// Parse a markdown string into ratatui Lines with syntax highlighting
pub fn render_markdown(text: &str, indent: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();

    for raw_line in text.lines() {
        if raw_line.trim_start().starts_with("```") {
            if in_code_block {
                // End code block
                lines.push(Line::from(Span::styled(
                    format!("{indent}  \u{2514}\u{2500}\u{2500}\u{2500}"),
                    Style::default().fg(DIM),
                )));
                in_code_block = false;
                code_lang.clear();
            } else {
                // Start code block
                code_lang = raw_line.trim_start().trim_start_matches('`').to_string();
                let label = if code_lang.is_empty() {
                    "code".to_string()
                } else {
                    code_lang.clone()
                };
                lines.push(Line::from(Span::styled(
                    format!("{indent}  \u{250C}\u{2500}\u{2500} {label} \u{2500}\u{2500}"),
                    Style::default().fg(DIM),
                )));
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("{indent}  \u{2502} {raw_line}"),
                Style::default().fg(CODE_FG).bg(CODE_BG),
            )));
            continue;
        }

        let trimmed = raw_line.trim_start();

        // Headers
        if trimmed.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &trimmed[4..]),
                Style::default()
                    .fg(H2_FG)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if trimmed.starts_with("## ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &trimmed[3..]),
                Style::default()
                    .fg(H2_FG)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if trimmed.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                format!("{indent}{}", &trimmed[2..]),
                Style::default()
                    .fg(H1_FG)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(Line::from(Span::styled(
                format!("{indent}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"),
                Style::default().fg(DIM),
            )));
            continue;
        }

        // Bullet lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            let mut spans = vec![
                Span::styled(
                    format!("{indent}  \u{2022} "),
                    Style::default().fg(LIST_BULLET),
                ),
            ];
            spans.extend(parse_inline_markdown(content));
            lines.push(Line::from(spans));
            continue;
        }

        // Numbered lists
        if let Some(rest) = try_parse_numbered_list(trimmed) {
            let num_end = trimmed.find('.').unwrap_or(0);
            let num = &trimmed[..num_end + 1];
            let mut spans = vec![
                Span::styled(
                    format!("{indent}  {num} "),
                    Style::default().fg(LIST_BULLET),
                ),
            ];
            spans.extend(parse_inline_markdown(rest));
            lines.push(Line::from(spans));
            continue;
        }

        // Empty line
        if trimmed.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        // Regular text with inline formatting
        let mut spans = vec![Span::raw(indent.to_string())];
        spans.extend(parse_inline_markdown(trimmed));
        lines.push(Line::from(spans));
    }

    // Close unclosed code block
    if in_code_block {
        lines.push(Line::from(Span::styled(
            format!("{indent}  \u{2514}\u{2500}\u{2500}\u{2500}"),
            Style::default().fg(DIM),
        )));
    }

    lines
}

/// Parse inline markdown: **bold**, *italic*, `code`, [links](url)
fn parse_inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '`' => {
                // Inline code
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), Style::default().fg(TEXT_FG)));
                    current.clear();
                }
                let mut code = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '`' {
                        chars.next();
                        break;
                    }
                    code.push(chars.next().unwrap());
                }
                spans.push(Span::styled(
                    format!(" {code} "),
                    Style::default().fg(INLINE_CODE_FG).bg(INLINE_CODE_BG),
                ));
            }
            '*' if chars.peek() == Some(&'*') => {
                // Bold
                chars.next();
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), Style::default().fg(TEXT_FG)));
                    current.clear();
                }
                let mut bold = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '*' {
                        chars.next();
                        if chars.peek() == Some(&'*') {
                            chars.next();
                        }
                        break;
                    }
                    bold.push(chars.next().unwrap());
                }
                spans.push(Span::styled(
                    bold,
                    Style::default()
                        .fg(BOLD_FG)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            '*' => {
                // Italic
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), Style::default().fg(TEXT_FG)));
                    current.clear();
                }
                let mut italic = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '*' {
                        chars.next();
                        break;
                    }
                    italic.push(chars.next().unwrap());
                }
                spans.push(Span::styled(
                    italic,
                    Style::default()
                        .fg(TEXT_FG)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            '[' => {
                // Link [text](url)
                if !current.is_empty() {
                    spans.push(Span::styled(current.clone(), Style::default().fg(TEXT_FG)));
                    current.clear();
                }
                let mut link_text = String::new();
                let mut found_close = false;
                while let Some(&next) = chars.peek() {
                    if next == ']' {
                        chars.next();
                        found_close = true;
                        break;
                    }
                    link_text.push(chars.next().unwrap());
                }
                if found_close && chars.peek() == Some(&'(') {
                    chars.next();
                    let mut url = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == ')' {
                            chars.next();
                            break;
                        }
                        url.push(chars.next().unwrap());
                    }
                    spans.push(Span::styled(
                        link_text,
                        Style::default()
                            .fg(LINK_FG)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                } else {
                    current.push('[');
                    current.push_str(&link_text);
                    if found_close {
                        current.push(']');
                    }
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, Style::default().fg(TEXT_FG)));
    }

    spans
}

fn try_parse_numbered_list(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b'.' {
        let rest = &s[i + 1..];
        Some(rest.trim_start())
    } else {
        None
    }
}

/// Render a spinner character based on tick count
pub fn spinner(tick: u64) -> &'static str {
    const FRAMES: &[&str] = &["\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280F}"];
    FRAMES[(tick as usize) % FRAMES.len()]
}
