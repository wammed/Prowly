//! 20+ Theme Color Engine for Prowly.
//! Provides comprehensive color palettes for dark and light terminal environments.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fmt;

/// All supported color theme identifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeId {
    #[default]
    #[serde(alias = "CatppuccinMocha", alias = "catppuccin-mocha")]
    CatppuccinMocha,
    #[serde(alias = "CatppuccinMacchiato", alias = "catppuccin-macchiato")]
    CatppuccinMacchiato,
    #[serde(alias = "CatppuccinFrappe", alias = "catppuccin-frappe")]
    CatppuccinFrappe,
    #[serde(alias = "CatppuccinLatte", alias = "catppuccin-latte")]
    CatppuccinLatte,
    #[serde(alias = "TokyoNight", alias = "tokyo-night")]
    TokyoNight,
    #[serde(alias = "TokyoNightStorm", alias = "tokyo-night-storm")]
    TokyoNightStorm,
    #[serde(alias = "TokyoNightLight", alias = "tokyo-night-light")]
    TokyoNightLight,
    #[serde(alias = "Nord", alias = "nord")]
    Nord,
    #[serde(alias = "Dracula", alias = "dracula")]
    Dracula,
    #[serde(alias = "GruvboxDark", alias = "gruvbox-dark")]
    GruvboxDark,
    #[serde(alias = "GruvboxLight", alias = "gruvbox-light")]
    GruvboxLight,
    #[serde(alias = "SolarizedDark", alias = "solarized-dark")]
    SolarizedDark,
    #[serde(alias = "SolarizedLight", alias = "solarized-light")]
    SolarizedLight,
    #[serde(alias = "OneDark", alias = "one-dark")]
    OneDark,
    #[serde(alias = "OneLight", alias = "one-light")]
    OneLight,
    #[serde(alias = "RosePine", alias = "rose-pine")]
    RosePine,
    #[serde(alias = "RosePineMoon", alias = "rose-pine-moon")]
    RosePineMoon,
    #[serde(alias = "RosePineDawn", alias = "rose-pine-dawn")]
    RosePineDawn,
    #[serde(alias = "MonokaiPro", alias = "monokai-pro")]
    MonokaiPro,
    #[serde(alias = "Synthwave84", alias = "synthwave-84", alias = "synthwave84")]
    Synthwave84,
    #[serde(alias = "EverforestDark", alias = "everforest-dark")]
    EverforestDark,
}

impl ThemeId {
    /// List of all 21 available theme identifiers.
    pub const ALL: &'static [ThemeId] = &[
        ThemeId::CatppuccinMocha,
        ThemeId::CatppuccinMacchiato,
        ThemeId::CatppuccinFrappe,
        ThemeId::CatppuccinLatte,
        ThemeId::TokyoNight,
        ThemeId::TokyoNightStorm,
        ThemeId::TokyoNightLight,
        ThemeId::Nord,
        ThemeId::Dracula,
        ThemeId::GruvboxDark,
        ThemeId::GruvboxLight,
        ThemeId::SolarizedDark,
        ThemeId::SolarizedLight,
        ThemeId::OneDark,
        ThemeId::OneLight,
        ThemeId::RosePine,
        ThemeId::RosePineMoon,
        ThemeId::RosePineDawn,
        ThemeId::MonokaiPro,
        ThemeId::Synthwave84,
        ThemeId::EverforestDark,
    ];

    /// Returns human-readable display name.
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinFrappe => "Catppuccin Frappe",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::TokyoNight => "Tokyo Night",
            Self::TokyoNightStorm => "Tokyo Night Storm",
            Self::TokyoNightLight => "Tokyo Night Light",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::GruvboxLight => "Gruvbox Light",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::OneDark => "One Dark",
            Self::OneLight => "One Light",
            Self::RosePine => "Rosé Pine",
            Self::RosePineMoon => "Rosé Pine Moon",
            Self::RosePineDawn => "Rosé Pine Dawn",
            Self::MonokaiPro => "Monokai Pro",
            Self::Synthwave84 => "Synthwave '84",
            Self::EverforestDark => "Everforest Dark",
        }
    }

    /// Returns compact chip label for UI pickers.
    pub const fn chip_name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Mocha",
            Self::CatppuccinMacchiato => "Macchiato",
            Self::CatppuccinFrappe => "Frappe",
            Self::CatppuccinLatte => "Latte",
            Self::TokyoNight => "TokyoNight",
            Self::TokyoNightStorm => "TokyoStorm",
            Self::TokyoNightLight => "TokyoLight",
            Self::Nord => "Nord",
            Self::Dracula => "Dracula",
            Self::GruvboxDark => "GruvboxDark",
            Self::GruvboxLight => "GruvboxLight",
            Self::SolarizedDark => "SolarizedDark",
            Self::SolarizedLight => "SolarizedLight",
            Self::OneDark => "OneDark",
            Self::OneLight => "OneLight",
            Self::RosePine => "RosePine",
            Self::RosePineMoon => "RoseMoon",
            Self::RosePineDawn => "RoseDawn",
            Self::MonokaiPro => "MonokaiPro",
            Self::Synthwave84 => "Synthwave84",
            Self::EverforestDark => "Everforest",
        }
    }

    /// Match from lowercase / slugified name.
    pub fn from_name(name: &str) -> Option<Self> {
        let normalized = name.to_lowercase().replace([' ', '-', '_', '\''], "");
        for id in Self::ALL {
            let id_str = format!("{:?}", id).to_lowercase();
            let disp = id
                .display_name()
                .to_lowercase()
                .replace([' ', '-', '_', '\''], "");
            if normalized == id_str || normalized == disp {
                return Some(*id);
            }
        }
        None
    }
}

impl fmt::Display for ThemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A comprehensive color palette for styling all Prowly UI components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub id: ThemeId,
    pub name: &'static str,
    pub is_dark: bool,

    // Core Canvas
    pub bg: Color,
    pub fg: Color,
    pub dim_fg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub status_bg: Color,
    pub status_fg: Color,

    // Borders & Dividers
    pub border: Color,
    pub border_focused: Color,

    // Selections & Highlighting
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub cursor: Color,

    // Semantics & Branding
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Git Status & Diff
    pub diff_add_fg: Color,
    pub diff_add_bg: Color,
    pub diff_del_fg: Color,
    pub diff_del_bg: Color,
    pub diff_mod_fg: Color,
    pub diff_mod_bg: Color,
    pub diff_hunk_fg: Color,
    pub diff_hunk_bg: Color,
    pub staged_fg: Color,
    pub unstaged_fg: Color,
    pub untracked_fg: Color,
    pub conflict_fg: Color,
    pub conflict_bg: Color,

    // Badges & Omnipresent AI
    pub badge_verified_fg: Color,
    pub badge_unverified_fg: Color,
    pub ai_sparkle: Color,
    pub ai_bg: Color,
    pub ai_fg: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

