//! Core background and platform integration services.

pub mod backup;
pub mod clipboard;
pub mod git_runner;

pub use backup::{BackupRecord, BackupService};
pub use clipboard::ClipboardService;
pub use git_runner::SafeGitRunner;
