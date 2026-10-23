//! Markdown parser and renderer for Ratatui text output.
//! Converts markdown syntax into styled Lines and Spans matching Prowly's active theme.

use crate::ui::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Parse a full markdown document into a list of styled Ratatui Lines.
pub fn render_markdown(input: &str, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();

    for raw_line in input.lines() {
        let trimmed = raw_line.trim_end();

        // 1. Code block fence (```)
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block
                lines.push(Line::from(vec![Span::styled(
                    "└─────────────────────────────────────────────────────────────",
                    Style::default().fg(theme.border),
                )]));
                in_code_block = false;
                code_block_lang.clear();
            } else {
                // Start of code block
                in_code_block = true;
                let lang = trimmed.trim_start_matches('`').trim();
                code_block_lang = lang.to_string();
                let lang_display = if lang.is_empty() {
                    "code".to_string()
                } else {
                    lang.to_string()
                };
                lines.push(Line::from(vec![
                    Span::styled("┌── ", Style::default().fg(theme.primary)),
                    Span::styled(
                        format!("[ {} ]", lang_display),
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " ───────────────────────────────────────────────────",
                        Style::default().fg(theme.border),
                    ),
                ]));
            }
            continue;
        }

        // Inside code block
        if in_code_block {
            lines.push(Line::from(vec![
                Span::styled("│ ", Style::default().fg(theme.border)),
                Span::styled(
                    raw_line.to_string(),
                    Style::default().fg(theme.fg).bg(theme.selection_bg),
                ),
            ]));
            continue;
        }

        // 2. Horizontal rule (--- or ***)
        let trimmed_line = trimmed.trim();
        if trimmed_line == "---" || trimmed_line == "***" || trimmed_line == "___" {
            lines.push(Line::from(vec![Span::styled(
                "─────────────────────────────────────────────────────────────",
                Style::default().fg(theme.dim_fg),
            )]));
            continue;
        }

        // 3. Headings (# H1, ## H2, ### H3, ...)
        if let Some(rest) = trimmed_line.strip_prefix("###### ") {
            lines.push(render_heading(rest, 6, theme));
            continue;
        } else if let Some(rest) = trimmed_line.strip_prefix("##### ") {
            lines.push(render_heading(rest, 5, theme));
            continue;
        } else if let Some(rest) = trimmed_line.strip_prefix("#### ") {
            lines.push(render_heading(rest, 4, theme));
            continue;
        } else if let Some(rest) = trimmed_line.strip_prefix("### ") {
            lines.push(render_heading(rest, 3, theme));
            continue;
        } else if let Some(rest) = trimmed_line.strip_prefix("## ") {
            lines.push(render_heading(rest, 2, theme));
            continue;
        } else if let Some(rest) = trimmed_line.strip_prefix("# ") {
            lines.push(render_heading(rest, 1, theme));
            continue;
        }

        // 4. Blockquote (> quote)
        if let Some(rest) = trimmed_line.strip_prefix('>') {
            let quote_text = rest.strip_prefix(' ').unwrap_or(rest);
            let mut spans = vec![Span::styled("▌ ", Style::default().fg(theme.secondary))];
            spans.extend(parse_inline_spans(
                quote_text,
                theme,
                Style::default()
                    .fg(theme.dim_fg)
                    .add_modifier(Modifier::ITALIC),
            ));
            lines.push(Line::from(spans));
            continue;
        }

        // 5. Checkboxes (- [ ], - [x], * [ ], * [x])
        if let Some(rest) = trimmed_line
            .strip_prefix("- [x] ")
            .or_else(|| trimmed_line.strip_prefix("* [x] "))
        {
            let mut spans = vec![Span::styled(
                "  ✔ ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(parse_inline_spans(
                rest,
                theme,
                Style::default().fg(theme.fg),
            ));
            lines.push(Line::from(spans));
            continue;
        }
        if let Some(rest) = trimmed_line
            .strip_prefix("- [ ] ")
            .or_else(|| trimmed_line.strip_prefix("* [ ] "))
        {
            let mut spans = vec![Span::styled("  ☐ ", Style::default().fg(theme.dim_fg))];
            spans.extend(parse_inline_spans(
                rest,
                theme,
                Style::default().fg(theme.fg),
            ));
            lines.push(Line::from(spans));
            continue;
        }

        // 6. Bullet lists (- item, * item, + item)
        if let Some(rest) = trimmed_line
            .strip_prefix("- ")
            .or_else(|| trimmed_line.strip_prefix("* "))
            .or_else(|| trimmed_line.strip_prefix("+ "))
        {
            let mut spans = vec![Span::styled(
                "  • ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )];
            spans.extend(parse_inline_spans(
                rest,
                theme,
                Style::default().fg(theme.fg),
            ));
            lines.push(Line::from(spans));
            continue;
        }

        // 7. Numbered list (e.g. 1. item)
        if let Some((num_str, rest)) = parse_numbered_list(trimmed_line) {
            let mut spans = vec![Span::styled(
                format!("  {}. ", num_str),
                Style::default().fg(theme.primary),
            )];
            spans.extend(parse_inline_spans(
                rest,
                theme,
                Style::default().fg(theme.fg),
            ));
            lines.push(Line::from(spans));
            continue;
        }

        // 8. Empty line
        if trimmed_line.is_empty() {
            lines.push(Line::from(""));
            continue;
        }

        // 9. Regular paragraph line with inline styles
        let spans = parse_inline_spans(raw_line, theme, Style::default().fg(theme.fg));
        lines.push(Line::from(spans));
    }

    lines
}

