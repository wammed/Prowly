//! Virtual scrolling engine with viewport calculation, mouse wheel support, and LRU caching.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::ops::Range;

/// Reusable state for virtual scrolling across massive Diff views, CI logs, and file lists.
#[derive(Debug, Clone)]
pub struct VirtualScroll {
    pub total_items: usize,
    pub selected_index: usize,
    pub viewport_offset: usize,
    pub viewport_height: usize,
}

impl Default for VirtualScroll {
    fn default() -> Self {
        Self::new(0, 20)
    }
}

impl VirtualScroll {
    /// Create new virtual scroll state.
    pub fn new(total_items: usize, viewport_height: usize) -> Self {
        Self {
            total_items,
            selected_index: 0,
            viewport_offset: 0,
            viewport_height: viewport_height.max(1),
        }
    }

    /// Update total items and adjust offset/selection if needed.
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        if self.total_items == 0 {
            self.selected_index = 0;
            self.viewport_offset = 0;
        } else if self.selected_index >= self.total_items {
            self.selected_index = self.total_items - 1;
            self.ensure_visible();
        }
    }

    /// Update active viewport height (e.g. on window resize).
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height.max(1);
        self.ensure_visible();
    }

    /// Returns range of visible item indices: `[start..end)`.
    pub fn visible_range(&self) -> Range<usize> {
        let start = self.viewport_offset.min(self.total_items);
        let end = (start + self.viewport_height).min(self.total_items);
        start..end
    }

    /// Moves selection down by 1.
    pub fn next(&mut self) {
        if self.total_items == 0 {
            return;
        }
        if self.selected_index + 1 < self.total_items {
            self.selected_index += 1;
            self.ensure_visible();
        }
    }

    /// Moves selection up by 1.
    pub fn previous(&mut self) {
        if self.total_items == 0 {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
        }
    }

    /// Scroll down by a given number of rows (e.g., mouse wheel down).
    pub fn scroll_down(&mut self, rows: usize) {
        if self.total_items == 0 {
            return;
        }
        let max_offset = self.total_items.saturating_sub(self.viewport_height);
        self.viewport_offset = (self.viewport_offset + rows).min(max_offset);
        if self.selected_index < self.viewport_offset {
            self.selected_index = self.viewport_offset;
        }
    }

    /// Scroll up by a given number of rows (e.g., mouse wheel up).
    pub fn scroll_up(&mut self, rows: usize) {
        self.viewport_offset = self.viewport_offset.saturating_sub(rows);
        let max_selected = self.viewport_offset + self.viewport_height.saturating_sub(1);
        if self.selected_index > max_selected && self.total_items > 0 {
            self.selected_index = max_selected.min(self.total_items - 1);
        }
    }

    /// Page down (viewport height - 2).
    pub fn page_down(&mut self) {
        let step = self.viewport_height.saturating_sub(2).max(1);
        for _ in 0..step {
            self.next();
        }
    }

    /// Page up (viewport height - 2).
    pub fn page_up(&mut self) {
        let step = self.viewport_height.saturating_sub(2).max(1);
        for _ in 0..step {
            self.previous();
        }
    }

    /// Jump directly to top.
    pub fn scroll_to_top(&mut self) {
        self.selected_index = 0;
        self.viewport_offset = 0;
    }

    /// Jump directly to bottom.
    pub fn scroll_to_bottom(&mut self) {
        if self.total_items > 0 {
            self.selected_index = self.total_items - 1;
            self.ensure_visible();
        }
    }

    /// Select an item index explicitly.
    pub fn select(&mut self, index: usize) {
        if self.total_items == 0 {
            self.selected_index = 0;
            self.viewport_offset = 0;
            return;
        }
        self.selected_index = index.min(self.total_items - 1);
        self.ensure_visible();
    }

    /// Map a visual click row (relative to widget top, 0-indexed) to an absolute item index.
    pub fn handle_click_row(&mut self, visual_row: usize) -> Option<usize> {
        let absolute_idx = self.viewport_offset + visual_row;
        if absolute_idx < self.total_items {
            self.select(absolute_idx);
            Some(absolute_idx)
        } else {
            None
        }
    }

    /// Keep selection visible in viewport.
    pub fn ensure_visible(&mut self) {
        if self.total_items == 0 {
            self.viewport_offset = 0;
            return;
        }
        if self.selected_index < self.viewport_offset {
            self.viewport_offset = self.selected_index;
        } else if self.selected_index >= self.viewport_offset + self.viewport_height {
            self.viewport_offset = self.selected_index + 1 - self.viewport_height;
        }
    }
}

/// Generic LRU Cache for rendered syntax highlight lines or heavy formatted text.
pub struct HighlightCache<K, V> {
    cache: LruCache<K, V>,
}

impl<K: std::hash::Hash + Eq, V> HighlightCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(100).unwrap());
        Self {
            cache: LruCache::new(cap),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.cache.get(key)
    }

    pub fn put(&mut self, key: K, val: V) {
        self.cache.put(key, val);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
