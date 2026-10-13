//! Modal dialogs and popups.

pub mod ai_assistant;
pub mod branch_switcher;
pub mod command_palette;
pub mod conflict_resolver;
pub mod context_menu;
pub mod git_subcommands;
pub mod history_actions;
pub mod open_repo;
pub mod rebase_modal;
pub mod remote_diff_preview;
pub mod settings;

pub use ai_assistant::AiAssistantModal;
pub use branch_switcher::BranchSwitcherModal;
pub use command_palette::{CommandPaletteModal, PaletteAction, PaletteEnterResult, PaletteLayer};
pub use conflict_resolver::ConflictResolverModal;
pub use context_menu::{ContextMenu, ContextMenuAction};
pub use git_subcommands::GitSubcommandDef;
pub use history_actions::{HistoryActionState, HistoryActionsModal, HistoryMenuAction};
pub use open_repo::{OpenRepoAction, OpenRepoModal};
pub use rebase_modal::RebaseModal;
pub use remote_diff_preview::RemoteDiffPreviewModal;
pub use settings::{SettingsAction, SettingsModal};

