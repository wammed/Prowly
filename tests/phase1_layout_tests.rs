use prowly::ui::layout::{contains_point, LayoutAreas};
use ratatui::layout::Rect;

#[test]
fn test_layout_computation_standard_size() {
    let area = Rect::new(0, 0, 120, 35);
    let layout = LayoutAreas::compute(area);

    assert_eq!(layout.root, area);
    assert_eq!(layout.header.height, 3);
    assert_eq!(layout.footer.height, 1);
    assert_eq!(layout.body.height, 31);

    // Left Directory Map width should be between 22 and 34
    assert!(layout.left_pane.width >= 22 && layout.left_pane.width <= 34);
    // Right pane should take remaining width
    assert_eq!(layout.left_pane.width + layout.right_pane.width, 120);

    // Header buttons should be on top row of header
    assert_eq!(layout.ask_ai_button.y, layout.header_top.y);
    assert_eq!(layout.help_button.y, layout.header_top.y);
    assert!(layout.ask_ai_button.x < layout.help_button.x);
}

#[test]
fn test_layout_computation_small_terminal() {
    let area = Rect::new(0, 0, 80, 24);
    let layout = LayoutAreas::compute(area);

    assert_eq!(layout.header.height, 3);
    assert_eq!(layout.footer.height, 1);
    assert_eq!(layout.body.height, 20);
    assert!(layout.left_pane.width >= 22);
}

#[test]
fn test_mouse_hit_testing() {
    let area = Rect::new(0, 0, 100, 30);
    let layout = LayoutAreas::compute(area);

    // Test Ask AI button hit
    let ai_x = layout.ask_ai_button.x + 1;
    let ai_y = layout.ask_ai_button.y;
    assert!(layout.hits_ask_ai(ai_x, ai_y));
    assert!(!layout.hits_help(ai_x, ai_y));

    // Test Help button hit
    let help_x = layout.help_button.x + 1;
    let help_y = layout.help_button.y;
    assert!(layout.hits_help(help_x, help_y));
    assert!(!layout.hits_ask_ai(help_x, help_y));

    // Test Left Directory Pane hit
    let left_x = layout.left_pane.x + 5;
    let left_y = layout.left_pane.y + 5;
    assert!(layout.hits_left_pane(left_x, left_y));
    assert!(!layout.hits_right_content(left_x, left_y));

    // Test Right Content hit
    let right_x = layout.right_content.x + 5;
    let right_y = layout.right_content.y + 5;
    assert!(layout.hits_right_content(right_x, right_y));
    assert!(!layout.hits_left_pane(right_x, right_y));

    // Test contains_point utility
    let rect = Rect::new(10, 10, 20, 5);
    assert!(contains_point(rect, 10, 10));
    assert!(contains_point(rect, 29, 14));
    assert!(!contains_point(rect, 30, 10));
    assert!(!contains_point(rect, 9, 10));
}
