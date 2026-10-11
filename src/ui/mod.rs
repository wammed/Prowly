//! UI presentation components, theme engine, and icons.

pub mod icons;
pub mod layout;
pub mod modals;
pub mod theme;
pub mod views;
pub mod virtual_scroll;

pub use icons::{IconId, Icons};
pub use layout::{contains_point, LayoutAreas};
pub use modals::AiAssistantModal;
pub use theme::{Theme, ThemeId};
pub use views::{ChangesFocus, ChangesView, DiffView};
pub use virtual_scroll::{HighlightCache, VirtualScroll};
