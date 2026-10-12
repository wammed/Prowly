//! Directory map component with change-density badges and path filtering.

use crate::git::GitFileItem;
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
use std::collections::{BTreeMap, BTreeSet};

/// A node in the directory map tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirMapNode {
    pub display_name: String,
    pub tree_prefix: String, // e.g. "├── " or "│   └── "
    pub path_prefix: String, // e.g. "" for root, "src/", "src/ui/"
    pub change_count: usize,
    pub is_root: bool,
    pub depth: usize,
}

/// Directory map state controller.
pub struct DirectoryMap {
    pub scroll: VirtualScroll,
    pub nodes: Vec<DirMapNode>,
    pub selected_filter: Option<String>, // If set, only files matching this prefix are shown in right pane
}

impl DirectoryMap {
    pub fn new() -> Self {
        Self {
            scroll: VirtualScroll::new(0, 10),
            nodes: Vec::new(),
            selected_filter: None,
        }
    }

    /// Rebuild directory tree nodes from repository directories and modified files.
    pub fn update_from_repo(&mut self, repo_dirs: &[String], files: &[GitFileItem]) {
        let changed_files_count = files
            .iter()
            .filter(|f| f.status != crate::git::FileStatus::Clean)
            .count();

        // Aggregate counts by directory prefix
        let mut dir_counts: BTreeMap<String, usize> = BTreeMap::new();
        let mut all_dirs: BTreeSet<String> = BTreeSet::new();

        // 1. Add known repository directories
        for dir in repo_dirs {
            let mut prefix = String::new();
            for part in dir.trim_end_matches('/').split('/') {
                if !part.is_empty() {
                    prefix.push_str(part);
                    prefix.push('/');
                    all_dirs.insert(prefix.clone());
                }
            }
        }

        // 2. Add changed files and accumulate their change counts
        for file in files {
            let is_changed = file.status != crate::git::FileStatus::Clean;
            let path = &file.path;
            let parts: Vec<&str> = path.split('/').collect();

            let mut prefix = String::new();
            for part in &parts[..parts.len().saturating_sub(1)] {
                prefix.push_str(part);
                prefix.push('/');
                if is_changed {
                    *dir_counts.entry(prefix.clone()).or_insert(0) += 1;
                }
                all_dirs.insert(prefix.clone());
            }
        }

        let mut nodes = Vec::new();
        // Root node
        let root_label = format!("/ (root) [{}]", changed_files_count);
        nodes.push(DirMapNode {
            display_name: root_label,
            tree_prefix: String::new(),
            path_prefix: "".to_string(),
            change_count: changed_files_count,
            is_root: true,
            depth: 0,
        });

        // Group directories by their immediate parent prefix
        let mut children_by_parent: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for dir in &all_dirs {
            let trimmed = dir.trim_end_matches('/');
            let parent = match trimmed.rfind('/') {
                Some(idx) => format!("{}/", &trimmed[..idx]),
                None => String::new(),
            };
            children_by_parent
                .entry(parent)
                .or_default()
                .push(dir.clone());
        }

        // Depth-first traversal to build correctly branched tree nodes
        fn build_nodes(
            parent: &str,
            children_by_parent: &BTreeMap<String, Vec<String>>,
            dir_counts: &BTreeMap<String, usize>,
            ancestors_is_last: &mut Vec<bool>,
            nodes: &mut Vec<DirMapNode>,
        ) {
            if let Some(children) = children_by_parent.get(parent) {
                let count = children.len();
                for (i, child) in children.iter().enumerate() {
                    let is_last = i + 1 == count;
                    let depth = child.matches('/').count();
                    let trimmed = child.trim_end_matches('/');
                    let leaf_name = trimmed.rsplit('/').next().unwrap_or(trimmed);
                    let change_count = dir_counts.get(child).copied().unwrap_or(0);

                    let mut prefix = String::new();
                    for &anc_last in ancestors_is_last.iter() {
                        if anc_last {
                            prefix.push_str("    ");
                        } else {
                            prefix.push_str("│   ");
                        }
                    }
                    if is_last {
                        prefix.push_str("└── ");
                    } else {
                        prefix.push_str("├── ");
                    }

                    nodes.push(DirMapNode {
                        display_name: format!("{}/ [{}]", leaf_name, change_count),
                        tree_prefix: prefix,
                        path_prefix: child.clone(),
                        change_count,
                        is_root: false,
                        depth,
                    });

                    ancestors_is_last.push(is_last);
                    build_nodes(
                        child,
                        children_by_parent,
                        dir_counts,
                        ancestors_is_last,
                        nodes,
                    );
                    ancestors_is_last.pop();
                }
            }
        }

        let mut ancestors_is_last = Vec::new();
        build_nodes(
            "",
            &children_by_parent,
            &dir_counts,
            &mut ancestors_is_last,
            &mut nodes,
        );

        self.scroll.set_total_items(nodes.len());
        self.nodes = nodes;
    }

