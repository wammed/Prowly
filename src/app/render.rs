use super::*;
use crate::ui::capsule::{key_badge, pill, split_pill};
use crate::ui::layout::LayoutAreas;
use crate::ui::views::changes::ChangesRenderParams;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

impl App {
    /// Render complete TUI interface into Ratatui frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        self.layout_areas = LayoutAreas::compute(area);

        // 1. Header Rendering
        self.render_header(frame);

        // 2. Left Directory Map Pane
        let is_dir_focused = self.focused_pane == FocusedPane::DirectoryMap;
        let is_git = self.git_service.is_some();
        self.dir_map.render(
            frame,
            is_dir_focused,
            is_git,
            &self.theme,
            &self.icons,
            self.layout_areas.left_pane,
        );

        // 3. Right Pane Tabs & Content
        self.render_right_pane(frame);

        // 4. Footer Rendering
        self.render_footer(frame);

        // 5. Context Menu Overlay (if open)
        if self.context_menu.is_open {
            self.context_menu
                .render(frame, &self.theme, &self.icons, area);
        }

        // 6. Omnipresent AI Modal Overlay (if open)
        if self.ai_modal.is_open {
            self.ai_modal.render(frame, &self.theme, area);
        }

        // 7. Command Palette Overlay (if open)
        if self.command_palette.is_open {
            self.command_palette
                .render(frame, &self.theme, &self.icons, area);
        }

        // 8. Conflict Resolver Modal Overlay (if open)
        if self.conflict_resolver.is_open {
            self.conflict_resolver
                .render(frame, &self.theme, &self.icons, area);
        }

        // 9. Quick Branch Switcher Overlay (if open)
        if self.branch_switcher.is_open {
            self.branch_switcher
                .render(frame, &self.theme, &self.icons, area);
        }

        // 10. Remote Diff Preview Overlay (if open)
        if self.remote_diff_preview.is_open {
            self.remote_diff_preview
                .render(frame, &self.theme, &self.icons, area);
        }

        // 11. Quick Rebase Modal Overlay (if open)
        if self.rebase_modal.is_open {
            self.rebase_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 12. Open Repository Modal Overlay (if open)
        if self.open_repo_modal.is_open {
            self.open_repo_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 13. Settings Modal Overlay (if open)
        if self.settings_modal.is_open {
            self.settings_modal.hover_pos = self.hover_pos;
            self.settings_modal
                .render(frame, &self.theme, &self.icons, &self.i18n, area);
        }

        // 14. History Actions Modal Overlay (if open)
        if self.history_modal.is_open {
            self.history_modal
                .render(frame, &self.theme, &self.icons, area);
        }

        // 15. Markdown Preview Modal Overlay (if open)
        if self.markdown_preview_modal.is_open {
            self.markdown_preview_modal
                .render(frame, &self.theme, &self.icons, area);
        }
    }

    /// Check whether mouse cursor is currently hovering within given rectangle.
    pub fn is_hovered(&self, rect: Rect) -> bool {
        if let Some((x, y)) = self.hover_pos {
            x >= rect.x
                && x < rect.x.saturating_add(rect.width)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.height)
        } else {
            false
        }
    }

    fn render_header(&mut self, frame: &mut Frame) {
        let top_area = self.layout_areas.header_top;
        let bottom_area = self.layout_areas.header_bottom;

        // Path formatting
        let repo_path = if let Some(git) = &self.git_service {
            let p = git.workdir();
            let full_str = if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::Path::new(&home);
                if let Ok(rel) = p.strip_prefix(home_path) {
                    format!("~/{}", rel.display())
                } else {
                    p.display().to_string()
                }
            } else {
                p.display().to_string()
            };
            if top_area.width < 140 && full_str.len() > 16 {
                format!(
                    "…/{}",
                    p.file_name().and_then(|n| n.to_str()).unwrap_or(&full_str)
                )
            } else {
                full_str
            }
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            if let Ok(home) = std::env::var("HOME") {
                let home_path = std::path::Path::new(&home);
                if let Ok(rel) = cwd.strip_prefix(home_path) {
                    format!("~/{} (Not a Repo - Click)", rel.display())
                } else {
                    format!("{} (Not a Repo - Click)", cwd.display())
                }
            } else {
                format!("{} (Not a Repo - Click)", cwd.display())
            }
        };

