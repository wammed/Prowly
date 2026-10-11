//! Modal dialogs and popups.

pub mod ai_assistant;
pub mod branch_switcher;
pub mod command_palette;
pub mod conflict_resolver;
pub mod context_menu;
pub mod rebase_modal;
pub mod remote_diff_preview;

pub use ai_assistant::AiAssistantModal;
pub use branch_switcher::BranchSwitcherModal;
pub use command_palette::{CommandPaletteModal, PaletteAction};
pub use conflict_resolver::ConflictResolverModal;
pub use context_menu::{ContextMenu, ContextMenuAction};
pub use rebase_modal::RebaseModal;
pub use remote_diff_preview::RemoteDiffPreviewModal;
