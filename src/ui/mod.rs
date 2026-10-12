//! UI presentation components, theme engine, and icons.

pub mod capsule;
pub mod icons;
pub mod layout;
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

