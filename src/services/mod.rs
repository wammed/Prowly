//! Core background and platform integration services.

pub mod backup;
pub mod clipboard;
pub mod git_runner;
pub mod terminal;

pub use backup::{BackupRecord, BackupService};
pub use clipboard::ClipboardService;
pub use git_runner::SafeGitRunner;
pub use terminal::{
    open_external_editor, open_external_editor_with_preference, reveal_in_file_manager,
    run_interactive_command,
};
