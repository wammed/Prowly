//! Prowly: Terminal Git & GitHub client with Omnipresent AI.

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
    AlertItem, GitHubRateLimit, GitHubRepoTarget, GitHubService, PullRequestItem, WorkflowRunItem,
};
pub use i18n::{I18n, Language};
pub use services::{BackupService, ClipboardService};
pub use ui::{Icons, Theme, ThemeId};
