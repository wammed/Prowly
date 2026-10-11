use prowly::ui::{Theme, ThemeId};

#[test]
fn test_all_21_themes_present() {
    assert_eq!(ThemeId::ALL.len(), 21);
    for id in ThemeId::ALL {
        let theme = Theme::get(*id);
        assert_eq!(theme.id, *id);
        assert!(!theme.name.is_empty());
    }
}

#[test]
fn test_theme_from_name_lookup() {
    assert_eq!(
        ThemeId::from_name("Catppuccin Mocha"),
        Some(ThemeId::CatppuccinMocha)
    );
    assert_eq!(
        ThemeId::from_name("catppuccin_mocha"),
        Some(ThemeId::CatppuccinMocha)
    );
    assert_eq!(ThemeId::from_name("tokyo night"), Some(ThemeId::TokyoNight));
    assert_eq!(
        ThemeId::from_name("tokyo-night-storm"),
        Some(ThemeId::TokyoNightStorm)
    );
    assert_eq!(ThemeId::from_name("nord"), Some(ThemeId::Nord));
    assert_eq!(ThemeId::from_name("dracula"), Some(ThemeId::Dracula));
    assert_eq!(
        ThemeId::from_name("gruvbox dark"),
        Some(ThemeId::GruvboxDark)
    );
    assert_eq!(
        ThemeId::from_name("gruvbox light"),
        Some(ThemeId::GruvboxLight)
    );
    assert_eq!(ThemeId::from_name("rose-pine"), Some(ThemeId::RosePine));
    assert_eq!(
        ThemeId::from_name("synthwave '84"),
        Some(ThemeId::Synthwave84)
    );
    assert_eq!(
        ThemeId::from_name("everforest dark"),
        Some(ThemeId::EverforestDark)
    );
}

#[test]
fn test_light_and_dark_classification() {
    let light_themes = [
        ThemeId::CatppuccinLatte,
        ThemeId::TokyoNightLight,
        ThemeId::GruvboxLight,
        ThemeId::SolarizedLight,
        ThemeId::OneLight,
        ThemeId::RosePineDawn,
    ];

    for id in ThemeId::ALL {
        let theme = Theme::get(*id);
        if light_themes.contains(id) {
            assert!(!theme.is_dark, "Theme {:?} should be light", id);
        } else {
            assert!(theme.is_dark, "Theme {:?} should be dark", id);
        }
    }
}

#[test]
fn test_semantic_colors_distinct() {
    for id in ThemeId::ALL {
        let theme = Theme::get(*id);
        // Diff add and diff del should never be identical
        assert_ne!(
            theme.diff_add_fg, theme.diff_del_fg,
            "Diff add and del colors must differ in theme {:?}",
            id
        );
        // Background and foreground should never be identical
        assert_ne!(
            theme.bg, theme.fg,
            "Background and foreground must differ in theme {:?}",
            id
        );
    }
}
