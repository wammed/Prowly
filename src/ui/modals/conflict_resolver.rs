//! Visual conflict resolution modal component.

use crate::git::conflict::{ConflictChoice, ConflictHunk};
use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

/// Result of handling a mouse click on the conflict resolver modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolverClickResult {
    /// Clicked outside the modal area or cancel -> close modal.
    Close,
    /// Resolve current hunk with specified choice.
    Resolve(ConflictChoice),
    /// Request AI smart merge for current hunk.
    RequestAi,
    /// Save resolved content and stage.
    Save,
    /// Clicked inside modal area -> consumed.
    Handled,
}

/// Conflict resolver modal state.
#[derive(Debug, Clone, Default)]
pub struct ConflictResolverModal {
    pub is_open: bool,
    pub file_path: String,
    pub original_content: String,
    pub hunks: Vec<ConflictHunk>,
    pub current_hunk_idx: usize,
    pub status_message: Option<String>,
}

impl ConflictResolverModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open resolver for a conflicted file.
    pub fn open(&mut self, file_path: String, content: String) {
        let (hunks, _) = crate::git::conflict::parse_conflicts(&content);
        self.is_open = true;
        self.file_path = file_path;
        self.original_content = content;
        self.hunks = hunks;
        self.current_hunk_idx = 0;
        self.status_message = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn current_hunk(&self) -> Option<&ConflictHunk> {
        self.hunks.get(self.current_hunk_idx)
    }

    pub fn current_hunk_mut(&mut self) -> Option<&mut ConflictHunk> {
        self.hunks.get_mut(self.current_hunk_idx)
    }

    pub fn next_hunk(&mut self) {
        if self.current_hunk_idx + 1 < self.hunks.len() {
            self.current_hunk_idx += 1;
        }
    }

    pub fn previous_hunk(&mut self) {
        if self.current_hunk_idx > 0 {
            self.current_hunk_idx -= 1;
        }
    }

    pub fn resolve_current(&mut self, choice: ConflictChoice) {
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.choice = Some(choice);
            hunk.custom_resolution = None;
        }
        self.next_hunk();
    }

    pub fn set_ai_resolution(&mut self, resolved_code: String) {
        if let Some(hunk) = self.current_hunk_mut() {
            hunk.custom_resolution = Some(resolved_code);
            hunk.choice = None;
        }
    }

    pub fn build_resolved_content(&self) -> String {
        crate::git::conflict::apply_resolutions(&self.original_content, &self.hunks)
    }

    /// Calculate modal bounding rectangle.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 90.min(viewport.width.saturating_sub(4));
        let height = 30.min(viewport.height.saturating_sub(4));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Handle mouse click inside or outside modal.
    pub fn handle_click(
        &mut self,
        x: u16,
        y: u16,
        viewport: Rect,
    ) -> Option<ConflictResolverClickResult> {
        if !self.is_open {
            return None;
        }

        let area = self.modal_rect(viewport);
        if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
            return Some(ConflictResolverClickResult::Close);
        }

        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(50),
                Constraint::Percentage(35),
                Constraint::Length(2),
            ])
            .split(inner);

        // Comparison chunk[1] (Current vs Incoming)
        if y >= chunks[1].y && y < chunks[1].y + chunks[1].height {
            let comp_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);
            if x >= comp_chunks[0].x && x < comp_chunks[0].x + comp_chunks[0].width {
                self.resolve_current(ConflictChoice::Current);
                return Some(ConflictResolverClickResult::Resolve(ConflictChoice::Current));
            } else if x >= comp_chunks[1].x && x < comp_chunks[1].x + comp_chunks[1].width {
                self.resolve_current(ConflictChoice::Incoming);
                return Some(ConflictResolverClickResult::Resolve(ConflictChoice::Incoming));
            }
        }

        // Action toolbar chunk[3] (Length 2)
        if y >= chunks[3].y && y < chunks[3].y + chunks[3].height {
            let rel_x = x.saturating_sub(chunks[3].x);
            if rel_x < 16 {
                self.resolve_current(ConflictChoice::Current);
                return Some(ConflictResolverClickResult::Resolve(ConflictChoice::Current));
            } else if rel_x < 33 {
                self.resolve_current(ConflictChoice::Incoming);
                return Some(ConflictResolverClickResult::Resolve(ConflictChoice::Incoming));
            } else if rel_x < 46 {
                self.resolve_current(ConflictChoice::Both);
                return Some(ConflictResolverClickResult::Resolve(ConflictChoice::Both));
            } else if rel_x < 76 {
                return Some(ConflictResolverClickResult::RequestAi);
            } else if rel_x < 93 {
                return Some(ConflictResolverClickResult::Save);
            } else {
                return Some(ConflictResolverClickResult::Close);
            }
        }

        Some(ConflictResolverClickResult::Handled)
    }

    pub fn render(&self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let area = self.modal_rect(viewport);

        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                format!(
                    " {} RESOLVE MERGE CONFLICTS: {} ",
                    icons.conflict(),
                    self.file_path
                ),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Percentage(50),
                Constraint::Percentage(35),
                Constraint::Length(2),
            ])
            .split(inner);

        // 1. Header with hunk counter
        let total = self.hunks.len();
        let curr = if total > 0 {
            self.current_hunk_idx + 1
        } else {
            0
        };
        let header_p = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                format!("Conflict Hunk {} of {}", curr, total),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  (Use Tab / n / p to navigate hunks)",
                Style::default().fg(theme.dim_fg),
            ),
        ])]);
        frame.render_widget(header_p, chunks[0]);

        // 2. Comparison split (Current vs Incoming)
        if let Some(hunk) = self.current_hunk() {
            let comp_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            // Left: Current (HEAD)
            let curr_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.primary))
                .title(format!(" Current: {} ", hunk.current_branch_name));
            let curr_inner = curr_block.inner(comp_chunks[0]);
            frame.render_widget(curr_block, comp_chunks[0]);

            let curr_lines: Vec<Line> = hunk
                .current_lines
                .iter()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.diff_add_fg))))
                .collect();
            frame.render_widget(Paragraph::new(curr_lines), curr_inner);

            // Right: Incoming
            let inc_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(163, 113, 247)))
                .title(format!(" Incoming: {} ", hunk.incoming_branch_name));
            let inc_inner = inc_block.inner(comp_chunks[1]);
            frame.render_widget(inc_block, comp_chunks[1]);

            let inc_lines: Vec<Line> = hunk
                .incoming_lines
                .iter()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(theme.diff_mod_fg))))
                .collect();
            frame.render_widget(Paragraph::new(inc_lines), inc_inner);

            // 3. Resolution preview
            let prev_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.success))
                .title(" Merged Result Preview ");
            let prev_inner = prev_block.inner(chunks[2]);
            frame.render_widget(prev_block, chunks[2]);

            let resolved = hunk.resolved_lines();
            let res_lines: Vec<Line> = resolved
                .iter()
                .map(|l| Line::from(Span::styled(l.clone(), Style::default().fg(theme.fg))))
                .collect();
            frame.render_widget(Paragraph::new(res_lines), prev_inner);
        }

        // 4. Action toolbar
        let actions = vec![Line::from(vec![
            Span::styled(
                "[ 1: Current ] ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[ 2: Incoming ] ",
                Style::default()
                    .fg(Color::Rgb(163, 113, 247))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("[ 3: Both ] ", Style::default().fg(theme.fg)),
            Span::styled(
                "[ 4/Ctrl+M: AI Smart Merge ] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[ Enter: Save ] ",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("[ Esc: Cancel ]", Style::default().fg(theme.dim_fg)),
        ])];
        frame.render_widget(Paragraph::new(actions), chunks[3]);
    }
}
