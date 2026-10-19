//! Repository directory switcher modal.
//! Supports two-pane layout:
//! - Left Pane: Repository Discovery (scans for .git repositories, parent/current dir)
//! - Right Pane: Recent Repositories (clean name-only display) & Direct Path Input with Tab completion
//! - Actions: Git Init and Git Clone with dedicated URL prompt.

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
    InitRepo(PathBuf),
    CloneRepo { url: String, target_dir: PathBuf },
}

/// Discovered repository or directory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredRepo {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_parent: bool,
    pub is_current: bool,
    pub is_git: bool,
}

/// Completion state when user cycles through multiple TAB matches.
#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    pub original_input: String,
    pub candidates: Vec<String>,
    pub current_index: usize,
}

/// Row and pane layout mapping for OpenRepoModal inner viewport.
#[derive(Debug, Clone, Copy)]
pub struct OpenRepoLayout {
    pub header_row: u16,         // 0
    pub input_row: u16,          // 2
    pub helper_row: u16,         // 3
    pub subdirs_header_row: u16, // 4
    pub subdirs_start: u16,      // 5
    pub subdirs_count: u16,      // 6
    pub recent_header_row: u16,  // 11
    pub recent_start: u16,       // 12
    pub recent_count: u16,       // 4
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

/// Calculated dynamic row and column layout mapping for two-pane OpenRepoModal.
#[derive(Debug, Clone, Copy)]
pub struct OpenRepoLayoutMap {
    pub inner_w: u16,
    pub inner_h: u16,
    pub split_x: u16,
    pub left_w: u16,
    pub right_w: u16,
    pub content_top: u16,
    pub content_rows: u16,
    pub discovery_visible_count: u16,
    pub recent_visible_count: u16,

    // Row positions in inner_y:
    pub header_y: u16,
    pub separator_y: u16,

    // Left Pane
    pub discovery_header_y: u16,
    pub discovery_list_start_y: u16,
    pub discovery_footer_y: u16,

    // Right Pane (Normal)
    pub recent_header_y: u16,
    pub recent_list_start_y: u16,
    pub recent_preview_y: u16,
    pub path_header_y: u16,
    pub path_input_y: u16,
    pub path_helper_y: u16,
    pub actions_y: u16,

    // Right Pane (Clone Mode)
    pub clone_header_y: u16,
    pub clone_target_y: u16,
    pub clone_url_label_y: u16,
    pub clone_url_input_y: u16,
    pub clone_helper_y: u16,
    pub clone_actions_y: u16,
}

impl OpenRepoLayoutMap {
    pub fn compute(inner: Rect) -> Self {
        let inner_w = inner.width;
        let inner_h = inner.height;
        // Allocate generous width for right pane (which has path input, actions, key guides, preview)
        // while left pane (discovery) comfortably needs around 42-44 columns.
        let left_w = if inner_w >= 92 {
            (44u16).min(inner_w.saturating_sub(56)).max(38)
        } else {
            (inner_w / 2).saturating_sub(1)
        };
        let split_x = left_w;
        let right_w = inner_w.saturating_sub(split_x + 1);

        let content_top = 2u16;
        let content_rows = inner_h.saturating_sub(content_top);
        let discovery_visible_count = content_rows.saturating_sub(2).max(1);
        let recent_visible_count = (6u16).min(content_rows.saturating_sub(8));

        Self {
            inner_w,
            inner_h,
            split_x,
            left_w,
            right_w,
            content_top,
            content_rows,
            discovery_visible_count,
            recent_visible_count,
            header_y: 0,
            separator_y: 1,
            discovery_header_y: content_top,
            discovery_list_start_y: content_top + 1,
            discovery_footer_y: content_top + content_rows.saturating_sub(1),
            recent_header_y: content_top,
            recent_list_start_y: content_top + 1,
            recent_preview_y: content_top + recent_visible_count + 1,
            path_header_y: content_top + recent_visible_count + 3,
            path_input_y: content_top + recent_visible_count + 4,
            path_helper_y: content_top + recent_visible_count + 5,
            actions_y: content_top + recent_visible_count + 7,
            clone_header_y: content_top,
            clone_target_y: content_top + 2,
            clone_url_label_y: content_top + 4,
            clone_url_input_y: content_top + 5,
            clone_helper_y: content_top + 7,
            clone_actions_y: content_top + 9,
        }
    }
}

/// Active section for user focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRepoSection {
    Discovery,
    Subdirs, // Backwards compatible alias for Discovery
    Recent,
    Input,
    CloneInput,
}

