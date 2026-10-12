//! Virtual scrolling Diff view with syntect syntax highlighting and diff backgrounds.

use crate::git::{DiffLineType, FileDiff};
use crate::ui::theme::Theme;
use crate::ui::virtual_scroll::VirtualScroll;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use std::num::NonZeroUsize;
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Type representing a single display line in a flattened diff.
pub type FlatDiffLine = (Option<u32>, Option<u32>, char, DiffLineType, String);

/// Component for rendering high-performance virtual scrolled diff views with LRU highlighting cache.
pub struct DiffView {
    pub scroll: VirtualScroll,
    pub highlight_cache: lru::LruCache<String, Vec<(ratatui::style::Color, String)>>,
}

impl DiffView {
    pub fn new() -> Self {
        Self {
            scroll: VirtualScroll::new(0, 20),
            highlight_cache: lru::LruCache::new(NonZeroUsize::new(500).unwrap()),
        }
    }

    /// Flatten all hunks and lines from a FileDiff into a sequential display list.
    pub fn flatten_diff_lines(diff: &FileDiff) -> Vec<FlatDiffLine> {
        let mut lines = Vec::new();
        for hunk in &diff.hunks {
            lines.push((
                None,
                None,
                '@',
                DiffLineType::HunkHeader,
                hunk.header.clone(),
            ));
            for line in &hunk.lines {
                lines.push((
                    line.old_lineno,
                    line.new_lineno,
                    line.origin,
                    line.line_type,
                    line.content.clone(),
                ));
            }
        }
        lines
    }

    /// Resolve syntax definition from file path without disk access, supporting modern language aliases.
    pub fn find_syntax_for_path(path_str: &str) -> &'static syntect::parsing::SyntaxReference {
        let path = std::path::Path::new(path_str);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 1. Direct extension lookup in syntect default set
        if !ext.is_empty() {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(ext) {
                return syntax;
            }
        }

        // 2. Direct filename / token lookup
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(file_name) {
            return syntax;
        }

        // 3. Fallback mappings for modern file types not present in default syntax set
        let mapped = match ext.to_lowercase().as_str() {
            "toml" => Some("yaml"),
            "ts" | "tsx" | "jsx" | "mjs" | "cjs" => Some("js"),
            "jsonc" | "json5" => Some("json"),
            "yml" => Some("yaml"),
            "rs" => Some("rs"),
            "py" | "pyi" => Some("py"),
            "md" | "markdown" => Some("md"),
            "sh" | "bash" | "zsh" => Some("sh"),
            "sql" => Some("sql"),
            "html" | "htm" => Some("html"),
            "css" | "scss" | "sass" | "less" => Some("css"),
            _ => match file_name.to_lowercase().as_str() {
                "dockerfile" | "containerfile" => Some("sh"),
                ".gitignore" | ".gitleaksignore" | ".env" | ".dockerignore" => Some("sh"),
                "cargo.lock" | ".gitleaks.toml" => Some("yaml"),
                _ => None,
            },
        };

        if let Some(mapped_ext) = mapped {
            if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(mapped_ext) {
                return syntax;
            }
        }

        if let Some(syntax) = SYNTAX_SET.find_syntax_by_token(file_name) {
            return syntax;
        }

        SYNTAX_SET.find_syntax_plain_text()
    }

    /// Render visible portion of the diff into frame.
    pub fn render(&mut self, frame: &mut Frame, diff: &FileDiff, theme: &Theme, area: Rect) {
        let mut title_spans = vec![
            Span::styled(
                " 󰈔 Diff: ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}  ", diff.file_path),
                Style::default()
                    .fg(theme.fg)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        title_spans.extend(crate::ui::capsule::diff_stat_bar(
            diff.stats_added,
            diff.stats_deleted,
            theme.diff_add_fg,
            theme.diff_del_fg,
            theme.dim_fg,
            theme.bg,
        ));
        title_spans.push(Span::raw(" "));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(Line::from(title_spans))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let flat_lines = Self::flatten_diff_lines(diff);
        let total_lines = flat_lines.len();
        self.scroll.set_total_items(total_lines);
        self.scroll.set_viewport_height(inner.height as usize);

        if total_lines == 0 {
            let empty_msg =
                Paragraph::new(" No changes to display").style(Style::default().fg(theme.dim_fg));
            frame.render_widget(empty_msg, inner);
            return;
        }

        // Determine syntax definition by file extension and name with modern language aliases
        let syntax = Self::find_syntax_for_path(&diff.file_path);

        let syn_theme = &THEME_SET.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, syn_theme);

        let visible = self.scroll.visible_range();
        let mut rendered_lines = Vec::with_capacity(visible.len());

        for idx in visible {
            let (old_num, new_num, _origin, line_type, content) = &flat_lines[idx];

            let (bg, fg, prefix) = match line_type {
                DiffLineType::Added => (theme.diff_add_bg, theme.diff_add_fg, "+"),
                DiffLineType::Deleted => (theme.diff_del_bg, theme.diff_del_fg, "-"),
                DiffLineType::HunkHeader => (theme.diff_hunk_bg, theme.diff_hunk_fg, "@"),
                DiffLineType::Context => (theme.bg, theme.fg, " "),
            };

            let old_str = old_num
                .map(|n| format!("{:4}", n))
                .unwrap_or_else(|| "    ".to_string());
            let new_str = new_num
                .map(|n| format!("{:4}", n))
                .unwrap_or_else(|| "    ".to_string());

            let gutter_style = Style::default().bg(bg).fg(theme.dim_fg);
            let marker_style = Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD);

            let mut spans = vec![
                Span::styled(format!("{} ", old_str), gutter_style),
                Span::styled(format!("{} ", new_str), gutter_style),
                Span::styled(format!("{} ", prefix), marker_style),
            ];

            // Highlight all code lines (Context, Added, Deleted); HunkHeader uses dedicated header styling
            if *line_type != DiffLineType::HunkHeader {
                if let Some(cached) = self.highlight_cache.get(content) {
                    for (c, text) in cached {
                        spans.push(Span::styled(text.clone(), Style::default().bg(bg).fg(*c)));
                    }
                } else if let Ok(ranges) = highlighter.highlight_line(content, &SYNTAX_SET) {
                    let mut cached_ranges = Vec::new();
                    for (style, text) in ranges {
                        let c = ratatui::style::Color::Rgb(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        );
                        cached_ranges.push((c, text.to_string()));
                        spans.push(Span::styled(
                            text.to_string(),
                            Style::default().bg(bg).fg(c),
                        ));
                    }
                    self.highlight_cache.put(content.clone(), cached_ranges);
                } else {
                    spans.push(Span::styled(
                        content.clone(),
                        Style::default().bg(bg).fg(fg),
                    ));
                }
            } else {
                spans.push(Span::styled(
                    content.clone(),
                    Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD),
                ));
            }

            rendered_lines.push(Line::from(spans));
        }

        frame.render_widget(Paragraph::new(rendered_lines), inner);
    }
}

impl Default for DiffView {
    fn default() -> Self {
        Self::new()
    }
}
