//! UI presentation components, theme engine, and icons.

pub mod capsule;
pub mod icons;
pub mod layout;
pub mod markdown;
pub mod modals;
pub mod theme;
pub mod views;
pub mod virtual_scroll;

pub use capsule::{diff_stat_bar, key_badge, pill, split_pill};
pub use icons::{IconId, Icons};
pub use layout::{contains_point, LayoutAreas};
pub use modals::AiAssistantModal;
pub use theme::{Theme, ThemeId};
pub use views::{ChangesFocus, ChangesView, DiffView};
pub use virtual_scroll::{HighlightCache, VirtualScroll};

use unicode_width::UnicodeWidthStr;

/// Safely truncate a string to fit within a maximum terminal display width, appending "..." if truncated.
/// Fully UTF-8 safe and respects full-width East Asian / Japanese characters.
pub fn truncate_display_width(s: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(s) <= max_width {
        return s.to_string();
    }
    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }
    let target_width = max_width - 3;
    let mut current_width = 0;
    let mut result = String::new();
    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > target_width {
            break;
        }
        current_width += char_width;
        result.push(c);
    }
    result.push_str("...");
    result
}

/// Safely slice up to `max_chars` Unicode characters from a string without byte boundary panics.
pub fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}