impl Theme {
    /// Retrieve full palette for a given theme ID.
    pub fn get(id: ThemeId) -> Self {
        match id {
            ThemeId::CatppuccinMocha => Self::catppuccin_mocha(),
            ThemeId::CatppuccinMacchiato => Self::catppuccin_macchiato(),
            ThemeId::CatppuccinFrappe => Self::catppuccin_frappe(),
            ThemeId::CatppuccinLatte => Self::catppuccin_latte(),
            ThemeId::TokyoNight => Self::tokyo_night(),
            ThemeId::TokyoNightStorm => Self::tokyo_night_storm(),
            ThemeId::TokyoNightLight => Self::tokyo_night_light(),
            ThemeId::Nord => Self::nord(),
            ThemeId::Dracula => Self::dracula(),
            ThemeId::GruvboxDark => Self::gruvbox_dark(),
            ThemeId::GruvboxLight => Self::gruvbox_light(),
            ThemeId::SolarizedDark => Self::solarized_dark(),
            ThemeId::SolarizedLight => Self::solarized_light(),
            ThemeId::OneDark => Self::one_dark(),
            ThemeId::OneLight => Self::one_light(),
            ThemeId::RosePine => Self::rose_pine(),
            ThemeId::RosePineMoon => Self::rose_pine_moon(),
            ThemeId::RosePineDawn => Self::rose_pine_dawn(),
            ThemeId::MonokaiPro => Self::monokai_pro(),
            ThemeId::Synthwave84 => Self::synthwave_84(),
            ThemeId::EverforestDark => Self::everforest_dark(),
        }
    }

    // 1. Catppuccin Mocha (Dark)
    fn catppuccin_mocha() -> Self {
        Self {
            id: ThemeId::CatppuccinMocha,
            name: "Catppuccin Mocha",
            is_dark: true,
            bg: rgb(30, 30, 46),        // Base #1e1e2e
            fg: rgb(205, 214, 244),     // Text #cdd6f4
            dim_fg: rgb(147, 153, 178), // Overlay0 #9399b2
            header_bg: rgb(24, 24, 37), // Mantle #181825
            header_fg: rgb(205, 214, 244),
            status_bg: rgb(17, 17, 27),         // Crust #11111b
            status_fg: rgb(166, 173, 200),      // Subtext0 #a6adc8
            border: rgb(69, 71, 90),            // Surface1 #45475a
            border_focused: rgb(137, 180, 250), // Blue #89b4fa
            selection_bg: rgb(88, 91, 112),     // Surface2 #585b70
            selection_fg: rgb(245, 224, 220),   // Rosewater #f5e0dc
            cursor: rgb(245, 224, 220),
            primary: rgb(137, 180, 250),   // Blue #89b4fa
            secondary: rgb(203, 166, 247), // Mauve #cba6f7
            accent: rgb(245, 194, 231),    // Pink #f5c2e7
            success: rgb(166, 227, 161),   // Green #a6e3a1
            warning: rgb(249, 226, 175),   // Yellow #f9e2af
            error: rgb(243, 139, 168),     // Red #f38ba8
            info: rgb(137, 220, 235),      // Sky #89dceb
            diff_add_fg: rgb(166, 227, 161),
            diff_add_bg: rgb(35, 52, 45),
            diff_del_fg: rgb(243, 139, 168),
            diff_del_bg: rgb(55, 33, 43),
            diff_mod_fg: rgb(137, 220, 235),
            diff_mod_bg: rgb(32, 48, 58),
            diff_hunk_fg: rgb(180, 190, 254),
            diff_hunk_bg: rgb(40, 42, 60),
            staged_fg: rgb(166, 227, 161),
            unstaged_fg: rgb(249, 226, 175),
            untracked_fg: rgb(147, 153, 178),
            conflict_fg: rgb(250, 179, 135), // Peach #fab387
            conflict_bg: rgb(60, 40, 30),
            badge_verified_fg: rgb(166, 227, 161),
            badge_unverified_fg: rgb(108, 112, 134),
            ai_sparkle: rgb(203, 166, 247),
            ai_bg: rgb(40, 35, 55),
            ai_fg: rgb(245, 224, 220),
        }
    }

    // 2. Catppuccin Macchiato (Medium Dark)
    fn catppuccin_macchiato() -> Self {
        Self {
            id: ThemeId::CatppuccinMacchiato,
            name: "Catppuccin Macchiato",
            is_dark: true,
            bg: rgb(36, 39, 58),
            fg: rgb(202, 211, 245),
            dim_fg: rgb(141, 149, 174),
            header_bg: rgb(30, 32, 48),
            header_fg: rgb(202, 211, 245),
            status_bg: rgb(24, 25, 38),
            status_fg: rgb(165, 173, 203),
            border: rgb(73, 77, 100),
            border_focused: rgb(138, 173, 244),
            selection_bg: rgb(91, 96, 120),
            selection_fg: rgb(244, 219, 214),
            cursor: rgb(244, 219, 214),
            primary: rgb(138, 173, 244),
            secondary: rgb(198, 160, 246),
            accent: rgb(245, 189, 230),
            success: rgb(166, 218, 149),
            warning: rgb(238, 212, 159),
            error: rgb(237, 135, 150),
            info: rgb(145, 215, 227),
            diff_add_fg: rgb(166, 218, 149),
            diff_add_bg: rgb(38, 55, 48),
            diff_del_fg: rgb(237, 135, 150),
            diff_del_bg: rgb(58, 35, 45),
            diff_mod_fg: rgb(145, 215, 227),
            diff_mod_bg: rgb(35, 50, 60),
            diff_hunk_fg: rgb(183, 189, 248),
            diff_hunk_bg: rgb(45, 48, 68),
            staged_fg: rgb(166, 218, 149),
            unstaged_fg: rgb(238, 212, 159),
            untracked_fg: rgb(141, 149, 174),
            conflict_fg: rgb(245, 169, 127),
            conflict_bg: rgb(62, 42, 32),
            badge_verified_fg: rgb(166, 218, 149),
            badge_unverified_fg: rgb(110, 115, 141),
            ai_sparkle: rgb(198, 160, 246),
            ai_bg: rgb(44, 38, 60),
            ai_fg: rgb(244, 219, 214),
        }
    }

