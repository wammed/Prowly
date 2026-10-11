use prowly::ui::modals::command_palette::{CommandPaletteModal, PaletteAction};

#[test]
fn test_command_palette_filtering_and_navigation() {
    let mut palette = CommandPaletteModal::new();
    palette.open();
    assert!(palette.is_open);

    let all_actions = palette.filtered_actions();
    assert_eq!(all_actions.len(), PaletteAction::ALL.len());

    // Type query "commit"
    for c in "commit".chars() {
        palette.push_char(c);
    }
    assert_eq!(palette.query, "commit");

    let filtered = palette.filtered_actions();
    assert!(!filtered.is_empty());
    assert!(filtered.iter().any(|a| a.id == "commit"));

    // Selection
    let selected = palette.selected_action();
    assert!(selected.is_some());

    // Pop characters
    for _ in 0..6 {
        palette.pop_char();
    }
    assert_eq!(palette.query, "");
    assert_eq!(palette.filtered_actions().len(), PaletteAction::ALL.len());

    // Test Navigation
    palette.next();
    assert_eq!(palette.selected_index, 1);
    palette.previous();
    assert_eq!(palette.selected_index, 0);

    // Close
    palette.close();
    assert!(!palette.is_open);
}

#[test]
fn test_palette_action_shortcuts() {
    let stage_all = PaletteAction::ALL
        .iter()
        .find(|a| a.id == "stage_all")
        .unwrap();
    assert_eq!(stage_all.shortcut, Some("a"));

    let cycle_theme = PaletteAction::ALL
        .iter()
        .find(|a| a.id == "cycle_theme")
        .unwrap();
    assert_eq!(cycle_theme.shortcut, Some("t"));
}
