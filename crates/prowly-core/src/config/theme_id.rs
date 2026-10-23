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