    // 3. Catppuccin Frappe (Soft Dark)
    fn catppuccin_frappe() -> Self {
        Self {
            id: ThemeId::CatppuccinFrappe,
            name: "Catppuccin Frappe",
            is_dark: true,
            bg: rgb(48, 52, 70),
            fg: rgb(198, 208, 245),
            dim_fg: rgb(131, 139, 167),
            header_bg: rgb(41, 44, 60),
            header_fg: rgb(198, 208, 245),
            status_bg: rgb(35, 38, 52),
            status_fg: rgb(165, 173, 206),
            border: rgb(81, 87, 109),
            border_focused: rgb(140, 170, 238),
            selection_bg: rgb(98, 104, 128),
            selection_fg: rgb(242, 213, 207),
            cursor: rgb(242, 213, 207),
            primary: rgb(140, 170, 238),
            secondary: rgb(202, 158, 230),
            accent: rgb(234, 153, 156),
            success: rgb(166, 209, 137),
            warning: rgb(229, 200, 144),
            error: rgb(231, 130, 132),
            info: rgb(153, 209, 219),
            diff_add_fg: rgb(166, 209, 137),
            diff_add_bg: rgb(45, 60, 50),
            diff_del_fg: rgb(231, 130, 132),
            diff_del_bg: rgb(65, 40, 48),
            diff_mod_fg: rgb(153, 209, 219),
            diff_mod_bg: rgb(40, 56, 68),
            diff_hunk_fg: rgb(186, 187, 241),
            diff_hunk_bg: rgb(55, 60, 80),
            staged_fg: rgb(166, 209, 137),
            unstaged_fg: rgb(229, 200, 144),
            untracked_fg: rgb(131, 139, 167),
            conflict_fg: rgb(239, 159, 118),
            conflict_bg: rgb(70, 45, 35),
            badge_verified_fg: rgb(166, 209, 137),
            badge_unverified_fg: rgb(115, 121, 148),
            ai_sparkle: rgb(202, 158, 230),
            ai_bg: rgb(54, 46, 70),
            ai_fg: rgb(242, 213, 207),
        }
    }

    // 4. Catppuccin Latte (Light)
    fn catppuccin_latte() -> Self {
        Self {
            id: ThemeId::CatppuccinLatte,
            name: "Catppuccin Latte",
            is_dark: false,
            bg: rgb(239, 241, 245),
            fg: rgb(76, 79, 105),
            dim_fg: rgb(156, 160, 176),
            header_bg: rgb(230, 233, 239),
            header_fg: rgb(76, 79, 105),
            status_bg: rgb(220, 224, 232),
            status_fg: rgb(92, 95, 119),
            border: rgb(172, 176, 190),
            border_focused: rgb(30, 102, 245),
            selection_bg: rgb(204, 208, 218),
            selection_fg: rgb(76, 79, 105),
            cursor: rgb(220, 138, 120),
            primary: rgb(30, 102, 245),
            secondary: rgb(136, 57, 239),
            accent: rgb(234, 118, 203),
            success: rgb(64, 160, 43),
            warning: rgb(223, 142, 29),
            error: rgb(210, 15, 57),
            info: rgb(4, 165, 229),
            diff_add_fg: rgb(64, 160, 43),
            diff_add_bg: rgb(220, 242, 220),
            diff_del_fg: rgb(210, 15, 57),
            diff_del_bg: rgb(250, 224, 228),
            diff_mod_fg: rgb(4, 165, 229),
            diff_mod_bg: rgb(225, 240, 252),
            diff_hunk_fg: rgb(114, 135, 253),
            diff_hunk_bg: rgb(230, 235, 250),
            staged_fg: rgb(64, 160, 43),
            unstaged_fg: rgb(223, 142, 29),
            untracked_fg: rgb(156, 160, 176),
            conflict_fg: rgb(254, 100, 11),
            conflict_bg: rgb(254, 235, 225),
            badge_verified_fg: rgb(64, 160, 43),
            badge_unverified_fg: rgb(172, 176, 190),
            ai_sparkle: rgb(136, 57, 239),
            ai_bg: rgb(238, 230, 250),
            ai_fg: rgb(76, 79, 105),
        }
    }

    // 5. Tokyo Night (Dark)
    fn tokyo_night() -> Self {
        Self {
            id: ThemeId::TokyoNight,
            name: "Tokyo Night",
            is_dark: true,
            bg: rgb(26, 27, 38),
            fg: rgb(192, 202, 245),
            dim_fg: rgb(86, 95, 137),
            header_bg: rgb(22, 22, 30),
            header_fg: rgb(192, 202, 245),
            status_bg: rgb(18, 18, 24),
            status_fg: rgb(122, 130, 166),
            border: rgb(59, 66, 97),
            border_focused: rgb(122, 162, 247),
            selection_bg: rgb(40, 52, 83),
            selection_fg: rgb(192, 202, 245),
            cursor: rgb(192, 202, 245),
            primary: rgb(122, 162, 247),
            secondary: rgb(187, 154, 247),
            accent: rgb(247, 118, 142),
            success: rgb(158, 206, 106),
            warning: rgb(224, 175, 104),
            error: rgb(247, 118, 142),
            info: rgb(125, 207, 255),
            diff_add_fg: rgb(158, 206, 106),
            diff_add_bg: rgb(32, 48, 38),
            diff_del_fg: rgb(247, 118, 142),
            diff_del_bg: rgb(54, 30, 40),
            diff_mod_fg: rgb(125, 207, 255),
            diff_mod_bg: rgb(30, 45, 60),
            diff_hunk_fg: rgb(187, 154, 247),
            diff_hunk_bg: rgb(38, 40, 60),
            staged_fg: rgb(158, 206, 106),
            unstaged_fg: rgb(224, 175, 104),
            untracked_fg: rgb(86, 95, 137),
            conflict_fg: rgb(255, 158, 100),
            conflict_bg: rgb(60, 38, 28),
            badge_verified_fg: rgb(158, 206, 106),
            badge_unverified_fg: rgb(86, 95, 137),
            ai_sparkle: rgb(187, 154, 247),
            ai_bg: rgb(36, 32, 54),
            ai_fg: rgb(192, 202, 245),
        }
    }

