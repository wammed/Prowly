use prowly::ui::{HighlightCache, VirtualScroll};

#[test]
fn test_virtual_scroll_navigation() {
    let mut vs = VirtualScroll::new(100, 10);
    assert_eq!(vs.selected_index, 0);
    assert_eq!(vs.viewport_offset, 0);
    assert_eq!(vs.visible_range(), 0..10);

    // Step down
    vs.next();
    assert_eq!(vs.selected_index, 1);
    assert_eq!(vs.viewport_offset, 0);

    // Scroll past viewport height
    for _ in 0..15 {
        vs.next();
    }
    assert_eq!(vs.selected_index, 16);
    // Viewport must have shifted so selected is visible
    assert!(vs.viewport_offset <= 16);
    assert!(vs.viewport_offset + vs.viewport_height > 16);

    // Step up
    vs.previous();
    assert_eq!(vs.selected_index, 15);

    // Jump to bottom
    vs.scroll_to_bottom();
    assert_eq!(vs.selected_index, 99);
    assert_eq!(vs.viewport_offset, 90);

    // Jump to top
    vs.scroll_to_top();
    assert_eq!(vs.selected_index, 0);
    assert_eq!(vs.viewport_offset, 0);
}

#[test]
fn test_mouse_wheel_and_click_row() {
    let mut vs = VirtualScroll::new(50, 10);

    // Mouse wheel scroll down 5 rows
    vs.scroll_down(5);
    assert_eq!(vs.viewport_offset, 5);
    assert_eq!(vs.selected_index, 5);

    // Mouse wheel scroll up 2 rows
    vs.scroll_up(2);
    assert_eq!(vs.viewport_offset, 3);

    // Clicking visual row 4 (0-indexed inside widget)
    let clicked = vs.handle_click_row(4);
    assert_eq!(clicked, Some(3 + 4)); // offset 3 + row 4 = 7
    assert_eq!(vs.selected_index, 7);

    // Clicking out of bounds row
    let out_of_bounds = vs.handle_click_row(100);
    assert_eq!(out_of_bounds, None);
}

#[test]
fn test_empty_collection_safety() {
    let mut vs = VirtualScroll::new(0, 10);
    assert_eq!(vs.visible_range(), 0..0);

    vs.next();
    assert_eq!(vs.selected_index, 0);
    vs.previous();
    assert_eq!(vs.selected_index, 0);
    vs.page_down();
    vs.page_up();
    vs.scroll_to_bottom();
    vs.scroll_to_top();
    assert_eq!(vs.selected_index, 0);
    assert_eq!(vs.viewport_offset, 0);
}

#[test]
fn test_lru_highlight_cache() {
    let mut cache = HighlightCache::new(3);
    cache.put(1, "line 1".to_string());
    cache.put(2, "line 2".to_string());
    cache.put(3, "line 3".to_string());

    assert_eq!(cache.get(&1).map(|s| s.as_str()), Some("line 1"));
    assert_eq!(cache.get(&2).map(|s| s.as_str()), Some("line 2"));

    // Add 4th item, evicting least recently used (item 3)
    cache.put(4, "line 4".to_string());
    assert_eq!(cache.get(&3), None);
    assert_eq!(cache.get(&4).map(|s| s.as_str()), Some("line 4"));
}