        // Sync status determination
        let is_sync_needed = self.repo_status.ahead > 0 || self.repo_status.behind > 0;
        let (sync_icon, sync_label, sync_color) = if is_sync_needed {
            (
                self.icons.get(crate::ui::IconId::CheckWarning),
                "Sync Needed",
                self.theme.warning,
            )
        } else {
            (self.icons.check_success(), "Synced", self.theme.success)
        };

        // Top Header Row with GUI Capsules
        let mut top_spans = Vec::with_capacity(64);
        top_spans.push(Span::raw(" "));
        // 1. Brand Capsule (vibrant accent)
        let brand_spans = pill(
            Some("󰄛"),
            "PROWLY",
            self.theme.bg,
            self.theme.primary,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        let brand_w = crate::ui::capsule::spans_width(&brand_spans) as u16;
        top_spans.extend(brand_spans);
        top_spans.push(Span::raw(" "));

        // 2. Repo Path Capsule
        let open_repo_x = top_area.x + 1 + brand_w + 1;
        let repo_w =
            crate::ui::capsule::pill_width(Some(""), &repo_path, self.icons.use_nerd_font) as u16;
        self.layout_areas.open_repo_button = Rect {
            x: open_repo_x,
            y: top_area.y,
            width: repo_w.min(top_area.width.saturating_sub(open_repo_x)),
            height: 1,
        };
        let is_repo_hovered = self.is_hovered(self.layout_areas.open_repo_button);
        let (repo_fg, repo_bg) = if is_repo_hovered {
            (self.theme.bg, self.theme.accent)
        } else {
            (self.theme.fg, self.theme.selection_bg)
        };
        let repo_spans = pill(
            Some(""),
            &repo_path,
            repo_fg,
            repo_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        top_spans.extend(repo_spans);
        top_spans.push(Span::raw(" "));

        // 3. Git Branch & Sync status Split Capsule
        let uncommitted_count = self.repo_status.files.len();
        let branch_stats = if uncommitted_count > 0 {
            format!(
                "↑{} ↓{} (*{})",
                self.repo_status.ahead, self.repo_status.behind, uncommitted_count
            )
        } else {
            format!("↑{} ↓{}", self.repo_status.ahead, self.repo_status.behind)
        };
        let is_compact_header = top_area.width < 115;
        let is_ultra_compact = top_area.width < 95;

        // Dynamic branch name truncation if narrow
        let branch_name = if top_area.width < 105 && self.repo_status.branch.chars().count() > 10 {
            crate::ui::truncate_chars(&self.repo_status.branch, 8)
        } else {
            &self.repo_status.branch
        };

        // Top right buttons: AI Status Chip, [ 󰍜 Menu ], [  Settings ], [? Help]
        let (ai_connected, ai_prov, ai_mod) = self.ai_connection_status();
        let (ai_icon, ai_label, ai_fg) = if ai_connected {
            let model_display = if top_area.width < 135 {
                "".to_string()
            } else if ai_mod.chars().count() > 12 {
                format!(" ({})", crate::ui::truncate_chars(&ai_mod, 10))
            } else if !ai_mod.is_empty() {
                format!(" ({})", ai_mod)
            } else {
                "".to_string()
            };
            (
                "󱚣",
                format!("AI: {}{}", ai_prov, model_display),
                self.theme.ai_sparkle,
            )
        } else {
            let disconnected_label = if self.config.language == crate::i18n::Language::Ja {
                "AI: 未接続"
            } else {
                "AI: Disconnected"
            };
            ("⚠", disconnected_label.to_string(), self.theme.warning)
        };

        let help_label = if is_compact_header { "" } else { "Help" };
        let help_w =
            crate::ui::capsule::pill_width(Some(""), help_label, self.icons.use_nerd_font) as u16;
        let help_x = top_area.x + top_area.width.saturating_sub(help_w);
        self.layout_areas.help_button = Rect {
            x: help_x,
            y: top_area.y,
            width: help_w,
            height: 1,
        };
        let is_help_hovered = self.is_hovered(self.layout_areas.help_button);
        let (help_fg, help_bg) = if is_help_hovered {
            (self.theme.bg, self.theme.fg)
        } else {
            (self.theme.dim_fg, self.theme.selection_bg)
        };
        let help_spans = pill(
            Some(""),
            help_label,
            help_fg,
            help_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );

        let settings_label = if is_compact_header { "" } else { "Settings" };
        let settings_w =
            crate::ui::capsule::pill_width(Some(""), settings_label, self.icons.use_nerd_font)
                as u16;
        let settings_x = help_x.saturating_sub(settings_w);
        self.layout_areas.settings_button = Rect {
            x: settings_x,
            y: top_area.y,
            width: settings_w,
            height: 1,
        };
        let is_settings_hovered = self.is_hovered(self.layout_areas.settings_button);
        let (settings_fg, settings_bg) = if is_settings_hovered {
            (self.theme.bg, self.theme.primary)
        } else {
            (self.theme.primary, self.theme.selection_bg)
        };
        let settings_spans = pill(
            Some(""),
            settings_label,
            settings_fg,
            settings_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );

        let palette_label = if is_compact_header { "" } else { "Palette" };
        let palette_w = crate::ui::capsule::pill_width(
            Some(self.icons.ai_sparkle()),
            palette_label,
            self.icons.use_nerd_font,
        ) as u16;
        let palette_x = settings_x.saturating_sub(palette_w);
        self.layout_areas.palette_button = Rect {
            x: palette_x,
            y: top_area.y,
            width: palette_w,
            height: 1,
        };
        let is_palette_hovered = self.is_hovered(self.layout_areas.palette_button);
        let (palette_fg, palette_bg) = if is_palette_hovered {
            (self.theme.bg, self.theme.accent)
        } else {
            (self.theme.accent, self.theme.selection_bg)
        };
        let palette_spans = pill(
            Some(self.icons.ai_sparkle()),
            palette_label,
            palette_fg,
            palette_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );

        let menu_label = if is_compact_header { "" } else { "Menu" };
        let menu_w =
            crate::ui::capsule::pill_width(Some("󰍜"), menu_label, self.icons.use_nerd_font) as u16;
        let menu_x = palette_x.saturating_sub(menu_w);
        self.layout_areas.menu_button = Rect {
            x: menu_x,
            y: top_area.y,
            width: menu_w,
            height: 1,
        };
        let is_menu_hovered = self.is_hovered(self.layout_areas.menu_button);
        let (menu_fg, menu_bg) = if is_menu_hovered {
            (self.theme.bg, self.theme.secondary)
        } else {
            (self.theme.secondary, self.theme.selection_bg)
        };
        let menu_spans = pill(
            Some("󰍜"),
            menu_label,
            menu_fg,
            menu_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );

        let branch_x = open_repo_x + repo_w + 1;
        let branch_w = crate::ui::capsule::split_pill_width(
            Some(self.icons.branch()),
            branch_name,
            &branch_stats,
            self.icons.use_nerd_font,
        ) as u16;
        self.layout_areas.branch_button = Rect {
            x: branch_x,
            y: top_area.y,
            width: branch_w.min(top_area.width.saturating_sub(branch_x)),
            height: 1,
        };
        let is_branch_hovered = self.is_hovered(self.layout_areas.branch_button);
        let branch_right_bg = if is_branch_hovered {
            self.theme.border_focused
        } else {
            self.theme.selection_bg
        };
        let branch_spans = split_pill(
            Some(self.icons.branch()),
            branch_name,
            self.theme.bg,
            self.theme.info,
            &branch_stats,
            self.theme.info,
            branch_right_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );
        top_spans.extend(branch_spans);
        top_spans.push(Span::raw(" "));

        let show_git_action_buttons = !is_ultra_compact;
        let last_left_x = if show_git_action_buttons {
            // 4. Fetch Capsule Button
            let fetch_label = if is_compact_header { "" } else { "Fetch" };
            let fetch_x = branch_x + branch_w + 1;
            let fetch_w =
                crate::ui::capsule::pill_width(Some("󰦗"), fetch_label, self.icons.use_nerd_font)
                    as u16;
            self.layout_areas.fetch_button = Rect {
                x: fetch_x,
                y: top_area.y,
                width: fetch_w.min(top_area.width.saturating_sub(fetch_x)),
                height: 1,
            };
            let is_fetch_hovered = self.is_hovered(self.layout_areas.fetch_button);
            let (fetch_fg, fetch_bg) = if is_fetch_hovered {
                (self.theme.bg, self.theme.primary)
            } else {
                (self.theme.primary, self.theme.selection_bg)
            };
            let fetch_spans = pill(
                Some("󰦗"),
                fetch_label,
                fetch_fg,
                fetch_bg,
                self.theme.header_bg,
                self.icons.use_nerd_font,
            );
            top_spans.extend(fetch_spans);
            top_spans.push(Span::raw(" "));

            // 5. Pull Capsule Button (highlights in warning if behind > 0)
            let pull_label = if is_compact_header {
                if self.repo_status.behind > 0 {
                    format!("↓{}", self.repo_status.behind)
                } else {
                    "".to_string()
                }
            } else if self.repo_status.behind > 0 {
                format!("Pull (↓{})", self.repo_status.behind)
            } else {
                "Pull".to_string()
            };
            let pull_x = fetch_x + fetch_w + 1;
            let pull_w =
                crate::ui::capsule::pill_width(Some("󰦖"), &pull_label, self.icons.use_nerd_font)
                    as u16;
            self.layout_areas.pull_button = Rect {
                x: pull_x,
                y: top_area.y,
                width: pull_w.min(top_area.width.saturating_sub(pull_x)),
                height: 1,
            };
            let is_pull_hovered = self.is_hovered(self.layout_areas.pull_button);
            let (pull_fg, pull_bg) = if is_pull_hovered {
                (self.theme.bg, self.theme.secondary)
            } else if self.repo_status.behind > 0 {
                (self.theme.bg, self.theme.warning)
            } else {
                (self.theme.secondary, self.theme.selection_bg)
            };
            let pull_spans = pill(
                Some("󰦖"),
                &pull_label,
                pull_fg,
                pull_bg,
                self.theme.header_bg,
                self.icons.use_nerd_font,
            );
            top_spans.extend(pull_spans);
            top_spans.push(Span::raw(" "));

            // 6. Sync State Capsule
            let sync_label_text = if is_compact_header { "" } else { sync_label };
            let sync_x = pull_x + pull_w + 1;
            let sync_w = crate::ui::capsule::pill_width(
                Some(sync_icon),
                sync_label_text,
                self.icons.use_nerd_font,
            ) as u16;
            self.layout_areas.sync_button = Rect {
                x: sync_x,
                y: top_area.y,
                width: sync_w.min(top_area.width.saturating_sub(sync_x)),
                height: 1,
            };
            let is_sync_hovered = self.is_hovered(self.layout_areas.sync_button);
            let (sync_fg, sync_bg) = if is_sync_hovered {
                (self.theme.bg, self.theme.info)
            } else {
                (self.theme.bg, sync_color)
            };
            let sync_spans = pill(
                Some(sync_icon),
                sync_label_text,
                sync_fg,
                sync_bg,
                self.theme.header_bg,
                self.icons.use_nerd_font,
            );
            top_spans.extend(sync_spans);

            sync_x + sync_w
        } else {
            self.layout_areas.fetch_button = Rect::default();
            self.layout_areas.pull_button = Rect::default();
            self.layout_areas.sync_button = Rect::default();
            branch_x + branch_w
        };

        let left_max_w = menu_x.saturating_sub(top_area.x);
        let left_area = Rect {
            x: top_area.x,
            y: top_area.y,
            width: left_max_w.min(top_area.width),
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(top_spans)).style(Style::default().bg(self.theme.header_bg)),
            left_area,
        );

        let ai_display_text = if is_compact_header {
            "".to_string()
        } else {
            ai_label
        };
        let raw_ai_w = crate::ui::capsule::pill_width(
            Some(ai_icon),
            &ai_display_text,
            self.icons.use_nerd_font,
        ) as u16;
        let min_ai_x = (last_left_x + 1).min(menu_x);
        let ideal_ai_x = menu_x.saturating_sub(raw_ai_w);
        let ai_x = ideal_ai_x.max(min_ai_x);
        let ai_w = menu_x.saturating_sub(ai_x);
        self.layout_areas.ai_status_button = Rect {
            x: ai_x,
            y: top_area.y,
            width: ai_w,
            height: 1,
        };
        self.layout_areas.ask_ai_button = self.layout_areas.ai_status_button;
        let is_ai_hovered = self.is_hovered(self.layout_areas.ai_status_button);
        let (ai_pill_fg, ai_pill_bg) = if is_ai_hovered {
            (self.theme.bg, self.theme.ai_sparkle)
        } else {
            (ai_fg, self.theme.selection_bg)
        };
        let ai_spans = pill(
            Some(ai_icon),
            &ai_display_text,
            ai_pill_fg,
            ai_pill_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        );

        if ai_w >= 3 {
            frame.render_widget(
                Paragraph::new(Line::from(ai_spans)),
                self.layout_areas.ai_status_button,
            );
        }
        frame.render_widget(
            Paragraph::new(Line::from(menu_spans)),
            self.layout_areas.menu_button,
        );
        frame.render_widget(
            Paragraph::new(Line::from(palette_spans)),
            self.layout_areas.palette_button,
        );
        frame.render_widget(
            Paragraph::new(Line::from(settings_spans)),
            self.layout_areas.settings_button,
        );
        frame.render_widget(
            Paragraph::new(Line::from(help_spans)),
            self.layout_areas.help_button,
        );

        // Bottom Header Row with Metadata Capsules
        let gh_target = self.github_target();
        let (repo_icon, repo_label) = if let Some(target) = gh_target {
            let is_priv = self.is_repo_private.unwrap_or(false);
            let (icon, vis) = if is_priv {
                (self.icons.lock_private(), self.i18n.t("header.private"))
            } else {
                (self.icons.globe_public(), self.i18n.t("header.public"))
            };
            (icon, format!("{}/{} ({})", target.owner, target.repo, vis))
        } else if let Some(git) = &self.git_service {
            let name = git
                .workdir()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("repo");
            (self.icons.folder_closed(), format!("local @ {}", name))
        } else {
            (self.icons.folder_closed(), "local @ repo".to_string())
        };

        let (ci_icon, ci_label) = if let Some(run) = self.actions_view.runs.first() {
            let conclusion = run.conclusion.as_deref().unwrap_or("in_progress");
            let icon = if conclusion == "success" {
                self.icons.check_success()
            } else if conclusion == "failure" {
                self.icons.check_failed()
            } else {
                self.icons.check_running()
            };
            (icon, format!("CI: #{} {}", run.id, conclusion))
        } else {
            (self.icons.check_success(), "CI: No runs".to_string())
        };

        let unread_count = self.alerts_view.alerts.iter().filter(|a| a.unread).count();
        let alert_label = format!("Alerts: {}", unread_count);

        let mut bottom_spans = Vec::new();
        bottom_spans.push(Span::raw("  "));
        bottom_spans.extend(pill(
            Some(repo_icon),
            &repo_label,
            self.theme.dim_fg,
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));
        bottom_spans.push(Span::raw(" "));
        bottom_spans.extend(pill(
            Some(ci_icon),
            &ci_label,
            self.theme.success,
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));
        bottom_spans.push(Span::raw(" "));
        bottom_spans.extend(pill(
            Some(self.icons.get(crate::ui::IconId::Alert)),
            &alert_label,
            if unread_count > 0 {
                self.theme.warning
            } else {
                self.theme.dim_fg
            },
            self.theme.selection_bg,
            self.theme.header_bg,
            self.icons.use_nerd_font,
        ));

        frame.render_widget(
            Paragraph::new(Line::from(bottom_spans))
                .style(Style::default().bg(self.theme.header_bg)),
            bottom_area,
        );
    }