/// Render a styled heading line with level-specific styles.
fn render_heading(text: &str, level: usize, theme: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    match level {
        1 => {
            spans.push(Span::styled(
                "◈ ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.extend(parse_inline_spans(
                text,
                theme,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ));
        }
        2 => {
            spans.push(Span::styled(
                "◆ ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.extend(parse_inline_spans(
                text,
                theme,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        3 => {
            spans.push(Span::styled(
                "◇ ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.extend(parse_inline_spans(
                text,
                theme,
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        _ => {
            spans.push(Span::styled("  ", Style::default()));
            spans.extend(parse_inline_spans(
                text,
                theme,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ));
        }
    }
    Line::from(spans)
}

/// Check if a line starts with a numbered list prefix like "1. ", "2. ", etc.
fn parse_numbered_list(line: &str) -> Option<(&str, &str)> {
    let mut end_idx = 0;
    for (idx, ch) in line.char_indices() {
        if ch.is_ascii_digit() {
            end_idx = idx + 1;
        } else {
            break;
        }
    }

    if end_idx > 0 && line[end_idx..].starts_with(". ") {
        let num = &line[..end_idx];
        let rest = &line[end_idx + 2..];
        Some((num, rest))
    } else {
        None
    }
}

/// Parse inline markdown decorations:
/// - `**bold**`
/// - `*italic*`
/// - `` `code` ``
/// - `~~strikethrough~~`
/// - `[text](url)`
pub fn parse_inline_spans(text: &str, theme: &Theme, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut idx = 0;
    let mut buf = String::new();

    let flush_buf = |buf: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !buf.is_empty() {
            spans.push(Span::styled(buf.clone(), style));
            buf.clear();
        }
    };

    while idx < len {
        // 1. Inline code: `code`
        if chars[idx] == '`' {
            flush_buf(&mut buf, &mut spans, base_style);
            idx += 1;
            let mut code_buf = String::new();
            while idx < len && chars[idx] != '`' {
                code_buf.push(chars[idx]);
                idx += 1;
            }
            if idx < len && chars[idx] == '`' {
                idx += 1; // consume closing `
            }
            spans.push(Span::styled(
                format!(" {} ", code_buf),
                Style::default()
                    .fg(theme.primary)
                    .bg(theme.selection_bg)
                    .add_modifier(Modifier::BOLD),
            ));
            continue;
        }

        // 2. Bold: **text**
        if idx + 1 < len && chars[idx] == '*' && chars[idx + 1] == '*' {
            flush_buf(&mut buf, &mut spans, base_style);
            idx += 2;
            let mut bold_buf = String::new();
            while idx + 1 < len && !(chars[idx] == '*' && chars[idx + 1] == '*') {
                bold_buf.push(chars[idx]);
                idx += 1;
            }
            if idx + 1 < len && chars[idx] == '*' && chars[idx + 1] == '*' {
                idx += 2; // consume closing **
            }
            let bold_style = base_style.add_modifier(Modifier::BOLD);
            spans.push(Span::styled(bold_buf, bold_style));
            continue;
        }

        // 3. Strikethrough: ~~text~~
        if idx + 1 < len && chars[idx] == '~' && chars[idx + 1] == '~' {
            flush_buf(&mut buf, &mut spans, base_style);
            idx += 2;
            let mut strike_buf = String::new();
            while idx + 1 < len && !(chars[idx] == '~' && chars[idx + 1] == '~') {
                strike_buf.push(chars[idx]);
                idx += 1;
            }
            if idx + 1 < len && chars[idx] == '~' && chars[idx + 1] == '~' {
                idx += 2; // consume closing ~~
            }
            let strike_style = base_style.add_modifier(Modifier::CROSSED_OUT);
            spans.push(Span::styled(strike_buf, strike_style));
            continue;
        }

        // 4. Italic: *text* (single *)
        if chars[idx] == '*' {
            flush_buf(&mut buf, &mut spans, base_style);
            idx += 1;
            let mut italic_buf = String::new();
            while idx < len && chars[idx] != '*' {
                italic_buf.push(chars[idx]);
                idx += 1;
            }
            if idx < len && chars[idx] == '*' {
                idx += 1; // consume closing *
            }
            let italic_style = base_style.add_modifier(Modifier::ITALIC);
            spans.push(Span::styled(italic_buf, italic_style));
            continue;
        }

        // 5. Markdown Link: [label](url)
        if chars[idx] == '[' {
            let start = idx;
            let mut label_buf = String::new();
            let mut closing_bracket = None;
            let mut i = idx + 1;
            while i < len {
                if chars[i] == ']' {
                    closing_bracket = Some(i);
                    break;
                }
                label_buf.push(chars[i]);
                i += 1;
            }

            if let Some(cb) = closing_bracket {
                if cb + 1 < len && chars[cb + 1] == '(' {
                    let mut url_buf = String::new();
                    let mut closing_paren = None;
                    let mut j = cb + 2;
                    while j < len {
                        if chars[j] == ')' {
                            closing_paren = Some(j);
                            break;
                        }
                        url_buf.push(chars[j]);
                        j += 1;
                    }

                    if let Some(cp) = closing_paren {
                        flush_buf(&mut buf, &mut spans, base_style);
                        // Matched [label](url)
                        spans.push(Span::styled(
                            format!("🔗 {}", label_buf),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        idx = cp + 1;
                        continue;
                    }
                }
            }
            // If not a full link, fallback to regular char
            idx = start;
        }

        buf.push(chars[idx]);
        idx += 1;
    }

    flush_buf(&mut buf, &mut spans, base_style);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::ThemeId;

    #[test]
    fn test_render_heading_and_formatting() {
        let theme = Theme::get(ThemeId::TokyoNight);
        let md = "# Heading 1\n## Heading 2\n\nSome **bold text** and `inline code` here.";
        let lines = render_markdown(md, &theme);
        assert!(lines.len() >= 4);
    }

    #[test]
    fn test_render_lists_and_checkboxes() {
        let theme = Theme::get(ThemeId::TokyoNight);
        let md = "- [ ] Task uncompleted\n- [x] Task completed\n* Simple bullet\n1. First item";
        let lines = render_markdown(md, &theme);
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn test_code_blocks() {
        let theme = Theme::get(ThemeId::TokyoNight);
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md, &theme);
        assert_eq!(lines.len(), 3);
    }
}