    // 6. Tokyo Night Storm (Dark)
    fn tokyo_night_storm() -> Self {
        Self {
            id: ThemeId::TokyoNightStorm,
            name: "Tokyo Night Storm",
            is_dark: true,
            bg: rgb(36, 40, 59),
            fg: rgb(192, 202, 245),
            dim_fg: rgb(90, 100, 145),
            header_bg: rgb(31, 35, 53),
            header_fg: rgb(192, 202, 245),
            status_bg: rgb(25, 28, 43),
            status_fg: rgb(122, 130, 166),
            border: rgb(65, 75, 110),
            border_focused: rgb(122, 162, 247),
            selection_bg: rgb(46, 60, 95),
            selection_fg: rgb(192, 202, 245),
            cursor: rgb(192, 202, 245),
            primary: rgb(122, 162, 247),
            secondary: rgb(187, 154, 247),
            accent: rgb(247, 118, 142),
            success: rgb(158, 206, 106),
            warning: rgb(224, 175, 104),
            error: rgb(247, 118, 142),
            info: rgb(125, 207, 255),
            diff_add_fg: rgb(158, 206, 106),
            diff_add_bg: rgb(40, 55, 48),
            diff_del_fg: rgb(247, 118, 142),
            diff_del_bg: rgb(60, 36, 46),
            diff_mod_fg: rgb(125, 207, 255),
            diff_mod_bg: rgb(36, 52, 68),
            diff_hunk_fg: rgb(187, 154, 247),
            diff_hunk_bg: rgb(45, 48, 70),
            staged_fg: rgb(158, 206, 106),
            unstaged_fg: rgb(224, 175, 104),
            untracked_fg: rgb(90, 100, 145),
            conflict_fg: rgb(255, 158, 100),
            conflict_bg: rgb(65, 42, 32),
            badge_verified_fg: rgb(158, 206, 106),
            badge_unverified_fg: rgb(90, 100, 145),
            ai_sparkle: rgb(187, 154, 247),
            ai_bg: rgb(46, 42, 65),
            ai_fg: rgb(192, 202, 245),
        }
    }

    // 7. Tokyo Night Light (Light)
    fn tokyo_night_light() -> Self {
        Self {
            id: ThemeId::TokyoNightLight,
            name: "Tokyo Night Light",
            is_dark: false,
            bg: rgb(213, 214, 219),
            fg: rgb(52, 59, 88),
            dim_fg: rgb(137, 142, 160),
            header_bg: rgb(200, 201, 208),
            header_fg: rgb(52, 59, 88),
            status_bg: rgb(188, 190, 198),
            status_fg: rgb(75, 82, 110),
            border: rgb(160, 165, 180),
            border_focused: rgb(52, 84, 140),
            selection_bg: rgb(180, 185, 200),
            selection_fg: rgb(52, 59, 88),
            cursor: rgb(52, 59, 88),
            primary: rgb(52, 84, 140),
            secondary: rgb(143, 80, 190),
            accent: rgb(200, 50, 80),
            success: rgb(72, 126, 40),
            warning: rgb(180, 115, 30),
            error: rgb(200, 50, 80),
            info: rgb(15, 110, 160),
            diff_add_fg: rgb(72, 126, 40),
            diff_add_bg: rgb(195, 225, 190),
            diff_del_fg: rgb(200, 50, 80),
            diff_del_bg: rgb(235, 195, 205),
            diff_mod_fg: rgb(15, 110, 160),
            diff_mod_bg: rgb(195, 215, 235),
            diff_hunk_fg: rgb(143, 80, 190),
            diff_hunk_bg: rgb(205, 200, 225),
            staged_fg: rgb(72, 126, 40),
            unstaged_fg: rgb(180, 115, 30),
            untracked_fg: rgb(137, 142, 160),
            conflict_fg: rgb(190, 95, 30),
            conflict_bg: rgb(235, 210, 190),
            badge_verified_fg: rgb(72, 126, 40),
            badge_unverified_fg: rgb(160, 165, 180),
            ai_sparkle: rgb(143, 80, 190),
            ai_bg: rgb(220, 210, 235),
            ai_fg: rgb(52, 59, 88),
        }
    }

    // 8. Nord (Dark)
    fn nord() -> Self {
        Self {
            id: ThemeId::Nord,
            name: "Nord",
            is_dark: true,
            bg: rgb(46, 52, 64),        // nord0
            fg: rgb(236, 239, 244),     // nord6
            dim_fg: rgb(140, 150, 170), // nord3.5
            header_bg: rgb(40, 45, 55),
            header_fg: rgb(236, 239, 244),
            status_bg: rgb(35, 40, 50),
            status_fg: rgb(216, 222, 233),      // nord4
            border: rgb(76, 86, 106),           // nord3
            border_focused: rgb(136, 192, 208), // nord8
            selection_bg: rgb(67, 76, 94),      // nord2
            selection_fg: rgb(236, 239, 244),
            cursor: rgb(236, 239, 244),
            primary: rgb(136, 192, 208),   // nord8
            secondary: rgb(129, 161, 193), // nord9
            accent: rgb(180, 142, 173),    // nord15
            success: rgb(163, 190, 140),   // nord14
            warning: rgb(235, 203, 139),   // nord13
            error: rgb(191, 97, 106),      // nord11
            info: rgb(143, 188, 187),      // nord7
            diff_add_fg: rgb(163, 190, 140),
            diff_add_bg: rgb(48, 64, 55),
            diff_del_fg: rgb(191, 97, 106),
            diff_del_bg: rgb(65, 45, 52),
            diff_mod_fg: rgb(136, 192, 208),
            diff_mod_bg: rgb(45, 60, 72),
            diff_hunk_fg: rgb(180, 142, 173),
            diff_hunk_bg: rgb(55, 62, 75),
            staged_fg: rgb(163, 190, 140),
            unstaged_fg: rgb(235, 203, 139),
            untracked_fg: rgb(140, 150, 170),
            conflict_fg: rgb(208, 135, 112), // nord12
            conflict_bg: rgb(68, 50, 45),
            badge_verified_fg: rgb(163, 190, 140),
            badge_unverified_fg: rgb(76, 86, 106),
            ai_sparkle: rgb(180, 142, 173),
            ai_bg: rgb(52, 56, 75),
            ai_fg: rgb(236, 239, 244),
        }
    }

