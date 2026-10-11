//! Split screen layout structure and click hit-testing areas.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Calculated bounding rectangles for mouse hit testing and component rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutAreas {
    pub root: Rect,
    pub header: Rect,
    pub header_top: Rect,
    pub header_bottom: Rect,
    pub ask_ai_button: Rect,
    pub help_button: Rect,
    pub branch_button: Rect,
    pub sync_button: Rect,
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
        // Prowly title (10), Path (~18), Branch button (~22), Sync status (~14)
        let branch_x = header_top.x + 30;
        let branch_button = Rect {
            x: branch_x.min(header_top.width),
            y: header_top.y,
            width: 24.min(header_top.width.saturating_sub(branch_x)),
            height: 1,
        };

        let sync_x = branch_x + 25;
        let sync_button = Rect {
            x: sync_x.min(header_top.width),
            y: header_top.y,
            width: 16.min(header_top.width.saturating_sub(sync_x)),
            height: 1,
        };

        // Top right buttons: [ 󱚣 Ask AI ] (13 cols) and [? Help] (9 cols)
        let ask_ai_width = 13;
        let help_width = 9;
        let ask_ai_x = header_top.x
            + header_top
                .width
                .saturating_sub(ask_ai_width + help_width + 1);
        let help_x = header_top.x + header_top.width.saturating_sub(help_width);

        let ask_ai_button = Rect {
            x: ask_ai_x,
            y: header_top.y,
            width: ask_ai_width.min(header_top.width),
            height: 1,
        };

        let help_button = Rect {
            x: help_x,
            y: header_top.y,
            width: help_width.min(header_top.width),
            height: 1,
        };

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
            ask_ai_button,
            help_button,
            branch_button,
            sync_button,
            body,
            left_pane,
            right_pane,
            right_tabs,
            right_content,
            footer,
        }
    }

    /// Check if coordinates (x, y) hit the Ask AI button in header.
    pub fn hits_ask_ai(&self, x: u16, y: u16) -> bool {
        contains_point(self.ask_ai_button, x, y)
    }

    /// Check if coordinates (x, y) hit the Help button in header.
    pub fn hits_help(&self, x: u16, y: u16) -> bool {
        contains_point(self.help_button, x, y)
    }

    /// Check if coordinates (x, y) hit the branch button in header.
    pub fn hits_branch(&self, x: u16, y: u16) -> bool {
        contains_point(self.branch_button, x, y)
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
