//! High-end GUI capsule and pill badge rendering utilities.
//! Produces pixel-smooth rounded pills using Nerd Font powerline glyphs ( / )
//! with automatic graceful fallback for standard terminal environments.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Left rounded cap glyph (Nerd Font U+E0B6).
pub const CAP_LEFT: &str = "\u{e0b6}";
/// Right rounded cap glyph (Nerd Font U+E0B4).
pub const CAP_RIGHT: &str = "\u{e0b4}";

/// Create a single capsule/pill badge.
///
/// Example with Nerd Font:
/// `` (fg=bg, bg=outer_bg) + ` [icon] text ` (fg=fg, bg=bg) + `` (fg=bg, bg=outer_bg)
pub fn pill(
    icon: Option<&str>,
    text: &str,
    fg: Color,
    bg: Color,
    outer_bg: Color,
    use_nerd_font: bool,
) -> Vec<Span<'static>> {
    if use_nerd_font {
        let content = if let Some(ic) = icon {
            format!(" {} {} ", ic, text)
        } else {
            format!(" {} ", text)
        };

        vec![
            Span::styled(CAP_LEFT, Style::default().fg(bg).bg(outer_bg)),
            Span::styled(
                content,
                Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
            ),
            Span::styled(CAP_RIGHT, Style::default().fg(bg).bg(outer_bg)),
        ]
    } else {
        let content = if let Some(ic) = icon {
            format!(" [{} {}] ", ic, text)
        } else {
            format!(" [{}] ", text)
        };

        vec![Span::styled(
            content,
            Style::default().fg(fg).bg(outer_bg).add_modifier(Modifier::BOLD),
        )]
    }
}

/// Create a sleek dual-segment pill badge (e.g., [ KEY | ACTION ] or [ BRANCH | AHEAD/BEHIND ]).
pub fn split_pill(
    left_icon: Option<&str>,
    left_text: &str,
    left_fg: Color,
    left_bg: Color,
    right_text: &str,
    right_fg: Color,
    right_bg: Color,
    outer_bg: Color,
    use_nerd_font: bool,
) -> Vec<Span<'static>> {
    if use_nerd_font {
        let left_content = if let Some(ic) = left_icon {
            format!(" {} {} ", ic, left_text)
        } else {
            format!(" {} ", left_text)
        };
        let right_content = format!(" {} ", right_text);

        vec![
            Span::styled(CAP_LEFT, Style::default().fg(left_bg).bg(outer_bg)),
            Span::styled(
                left_content,
                Style::default()
                    .fg(left_fg)
                    .bg(left_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                right_content,
                Style::default().fg(right_fg).bg(right_bg),
            ),
            Span::styled(CAP_RIGHT, Style::default().fg(right_bg).bg(outer_bg)),
        ]
    } else {
        let left_content = if let Some(ic) = left_icon {
            format!("[{} {} | ", ic, left_text)
        } else {
            format!("[{} | ", left_text)
        };
        let right_content = format!("{}]", right_text);

        vec![
            Span::styled(left_content, Style::default().fg(left_fg).bg(outer_bg)),
            Span::styled(right_content, Style::default().fg(right_fg).bg(outer_bg)),
        ]
    }
}

/// Create a desktop-style keyboard shortcut badge (e.g. `[Tab]` KeyCap followed by `Panes`).
pub fn key_badge(
    key: &str,
    desc: &str,
    key_fg: Color,
    key_bg: Color,
    desc_fg: Color,
    outer_bg: Color,
    use_nerd_font: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    if use_nerd_font {
        spans.push(Span::styled(CAP_LEFT, Style::default().fg(key_bg).bg(outer_bg)));
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(key_fg)
                .bg(key_bg)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(CAP_RIGHT, Style::default().fg(key_bg).bg(outer_bg)));
    } else {
        spans.push(Span::styled(
            format!("[{}]", key),
            Style::default()
                .fg(key_fg)
                .bg(outer_bg)
                .add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::styled(
        format!(" {} ", desc),
        Style::default().fg(desc_fg).bg(outer_bg),
    ));
    spans
}

/// Render a 5-bar GitHub Desktop style visual diff ratio gauge (e.g. `+24 -8 ▇▇▇▆ `).
pub fn diff_stat_bar(
    added: usize,
    deleted: usize,
    add_fg: Color,
    del_fg: Color,
    dim_fg: Color,
    outer_bg: Color,
) -> Vec<Span<'static>> {
    let total = added + deleted;
    let mut spans = vec![
        Span::styled(
            format!("+{}", added),
            Style::default()
                .fg(add_fg)
                .bg(outer_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("-{}", deleted),
            Style::default()
                .fg(del_fg)
                .bg(outer_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ];

    if total == 0 {
        spans.push(Span::styled(
            "󰝤󰝤󰝤󰝤󰝤",
            Style::default().fg(dim_fg).bg(outer_bg),
        ));
        return spans;
    }

    // Allocate 5 block cells proportionally
    let add_blocks = ((added as f64 / total as f64) * 5.0).round() as usize;
    let del_blocks = 5_usize.saturating_sub(add_blocks);

    for _ in 0..add_blocks {
        spans.push(Span::styled("■", Style::default().fg(add_fg).bg(outer_bg)));
    }
    for _ in 0..del_blocks {
        spans.push(Span::styled("■", Style::default().fg(del_fg).bg(outer_bg)));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pill_nerd_font_capsules() {
        let p = pill(
            Some("󰄛"),
            "Prowly",
            Color::White,
            Color::Blue,
            Color::Black,
            true,
        );
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].content, CAP_LEFT);
        assert!(p[1].content.contains("󰄛"));
        assert!(p[1].content.contains("Prowly"));
        assert_eq!(p[2].content, CAP_RIGHT);
    }

    #[test]
    fn test_pill_fallback() {
        let p = pill(
            Some("P"),
            "Prowly",
            Color::White,
            Color::Blue,
            Color::Black,
            false,
        );
        assert_eq!(p.len(), 1);
        assert!(p[0].content.contains("[P Prowly]"));
    }

    #[test]
    fn test_split_pill_segments() {
        let sp = split_pill(
            Some(""),
            "main",
            Color::White,
            Color::Blue,
            "↑1 ↓0",
            Color::Black,
            Color::Cyan,
            Color::Black,
            true,
        );
        assert_eq!(sp.len(), 4);
        assert_eq!(sp[0].content, CAP_LEFT);
        assert_eq!(sp[3].content, CAP_RIGHT);
    }

    #[test]
    fn test_key_badge() {
        let kb = key_badge(
            "Tab",
            "Panes",
            Color::White,
            Color::DarkGray,
            Color::Gray,
            Color::Black,
            true,
        );
        assert_eq!(kb.len(), 4);
        assert_eq!(kb[0].content, CAP_LEFT);
        assert_eq!(kb[1].content, " Tab ");
        assert_eq!(kb[2].content, CAP_RIGHT);
        assert_eq!(kb[3].content, " Panes ");
    }

    #[test]
    fn test_diff_stat_bar() {
        let bar = diff_stat_bar(
            15,
            5,
            Color::Green,
            Color::Red,
            Color::DarkGray,
            Color::Black,
        );
        assert!(bar.len() >= 4);
    }
}