    // 9. Dracula (Dark)
    fn dracula() -> Self {
        Self {
            id: ThemeId::Dracula,
            name: "Dracula",
            is_dark: true,
            bg: rgb(40, 42, 54),
            fg: rgb(248, 248, 242),
            dim_fg: rgb(98, 114, 164),
            header_bg: rgb(33, 34, 44),
            header_fg: rgb(248, 248, 242),
            status_bg: rgb(25, 26, 34),
            status_fg: rgb(189, 147, 249),
            border: rgb(68, 71, 90),
            border_focused: rgb(189, 147, 249),
            selection_bg: rgb(68, 71, 90),
            selection_fg: rgb(248, 248, 242),
            cursor: rgb(248, 248, 242),
            primary: rgb(189, 147, 249),
            secondary: rgb(255, 121, 198),
            accent: rgb(139, 233, 253),
            success: rgb(80, 250, 123),
            warning: rgb(241, 250, 140),
            error: rgb(255, 85, 85),
            info: rgb(139, 233, 253),
            diff_add_fg: rgb(80, 250, 123),
            diff_add_bg: rgb(35, 55, 45),
            diff_del_fg: rgb(255, 85, 85),
            diff_del_bg: rgb(65, 35, 45),
            diff_mod_fg: rgb(139, 233, 253),
            diff_mod_bg: rgb(35, 50, 65),
            diff_hunk_fg: rgb(255, 121, 198),
            diff_hunk_bg: rgb(50, 50, 70),
            staged_fg: rgb(80, 250, 123),
            unstaged_fg: rgb(241, 250, 140),
            untracked_fg: rgb(98, 114, 164),
            conflict_fg: rgb(255, 184, 108),
            conflict_bg: rgb(70, 48, 35),
            badge_verified_fg: rgb(80, 250, 123),
            badge_unverified_fg: rgb(98, 114, 164),
            ai_sparkle: rgb(255, 121, 198),
            ai_bg: rgb(52, 45, 72),
            ai_fg: rgb(248, 248, 242),
        }
    }

    // 10. Gruvbox Dark (Dark)
    fn gruvbox_dark() -> Self {
        Self {
            id: ThemeId::GruvboxDark,
            name: "Gruvbox Dark",
            is_dark: true,
            bg: rgb(40, 40, 40),        // #282828
            fg: rgb(235, 219, 178),     // #ebdbb2
            dim_fg: rgb(146, 131, 116), // #928374
            header_bg: rgb(30, 30, 30),
            header_fg: rgb(235, 219, 178),
            status_bg: rgb(20, 20, 20),
            status_fg: rgb(213, 196, 161),
            border: rgb(80, 73, 69),
            border_focused: rgb(250, 189, 47),
            selection_bg: rgb(60, 56, 54),
            selection_fg: rgb(235, 219, 178),
            cursor: rgb(235, 219, 178),
            primary: rgb(250, 189, 47),    // Yellow
            secondary: rgb(131, 165, 152), // Blue
            accent: rgb(211, 134, 155),    // Purple
            success: rgb(184, 187, 38),    // Green
            warning: rgb(254, 128, 25),    // Orange
            error: rgb(251, 73, 52),       // Red
            info: rgb(142, 192, 124),      // Aqua
            diff_add_fg: rgb(184, 187, 38),
            diff_add_bg: rgb(48, 52, 35),
            diff_del_fg: rgb(251, 73, 52),
            diff_del_bg: rgb(60, 35, 35),
            diff_mod_fg: rgb(142, 192, 124),
            diff_mod_bg: rgb(40, 52, 50),
            diff_hunk_fg: rgb(211, 134, 155),
            diff_hunk_bg: rgb(52, 48, 55),
            staged_fg: rgb(184, 187, 38),
            unstaged_fg: rgb(250, 189, 47),
            untracked_fg: rgb(146, 131, 116),
            conflict_fg: rgb(254, 128, 25),
            conflict_bg: rgb(65, 45, 30),
            badge_verified_fg: rgb(184, 187, 38),
            badge_unverified_fg: rgb(146, 131, 116),
            ai_sparkle: rgb(211, 134, 155),
            ai_bg: rgb(55, 45, 52),
            ai_fg: rgb(235, 219, 178),
        }
    }

    // 11. Gruvbox Light (Light)
    fn gruvbox_light() -> Self {
        Self {
            id: ThemeId::GruvboxLight,
            name: "Gruvbox Light",
            is_dark: false,
            bg: rgb(251, 241, 199),     // #fbf1c7
            fg: rgb(60, 56, 54),        // #3c3836
            dim_fg: rgb(168, 153, 132), // #a89984
            header_bg: rgb(242, 229, 188),
            header_fg: rgb(60, 56, 54),
            status_bg: rgb(235, 219, 178),
            status_fg: rgb(80, 73, 69),
            border: rgb(213, 196, 161),
            border_focused: rgb(181, 118, 20),
            selection_bg: rgb(235, 219, 178),
            selection_fg: rgb(60, 56, 54),
            cursor: rgb(60, 56, 54),
            primary: rgb(181, 118, 20),
            secondary: rgb(69, 133, 136),
            accent: rgb(177, 98, 134),
            success: rgb(152, 151, 26),
            warning: rgb(175, 58, 3),
            error: rgb(204, 36, 29),
            info: rgb(104, 157, 106),
            diff_add_fg: rgb(152, 151, 26),
            diff_add_bg: rgb(230, 235, 200),
            diff_del_fg: rgb(204, 36, 29),
            diff_del_bg: rgb(248, 215, 210),
            diff_mod_fg: rgb(69, 133, 136),
            diff_mod_bg: rgb(220, 235, 235),
            diff_hunk_fg: rgb(177, 98, 134),
            diff_hunk_bg: rgb(238, 225, 232),
            staged_fg: rgb(152, 151, 26),
            unstaged_fg: rgb(181, 118, 20),
            untracked_fg: rgb(168, 153, 132),
            conflict_fg: rgb(175, 58, 3),
            conflict_bg: rgb(250, 220, 205),
            badge_verified_fg: rgb(152, 151, 26),
            badge_unverified_fg: rgb(168, 153, 132),
            ai_sparkle: rgb(177, 98, 134),
            ai_bg: rgb(242, 225, 235),
            ai_fg: rgb(60, 56, 54),
        }
    }

    // 12. Solarized Dark (Dark)
    fn solarized_dark() -> Self {
        Self {
            id: ThemeId::SolarizedDark,
            name: "Solarized Dark",
            is_dark: true,
            bg: rgb(0, 43, 54),            // base03
            fg: rgb(131, 148, 150),        // base0
            dim_fg: rgb(88, 110, 117),     // base01
            header_bg: rgb(7, 54, 66),     // base02
            header_fg: rgb(147, 161, 161), // base1
            status_bg: rgb(0, 33, 42),
            status_fg: rgb(147, 161, 161),
            border: rgb(88, 110, 117),
            border_focused: rgb(38, 139, 210), // blue
            selection_bg: rgb(7, 54, 66),
            selection_fg: rgb(147, 161, 161),
            cursor: rgb(147, 161, 161),
            primary: rgb(38, 139, 210),    // blue
            secondary: rgb(108, 113, 196), // violet
            accent: rgb(211, 54, 130),     // magenta
            success: rgb(133, 153, 0),     // green
            warning: rgb(181, 137, 0),     // yellow
            error: rgb(220, 50, 47),       // red
            info: rgb(42, 161, 152),       // cyan
            diff_add_fg: rgb(133, 153, 0),
            diff_add_bg: rgb(15, 60, 45),
            diff_del_fg: rgb(220, 50, 47),
            diff_del_bg: rgb(55, 35, 40),
            diff_mod_fg: rgb(42, 161, 152),
            diff_mod_bg: rgb(15, 58, 65),
            diff_hunk_fg: rgb(108, 113, 196),
            diff_hunk_bg: rgb(25, 52, 75),
            staged_fg: rgb(133, 153, 0),
            unstaged_fg: rgb(181, 137, 0),
            untracked_fg: rgb(88, 110, 117),
            conflict_fg: rgb(203, 75, 22), // orange
            conflict_bg: rgb(60, 45, 30),
            badge_verified_fg: rgb(133, 153, 0),
            badge_unverified_fg: rgb(88, 110, 117),
            ai_sparkle: rgb(211, 54, 130),
            ai_bg: rgb(25, 48, 65),
            ai_fg: rgb(147, 161, 161),
        }
    }

