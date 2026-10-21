use prowly::ui::{truncate_chars, truncate_display_width};

#[test]
fn test_truncate_chars_multibyte() {
    let text = "山田太郎";
    // 2 characters -> "山田"
    let truncated = truncate_chars(text, 2);
    assert_eq!(truncated, "山田");

    // 10 characters -> "山田太郎" (full string without panic)
    let full = truncate_chars(text, 10);
    assert_eq!(full, "山田太郎");
}

#[test]
fn test_truncate_display_width_japanese() {
    // Each full-width kanji has width 2
    let text = "コミットメッセージのテストです";
    // Target display width 15. "コミットメ" is 10 width + "..." (3 width) = 13 width <= 15
    let truncated = truncate_display_width(text, 15);
    assert!(truncated.ends_with("..."));
    let width = unicode_width::UnicodeWidthStr::width(truncated.as_str());
    assert!(width <= 15);
}
