//! Repository directory switcher modal.
//! Supports typing paths, TAB path autocompletion, navigating parent/child directories,
//! and selecting/managing recent repositories.

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

/// Action returned when user confirms, removes an entry, or closes the open repository modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRepoAction {
    Open(PathBuf),
    Close,
    RemoveRecent(String),
}

/// Completion state when user cycles through multiple TAB matches.
#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    pub original_input: String,
    pub candidates: Vec<String>,
    pub current_index: usize,
}

/// Fixed row layout mapping for OpenRepoModal inner viewport.
#[derive(Debug, Clone, Copy)]
pub struct OpenRepoLayout {
    pub header_row: u16,         // 0
    pub input_row: u16,          // 2
    pub helper_row: u16,         // 3
    pub subdirs_header_row: u16, // 4
    pub subdirs_start: u16,      // 5
    pub subdirs_count: u16,      // 6 (rows 5..11)
    pub recent_header_row: u16,  // 11
    pub recent_start: u16,       // 12
    pub recent_count: u16,       // 4 (rows 12..16)
    pub bottom_action_row: u16,  // 17
}

impl OpenRepoLayout {
    pub const fn default_layout() -> Self {
        Self {
            header_row: 0,
            input_row: 2,
            helper_row: 3,
            subdirs_header_row: 4,
            subdirs_start: 5,
            subdirs_count: 6,
            recent_header_row: 11,
            recent_start: 12,
            recent_count: 4,
            bottom_action_row: 17,
        }
    }
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
    pub completion_state: Option<CompletionState>,
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
            subdir_scroll: VirtualScroll::new(0, 6),
            recent_scroll: VirtualScroll::new(0, 4),
            active_section: OpenRepoSection::Input,
            completion_state: None,
        };
        modal.refresh_subdirs();
        modal
    }

    /// Open modal initialized with current directory and recent repository list.
    pub fn open_at(&mut self, current: Option<&Path>, recents: &[String]) {
        self.is_open = true;
        self.completion_state = None;
        if let Some(p) = current {
            self.current_dir = p.to_path_buf();
        } else if let Ok(cwd) = std::env::current_dir() {
            self.current_dir = cwd;
        }
        self.path_input = self.current_dir.display().to_string();
        self.recent_repos = recents.to_vec();
        self.recent_scroll.set_total_items(self.recent_repos.len());
        self.recent_scroll.selected_index = 0;
        self.active_section = OpenRepoSection::Input;
        self.refresh_subdirs();
    }

    /// Close repository switcher.
    pub fn close(&mut self) {
        self.is_open = false;
        self.completion_state = None;
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
            if let Some(home) = dirs::home_dir() {
                return PathBuf::from(trimmed.replacen('~', &home.display().to_string(), 1));
            } else if let Ok(home) = std::env::var("HOME") {
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

    /// Attempt TAB path autocompletion on `path_input`.
    pub fn complete_path(&mut self) {
        let trimmed = self.path_input.trim().to_string();

        // 1. If currently cycling through active candidates
        if let Some(ref mut state) = self.completion_state {
            if !state.candidates.is_empty() {
                state.current_index = (state.current_index + 1) % state.candidates.len();
                self.path_input = state.candidates[state.current_index].clone();
                let resolved = self.resolved_path();
                if resolved.is_dir() {
                    self.current_dir = resolved;
                    self.refresh_subdirs();
                }
                return;
            }
        }

        // 2. Expand tilde for scanning
        let (has_tilde, expanded) = if let Some(stripped) = trimmed.strip_prefix('~') {
            if let Some(home) = dirs::home_dir() {
                (true, format!("{}{}", home.display(), stripped))
            } else {
                (false, trimmed.to_string())
            }
        } else {
            (false, trimmed.to_string())
        };

        // Determine directory to read and prefix to match
        let (search_dir, prefix, base_display) = if expanded.is_empty() {
            (self.current_dir.clone(), String::new(), String::new())
        } else if expanded.ends_with('/') || expanded.ends_with(std::path::MAIN_SEPARATOR) {
            let p = PathBuf::from(&expanded);
            let dir = if p.is_relative() {
                self.current_dir.join(&p)
            } else {
                p
            };
            (dir, String::new(), trimmed.to_string())
        } else {
            let p = Path::new(&expanded);
            let parent = p.parent().unwrap_or(Path::new(""));
            let prefix = p
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .to_string();
            let dir = if parent.as_os_str().is_empty() {
                self.current_dir.clone()
            } else if p.is_relative() {
                self.current_dir.join(parent)
            } else {
                parent.to_path_buf()
            };
            let base_str = if let Some(idx) = trimmed.rfind('/') {
                format!("{}/", &trimmed[..idx])
            } else if has_tilde {
                "~/".to_string()
            } else {
                String::new()
            };
            (dir, prefix, base_str)
        };

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            let mut matches: Vec<(String, bool)> = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') && !prefix.starts_with('.') {
                    continue;
                }
                if name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    matches.push((name, is_dir));
                }
            }

            if matches.is_empty() {
                return;
            }

            // Prefer directories, then alphabetical
            matches.sort_by(|a, b| {
                b.1.cmp(&a.1)
                    .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
            });

            if matches.len() == 1 {
                let (name, is_dir) = &matches[0];
                let suffix = if *is_dir { "/" } else { "" };
                self.path_input = format!("{}{}{}", base_display, name, suffix);
                self.completion_state = None;
                let resolved = self.resolved_path();
                if resolved.is_dir() {
                    self.current_dir = resolved;
                    self.refresh_subdirs();
                }
            } else {
                // Find longest common prefix among matches
                let first_name = &matches[0].0;
                let mut common_len = first_name.len();
                for (name, _) in &matches[1..] {
                    let mut cl = 0;
                    for (c1, c2) in first_name.chars().zip(name.chars()) {
                        if c1.to_lowercase().to_string() == c2.to_lowercase().to_string() {
                            cl += c1.len_utf8();
                        } else {
                            break;
                        }
                    }
                    common_len = common_len.min(cl);
                }
                let common_prefix = &first_name[..common_len];

                let candidates: Vec<String> = matches
                    .iter()
                    .map(|(name, is_dir)| {
                        let suffix = if *is_dir { "/" } else { "" };
                        format!("{}{}{}", base_display, name, suffix)
                    })
                    .collect();

                if common_prefix.len() > prefix.len() {
                    self.path_input = format!("{}{}", base_display, common_prefix);
                    self.completion_state = Some(CompletionState {
                        original_input: trimmed.to_string(),
                        candidates,
                        current_index: 0,
                    });
                } else {
                    self.path_input = candidates[0].clone();
                    self.completion_state = Some(CompletionState {
                        original_input: trimmed.to_string(),
                        candidates,
                        current_index: 0,
                    });
                }
                let resolved = self.resolved_path();
                if resolved.is_dir() {
                    self.current_dir = resolved;
                    self.refresh_subdirs();
                }
            }
        }
    }

    /// Calculate modal bounding box.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 72.min(viewport.width.saturating_sub(4));
        let height = 22.min(viewport.height.saturating_sub(2));
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
            KeyCode::Enter => match self.active_section {
                OpenRepoSection::Input => {
                    let path = self.resolved_path();
                    Some(OpenRepoAction::Open(path))
                }
                OpenRepoSection::Subdirs => {
                    if let Some(dirname) =
                        self.subdirs.get(self.subdir_scroll.selected_index).cloned()
                    {
                        if dirname.starts_with("..") {
                            if let Some(parent) = self.current_dir.parent() {
                                self.current_dir = parent.to_path_buf();
                                self.path_input = self.current_dir.display().to_string();
                                self.refresh_subdirs();
                            }
                        } else {
                            let next = self.current_dir.join(&dirname);
                            if next.is_dir() {
                                self.current_dir = next;
                                self.path_input = self.current_dir.display().to_string();
                                self.refresh_subdirs();
                            }
                        }
                    }
                    None
                }
                OpenRepoSection::Recent => {
                    if let Some(repo_path) =
                        self.recent_repos.get(self.recent_scroll.selected_index)
                    {
                        Some(OpenRepoAction::Open(PathBuf::from(repo_path)))
                    } else {
                        None
                    }
                }
            },
            KeyCode::Tab => {
                match self.active_section {
                    OpenRepoSection::Input => {
                        // TAB autocompletion for path input
                        self.complete_path();
                    }
                    OpenRepoSection::Subdirs => {
                        if self.recent_repos.is_empty() {
                            self.active_section = OpenRepoSection::Input;
                        } else {
                            self.active_section = OpenRepoSection::Recent;
                        }
                    }
                    OpenRepoSection::Recent => {
                        self.active_section = OpenRepoSection::Input;
                    }
                }
                None
            }
            KeyCode::BackTab => {
                self.active_section = match self.active_section {
                    OpenRepoSection::Input => {
                        if self.recent_repos.is_empty() {
                            OpenRepoSection::Subdirs
                        } else {
                            OpenRepoSection::Recent
                        }
                    }
                    OpenRepoSection::Subdirs => OpenRepoSection::Input,
                    OpenRepoSection::Recent => OpenRepoSection::Subdirs,
                };
                None
            }
            KeyCode::Up => {
                match self.active_section {
                    OpenRepoSection::Input => {}
                    OpenRepoSection::Subdirs => {
                        if self.subdir_scroll.selected_index == 0 {
                            self.active_section = OpenRepoSection::Input;
                        } else {
                            self.subdir_scroll.previous();
                        }
                    }
                    OpenRepoSection::Recent => {
                        if self.recent_scroll.selected_index == 0 {
                            self.active_section = OpenRepoSection::Subdirs;
                        } else {
                            self.recent_scroll.previous();
                            if let Some(r) =
                                self.recent_repos.get(self.recent_scroll.selected_index)
                            {
                                self.path_input = r.clone();
                            }
                        }
                    }
                }
                None
            }
            KeyCode::Down => {
                match self.active_section {
                    OpenRepoSection::Input => {
                        self.active_section = OpenRepoSection::Subdirs;
                        self.subdir_scroll.selected_index = 0;
                    }
                    OpenRepoSection::Subdirs => {
                        if self.subdir_scroll.selected_index + 1 >= self.subdirs.len() {
                            if !self.recent_repos.is_empty() {
                                self.active_section = OpenRepoSection::Recent;
                                self.recent_scroll.selected_index = 0;
                                if let Some(r) = self.recent_repos.first() {
                                    self.path_input = r.clone();
                                }
                            }
                        } else {
                            self.subdir_scroll.next();
                        }
                    }
                    OpenRepoSection::Recent => {
                        self.recent_scroll.next();
                        if let Some(r) =
                            self.recent_repos.get(self.recent_scroll.selected_index)
                        {
                            self.path_input = r.clone();
                        }
                    }
                }
                None
            }
            KeyCode::Delete
            | KeyCode::Char('d')
            | KeyCode::Char('x')
                if self.active_section == OpenRepoSection::Recent =>
            {
                if let Some(r) = self.recent_repos.get(self.recent_scroll.selected_index) {
                    return Some(OpenRepoAction::RemoveRecent(r.clone()));
                }
                None
            }
            KeyCode::Backspace => {
                if self.active_section == OpenRepoSection::Input {
                    self.completion_state = None;
                    self.path_input.pop();
                    let resolved = self.resolved_path();
                    if resolved.is_dir() {
                        self.current_dir = resolved;
                        self.refresh_subdirs();
                    }
                }
                None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.active_section == OpenRepoSection::Input {
                    self.completion_state = None;
                    self.path_input.push(c);
                    let resolved = self.resolved_path();
                    if resolved.is_dir() {
                        self.current_dir = resolved;
                        self.refresh_subdirs();
                    }
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
        let layout = OpenRepoLayout::default_layout();

        // 1. Header Buttons: [  Open ] (width-20..width-10), [ ❌ Cancel ] (width-9..)
        if inner_y == layout.header_row {
            if inner_x >= rect.width.saturating_sub(20) && inner_x < rect.width.saturating_sub(10) {
                return Some(OpenRepoAction::Open(self.resolved_path()));
            }
            if inner_x >= rect.width.saturating_sub(10) {
                return Some(OpenRepoAction::Close);
            }
        }

        // 2. Input box / helper row click
        if inner_y == layout.input_row || inner_y == layout.helper_row {
            self.active_section = OpenRepoSection::Input;
            return None;
        }

        // 3. Subdirectories area: exactly subdirs_start .. subdirs_start + subdirs_count
        if inner_y >= layout.subdirs_start && inner_y < layout.subdirs_start + layout.subdirs_count {
            let row_idx = (inner_y - layout.subdirs_start) as usize;
            let target_idx = self.subdir_scroll.viewport_offset + row_idx;
            self.active_section = OpenRepoSection::Subdirs;
            self.subdir_scroll.selected_index = target_idx;

            if let Some(dirname) = self.subdirs.get(target_idx).cloned() {
                if dirname.starts_with("..") {
                    if let Some(parent) = self.current_dir.parent() {
                        self.current_dir = parent.to_path_buf();
                        self.path_input = self.current_dir.display().to_string();
                        self.refresh_subdirs();
                    }
                } else {
                    let next = self.current_dir.join(&dirname);
                    if next.is_dir() {
                        self.current_dir = next;
                        self.path_input = self.current_dir.display().to_string();
                        self.refresh_subdirs();
                    }
                }
            }
            return None;
        }

        // 4. Recent Repositories area: exactly recent_start .. recent_start + recent_count
        if inner_y >= layout.recent_start && inner_y < layout.recent_start + layout.recent_count {
            let row_idx = (inner_y - layout.recent_start) as usize;
            let target_idx = self.recent_scroll.viewport_offset + row_idx;

            if let Some(repo_path) = self.recent_repos.get(target_idx).cloned() {
                // Delete button [ ✕ ] clicked
                let remove_x = rect.width.saturating_sub(8);
                if inner_x >= remove_x {
                    return Some(OpenRepoAction::RemoveRecent(repo_path));
                }

                let was_already_selected = self.active_section == OpenRepoSection::Recent
                    && self.recent_scroll.selected_index == target_idx;

                self.active_section = OpenRepoSection::Recent;
                self.recent_scroll.selected_index = target_idx;
                self.path_input = repo_path.clone();
                self.current_dir = PathBuf::from(&repo_path);
                self.refresh_subdirs();

                // If user clicks the [  Open ] button or clicks an already-selected recent item: open it!
                let open_button_x = rect.width.saturating_sub(20);
                if was_already_selected || (inner_x >= open_button_x && inner_x < remove_x) {
                    return Some(OpenRepoAction::Open(PathBuf::from(repo_path)));
                }
            }
            return None;
        }

        // 5. Bottom Action Bar: row 17 or higher
        if inner_y >= layout.bottom_action_row {
            if inner_x >= 3 && inner_x < 32 {
                return Some(OpenRepoAction::Open(self.resolved_path()));
            }
            if inner_x >= 34 && inner_x < 52 {
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
        let layout = OpenRepoLayout::default_layout();

        // Drop shadow behind open repo modal
        if viewport.width > rect.width + 1 && viewport.height > rect.height + 1 {
            let shadow_rect = Rect {
                x: (rect.x + 1).min(viewport.width.saturating_sub(1)),
                y: (rect.y + 1).min(viewport.height.saturating_sub(1)),
                width: rect.width.min(viewport.width.saturating_sub(rect.x + 1)),
                height: rect.height.min(viewport.height.saturating_sub(rect.y + 1)),
            };
            let shadow_block =
                Block::default().style(Style::default().bg(ratatui::style::Color::Rgb(12, 14, 20)));
            frame.render_widget(shadow_block, shadow_rect);
        }

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

        // Row 0: Header controls
        lines.push(Line::from(vec![
            Span::styled(
                "Enter path or browse directories below:",
                Style::default().fg(theme.dim_fg),
            ),
            Span::raw(" ".repeat((inner.width as usize).saturating_sub(60))),
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

        // Row 1: Empty
        lines.push(Line::from(""));

        // Row 2: Input Box
        let input_style = if self.active_section == OpenRepoSection::Input {
            Style::default()
                .bg(theme.selection_bg)
                .fg(theme.selection_fg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(theme.header_bg).fg(theme.fg)
        };
        let input_display = if self.path_input.is_empty() {
            " (Type path or press Tab to complete) "
        } else {
            self.path_input.as_str()
        };
        lines.push(Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(theme.primary)),
            Span::styled(format!(" {:<56} ", input_display), input_style),
        ]));

        // Row 3: Helper hint bar
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(
                "[ ⇥ Tab: Autocomplete | ↵ Enter: Open | ↓: Subdirectories | Esc: Cancel ]",
                Style::default().fg(theme.dim_fg),
            ),
        ]));

        // Row 4: Subdirectories header
        lines.push(Line::from(Span::styled(
            "── Subdirectories (Click/Enter to enter, '..' to go up) ────────────",
            Style::default().fg(theme.secondary),
        )));

        // Rows 5..11: Subdirectories list (6 rows)
        self.subdir_scroll.set_viewport_height(layout.subdirs_count as usize);
        let visible_subdirs = self.subdir_scroll.visible_range();
        let subdirs_rendered = visible_subdirs.len();
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
                Span::styled(format!("{:<44}", dirname), style),
            ]));
        }

        // Fill remaining rows if subdirs < subdirs_count
        for _ in subdirs_rendered..(layout.subdirs_count as usize) {
            lines.push(Line::from(""));
        }

        // Row 11: Recent Repositories header
        lines.push(Line::from(Span::styled(
            "── Recent Repositories (Click to select, click again or [Open] to load) ──",
            Style::default().fg(theme.info),
        )));

        // Rows 12..16: Recent Repositories list (4 rows)
        self.recent_scroll.set_viewport_height(layout.recent_count as usize);
        let visible_recents = self.recent_scroll.visible_range();
        let recents_rendered = visible_recents.len();
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

            let max_path_w = (inner.width as usize).saturating_sub(24);
            let display_path = if repo_path.len() > max_path_w {
                format!("...{}", &repo_path[repo_path.len().saturating_sub(max_path_w - 3)..])
            } else {
                format!("{:<width$}", repo_path, width = max_path_w)
            };

            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{} ", icons.branch()), style),
                Span::styled(display_path, style),
                Span::raw(" "),
                Span::styled(
                    "[  Open ]",
                    Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled("[ ✕ ]", Style::default().fg(theme.error)),
            ]));
        }

        // Fill remaining rows if recents < recent_count
        for _ in recents_rendered..(layout.recent_count as usize) {
            lines.push(Line::from(""));
        }

        // Row 16: Empty
        lines.push(Line::from(""));

        // Row 17: Bottom action bar
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

        if self.active_section == OpenRepoSection::Input {
            let label_w = 8u16; // " Path:  "
            let text_w = unicode_width::UnicodeWidthStr::width(self.path_input.as_str()) as u16;
            let cur_x = (inner.x + label_w + text_w).min(inner.x + inner.width.saturating_sub(1));
            frame.set_cursor_position((cur_x, inner.y + layout.input_row));
        }
    }
}
