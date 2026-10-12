//! Repository directory switcher modal.
//! Supports typing paths, navigating parent/child directories, and selecting recent repositories.

use crate::ui::icons::Icons;
use crate::ui::theme::Theme;
use crate::ui::virtual_scroll::VirtualScroll;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use std::path::{Path, PathBuf};

/// Action returned when user confirms or closes the open repository modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRepoAction {
    Open(PathBuf),
    Close,
}

/// Open repository modal state.
pub struct OpenRepoModal {
    pub is_open: bool,
    pub path_input: String,
    pub current_dir: PathBuf,
    pub subdirs: Vec<String>,
    pub recent_repos: Vec<String>,
    pub subdir_scroll: VirtualScroll,
    pub recent_scroll: VirtualScroll,
    pub active_section: OpenRepoSection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRepoSection {
    Input,
    Subdirs,
    Recent,
}

impl Default for OpenRepoModal {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenRepoModal {
    pub fn new() -> Self {
        let cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut modal = Self {
            is_open: false,
            path_input: cur.display().to_string(),
            current_dir: cur,
            subdirs: Vec::new(),
            recent_repos: Vec::new(),
            subdir_scroll: VirtualScroll::new(0, 8),
            recent_scroll: VirtualScroll::new(0, 5),
            active_section: OpenRepoSection::Input,
        };
        modal.refresh_subdirs();
        modal
    }

    /// Open modal initialized with current directory and recent repository list.
    pub fn open_at(&mut self, current: Option<&Path>, recents: &[String]) {
        self.is_open = true;
        if let Some(p) = current {
            self.current_dir = p.to_path_buf();
        } else if let Ok(cwd) = std::env::current_dir() {
            self.current_dir = cwd;
        }
        self.path_input = self.current_dir.display().to_string();
        self.recent_repos = recents.to_vec();
        self.recent_scroll.set_total_items(self.recent_repos.len());
        self.refresh_subdirs();
    }

