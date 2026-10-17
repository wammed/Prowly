//! Modal dialogs and popups.

pub mod ai_assistant;
pub mod branch_switcher;
pub mod command_palette;
pub mod conflict_resolver;
pub mod context_menu;
pub mod git_subcommands;
pub mod history_actions;
pub mod markdown_preview;
pub mod open_repo;
pub mod rebase_modal;
pub mod remote_diff_preview;
pub mod settings;

pub use ai_assistant::{AiAssistantClickResult, AiAssistantModal};
pub use branch_switcher::{BranchSwitcherClickResult, BranchSwitcherModal};
pub use command_palette::{
    CommandPaletteModal, PaletteAction, PaletteClickResult, PaletteEnterResult, PaletteLayer,
};
pub use conflict_resolver::{ConflictResolverClickResult, ConflictResolverModal};
pub use context_menu::{ContextMenu, ContextMenuAction};
pub use git_subcommands::GitSubcommandDef;
pub use history_actions::{
    HistoryActionState, HistoryActionsModal, HistoryMenuAction, HistoryModalClickResult,
};
pub use markdown_preview::{MarkdownPreviewClickResult, MarkdownPreviewModal};
pub use open_repo::{OpenRepoAction, OpenRepoModal};
pub use rebase_modal::{RebaseClickResult, RebaseModal};
pub use remote_diff_preview::{RemoteDiffClickResult, RemoteDiffPreviewModal};
pub use settings::{SettingsAction, SettingsModal};
