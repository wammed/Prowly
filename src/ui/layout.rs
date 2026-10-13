//! Split screen layout structure and click hit-testing areas.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Calculated bounding rectangles for mouse hit testing and component rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutAreas {
    pub root: Rect,
    pub header: Rect,
    pub header_top: Rect,
    pub header_bottom: Rect,
    pub open_repo_button: Rect,
    pub branch_button: Rect,
    pub fetch_button: Rect,
    pub pull_button: Rect,
    pub sync_button: Rect,
    pub ai_status_button: Rect,
    pub palette_button: Rect,
    pub settings_button: Rect,
    pub menu_button: Rect,
    pub help_button: Rect,
    pub ask_ai_button: Rect,
    pub body: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    pub right_tabs: Rect,
    pub right_content: Rect,
    pub footer: Rect,
}

impl LayoutAreas {
    /// Compute all layout chunks from the total terminal viewport rect.
    pub fn compute(area: Rect) -> Self {
        // Vertical split: Header (3 lines) -> Main Body (Min 0) -> Footer (1 line)
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);

        let header = vertical[0];
        let body = vertical[1];
        let footer = vertical[2];

        // Split Header into top row and bottom row
        let header_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(2)])
            .split(header);
        let header_top = header_rows[0];
        let header_bottom = header_rows[1];

        // Header buttons on the left/middle:
        // Prowly title (10), Open Repo / Path (~22), Branch button (~22), Sync status (~14)
        let open_repo_x = header_top.x + 10;
        let open_repo_button = Rect {
            x: open_repo_x.min(header_top.width),
            y: header_top.y,
            width: 22.min(header_top.width.saturating_sub(open_repo_x)),
            height: 1,
        };

        let branch_x = open_repo_x + 23;
        let branch_button = Rect {
            x: branch_x.min(header_top.width),
            y: header_top.y,
            width: 22.min(header_top.width.saturating_sub(branch_x)),
            height: 1,
        };

        let sync_x = branch_x + 23;
        let sync_button = Rect {
            x: sync_x.min(header_top.width),
            y: header_top.y,
            width: 16.min(header_top.width.saturating_sub(sync_x)),
            height: 1,
        };

        // Top right buttons:
        // [? Help] (10), [  Settings ] (14), [ 󰘳 Palette ] (13), [ 󰍜 Menu ] (10), [ 󱚣 AI: ... ] (34)
        let help_width = 10;
        let settings_width = 14;
        let palette_width = 13;
        let menu_width = 10;
        let ai_width = 34;

        let help_x = header_top.x + header_top.width.saturating_sub(help_width);
        let settings_x = help_x.saturating_sub(settings_width);
        let palette_x = settings_x.saturating_sub(palette_width);
        let menu_x = palette_x.saturating_sub(menu_width);
        let ai_x = menu_x.saturating_sub(ai_width);

        let help_button = Rect {
            x: help_x,
            y: header_top.y,
            width: help_width.min(header_top.width),
            height: 1,
        };

        let settings_button = Rect {
            x: settings_x,
            y: header_top.y,
            width: settings_width.min(header_top.width),
            height: 1,
        };

        let palette_button = Rect {
            x: palette_x,
            y: header_top.y,
            width: palette_width.min(header_top.width),
            height: 1,
        };

        let menu_button = Rect {
            x: menu_x,
            y: header_top.y,
            width: menu_width.min(header_top.width),
            height: 1,
        };

        let ai_status_button = Rect {
            x: ai_x,
            y: header_top.y,
            width: ai_width.min(header_top.width),
            height: 1,
        };

        let ask_ai_button = ai_status_button;

        // Horizontal split for Body: Left Directory Map (26 cols or 25%) -> Right Main View
        let left_width = (body.width / 4).clamp(22, 34);
        let body_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(left_width), Constraint::Min(40)])
            .split(body);

        let left_pane = body_cols[0];
        let right_pane = body_cols[1];

        // Split Right Pane: Tabs bar (2 lines) -> Content area
        let right_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(right_pane);

        let right_tabs = right_split[0];
        let right_content = right_split[1];

        Self {
            root: area,
            header,
            header_top,
            header_bottom,
            open_repo_button,
            branch_button,
            fetch_button: Rect::default(),
            pull_button: Rect::default(),
            sync_button,
            ai_status_button,
            palette_button,
            settings_button,
            menu_button,
            help_button,
            ask_ai_button,
            body,
            left_pane,
            right_pane,
            right_tabs,
            right_content,
            footer,
        }
    }

    /// Check if coordinates (x, y) hit the Open Repo button in header.
    pub fn hits_open_repo(&self, x: u16, y: u16) -> bool {
        contains_point(self.open_repo_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Ask AI / AI status button in header.
    pub fn hits_ask_ai(&self, x: u16, y: u16) -> bool {
        contains_point(self.ai_status_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Command Palette button in header.
    pub fn hits_palette(&self, x: u16, y: u16) -> bool {
        contains_point(self.palette_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Settings button in header.
    pub fn hits_settings(&self, x: u16, y: u16) -> bool {
        contains_point(self.settings_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Menu button in header.
    pub fn hits_menu(&self, x: u16, y: u16) -> bool {
        contains_point(self.menu_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Help button in header.
    pub fn hits_help(&self, x: u16, y: u16) -> bool {
        contains_point(self.help_button, x, y)
    }

    /// Check if coordinates (x, y) hit the branch button in header.
    pub fn hits_branch(&self, x: u16, y: u16) -> bool {
        contains_point(self.branch_button, x, y)
    }

    /// Check if coordinates (x, y) hit the fetch button in header.
    pub fn hits_fetch(&self, x: u16, y: u16) -> bool {
        contains_point(self.fetch_button, x, y)
    }

    /// Check if coordinates (x, y) hit the pull button in header.
    pub fn hits_pull(&self, x: u16, y: u16) -> bool {
        contains_point(self.pull_button, x, y)
    }

    /// Check if coordinates (x, y) hit the remote sync indicator in header.
    pub fn hits_sync(&self, x: u16, y: u16) -> bool {
        contains_point(self.sync_button, x, y)
    }

    /// Check if coordinates (x, y) are inside the left directory map pane.
    pub fn hits_left_pane(&self, x: u16, y: u16) -> bool {
        contains_point(self.left_pane, x, y)
    }

    /// Check if coordinates (x, y) are inside the right tabs bar.
    pub fn hits_tabs(&self, x: u16, y: u16) -> bool {
        contains_point(self.right_tabs, x, y)
    }

    /// Check if coordinates (x, y) are inside the right content view.
    pub fn hits_right_content(&self, x: u16, y: u16) -> bool {
        contains_point(self.right_content, x, y)
    }
}

pub fn contains_point(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
