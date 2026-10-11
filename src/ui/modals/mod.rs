//! Modal dialogs and popups.

pub mod ai_assistant;
pub mod command_palette;
pub mod conflict_resolver;
pub mod context_menu;

pub use ai_assistant::AiAssistantModal;
pub use command_palette::{CommandPaletteModal, PaletteAction};
pub use conflict_resolver::ConflictResolverModal;
pub use context_menu::{ContextMenu, ContextMenuAction};