/// Open repository modal state.
pub struct OpenRepoModal {
    pub is_open: bool,
    pub path_input: String,
    pub clone_url_input: String,
    pub is_clone_mode: bool,
    pub current_dir: PathBuf,
    pub discovered_repos: Vec<DiscoveredRepo>,
    pub subdirs: Vec<String>, // Backwards compatibility for tests & callers
    pub recent_repos: Vec<String>,
    pub discovery_scroll: VirtualScroll,
    pub subdir_scroll: VirtualScroll, // Backwards compatibility
    pub recent_scroll: VirtualScroll,
    pub active_section: OpenRepoSection,
    pub completion_state: Option<CompletionState>,
    pub show_all_dirs: bool,
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
            clone_url_input: String::new(),
            is_clone_mode: false,
            current_dir: cur,
            discovered_repos: Vec::new(),
            subdirs: Vec::new(),
            recent_repos: Vec::new(),
            discovery_scroll: VirtualScroll::new(0, 14),
            subdir_scroll: VirtualScroll::new(0, 6),
            recent_scroll: VirtualScroll::new(0, 7),
            active_section: OpenRepoSection::Discovery,
            completion_state: None,
            show_all_dirs: false,
        };
        modal.refresh_discovery();
        modal
    }

    /// Open modal initialized with current directory and recent repository list.
    pub fn open_at(&mut self, current: Option<&Path>, recents: &[String]) {
        self.is_open = true;
        self.is_clone_mode = false;
        self.clone_url_input.clear();
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
        self.refresh_discovery();
    }

    /// Close repository switcher.
    pub fn close(&mut self) {
        self.is_open = false;
        self.is_clone_mode = false;
        self.completion_state = None;
    }

    /// Refresh repository discovery in `current_dir`.
    pub fn refresh_discovery(&mut self) {
        let mut list = Vec::new();

        // 1. Parent Directory
        if let Some(parent) = self.current_dir.parent() {
            list.push(DiscoveredRepo {
                name: ".. (Parent Directory)".to_string(),
                path: parent.to_path_buf(),
                branch: None,
                is_parent: true,
                is_current: false,
                is_git: parent.join(".git").exists(),
            });
        }

        // 2. Current Directory entry (allows opening/init current non-git or git folder)
        let cur_name = self
            .current_dir
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let cur_is_git = self.current_dir.join(".git").exists();
        let cur_branch = if cur_is_git {
            read_branch_from_head(&self.current_dir)
        } else {
            None
        };
        list.push(DiscoveredRepo {
            name: format!(". (Current: {})", cur_name),
            path: self.current_dir.clone(),
            branch: cur_branch,
            is_parent: false,
            is_current: true,
            is_git: cur_is_git,
        });

        // 3. Scan subdirectories in current_dir
        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut repos = Vec::new();
            let mut other_dirs = Vec::new();

            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if !ft.is_dir() {
                        continue;
                    }
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.')
                        || file_name == "target"
                        || file_name == "node_modules"
                        || file_name == ".cargo"
                    {
                        continue;
                    }

                    let full_path = entry.path();
                    let is_git = full_path.join(".git").exists();
                    let branch = if is_git {
                        read_branch_from_head(&full_path)
                    } else {
                        None
                    };

                    let repo = DiscoveredRepo {
                        name: file_name,
                        path: full_path.clone(),
                        branch,
                        is_parent: false,
                        is_current: false,
                        is_git,
                    };

                    if is_git {
                        repos.push(repo);
                    } else {
                        // Check 1 level deeper for git repos
                        let mut found_nested = false;
                        if let Ok(nested_entries) = std::fs::read_dir(&full_path) {
                            for n_entry in nested_entries.flatten() {
                                if let Ok(nft) = n_entry.file_type() {
                                    if nft.is_dir() {
                                        let n_name =
                                            n_entry.file_name().to_string_lossy().to_string();
                                        if !n_name.starts_with('.')
                                            && n_name != "target"
                                            && n_name != "node_modules"
                                        {
                                            let n_path = n_entry.path();
                                            if n_path.join(".git").exists() {
                                                found_nested = true;
                                                repos.push(DiscoveredRepo {
                                                    name: format!("{}/{}", repo.name, n_name),
                                                    path: n_path.clone(),
                                                    branch: read_branch_from_head(&n_path),
                                                    is_parent: false,
                                                    is_current: false,
                                                    is_git: true,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if !found_nested {
                            other_dirs.push(repo);
                        }
                    }
                }
            }

            repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            list.extend(repos);

            if self.show_all_dirs {
                other_dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                list.extend(other_dirs);
            }
        }

        self.discovery_scroll.set_total_items(list.len());
        self.subdirs = list.iter().map(|r| r.name.clone()).collect();
        self.subdir_scroll.set_total_items(self.subdirs.len());
        self.discovered_repos = list;
    }

    /// Refresh directory listing in current_dir (backwards compatibility).
    pub fn refresh_subdirs(&mut self) {
        self.refresh_discovery();
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
                    self.refresh_discovery();
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
                    self.refresh_discovery();
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
                    self.refresh_discovery();
                }
            }
        }
    }

    /// Calculate modal bounding box.
    pub fn modal_rect(&self, viewport: Rect) -> Rect {
        let width = 104.min(viewport.width.saturating_sub(2));
        let height = 26.min(viewport.height.saturating_sub(2));
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
        // Clone URL Input Mode handling
        if self.is_clone_mode {
            match key.code {
                KeyCode::Esc => {
                    self.is_clone_mode = false;
                    self.clone_url_input.clear();
                    self.active_section = OpenRepoSection::Input;
                    return None;
                }
                KeyCode::Enter => {
                    let trimmed = self.clone_url_input.trim();
                    if !trimmed.is_empty() {
                        let target_dir = self.resolved_path();
                        let action = OpenRepoAction::CloneRepo {
                            url: trimmed.to_string(),
                            target_dir,
                        };
                        self.is_clone_mode = false;
                        self.clone_url_input.clear();
                        return Some(action);
                    }
                    return None;
                }
                KeyCode::Backspace => {
                    self.clone_url_input.pop();
                    return None;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.clone_url_input.push(c);
                    return None;
                }
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Esc => Some(OpenRepoAction::Close),
            KeyCode::Enter => match self.active_section {
                OpenRepoSection::Input => {
                    let path = self.resolved_path();
                    Some(OpenRepoAction::Open(path))
                }
                OpenRepoSection::Discovery | OpenRepoSection::Subdirs => {
                    if let Some(entry) = self
                        .discovered_repos
                        .get(self.discovery_scroll.selected_index)
                        .cloned()
                    {
                        if entry.is_parent {
                            if let Some(parent) = self.current_dir.parent() {
                                self.current_dir = parent.to_path_buf();
                                self.path_input = self.current_dir.display().to_string();
                                self.refresh_discovery();
                            }
                            None
                        } else if entry.is_current {
                            Some(OpenRepoAction::Open(entry.path))
                        } else if entry.is_git {
                            Some(OpenRepoAction::Open(entry.path))
                        } else if entry.path.is_dir() {
                            // Subdirectory without git - enter it
                            self.current_dir = entry.path;
                            self.path_input = self.current_dir.display().to_string();
                            self.refresh_discovery();
                            None
                        } else {
                            None
                        }
                    } else {
                        None
                    }
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
                OpenRepoSection::CloneInput => None,
            },
            KeyCode::Tab => {
                match self.active_section {
                    OpenRepoSection::Input => {
                        // TAB autocompletion for path input
                        self.complete_path();
                    }
                    OpenRepoSection::Discovery | OpenRepoSection::Subdirs => {
                        if !self.recent_repos.is_empty() {
                            self.active_section = OpenRepoSection::Recent;
                        } else {
                            self.active_section = OpenRepoSection::Input;
                        }
                    }
                    OpenRepoSection::Recent => {
                        self.active_section = OpenRepoSection::Input;
                    }
                    OpenRepoSection::CloneInput => {
                        self.active_section = OpenRepoSection::Input;
                    }
                }
                None
            }
            KeyCode::BackTab => {
                self.active_section = match self.active_section {
                    OpenRepoSection::Input => {
                        if !self.recent_repos.is_empty() {
                            OpenRepoSection::Recent
                        } else {
                            OpenRepoSection::Discovery
                        }
                    }
                    OpenRepoSection::Recent => OpenRepoSection::Discovery,
                    OpenRepoSection::Discovery | OpenRepoSection::Subdirs => OpenRepoSection::Input,
                    OpenRepoSection::CloneInput => OpenRepoSection::Input,
                };
                None
            }
            KeyCode::Left => {
                // Move from right pane to left pane (Discovery)
                if self.active_section == OpenRepoSection::Recent
                    || self.active_section == OpenRepoSection::Input
                {
                    self.active_section = OpenRepoSection::Discovery;
                }
                None
            }
            KeyCode::Right => {
                // Move from left pane to right pane (Recent or Input)
                if self.active_section == OpenRepoSection::Discovery
                    || self.active_section == OpenRepoSection::Subdirs
                {
                    if !self.recent_repos.is_empty() {
                        self.active_section = OpenRepoSection::Recent;
                    } else {
                        self.active_section = OpenRepoSection::Input;
                    }
                }
                None
            }
            KeyCode::Up => {
                match self.active_section {
                    OpenRepoSection::Input => {
                        if !self.recent_repos.is_empty() {
                            self.active_section = OpenRepoSection::Recent;
                        } else {
                            self.active_section = OpenRepoSection::Discovery;
                        }
                    }
                    OpenRepoSection::Discovery | OpenRepoSection::Subdirs => {
                        self.discovery_scroll.previous();
                        self.subdir_scroll.selected_index = self.discovery_scroll.selected_index;
                    }
                    OpenRepoSection::Recent => {
                        self.recent_scroll.previous();
                        if let Some(r) = self.recent_repos.get(self.recent_scroll.selected_index) {
                            self.path_input = r.clone();
                        }
                    }
                    OpenRepoSection::CloneInput => {}
                }
                None
            }
            KeyCode::Down => {
                match self.active_section {
                    OpenRepoSection::Input => {
                        self.active_section = OpenRepoSection::Discovery;
                        self.discovery_scroll.selected_index = 0;
                    }
                    OpenRepoSection::Discovery | OpenRepoSection::Subdirs => {
                        self.discovery_scroll.next();
                        self.subdir_scroll.selected_index = self.discovery_scroll.selected_index;
                    }
                    OpenRepoSection::Recent => {
                        if self.recent_scroll.selected_index + 1 >= self.recent_repos.len() {
                            self.active_section = OpenRepoSection::Input;
                        } else {
                            self.recent_scroll.next();
                            if let Some(r) =
                                self.recent_repos.get(self.recent_scroll.selected_index)
                            {
                                self.path_input = r.clone();
                            }
                        }
                    }
                    OpenRepoSection::CloneInput => {}
                }
                None
            }
            KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('x')
                if self.active_section == OpenRepoSection::Recent =>
            {
                if let Some(r) = self.recent_repos.get(self.recent_scroll.selected_index) {
                    return Some(OpenRepoAction::RemoveRecent(r.clone()));
                }
                None
            }
            // Shortcut 'c' (when not typing in Input): Open Clone prompt
            KeyCode::Char('c') if self.active_section != OpenRepoSection::Input => {
                self.is_clone_mode = true;
                self.clone_url_input.clear();
                self.active_section = OpenRepoSection::CloneInput;
                None
            }
            // Shortcut 'i' (when not typing in Input): Init repo here
            KeyCode::Char('i') if self.active_section != OpenRepoSection::Input => {
                Some(OpenRepoAction::InitRepo(self.resolved_path()))
            }
            // Shortcut 'a' (when not typing in Input): Toggle all dirs vs git only
            KeyCode::Char('a') if self.active_section != OpenRepoSection::Input => {
                self.show_all_dirs = !self.show_all_dirs;
                self.refresh_discovery();
                None
            }
            KeyCode::Backspace => {
                if self.active_section == OpenRepoSection::Input {
                    self.completion_state = None;
                    self.path_input.pop();
                    let resolved = self.resolved_path();
                    if resolved.is_dir() {
                        self.current_dir = resolved;
                        self.refresh_discovery();
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
                        self.refresh_discovery();
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Handle mouse wheel scroll up
    pub fn scroll_up(&mut self, x: u16, _y: u16, viewport: Rect) {
        let rect = self.modal_rect(viewport);
        let inner_rect = Rect {
            x: rect.x + 1,
            y: rect.y + 1,
            width: rect.width.saturating_sub(2),
            height: rect.height.saturating_sub(2),
        };
        let layout = OpenRepoLayoutMap::compute(inner_rect);
        let inner_x = x.saturating_sub(rect.x + 1);

        if inner_x < layout.split_x {
            self.discovery_scroll.previous();
            self.subdir_scroll.selected_index = self.discovery_scroll.selected_index;
        } else {
            self.recent_scroll.previous();
        }
    }

    /// Handle mouse wheel scroll down
    pub fn scroll_down(&mut self, x: u16, _y: u16, viewport: Rect) {
        let rect = self.modal_rect(viewport);
        let inner_rect = Rect {
            x: rect.x + 1,
            y: rect.y + 1,
            width: rect.width.saturating_sub(2),
            height: rect.height.saturating_sub(2),
        };
        let layout = OpenRepoLayoutMap::compute(inner_rect);
        let inner_x = x.saturating_sub(rect.x + 1);

        if inner_x < layout.split_x {
            self.discovery_scroll.next();
            self.subdir_scroll.selected_index = self.discovery_scroll.selected_index;
        } else {
            self.recent_scroll.next();
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
        let inner_rect = Rect {
            x: rect.x + 1,
            y: rect.y + 1,
            width: rect.width.saturating_sub(2),
            height: rect.height.saturating_sub(2),
        };
        let layout = OpenRepoLayoutMap::compute(inner_rect);

        // 1. Top Header Row (inner_y == layout.header_y)
        if inner_y == layout.header_y {
            // [  Open ] button: inner_w - 25 .. inner_w - 12
            if inner_x >= layout.inner_w.saturating_sub(25)
                && inner_x < layout.inner_w.saturating_sub(12)
            {
                return Some(OpenRepoAction::Open(self.resolved_path()));
            }
            // [ ✕ Cancel ] button: inner_w - 12 .. inner_w
            if inner_x >= layout.inner_w.saturating_sub(12) {
                return Some(OpenRepoAction::Close);
            }
            return None;
        }

        // 2. Left Pane (Repository Discovery): inner_x < layout.split_x
        if inner_x < layout.split_x {
            // Header: inner_y == layout.discovery_header_y
            if inner_y == layout.discovery_header_y {
                self.active_section = OpenRepoSection::Discovery;
                // Click on [a: All / g: Git] toggle at right end of left pane
                if inner_x >= layout.split_x.saturating_sub(16) {
                    self.show_all_dirs = !self.show_all_dirs;
                    self.refresh_discovery();
                }
                return None;
            }

            // List rows: discovery_list_start_y .. discovery_list_start_y + discovery_visible_count
            if inner_y >= layout.discovery_list_start_y
                && inner_y < layout.discovery_list_start_y + layout.discovery_visible_count
            {
                let row_idx = (inner_y - layout.discovery_list_start_y) as usize;
                let target_idx = self.discovery_scroll.viewport_offset + row_idx;

                if let Some(entry) = self.discovered_repos.get(target_idx).cloned() {
                    let was_selected = (self.active_section == OpenRepoSection::Discovery
                        || self.active_section == OpenRepoSection::Subdirs)
                        && self.discovery_scroll.selected_index == target_idx;

                    self.active_section = OpenRepoSection::Discovery;
                    self.discovery_scroll.selected_index = target_idx;
                    self.subdir_scroll.selected_index = target_idx;

                    if entry.is_parent {
                        if let Some(parent) = self.current_dir.parent() {
                            self.current_dir = parent.to_path_buf();
                            self.path_input = self.current_dir.display().to_string();
                            self.refresh_discovery();
                        }
                    } else if entry.is_current || entry.is_git {
                        self.path_input = entry.path.display().to_string();
                        if was_selected {
                            return Some(OpenRepoAction::Open(entry.path));
                        }
                    } else if entry.path.is_dir() {
                        self.current_dir = entry.path;
                        self.path_input = self.current_dir.display().to_string();
                        self.refresh_discovery();
                    }
                }
                return None;
            }

            // Footer row: inner_y == layout.discovery_footer_y
            if inner_y == layout.discovery_footer_y {
                self.active_section = OpenRepoSection::Discovery;
                return None;
            }
        }

        // 3. Right Pane: inner_x > layout.split_x
        if inner_x > layout.split_x {
            let right_inner_x = inner_x - (layout.split_x + 1);
            let right_w = layout.right_w;

            // Clone mode handling
            if self.is_clone_mode {
                // URL input row
                if inner_y == layout.clone_url_input_y || inner_y == layout.clone_url_label_y {
                    self.active_section = OpenRepoSection::CloneInput;
                    return None;
                }
                // Action buttons row (layout.clone_actions_y)
                if inner_y == layout.clone_actions_y {
                    // [  Start Clone ]: x 1..22
                    if right_inner_x >= 1 && right_inner_x <= 22 {
                        let trimmed = self.clone_url_input.trim();
                        if !trimmed.is_empty() {
                            let target_dir = self.resolved_path();
                            let act = OpenRepoAction::CloneRepo {
                                url: trimmed.to_string(),
                                target_dir,
                            };
                            self.is_clone_mode = false;
                            self.clone_url_input.clear();
                            return Some(act);
                        }
                    }
                    // [ ❌ Cancel ]: x 23..38
                    if right_inner_x >= 23 && right_inner_x <= 38 {
                        self.is_clone_mode = false;
                        self.clone_url_input.clear();
                        self.active_section = OpenRepoSection::Input;
                    }
                }
                return None;
            }

            // Normal mode: Top section - Recent Repositories
            if inner_y == layout.recent_header_y {
                self.active_section = OpenRepoSection::Recent;
                return None;
            }

            if inner_y >= layout.recent_list_start_y
                && inner_y < layout.recent_list_start_y + layout.recent_visible_count
            {
                let row_idx = (inner_y - layout.recent_list_start_y) as usize;
                let target_idx = self.recent_scroll.viewport_offset + row_idx;

                if let Some(repo_path) = self.recent_repos.get(target_idx).cloned() {
                    // Delete button [ ✕ ] at right end (right_w - 7 .. right_w)
                    let remove_x = right_w.saturating_sub(7);
                    if right_inner_x >= remove_x {
                        return Some(OpenRepoAction::RemoveRecent(repo_path));
                    }

                    // [  Open ] button: right_w - 19 .. right_w - 8
                    let open_button_x = right_w.saturating_sub(19);
                    let is_open_btn = right_inner_x >= open_button_x && right_inner_x < remove_x;

                    let was_already_selected = self.active_section == OpenRepoSection::Recent
                        && self.recent_scroll.selected_index == target_idx;

                    self.active_section = OpenRepoSection::Recent;
                    self.recent_scroll.selected_index = target_idx;
                    self.path_input = repo_path.clone();

                    if is_open_btn || was_already_selected {
                        return Some(OpenRepoAction::Open(PathBuf::from(repo_path)));
                    }
                }
                return None;
            }

            // Normal mode: Bottom section - Direct Path Input
            // Path input box row: layout.path_input_y (or header / helper)
            if inner_y >= layout.path_header_y && inner_y <= layout.path_helper_y {
                self.active_section = OpenRepoSection::Input;
                return None;
            }

            // Normal mode: Action buttons row: layout.actions_y
            if inner_y == layout.actions_y || inner_y == layout.actions_y + 1 {
                // [ i ] Init Repo Here: right_inner_x 1..24
                if right_inner_x >= 1 && right_inner_x <= 24 {
                    return Some(OpenRepoAction::InitRepo(self.resolved_path()));
                }
                // [ c ] Clone Remote Here: right_inner_x 25..52
                if right_inner_x >= 25 && right_inner_x <= 52 {
                    self.is_clone_mode = true;
                    self.clone_url_input.clear();
                    self.active_section = OpenRepoSection::CloneInput;
                    return None;
                }
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

        let layout = OpenRepoLayoutMap::compute(inner);
        let left_w = layout.left_w;
        let right_w = layout.right_w;

        let mut lines = Vec::new();

        // ─────────────────────────────────────────────────────────────
        // Row 0: Top Header with Current Directory & Quick Action Buttons
        // ─────────────────────────────────────────────────────────────
        let cur_display = self.current_dir.display().to_string();
        let buttons_total_w = 25usize; // "[  Open ] " (12) + " " (1) + "[ ✕ Cancel ]" (12)
        let label_prefix = " Location: ";
        let label_w = label_prefix.len();
        let max_cur_w = (layout.inner_w as usize).saturating_sub(label_w + buttons_total_w + 2);
        let short_cur = if cur_display.len() > max_cur_w {
            format!(
                "...{}",
                &cur_display[cur_display
                    .len()
                    .saturating_sub(max_cur_w.saturating_sub(3))..]
            )
        } else {
            cur_display
        };

        let pad_w =
            (layout.inner_w as usize).saturating_sub(label_w + short_cur.len() + buttons_total_w);

        lines.push(Line::from(vec![
            Span::styled(label_prefix, Style::default().fg(theme.dim_fg)),
            Span::styled(
                short_cur,
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(pad_w)),
            Span::styled(
                " [  Open ] ",
                Style::default()
                    .bg(theme.primary)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                "[ ✕ Cancel ]",
                Style::default()
                    .bg(theme.header_bg)
                    .fg(theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Row 1: Header separator
        lines.push(Line::from(Span::styled(
            "─".repeat(layout.inner_w as usize),
            Style::default().fg(theme.border),
        )));

        // ─────────────────────────────────────────────────────────────
        // Rows 2..: Two-Pane Split Layout
        // Left Pane: Discovery List
        // Right Pane: Recent Repositories (top) + Direct Path & Clone / Init (bottom)
        // ─────────────────────────────────────────────────────────────
        self.discovery_scroll
            .set_viewport_height(layout.discovery_visible_count as usize);
        self.recent_scroll
            .set_viewport_height(layout.recent_visible_count as usize);

        for row_i in 0..layout.content_rows {
            let mut row_spans = Vec::new();

            // ── Left Pane Column ──
            if row_i == 0 {
                // Left Header
                let toggle_txt = if self.show_all_dirs {
                    "[a: All Dirs]"
                } else {
                    "[g: Git Only]"
                };
                let title = " 🔍 Repository Discovery ";
                let pad = (left_w as usize).saturating_sub(title.len() + toggle_txt.len() + 1);
                row_spans.push(Span::styled(
                    title,
                    Style::default()
                        .fg(theme.secondary)
                        .add_modifier(Modifier::BOLD),
                ));
                row_spans.push(Span::raw(" ".repeat(pad)));
                row_spans.push(Span::styled(toggle_txt, Style::default().fg(theme.info)));
            } else if row_i <= layout.discovery_visible_count {
                let disc_idx =
                    (row_i.saturating_sub(1)) as usize + self.discovery_scroll.viewport_offset;
                if let Some(item) = self.discovered_repos.get(disc_idx) {
                    let is_sel = disc_idx == self.discovery_scroll.selected_index
                        && (self.active_section == OpenRepoSection::Discovery
                            || self.active_section == OpenRepoSection::Subdirs);

                    let (icon, branch_info) = if item.is_parent {
                        (icons.arrow_up(), String::new())
                    } else if item.is_current {
                        let b_str = item.branch.as_deref().unwrap_or(if item.is_git {
                            "git"
                        } else {
                            "non-git"
                        });
                        ("", format!("({})", b_str))
                    } else if item.is_git {
                        let b_str = item.branch.as_deref().unwrap_or("git");
                        ("", format!("({})", b_str))
                    } else {
                        (icons.folder_closed(), "(dir)".to_string())
                    };

                    let style = if is_sel {
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.selection_fg)
                            .add_modifier(Modifier::BOLD)
                    } else if item.is_git {
                        Style::default().fg(theme.fg)
                    } else {
                        Style::default().fg(theme.dim_fg)
                    };

                    let branch_style = if is_sel {
                        style
                    } else {
                        Style::default().fg(theme.dim_fg)
                    };

                    let name_max_w = (left_w as usize).saturating_sub(branch_info.len() + 6);
                    let display_name = if item.name.len() > name_max_w {
                        format!("{}...", &item.name[..name_max_w.saturating_sub(3)])
                    } else {
                        item.name.clone()
                    };

                    let line_w = 4 + display_name.len() + branch_info.len();
                    let pad = (left_w as usize).saturating_sub(line_w);

                    row_spans.push(Span::styled(format!(" {} ", icon), style));
                    row_spans.push(Span::styled(display_name, style));
                    row_spans.push(Span::raw(" ".repeat(pad)));
                    if !branch_info.is_empty() {
                        row_spans.push(Span::styled(branch_info, branch_style));
                    }
                    row_spans.push(Span::raw(" "));
                } else {
                    row_spans.push(Span::raw(" ".repeat(left_w as usize)));
                }
            } else if row_i == layout.content_rows.saturating_sub(1) {
                // Discovery footer status / indicator
                let count_str = format!(
                    " [ ▲ {}/{} repos ▼ ] ",
                    (self.discovery_scroll.selected_index + 1).min(self.discovered_repos.len()),
                    self.discovered_repos.len()
                );
                let count_len = count_str.len();
                let pad = (left_w as usize).saturating_sub(count_len);
                row_spans.push(Span::raw(" ".repeat(pad / 2)));
                row_spans.push(Span::styled(count_str, Style::default().fg(theme.dim_fg)));
                row_spans.push(Span::raw(
                    " ".repeat((left_w as usize).saturating_sub(pad / 2 + count_len)),
                ));
            } else {
                row_spans.push(Span::raw(" ".repeat(left_w as usize)));
            }

            // ── Pane Vertical Border ──
            row_spans.push(Span::styled("│", Style::default().fg(theme.border)));

            // ── Right Pane Column ──
            if self.is_clone_mode {
                // Right Pane: Clone URL input mode
                if row_i == 0 {
                    row_spans.push(Span::styled(
                        "  Clone Remote Repository ",
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else if row_i == 2 {
                    let target_display = format!(" Target Dir: {}", self.resolved_path().display());
                    row_spans.push(Span::styled(
                        target_display,
                        Style::default().fg(theme.dim_fg),
                    ));
                } else if row_i == 4 {
                    row_spans.push(Span::styled(
                        " Remote URL (https://... or git@...): ",
                        Style::default().fg(theme.info),
                    ));
                } else if row_i == 5 {
                    let url_disp = if self.clone_url_input.is_empty() {
                        " (Type remote repository URL to clone) "
                    } else {
                        &self.clone_url_input
                    };
                    row_spans.push(Span::styled(
                        format!(
                            " {:<width$} ",
                            url_disp,
                            width = (right_w as usize).saturating_sub(4)
                        ),
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.selection_fg)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else if row_i == 7 {
                    row_spans.push(Span::styled(
                        " [ ↵ Enter: Start Clone | ⎋ Esc: Cancel ]",
                        Style::default().fg(theme.dim_fg),
                    ));
                } else if row_i == 9 {
                    row_spans.push(Span::styled(
                        " [  Start Clone ] ",
                        Style::default()
                            .bg(theme.primary)
                            .fg(theme.bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                    row_spans.push(Span::raw("   "));
                    row_spans.push(Span::styled(
                        " [ ❌ Cancel ] ",
                        Style::default().fg(theme.error),
                    ));
                }
            } else {
                // Right Pane: Standard mode (Recent top + Path/Init/Clone bottom)
                if row_i == 0 {
                    // Recent Repositories Header
                    row_spans.push(Span::styled(
                        " 🕒 Recent Repositories (clean name) ",
                        Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
                    ));
                } else if row_i <= layout.recent_visible_count {
                    let recent_idx =
                        (row_i.saturating_sub(1)) as usize + self.recent_scroll.viewport_offset;
                    if let Some(repo_path) = self.recent_repos.get(recent_idx) {
                        let is_sel = recent_idx == self.recent_scroll.selected_index
                            && self.active_section == OpenRepoSection::Recent;

                        // Display clean repository name (last component of path)
                        let clean_name = Path::new(repo_path)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| repo_path.clone());

                        let style = if is_sel {
                            Style::default()
                                .bg(theme.selection_bg)
                                .fg(theme.selection_fg)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.fg)
                        };

                        let btn_w = 17; // "[  Open ] [ ✕ ]" (10 + 1 + 5 + 1)
                        let prefix = "  ";
                        let max_name_w =
                            (right_w as usize).saturating_sub(prefix.len() + btn_w + 2);
                        let disp_name = if clean_name.len() > max_name_w {
                            format!("{}...", &clean_name[..max_name_w.saturating_sub(3)])
                        } else {
                            clean_name
                        };

                        let pad = (right_w as usize)
                            .saturating_sub(prefix.len() + disp_name.len() + btn_w);

                        row_spans.push(Span::styled(format!("{}{}", prefix, disp_name), style));
                        row_spans.push(Span::raw(" ".repeat(pad)));
                        row_spans.push(Span::styled(
                            "[  Open ]",
                            Style::default()
                                .fg(theme.primary)
                                .add_modifier(Modifier::BOLD),
                        ));
                        row_spans.push(Span::raw(" "));
                        row_spans.push(Span::styled("[ ✕ ]", Style::default().fg(theme.error)));
                    }
                } else if row_i == layout.recent_visible_count + 1 {
                    // Preview of selected recent repo full path
                    if let Some(sel_path) = self.recent_repos.get(self.recent_scroll.selected_index)
                    {
                        let max_p = (right_w as usize).saturating_sub(12);
                        let short_p = if sel_path.len() > max_p {
                            format!(
                                "...{}",
                                &sel_path[sel_path.len().saturating_sub(max_p - 3)..]
                            )
                        } else {
                            sel_path.clone()
                        };
                        row_spans.push(Span::styled(
                            format!(" Selected: {}", short_p),
                            Style::default().fg(theme.dim_fg),
                        ));
                    }
                } else if row_i == layout.recent_visible_count + 2 {
                    // Separator for Direct Path & Actions
                    row_spans.push(Span::styled(
                        format!("─{}", "─".repeat((right_w as usize).saturating_sub(1))),
                        Style::default().fg(theme.border),
                    ));
                } else if row_i == layout.recent_visible_count + 3 {
                    row_spans.push(Span::styled(
                        "  Direct Path / Clone / Init ",
                        Style::default()
                            .fg(theme.secondary)
                            .add_modifier(Modifier::BOLD),
                    ));
                } else if row_i == layout.recent_visible_count + 4 {
                    // Path input field
                    let input_style = if self.active_section == OpenRepoSection::Input {
                        Style::default()
                            .bg(theme.selection_bg)
                            .fg(theme.selection_fg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(theme.header_bg).fg(theme.fg)
                    };
                    let input_display = if self.path_input.is_empty() {
                        "(Type path or Tab to autocomplete)"
                    } else {
                        self.path_input.as_str()
                    };
                    let box_w = (right_w as usize).saturating_sub(10);
                    let clipped_input = if input_display.len() > box_w {
                        &input_display[input_display.len().saturating_sub(box_w)..]
                    } else {
                        input_display
                    };
                    row_spans.push(Span::styled(" Path: ", Style::default().fg(theme.primary)));
                    row_spans.push(Span::styled(
                        format!(" {:<width$} ", clipped_input, width = box_w),
                        input_style,
                    ));
                } else if row_i == layout.recent_visible_count + 5 {
                    let help_txt = if right_w >= 54 {
                        "  [ ⇥ Tab: Autocomplete | ↵ Enter: Open | Esc: Close ]"
                    } else {
                        "  [ ⇥ Tab: Complete | ↵ Open | Esc: Close ]"
                    };
                    row_spans.push(Span::styled(help_txt, Style::default().fg(theme.dim_fg)));
                } else if row_i == layout.recent_visible_count + 7 {
                    // Action buttons: Init Repo Here / Clone Remote Here
                    row_spans.push(Span::raw(" "));
                    row_spans.push(Span::styled(
                        " [ i ] Init Repo Here ",
                        Style::default()
                            .bg(theme.status_bg)
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                    ));
                    row_spans.push(Span::raw(" "));
                    row_spans.push(Span::styled(
                        " [ c ] Clone Remote Here ",
                        Style::default()
                            .bg(theme.status_bg)
                            .fg(theme.info)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }

            lines.push(Line::from(row_spans));
        }

        frame.render_widget(Paragraph::new(lines), inner);

        // Position cursor for text input
        if self.is_clone_mode {
            let label_w = layout.split_x + 1 + 2;
            let text_w =
                unicode_width::UnicodeWidthStr::width(self.clone_url_input.as_str()) as u16;
            let cur_x = (inner.x + label_w + text_w).min(inner.x + inner.width.saturating_sub(1));
            frame.set_cursor_position((cur_x, inner.y + layout.clone_url_input_y));
        } else if self.active_section == OpenRepoSection::Input {
            let label_w = layout.split_x + 1 + 8; // " Path:  "
            let text_w = unicode_width::UnicodeWidthStr::width(self.path_input.as_str()) as u16;
            let cur_x = (inner.x + label_w + text_w).min(inner.x + inner.width.saturating_sub(1));
            frame.set_cursor_position((cur_x, inner.y + layout.path_input_y));
        }
    }
}

/// Helper function to read the current branch name from `.git/HEAD` without spawning git process.
fn read_branch_from_head(repo_path: &Path) -> Option<String> {
    let head_file = repo_path.join(".git").join("HEAD");
    if let Ok(content) = std::fs::read_to_string(head_file) {
        let trimmed = content.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            return Some(branch.to_string());
        }
        if trimmed.len() >= 7 {
            return Some(trimmed[..7].to_string());
        }
    }
    None
}