    // 13. Solarized Light (Light)
    fn solarized_light() -> Self {
        Self {
            id: ThemeId::SolarizedLight,
            name: "Solarized Light",
            is_dark: false,
            bg: rgb(253, 246, 227),        // base3
            fg: rgb(101, 123, 131),        // base00
            dim_fg: rgb(147, 161, 161),    // base1
            header_bg: rgb(238, 232, 213), // base2
            header_fg: rgb(101, 123, 131),
            status_bg: rgb(230, 222, 200),
            status_fg: rgb(88, 110, 117),
            border: rgb(200, 195, 175),
            border_focused: rgb(38, 139, 210),
            selection_bg: rgb(238, 232, 213),
            selection_fg: rgb(88, 110, 117),
            cursor: rgb(88, 110, 117),
            primary: rgb(38, 139, 210),
            secondary: rgb(108, 113, 196),
            accent: rgb(211, 54, 130),
            success: rgb(133, 153, 0),
            warning: rgb(181, 137, 0),
            error: rgb(220, 50, 47),
            info: rgb(42, 161, 152),
            diff_add_fg: rgb(133, 153, 0),
            diff_add_bg: rgb(230, 242, 210),
            diff_del_fg: rgb(220, 50, 47),
            diff_del_bg: rgb(250, 225, 225),
            diff_mod_fg: rgb(42, 161, 152),
            diff_mod_bg: rgb(225, 242, 240),
            diff_hunk_fg: rgb(108, 113, 196),
            diff_hunk_bg: rgb(235, 232, 245),
            staged_fg: rgb(133, 153, 0),
            unstaged_fg: rgb(181, 137, 0),
            untracked_fg: rgb(147, 161, 161),
            conflict_fg: rgb(203, 75, 22),
            conflict_bg: rgb(250, 230, 215),
            badge_verified_fg: rgb(133, 153, 0),
            badge_unverified_fg: rgb(147, 161, 161),
            ai_sparkle: rgb(211, 54, 130),
            ai_bg: rgb(245, 235, 245),
            ai_fg: rgb(101, 123, 131),
        }
    }

    // 14. One Dark (Atom) (Dark)
    fn one_dark() -> Self {
        Self {
            id: ThemeId::OneDark,
            name: "One Dark",
            is_dark: true,
            bg: rgb(40, 44, 52),
            fg: rgb(171, 178, 191),
            dim_fg: rgb(92, 99, 112),
            header_bg: rgb(33, 37, 43),
            header_fg: rgb(171, 178, 191),
            status_bg: rgb(27, 30, 35),
            status_fg: rgb(150, 156, 168),
            border: rgb(62, 68, 81),
            border_focused: rgb(97, 175, 239),
            selection_bg: rgb(62, 68, 81),
            selection_fg: rgb(220, 223, 228),
            cursor: rgb(220, 223, 228),
            primary: rgb(97, 175, 239),    // Blue
            secondary: rgb(198, 120, 221), // Purple
            accent: rgb(224, 108, 117),    // Red
            success: rgb(152, 195, 121),   // Green
            warning: rgb(229, 192, 123),   // Yellow
            error: rgb(224, 108, 117),     // Red
            info: rgb(86, 182, 194),       // Cyan
            diff_add_fg: rgb(152, 195, 121),
            diff_add_bg: rgb(45, 60, 48),
            diff_del_fg: rgb(224, 108, 117),
            diff_del_bg: rgb(62, 40, 45),
            diff_mod_fg: rgb(86, 182, 194),
            diff_mod_bg: rgb(38, 55, 65),
            diff_hunk_fg: rgb(198, 120, 221),
            diff_hunk_bg: rgb(50, 48, 68),
            staged_fg: rgb(152, 195, 121),
            unstaged_fg: rgb(229, 192, 123),
            untracked_fg: rgb(92, 99, 112),
            conflict_fg: rgb(209, 154, 102), // Orange
            conflict_bg: rgb(65, 48, 35),
            badge_verified_fg: rgb(152, 195, 121),
            badge_unverified_fg: rgb(92, 99, 112),
            ai_sparkle: rgb(198, 120, 221),
            ai_bg: rgb(50, 42, 62),
            ai_fg: rgb(220, 223, 228),
        }
    }

    // 15. One Light (Light)
    fn one_light() -> Self {
        Self {
            id: ThemeId::OneLight,
            name: "One Light",
            is_dark: false,
            bg: rgb(250, 250, 250),
            fg: rgb(56, 58, 66),
            dim_fg: rgb(160, 161, 167),
            header_bg: rgb(240, 240, 240),
            header_fg: rgb(56, 58, 66),
            status_bg: rgb(230, 230, 230),
            status_fg: rgb(90, 92, 100),
            border: rgb(215, 215, 215),
            border_focused: rgb(64, 120, 242),
            selection_bg: rgb(229, 231, 235),
            selection_fg: rgb(56, 58, 66),
            cursor: rgb(56, 58, 66),
            primary: rgb(64, 120, 242),
            secondary: rgb(166, 38, 164),
            accent: rgb(228, 86, 73),
            success: rgb(80, 161, 79),
            warning: rgb(193, 132, 1),
            error: rgb(228, 86, 73),
            info: rgb(1, 132, 188),
            diff_add_fg: rgb(80, 161, 79),
            diff_add_bg: rgb(225, 242, 225),
            diff_del_fg: rgb(228, 86, 73),
            diff_del_bg: rgb(252, 226, 225),
            diff_mod_fg: rgb(1, 132, 188),
            diff_mod_bg: rgb(225, 240, 250),
            diff_hunk_fg: rgb(166, 38, 164),
            diff_hunk_bg: rgb(242, 230, 245),
            staged_fg: rgb(80, 161, 79),
            unstaged_fg: rgb(193, 132, 1),
            untracked_fg: rgb(160, 161, 167),
            conflict_fg: rgb(152, 104, 1),
            conflict_bg: rgb(250, 235, 220),
            badge_verified_fg: rgb(80, 161, 79),
            badge_unverified_fg: rgb(160, 161, 167),
            ai_sparkle: rgb(166, 38, 164),
            ai_bg: rgb(245, 235, 250),
            ai_fg: rgb(56, 58, 66),
        }
    }

