//! Prowly Core: Headless Git, GitHub, AI, services, i18n, and config engine.

pub mod ai;
pub mod config;
pub mod git;
pub mod github;
pub mod i18n;
pub mod services;

pub use config::Config;
pub use git::GitService;
pub use github::{
    AlertItem, GitHubError, GitHubRateLimit, GitHubRepoTarget, GitHubService, PullRequestItem,
    RepoMetadata, WorkflowRunItem,
};
pub use i18n::{I18n, Language};
pub use services::{BackupService, ClipboardService};
