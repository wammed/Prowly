//! Prowly: Terminal Git & GitHub client with Omnipresent AI.

#![allow(
    clippy::manual_range_contains,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::useless_format,
    clippy::manual_pattern_char_comparison,
    clippy::while_let_on_iterator,
    clippy::unnecessary_sort_by,
    clippy::if_same_then_else,
    clippy::manual_map
)]

pub mod ai;
pub mod app;
pub mod config;
pub mod event;
pub mod git;
pub mod github;
pub mod i18n;
pub mod services;
pub mod ui;

pub use app::{App, AppTab, FocusedPane};
pub use config::Config;
pub use event::{AppEvent, EventHandler};
pub use git::GitService;
pub use github::{
    AlertItem, GitHubRateLimit, GitHubRepoTarget, GitHubService, PullRequestItem, RepoMetadata,
    WorkflowRunItem,
};
pub use i18n::{I18n, Language};
pub use services::{BackupService, ClipboardService};
pub use ui::{Icons, Theme, ThemeId};
