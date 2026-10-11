//! GitHub Alerts and Notifications view.

use crate::github::AlertItem;
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::virtual_scroll::VirtualScroll;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

/// Alerts & Notifications view component.
pub struct AlertsView {
    pub alerts: Vec<AlertItem>,
    pub scroll: VirtualScroll,
    pub status_message: Option<String>,
}

impl AlertsView {
    pub fn new() -> Self {
        Self {
            alerts: Vec::new(),
            scroll: VirtualScroll::new(0, 10),
            status_message: None,
        }
    }

    pub fn set_alerts(&mut self, alerts: Vec<AlertItem>) {
        self.scroll.set_total_items(alerts.len());
        self.alerts = alerts;
    }

    pub fn selected_alert(&self) -> Option<&AlertItem> {
        self.alerts.get(self.scroll.selected_index)
    }

    pub fn mark_selected_read(&mut self) -> Option<String> {
        let idx = self.scroll.selected_index;
        if let Some(item) = self.alerts.get_mut(idx) {
            item.unread = false;
            return Some(item.id.clone());
        }
        None
    }

    pub fn next(&mut self) {
        self.scroll.next();
    }

    pub fn previous(&mut self) {
        self.scroll.previous();
    }

    pub fn render(
        &mut self,
        frame: &mut Frame,
        is_focused: bool,
        theme: &Theme,
        icons: &Icons,
        area: Rect,
    ) {
        let border_color = if is_focused {
            theme.border_focused
        } else {
            theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(
                    " {} NOTIFICATIONS & ALERTS ",
                    icons.get(crate::ui::IconId::Alert)
                ),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.alerts.is_empty() {
            let msg = if let Some(ref m) = self.status_message {
                m.as_str()
            } else {
                "No unread notifications or security alerts."
            };
            let p = Paragraph::new(format!("\n  {}", msg)).style(Style::default().fg(theme.dim_fg));
            frame.render_widget(p, inner);
            return;
        }

        self.scroll.viewport_height = (inner.height as usize / 2).max(1);
        let range = self.scroll.visible_range();

        let mut lines = Vec::new();
        for (i, alert) in self
            .alerts
            .iter()
            .enumerate()
            .skip(range.start)
            .take(range.end - range.start)
        {
            let is_selected = i == self.scroll.selected_index;
            let (status_glyph, status_color) = if alert.unread {
                ("●", theme.accent)
            } else {
                ("○", theme.dim_fg)
            };

            let prefix = if is_selected { "▶ " } else { "  " };
            let line_bg = if is_selected {
                theme.selection_bg
            } else {
                theme.bg
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.primary).bg(line_bg)),
                Span::styled(
                    format!("{} ", status_glyph),
                    Style::default().fg(status_color).bg(line_bg),
                ),
                Span::styled(
                    format!("[{}] ", alert.reason),
                    Style::default().fg(theme.primary).bg(line_bg),
                ),
                Span::styled(
                    alert.title.clone(),
                    Style::default()
                        .fg(theme.fg)
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        })
                        .bg(line_bg),
                ),
            ]));

            // Subline: repository
            lines.push(Line::from(vec![
                Span::styled("    repo: ", Style::default().fg(theme.dim_fg).bg(line_bg)),
                Span::styled(
                    &alert.repository,
                    Style::default().fg(theme.dim_fg).bg(line_bg),
                ),
            ]));
        }

        let p = Paragraph::new(lines);
        frame.render_widget(p, inner);
    }
}

impl Default for AlertsView {
    fn default() -> Self {
        Self::new()
    }
}
