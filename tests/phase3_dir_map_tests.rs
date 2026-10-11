use prowly::git::{FileStatus, GitFileItem};
use prowly::ui::views::dir_map::DirectoryMap;

fn make_file(path: &str, staged: bool) -> GitFileItem {
    GitFileItem {
        path: path.to_string(),
        status: FileStatus::Modified,
        staged,
    }
}

#[test]
fn test_dir_map_empty_files() {
    let mut dir_map = DirectoryMap::new();
    dir_map.update_from_files(&[]);

    assert_eq!(dir_map.nodes.len(), 1);
    assert_eq!(dir_map.nodes[0].display_name, "/ (root) [0]");
    assert_eq!(dir_map.nodes[0].change_count, 0);
    assert!(dir_map.nodes[0].is_root);
}

#[test]
fn test_dir_map_aggregation_and_counts() {
    let mut dir_map = DirectoryMap::new();
    let files = vec![
        make_file("src/main.rs", false),
        make_file("src/ui/theme.rs", true),
        make_file("src/ui/layout.rs", false),
        make_file("README.md", false),
    ];

    dir_map.update_from_files(&files);

    // Root should have 4 changes
    assert_eq!(dir_map.nodes[0].change_count, 4);
    assert!(dir_map.nodes[0].display_name.contains("[4]"));

    // Find src/ node
    let src_node = dir_map
        .nodes
        .iter()
        .find(|n| n.path_prefix == "src/")
        .unwrap();
    assert_eq!(src_node.change_count, 3);

    // Find src/ui/ node
    let ui_node = dir_map
        .nodes
        .iter()
        .find(|n| n.path_prefix == "src/ui/")
        .unwrap();
    assert_eq!(ui_node.change_count, 2);
}

#[test]
fn test_dir_map_apply_selection_and_filter() {
    let mut dir_map = DirectoryMap::new();
    let files = vec![
        make_file("src/main.rs", false),
        make_file("src/ui/theme.rs", true),
        make_file("docs/index.md", false),
    ];

    dir_map.update_from_files(&files);

    // Root node selected initially (index 0)
    let selected = dir_map.apply_selection();
    assert_eq!(selected, None);
    assert_eq!(dir_map.current_selected_prefix(), None);

    // Find index of src/
    let src_idx = dir_map
        .nodes
        .iter()
        .position(|n| n.path_prefix == "src/")
        .unwrap();
    dir_map.scroll.selected_index = src_idx;
    let selected = dir_map.apply_selection();
    assert_eq!(selected, Some("src/".to_string()));
    assert_eq!(dir_map.current_selected_prefix(), Some("src/"));
}