    /// Close repository switcher.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Refresh directory listing in current_dir.
    pub fn refresh_subdirs(&mut self) {
        let mut dirs = vec![".. (Parent Directory)".to_string()];
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut found: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    if let Ok(ft) = e.file_type() {
                        if ft.is_dir() {
                            let name = e.file_name().to_string_lossy().to_string();
                            if !name.starts_with('.') && name != "target" && name != "node_modules"
                            {
                                return Some(name);
                            }
                        }
                    }
                    None
                })
                .collect();
            found.sort();
            dirs.extend(found);
        }
        self.subdir_scroll.set_total_items(dirs.len());
        self.subdirs = dirs;
    }

    /// Resolve ~ and relative strings to absolute path.
    pub fn resolved_path(&self) -> PathBuf {
        let trimmed = self.path_input.trim();
        if trimmed.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                return PathBuf::from(trimmed.replacen('~', &home, 1));
            }
        }
        let p = PathBuf::from(trimmed);
        if p.is_relative() {
            self.current_dir.join(p)
        } else {
            p
        }
    }

    /// Calculate modal bounding box.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 68.min(viewport.width.saturating_sub(4));
        let height = 24.min(viewport.height.saturating_sub(2));
        let x = (viewport.width.saturating_sub(width)) / 2;
        let y = (viewport.height.saturating_sub(height)) / 2;
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Handle keyboard navigation and text input.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<OpenRepoAction> {
        match key.code {
            KeyCode::Esc => Some(OpenRepoAction::Close),
            KeyCode::Enter => {
                let path = self.resolved_path();
                Some(OpenRepoAction::Open(path))
            }
            KeyCode::Tab => {
                self.active_section = match self.active_section {
                    OpenRepoSection::Input => OpenRepoSection::Subdirs,
                    OpenRepoSection::Subdirs => {
                        if self.recent_repos.is_empty() {
                            OpenRepoSection::Input
                        } else {
                            OpenRepoSection::Recent
                        }
                    }
                    OpenRepoSection::Recent => OpenRepoSection::Input,
                };
                None
            }
            KeyCode::Up => {
                match self.active_section {
                    OpenRepoSection::Subdirs => self.subdir_scroll.previous(),
                    OpenRepoSection::Recent => self.recent_scroll.previous(),
                    _ => {}
                }
                None
            }
            KeyCode::Down => {
                match self.active_section {
                    OpenRepoSection::Subdirs => self.subdir_scroll.next(),
                    OpenRepoSection::Recent => self.recent_scroll.next(),
                    _ => {}
                }
                None
            }
            KeyCode::Backspace => {
                if self.active_section == OpenRepoSection::Input {
                    self.path_input.pop();
                }
                None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_section == OpenRepoSection::Input {
                    self.path_input.push(c);
                }
                None
            }
            _ => None,
        }
    }

    /// Handle mouse clicks inside open repository modal.
    pub fn handle_click(&mut self, x: u16, y: u16, viewport: Rect) -> Option<OpenRepoAction> {
        let rect = self.modal_rect(viewport);
        if x < rect.x || x >= rect.x + rect.width || y < rect.y || y >= rect.y + rect.height {
            return Some(OpenRepoAction::Close);
        }

        let inner_x = x.saturating_sub(rect.x + 1);
        let inner_y = y.saturating_sub(rect.y + 1);

        // Header Buttons: [  Open ] (width-20..width-10), [ ❌ Cancel ] (width-9..)
        if inner_y == 0 {
            if inner_x >= rect.width.saturating_sub(20) && inner_x < rect.width.saturating_sub(10) {
                return Some(OpenRepoAction::Open(self.resolved_path()));
            }
            if inner_x >= rect.width.saturating_sub(10) {
                return Some(OpenRepoAction::Close);
            }
        }

        // Row 2: Input box click
        if inner_y == 2 {
            self.active_section = OpenRepoSection::Input;
            return None;
        }

        // Subdirectories area: Row 5 to 13
        if inner_y >= 5 && inner_y < 14 {
            let row_idx = (inner_y - 5) as usize;
            let target_idx = self.subdir_scroll.viewport_offset + row_idx;
            if let Some(dirname) = self.subdirs.get(target_idx) {
                if dirname.starts_with("..") {
                    if let Some(parent) = self.current_dir.parent() {
                        self.current_dir = parent.to_path_buf();
                        self.path_input = self.current_dir.display().to_string();
                        self.refresh_subdirs();
                    }
                } else {
                    let next = self.current_dir.join(dirname);
                    if next.is_dir() {
                        self.current_dir = next;
                        self.path_input = self.current_dir.display().to_string();
                        self.refresh_subdirs();
                    }
                }
            }
            return None;
        }

        // Recent Repositories area: Row 16 to 20
        if inner_y >= 16 && inner_y < 21 {
            let row_idx = (inner_y - 16) as usize;
            let target_idx = self.recent_scroll.viewport_offset + row_idx;
            if let Some(repo_path) = self.recent_repos.get(target_idx) {
                self.path_input = repo_path.clone();
                self.current_dir = PathBuf::from(repo_path);
                self.refresh_subdirs();
            }
            return None;
        }

        // Bottom Action Bar: Row height - 3
        if inner_y >= rect.height.saturating_sub(3) {
            if inner_x >= 4 && inner_x < 24 {
                return Some(OpenRepoAction::Open(self.resolved_path()));
            }
            if inner_x >= 26 && inner_x < 44 {
                return Some(OpenRepoAction::Close);
            }
        }

        None
    }

    /// Render directory switcher modal.
    pub fn render(&mut self, frame: &mut Frame, theme: &Theme, icons: &Icons, viewport: Rect) {
        if !self.is_open {
            return;
        }

        let rect = self.modal_rect(viewport);
        frame.render_widget(Clear, rect);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.primary))
            .title(Span::styled(
                format!(
                    " {} OPEN REPOSITORY / DIRECTORY SWITCHER ",
                    icons.folder_open()
                ),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(rect);
        frame.render_widget(block, rect);

        let mut lines = Vec::new();

        // 1. Header controls
        lines.push(Line::from(vec![
            Span::styled(
                "Enter path or browse directories below:",
                Style::default().fg(theme.dim_fg),
            ),
            Span::raw(" ".repeat((inner.width as usize).saturating_sub(56))),
            Span::styled(
                " [  Open ] ",
                Style::default()
                    .bg(theme.primary)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(" [ ❌ Cancel ] ", Style::default().fg(theme.dim_fg)),
        ]));

        lines.push(Line::from(""));

        // 2. Input Box
        let input_style = if self.active_section == OpenRepoSection::Input {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        lines.push(Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<54} ", self.path_input), input_style),
        ]));

        lines.push(Line::from(""));

        // 3. Subdirectories list header
        lines.push(Line::from(Span::styled(
            "── Subdirectories (Click to enter, or '..' to go up) ────────────────",
            Style::default().fg(theme.secondary),
        )));

        self.subdir_scroll.set_viewport_height(7);
        let visible_subdirs = self.subdir_scroll.visible_range();
        for (rel_idx, dirname) in self.subdirs[visible_subdirs].iter().enumerate() {
            let abs_idx = self.subdir_scroll.viewport_offset + rel_idx;
            let is_sel = abs_idx == self.subdir_scroll.selected_index
                && self.active_section == OpenRepoSection::Subdirs;

            let icon_str = if dirname.starts_with("..") {
                icons.arrow_up()
            } else {
                icons.folder_closed()
            };

            let style = if is_sel {
                Style::default()
                    .bg(theme.selection_bg)
                    .fg(theme.selection_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{} ", icon_str), style),
                Span::styled(format!("{:<40}", dirname), style),
            ]));
        }

        // Fill empty rows if subdirs < 7
        for _ in self.subdirs.len()..7 {
            lines.push(Line::from(""));
        }

        // 4. Recent Repositories section
        if !self.recent_repos.is_empty() {
            lines.push(Line::from(Span::styled(
                "── Recent Repositories (Click to select) ───────────────────────────",
                Style::default().fg(theme.info),
            )));
            self.recent_scroll.set_viewport_height(3);
            let visible_recents = self.recent_scroll.visible_range();
            for (rel_idx, repo_path) in self.recent_repos[visible_recents].iter().enumerate() {
                let abs_idx = self.recent_scroll.viewport_offset + rel_idx;
                let is_sel = abs_idx == self.recent_scroll.selected_index
                    && self.active_section == OpenRepoSection::Recent;

                let style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.dim_fg)
                };

                lines.push(Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("{} ", icons.branch()), style),
                    Span::styled(format!("{:<52}", repo_path), style),
                ]));
            }
        }

        // 5. Bottom action bar
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                " [  Open Repository (Enter) ] ",
                Style::default()
                    .bg(theme.primary)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                " [ ❌ Cancel (Esc) ] ",
                Style::default().bg(theme.status_bg).fg(theme.dim_fg),
            ),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}
