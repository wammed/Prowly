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

#[test]
fn test_multi_layer_navigation_and_rollback() {
    use prowly::ui::modals::command_palette::{PaletteEnterResult, PaletteLayer};

    let mut palette = CommandPaletteModal::new();
    palette.open();
    assert_eq!(palette.layer, PaletteLayer::Root);

    // Filter and select "browse_git_cli"
    for c in "browse".chars() {
        palette.push_char(c);
    }
    assert_eq!(palette.selected_action().unwrap().id, "browse_git_cli");

    // Press Enter to enter Layer 2
    let res = palette.handle_enter();
    assert_eq!(res, PaletteEnterResult::LayerChanged);
    assert_eq!(palette.layer, PaletteLayer::GitSubcommand);
    assert!(palette.filtered_subcommands().len() >= 100);

    // Search for "bisect" in Layer 2
    for c in "bisect".chars() {
        palette.push_char(c);
    }
    let subs = palette.filtered_subcommands();
    assert!(!subs.is_empty());
    assert_eq!(subs[0].name, "bisect");

    // Press Enter to enter Layer 3
    let res = palette.handle_enter();
    assert_eq!(res, PaletteEnterResult::LayerChanged);
    match &palette.layer {
        PaletteLayer::ArgsInput { subcommand, .. } => assert_eq!(subcommand, "bisect"),
        _ => panic!("Expected ArgsInput layer"),
    }

    // When query is empty, pop_char() acts as Backspace rollback to Layer 2
    palette.pop_char();
    assert_eq!(palette.layer, PaletteLayer::GitSubcommand);

    // pop_char() again with empty query rolls back to Layer 1 (Root)
    palette.pop_char();
    assert_eq!(palette.layer, PaletteLayer::Root);

    // Esc navigation test
    palette.handle_enter(); // browse_git_cli -> Layer 2
    assert_eq!(palette.layer, PaletteLayer::GitSubcommand);
    assert!(palette.handle_esc()); // back to Root
    assert_eq!(palette.layer, PaletteLayer::Root);
    assert!(!palette.handle_esc()); // Esc at Root returns false (to close)
}

#[test]
fn test_tab_flag_autocomplete() {
    use prowly::ui::modals::command_palette::PaletteLayer;

    let mut palette = CommandPaletteModal::new();
    palette.open();

    // Enter Layer 3 for "commit"
    palette.layer = PaletteLayer::ArgsInput {
        subcommand: "commit".to_string(),
        description: "Record changes to repository".to_string(),
        common_flags: vec!["-m".to_string(), "--amend".to_string(), "-a".to_string()],
    };

    // First tab inserts "-m"
    palette.handle_tab();
    assert_eq!(palette.query, "-m");

    // Second tab appends next flag
    palette.handle_tab();
    assert_eq!(palette.query, "-m --amend");
}

#[test]
fn test_direct_shortcut_jump() {
    use prowly::ui::modals::command_palette::{PaletteEnterResult, PaletteLayer};

    let mut palette = CommandPaletteModal::new();
    palette.open();

    // Shortcut jump with args: `git cherry-pick --continue`
    for c in "git cherry-pick --continue".chars() {
        palette.push_char(c);
    }
    let res = palette.handle_enter();
    assert_eq!(
        res,
        PaletteEnterResult::ExecuteGitCommand {
            subcommand: "cherry-pick".to_string(),
            args: "--continue".to_string(),
        }
    );

    // Shortcut jump without args: `cherry-pick` directly opens Layer 3
    palette.open();
    for c in "cherry-pick".chars() {
        palette.push_char(c);
    }
    let res = palette.handle_enter();
    assert_eq!(res, PaletteEnterResult::LayerChanged);
    match &palette.layer {
        PaletteLayer::ArgsInput { subcommand, .. } => assert_eq!(subcommand, "cherry-pick"),
        _ => panic!("Expected ArgsInput layer"),
    }
}

#[test]
fn test_safety_guard_destructive_actions() {
    use prowly::ui::modals::command_palette::{PaletteEnterResult, PaletteLayer};

    let mut palette = CommandPaletteModal::new();
    palette.open();

    palette.layer = PaletteLayer::ArgsInput {
        subcommand: "push".to_string(),
        description: "Update remote refs".to_string(),
        common_flags: vec!["--force".to_string()],
    };

    // User types dangerous flag
    for c in "--force origin main".chars() {
        palette.push_char(c);
    }

    // First Enter triggers safety guard
    let res1 = palette.handle_enter();
    match res1 {
        PaletteEnterResult::DangerWarning(msg) => {
            assert!(msg.contains("Force push"));
        }
        _ => panic!("Expected DangerWarning"),
    }
    assert!(palette.is_confirmed_danger());

    // Second Enter proceeds with execution
    let res2 = palette.handle_enter();
    assert_eq!(
        res2,
        PaletteEnterResult::ExecuteGitCommand {
            subcommand: "push".to_string(),
            args: "--force origin main".to_string(),
        }
    );
}

