use prowly::ui::modals::context_menu::{ContextMenu, ContextMenuAction};

#[test]
fn test_context_menu_open_and_close() {
    let mut menu = ContextMenu::new();
    assert!(!menu.is_open);

    menu.open_at(Some("src/main.rs".to_string()), 15, 20);
    assert!(menu.is_open);
    assert_eq!(menu.target_file, Some("src/main.rs".to_string()));
    assert_eq!(menu.anchor_x, 15);
    assert_eq!(menu.anchor_y, 20);
    assert_eq!(menu.selected_index, 0);

    menu.close();
    assert!(!menu.is_open);
}

#[test]
fn test_context_menu_navigation() {
    let mut menu = ContextMenu::new();
    menu.open_at(Some("src/ui/theme.rs".to_string()), 10, 10);

    assert_eq!(menu.selected_action(), ContextMenuAction::ToggleStage);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::SafeDiscard);

    menu.next();
    assert_eq!(menu.selected_action(), ContextMenuAction::CopyPath);

    menu.previous();
    assert_eq!(menu.selected_action(), ContextMenuAction::SafeDiscard);

    // Navigate to PreviewMarkdown
    menu.next(); // CopyPath
    menu.next(); // CopyDiff
    menu.next(); // AskAi
    menu.next(); // PreviewMarkdown
    assert_eq!(menu.selected_action(), ContextMenuAction::PreviewMarkdown);
    assert_eq!(menu.selected_action().label(), "Preview Markdown (m)");
}

#[test]
fn test_reveal_in_file_manager_path_resolution() {
    use prowly::services::reveal_in_file_manager;
    use std::path::Path;

    // Relative and absolute files should resolve gracefully without panic or erroring out
    let test_path = Path::new("Cargo.toml");
    let res = reveal_in_file_manager(test_path);
    assert!(res.is_ok());

    let non_existent = Path::new("non_existent_folder/file.rs");
    let res = reveal_in_file_manager(non_existent);
    assert!(res.is_ok());
}
