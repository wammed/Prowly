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

#[test]
fn test_rebase_command_search_and_scrolling() {
    let mut palette = CommandPaletteModal::new();
    palette.open();

    // 1. Total items should exceed initial 13
    assert!(PaletteAction::ALL.len() > 13);

    // 2. Search for "rebase"
    for c in "rebase".chars() {
        palette.push_char(c);
    }
    let filtered = palette.filtered_actions();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "rebase");
    assert_eq!(filtered[0].title, "Git: Interactive Rebase & Squash");
    assert_eq!(filtered[0].shortcut, Some("r"));

    // 3. Clear and test scrolling past 13 items
    palette.open();
    for _ in 0..15 {
        palette.next();
    }
    assert_eq!(palette.selected_index, 15);
    assert!(palette.selected_action().is_some());
}

