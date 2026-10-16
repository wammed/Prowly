//! GitHub API integration types, service client, and rate-limit guard.

pub mod client;

pub use client::{
    AlertItem, GitHubRateLimit, GitHubRepoTarget, GitHubService, IssueItem, PullRequestItem,
    RepoMetadata, WorkflowRunItem,
};
