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