    // 16. Rosé Pine (Dark)
    fn rose_pine() -> Self {
        Self {
            id: ThemeId::RosePine,
            name: "Rosé Pine",
            is_dark: true,
            bg: rgb(25, 23, 36),
            fg: rgb(224, 222, 244),
            dim_fg: rgb(110, 106, 134),
            header_bg: rgb(31, 29, 46),
            header_fg: rgb(224, 222, 244),
            status_bg: rgb(20, 18, 29),
            status_fg: rgb(144, 140, 170),
            border: rgb(68, 65, 90),
            border_focused: rgb(156, 207, 216),
            selection_bg: rgb(42, 40, 62),
            selection_fg: rgb(224, 222, 244),
            cursor: rgb(224, 222, 244),
            primary: rgb(156, 207, 216),   // Foam
            secondary: rgb(196, 167, 231), // Iris
            accent: rgb(235, 111, 146),    // Love
            success: rgb(49, 116, 143),    // Pine
            warning: rgb(246, 193, 119),   // Gold
            error: rgb(235, 111, 146),     // Love
            info: rgb(156, 207, 216),      // Foam
            diff_add_fg: rgb(156, 207, 216),
            diff_add_bg: rgb(30, 45, 52),
            diff_del_fg: rgb(235, 111, 146),
            diff_del_bg: rgb(55, 30, 42),
            diff_mod_fg: rgb(196, 167, 231),
            diff_mod_bg: rgb(42, 38, 60),
            diff_hunk_fg: rgb(235, 188, 186), // Rose
            diff_hunk_bg: rgb(45, 38, 55),
            staged_fg: rgb(156, 207, 216),
            unstaged_fg: rgb(246, 193, 119),
            untracked_fg: rgb(110, 106, 134),
            conflict_fg: rgb(235, 188, 186),
            conflict_bg: rgb(60, 40, 48),
            badge_verified_fg: rgb(156, 207, 216),
            badge_unverified_fg: rgb(110, 106, 134),
            ai_sparkle: rgb(196, 167, 231),
            ai_bg: rgb(40, 32, 54),
            ai_fg: rgb(224, 222, 244),
        }
    }

    // 17. Rosé Pine Moon (Medium Dark)
    fn rose_pine_moon() -> Self {
        Self {
            id: ThemeId::RosePineMoon,
            name: "Rosé Pine Moon",
            is_dark: true,
            bg: rgb(35, 33, 54),
            fg: rgb(224, 222, 244),
            dim_fg: rgb(110, 106, 134),
            header_bg: rgb(42, 39, 63),
            header_fg: rgb(224, 222, 244),
            status_bg: rgb(28, 26, 43),
            status_fg: rgb(144, 140, 170),
            border: rgb(75, 72, 100),
            border_focused: rgb(156, 207, 216),
            selection_bg: rgb(57, 53, 82),
            selection_fg: rgb(224, 222, 244),
            cursor: rgb(224, 222, 244),
            primary: rgb(156, 207, 216),
            secondary: rgb(196, 167, 231),
            accent: rgb(235, 111, 146),
            success: rgb(62, 143, 176),
            warning: rgb(246, 193, 119),
            error: rgb(235, 111, 146),
            info: rgb(156, 207, 216),
            diff_add_fg: rgb(156, 207, 216),
            diff_add_bg: rgb(38, 52, 62),
            diff_del_fg: rgb(235, 111, 146),
            diff_del_bg: rgb(62, 36, 48),
            diff_mod_fg: rgb(196, 167, 231),
            diff_mod_bg: rgb(50, 44, 70),
            diff_hunk_fg: rgb(235, 188, 186),
            diff_hunk_bg: rgb(52, 45, 68),
            staged_fg: rgb(156, 207, 216),
            unstaged_fg: rgb(246, 193, 119),
            untracked_fg: rgb(110, 106, 134),
            conflict_fg: rgb(235, 188, 186),
            conflict_bg: rgb(68, 46, 54),
            badge_verified_fg: rgb(156, 207, 216),
            badge_unverified_fg: rgb(110, 106, 134),
            ai_sparkle: rgb(196, 167, 231),
            ai_bg: rgb(48, 38, 65),
            ai_fg: rgb(224, 222, 244),
        }
    }

    // 18. Rosé Pine Dawn (Light)
    fn rose_pine_dawn() -> Self {
        Self {
            id: ThemeId::RosePineDawn,
            name: "Rosé Pine Dawn",
            is_dark: false,
            bg: rgb(250, 244, 237),
            fg: rgb(87, 82, 121),
            dim_fg: rgb(152, 147, 165),
            header_bg: rgb(242, 233, 222),
            header_fg: rgb(87, 82, 121),
            status_bg: rgb(235, 224, 210),
            status_fg: rgb(121, 117, 147),
            border: rgb(206, 202, 218),
            border_focused: rgb(40, 105, 131),
            selection_bg: rgb(235, 228, 220),
            selection_fg: rgb(87, 82, 121),
            cursor: rgb(87, 82, 121),
            primary: rgb(40, 105, 131),
            secondary: rgb(144, 122, 169),
            accent: rgb(180, 99, 122),
            success: rgb(86, 148, 159),
            warning: rgb(234, 157, 52),
            error: rgb(180, 99, 122),
            info: rgb(40, 105, 131),
            diff_add_fg: rgb(86, 148, 159),
            diff_add_bg: rgb(225, 240, 242),
            diff_del_fg: rgb(180, 99, 122),
            diff_del_bg: rgb(248, 225, 230),
            diff_mod_fg: rgb(144, 122, 169),
            diff_mod_bg: rgb(238, 232, 246),
            diff_hunk_fg: rgb(215, 130, 126),
            diff_hunk_bg: rgb(242, 230, 232),
            staged_fg: rgb(86, 148, 159),
            unstaged_fg: rgb(234, 157, 52),
            untracked_fg: rgb(152, 147, 165),
            conflict_fg: rgb(215, 130, 126),
            conflict_bg: rgb(250, 230, 225),
            badge_verified_fg: rgb(86, 148, 159),
            badge_unverified_fg: rgb(152, 147, 165),
            ai_sparkle: rgb(144, 122, 169),
            ai_bg: rgb(245, 235, 245),
            ai_fg: rgb(87, 82, 121),
        }
    }