    fn render_right_pane(&mut self, frame: &mut Frame) {
        // Render Segmented Control Pill Tabs
        let mut tab_spans = Vec::new();
        tab_spans.push(Span::raw(" "));
        let mut cur_x = 1u16;
        for tab in AppTab::ALL {
            let key = tab.title_key();
            let text = self.i18n.t(key);
            let icon_str = match tab {
                AppTab::Changes => self.icons.branch(),
                AppTab::History => self.icons.commit(),
                AppTab::Issues => self.icons.get(crate::ui::IconId::IssueOpen),
                AppTab::Prs => self.icons.pr_open(),
                AppTab::Actions => self.icons.check_success(),
                AppTab::Alerts => self.icons.get(crate::ui::IconId::Alert),
                AppTab::Worktrees => self.icons.worktree(),
            };

            let is_active = *tab == self.active_tab;
            let sample_pill = pill(
                Some(icon_str),
                &text,
                self.theme.bg,
                self.theme.primary,
                self.theme.bg,
                self.icons.use_nerd_font,
            );
            let pill_w = crate::ui::capsule::spans_width(&sample_pill) as u16;
            let tab_rect = Rect {
                x: self.layout_areas.right_tabs.x + cur_x,
                y: self.layout_areas.right_tabs.y,
                width: pill_w,
                height: 1,
            };
            let is_hovered = self.is_hovered(tab_rect);

            let (fg, bg) = if is_active {
                (self.theme.bg, self.theme.primary)
            } else if is_hovered {
                (self.theme.primary, self.theme.selection_bg)
            } else {
                (self.theme.dim_fg, self.theme.selection_bg)
            };

            tab_spans.extend(pill(
                Some(icon_str),
                &text,
                fg,
                bg,
                self.theme.bg,
                self.icons.use_nerd_font,
            ));
            tab_spans.push(Span::raw(" "));
            cur_x += pill_w + 1;
        }

        frame.render_widget(
            Paragraph::new(Line::from(tab_spans)),
            self.layout_areas.right_tabs,
        );

        // Render Active Tab Content with Elevated Border
        let is_focused = self.focused_pane == FocusedPane::MainContent;
        let border_color = if is_focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(if is_focused {
                Span::styled(
                    " ● ACTIVE ",
                    Style::default()
                        .fg(self.theme.border_focused)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("")
            })
            .style(Style::default().bg(self.theme.bg));

        let inner = block.inner(self.layout_areas.right_content);
        frame.render_widget(block, self.layout_areas.right_content);

        match self.active_tab {
            AppTab::Changes => {
                let filtered = self.filtered_files();
                let diff = self.current_diff.clone();
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                let operation_state = self.repo_status.operation_state;
                let params = ChangesRenderParams {
                    files: &filtered,
                    active_diff: &diff,
                    theme: &theme,
                    i18n: &self.i18n,
                    icons: &icons,
                    area: inner,
                    hover_pos: self.hover_pos,
                    operation_state,
                };
                self.changes_view.render(frame, params);
            }
            AppTab::History => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.history_view.render(frame, &theme, &icons, inner);
            }
            AppTab::Issues => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.issues_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Prs => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.prs_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Actions => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.actions_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Alerts => {
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.alerts_view
                    .render(frame, is_focused, &theme, &icons, inner);
            }
            AppTab::Worktrees => {
                let worktrees = self.worktrees.clone();
                let theme = self.theme.clone();
                let icons = self.icons.clone();
                self.worktrees_view
                    .render(frame, &worktrees, &theme, &icons, inner);
            }
        }
    }

