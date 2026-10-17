use prowly::ui::markdown::render_markdown;
use prowly::ui::modals::markdown_preview::MarkdownPreviewModal;
use prowly::ui::{Theme, ThemeId};

#[test]
fn test_markdown_preview_rendering() {
    let theme = Theme::get(ThemeId::TokyoNight);
    let sample = r#"# Sample Document
This is **bold** text and *italic* text with `inline_code`.

## Features
- [ ] Task 1
- [x] Task 2 completed
- Bullet 1
1. Numbered 1

> Important note quote

```rust
fn hello() {
    println!("Hello, World!");
}
```

---
[Visit GitHub](https://github.com)
"#;

    let lines = render_markdown(sample, &theme);
    assert!(!lines.is_empty());

    // Headings, checkboxes, blockquotes, code fences should be present
    assert!(lines.len() >= 12);
}

#[test]
fn test_markdown_preview_modal_lifecycle_and_scrolling() {
    let theme = Theme::get(ThemeId::TokyoNight);
    let mut modal = MarkdownPreviewModal::new();
    assert!(!modal.is_open);

    let content = (0..50)
        .map(|i| format!("Line {} content for testing", i))
        .collect::<Vec<_>>()
        .join("\n");

    modal.open("README.md".to_string(), content, &theme);
    assert!(modal.is_open);
    assert_eq!(modal.file_path, "README.md");
    assert_eq!(modal.scroll_offset, 0);
    assert_eq!(modal.rendered_lines.len(), 50);

    // Viewport height simulation
    modal.viewport_height = 20;

    // Scroll down
    modal.scroll_down(5);
    assert_eq!(modal.scroll_offset, 5);

    // Scroll up
    modal.scroll_up(2);
    assert_eq!(modal.scroll_offset, 3);

    // Scroll to bottom (50 - 20 = 30)
    modal.scroll_to_bottom();
    assert_eq!(modal.scroll_offset, 30);

    // Scroll to top
    modal.scroll_to_top();
    assert_eq!(modal.scroll_offset, 0);

    // Close
    modal.close();
    assert!(!modal.is_open);
}

#[test]
fn test_markdown_preview_modal_click_interaction() {
    use prowly::ui::modals::MarkdownPreviewClickResult;
    use ratatui::layout::Rect;

    let theme = Theme::get(ThemeId::TokyoNight);
    let mut modal = MarkdownPreviewModal::new();
    let viewport = Rect::new(0, 0, 100, 30);

    assert_eq!(modal.handle_click(10, 10, viewport), None);

    modal.open("test.md".to_string(), "# Hello".to_string(), &theme);
    assert!(modal.is_open);

    let rect = modal.modal_rect(viewport);

    // Click outside -> Close
    assert_eq!(
        modal.handle_click(0, 0, viewport),
        Some(MarkdownPreviewClickResult::Close)
    );

    // Click inside body -> Handled
    assert_eq!(
        modal.handle_click(rect.x + 5, rect.y + 5, viewport),
        Some(MarkdownPreviewClickResult::Handled)
    );

    // Click footer close area
    let inner_y = rect.y + 1;
    let inner_h = rect.height.saturating_sub(2);
    let footer_y = inner_y + inner_h.saturating_sub(1);
    let inner_x = rect.x + 1;
    assert_eq!(
        modal.handle_click(inner_x + 65, footer_y, viewport),
        Some(MarkdownPreviewClickResult::Close)
    );
}