    // 19. Monokai Pro (Dark)
    fn monokai_pro() -> Self {
        Self {
            id: ThemeId::MonokaiPro,
            name: "Monokai Pro",
            is_dark: true,
            bg: rgb(45, 42, 46),
            fg: rgb(252, 252, 250),
            dim_fg: rgb(114, 112, 114),
            header_bg: rgb(34, 31, 34),
            header_fg: rgb(252, 252, 250),
            status_bg: rgb(25, 24, 26),
            status_fg: rgb(180, 178, 180),
            border: rgb(64, 62, 65),
            border_focused: rgb(120, 220, 232),
            selection_bg: rgb(64, 62, 65),
            selection_fg: rgb(252, 252, 250),
            cursor: rgb(252, 252, 250),
            primary: rgb(120, 220, 232),   // Cyan
            secondary: rgb(171, 157, 242), // Purple
            accent: rgb(255, 97, 136),     // Red / Pink
            success: rgb(169, 220, 107),   // Green
            warning: rgb(255, 216, 102),   // Yellow
            error: rgb(255, 97, 136),      // Red
            info: rgb(120, 220, 232),      // Cyan
            diff_add_fg: rgb(169, 220, 107),
            diff_add_bg: rgb(45, 58, 42),
            diff_del_fg: rgb(255, 97, 136),
            diff_del_bg: rgb(65, 36, 44),
            diff_mod_fg: rgb(120, 220, 232),
            diff_mod_bg: rgb(38, 54, 62),
            diff_hunk_fg: rgb(171, 157, 242),
            diff_hunk_bg: rgb(52, 48, 66),
            staged_fg: rgb(169, 220, 107),
            unstaged_fg: rgb(255, 216, 102),
            untracked_fg: rgb(114, 112, 114),
            conflict_fg: rgb(252, 152, 103), // Orange
            conflict_bg: rgb(68, 48, 36),
            badge_verified_fg: rgb(169, 220, 107),
            badge_unverified_fg: rgb(114, 112, 114),
            ai_sparkle: rgb(171, 157, 242),
            ai_bg: rgb(52, 42, 62),
            ai_fg: rgb(252, 252, 250),
        }
    }

    // 20. Synthwave '84 (Dark)
    fn synthwave_84() -> Self {
        Self {
            id: ThemeId::Synthwave84,
            name: "Synthwave '84",
            is_dark: true,
            bg: rgb(38, 35, 53),
            fg: rgb(249, 2, 237),
            dim_fg: rgb(109, 87, 135),
            header_bg: rgb(30, 27, 43),
            header_fg: rgb(254, 228, 97),
            status_bg: rgb(22, 19, 32),
            status_fg: rgb(54, 243, 223),
            border: rgb(85, 60, 115),
            border_focused: rgb(249, 44, 114),
            selection_bg: rgb(65, 45, 90),
            selection_fg: rgb(254, 228, 97),
            cursor: rgb(249, 44, 114),
            primary: rgb(249, 44, 114),   // Neon Pink
            secondary: rgb(54, 243, 223), // Neon Cyan
            accent: rgb(254, 228, 97),    // Neon Yellow
            success: rgb(114, 241, 184),  // Neon Mint
            warning: rgb(255, 142, 60),   // Neon Orange
            error: rgb(254, 68, 80),      // Neon Red
            info: rgb(54, 243, 223),      // Neon Cyan
            diff_add_fg: rgb(114, 241, 184),
            diff_add_bg: rgb(35, 60, 52),
            diff_del_fg: rgb(254, 68, 80),
            diff_del_bg: rgb(68, 32, 48),
            diff_mod_fg: rgb(54, 243, 223),
            diff_mod_bg: rgb(30, 55, 68),
            diff_hunk_fg: rgb(249, 2, 237),
            diff_hunk_bg: rgb(55, 38, 75),
            staged_fg: rgb(114, 241, 184),
            unstaged_fg: rgb(254, 228, 97),
            untracked_fg: rgb(109, 87, 135),
            conflict_fg: rgb(255, 142, 60),
            conflict_bg: rgb(75, 44, 38),
            badge_verified_fg: rgb(114, 241, 184),
            badge_unverified_fg: rgb(109, 87, 135),
            ai_sparkle: rgb(249, 44, 114),
            ai_bg: rgb(55, 32, 70),
            ai_fg: rgb(254, 228, 97),
        }
    }

    // 21. Everforest Dark (Dark)
    fn everforest_dark() -> Self {
        Self {
            id: ThemeId::EverforestDark,
            name: "Everforest Dark",
            is_dark: true,
            bg: rgb(45, 53, 59),        // #2d353b
            fg: rgb(211, 198, 170),     // #d3c6aa
            dim_fg: rgb(122, 132, 120), // #7a8478
            header_bg: rgb(35, 42, 46), // #232a2e
            header_fg: rgb(211, 198, 170),
            status_bg: rgb(26, 32, 36),
            status_fg: rgb(157, 169, 160),
            border: rgb(71, 86, 92),
            border_focused: rgb(167, 192, 128), // Green
            selection_bg: rgb(60, 74, 80),
            selection_fg: rgb(211, 198, 170),
            cursor: rgb(211, 198, 170),
            primary: rgb(167, 192, 128),   // Green
            secondary: rgb(127, 187, 179), // Aqua
            accent: rgb(214, 153, 182),    // Purple
            success: rgb(167, 192, 128),   // Green
            warning: rgb(219, 188, 127),   // Yellow
            error: rgb(230, 126, 128),     // Red
            info: rgb(127, 187, 179),      // Aqua
            diff_add_fg: rgb(167, 192, 128),
            diff_add_bg: rgb(42, 58, 48),
            diff_del_fg: rgb(230, 126, 128),
            diff_del_bg: rgb(60, 42, 46),
            diff_mod_fg: rgb(127, 187, 179),
            diff_mod_bg: rgb(38, 54, 58),
            diff_hunk_fg: rgb(214, 153, 182),
            diff_hunk_bg: rgb(52, 50, 64),
            staged_fg: rgb(167, 192, 128),
            unstaged_fg: rgb(219, 188, 127),
            untracked_fg: rgb(122, 132, 120),
            conflict_fg: rgb(229, 157, 114), // Orange
            conflict_bg: rgb(65, 48, 38),
            badge_verified_fg: rgb(167, 192, 128),
            badge_unverified_fg: rgb(122, 132, 120),
            ai_sparkle: rgb(214, 153, 182),
            ai_bg: rgb(52, 45, 58),
            ai_fg: rgb(211, 198, 170),
        }
    }
}