    fn render_footer(&self, frame: &mut Frame) {
        let status_msg = self.last_status.as_deref().unwrap_or("Ready");
        let (ai_connected, ai_prov, ai_mod) = self.ai_connection_status();
        let ai_footer = if ai_connected {
            format!("{}: {}", ai_prov, ai_mod)
        } else if self.config.language == crate::i18n::Language::Ja {
            "未接続".to_string()
        } else {
            "Disconnected".to_string()
        };

        let mut left_spans = Vec::new();
        left_spans.push(Span::raw(" "));
        left_spans.extend(pill(
            Some("󰄛"),
            "NORMAL",
            self.theme.bg,
            self.theme.primary,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.push(Span::styled(
            format!(" 󰋼 {}  ", status_msg),
            Style::default().fg(self.theme.status_fg),
        ));

        // Keycap badges
        left_spans.extend(key_badge(
            "Tab",
            "Panes",
            self.theme.primary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "Space",
            "Stage",
            self.theme.success,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "c",
            "Commit",
            self.theme.accent,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "F1",
            "Palette",
            self.theme.primary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            ",",
            "Settings",
            self.theme.secondary,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "Ctrl+O",
            "Open",
            self.theme.info,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        left_spans.extend(key_badge(
            "q",
            "Quit",
            self.theme.error,
            self.theme.selection_bg,
            self.theme.dim_fg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));

        let mut right_spans = Vec::new();
        right_spans.extend(pill(
            Some("󱚣"),
            &ai_footer,
            if ai_connected {
                self.theme.ai_sparkle
            } else {
                self.theme.warning
            },
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));
        right_spans.extend(pill(
            Some(""),
            self.theme.name,
            self.theme.fg,
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));
        right_spans.extend(pill(
            Some(""),
            self.config.language.code().to_uppercase().as_str(),
            self.theme.info,
            self.theme.selection_bg,
            self.theme.status_bg,
            self.icons.use_nerd_font,
        ));
        right_spans.push(Span::raw(" "));

        let left_len = crate::ui::capsule::spans_width(&left_spans);
        let right_len = crate::ui::capsule::spans_width(&right_spans);
        let width = self.layout_areas.footer.width as usize;
        let pad = width.saturating_sub(left_len + right_len);

        let mut all_spans = left_spans;
        all_spans.push(Span::styled(
            " ".repeat(pad),
            Style::default().bg(self.theme.status_bg),
        ));
        all_spans.extend(right_spans);

        let p = Paragraph::new(Line::from(all_spans)).style(
            Style::default()
                .bg(self.theme.status_bg)
                .fg(self.theme.status_fg),
        );
        frame.render_widget(p, self.layout_areas.footer);
    }
}