    /// Rebuild directory tree nodes from modified/untracked files.
    pub fn update_from_files(&mut self, files: &[GitFileItem]) {
        self.update_from_repo(&[], files);
    }

    /// Retrieve the currently selected directory prefix for filtering.
    pub fn current_selected_prefix(&self) -> Option<&str> {
        self.selected_filter.as_deref()
    }

    /// Select node at the current scroll index and update filter.
    pub fn apply_selection(&mut self) -> Option<String> {
        let idx = self.scroll.selected_index;
        if let Some(node) = self.nodes.get(idx) {
            if node.is_root {
                self.selected_filter = None;
            } else {
                self.selected_filter = Some(node.path_prefix.clone());
            }
        }
        self.selected_filter.clone()
    }

    /// Render directory map tree into frame.
    pub fn render(
        &mut self,
        frame: &mut Frame,
        is_focused: bool,
        is_git_repo: bool,
        theme: &Theme,
        icons: &Icons,
        area: Rect,
    ) {
        let border_color = if is_focused {
            theme.border_focused
        } else {
            theme.border
        };

        let mut title_spans = vec![Span::styled(
            format!(" {} DIRECTORY MAP ", icons.folder_open()),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )];
        if is_focused {
            title_spans.push(Span::styled(
                " ● ACTIVE ",
                Style::default()
                    .fg(theme.border_focused)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(title_spans))
            .style(Style::default().bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Non-git repository fallback guidance
        if !is_git_repo {
            let non_git_lines = vec![
                Line::from(Span::styled(
                    " Not a git repository",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" [ {} Init Repo ] ", icons.commit()),
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!(" [ {} Clone Repo ] ", icons.sync()),
                    Style::default().fg(theme.secondary),
                )),
            ];
            frame.render_widget(Paragraph::new(non_git_lines), inner);
            return;
        }

        self.scroll.set_viewport_height(inner.height as usize);
        let visible = self.scroll.visible_range();

        let lines: Vec<Line> = self.nodes[visible]
            .iter()
            .enumerate()
            .map(|(rel_idx, node)| {
                let abs_idx = self.scroll.viewport_offset + rel_idx;
                let is_sel = abs_idx == self.scroll.selected_index;
                let is_active_filter = self.selected_filter.as_deref() == Some(&node.path_prefix);

                let icon_str = if node.is_root {
                    icons.folder_open()
                } else {
                    icons.folder_closed()
                };

                let style = if is_sel {
                    Style::default()
                        .bg(theme.selection_bg)
                        .fg(theme.selection_fg)
                        .add_modifier(Modifier::BOLD)
                } else if is_active_filter {
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default().fg(theme.fg)
                };

                let spans = if node.is_root {
                    vec![
                        Span::styled(format!(" {} ", icon_str), style),
                        Span::styled(node.display_name.clone(), style),
                    ]
                } else {
                    vec![
                        Span::styled(format!(" {}", node.tree_prefix), style),
                        Span::styled(format!("{} ", icon_str), style),
                        Span::styled(node.display_name.clone(), style),
                    ]
                };

                Line::from(spans)
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

impl Default for DirectoryMap {
    fn default() -> Self {
        Self::new()
    }
}
