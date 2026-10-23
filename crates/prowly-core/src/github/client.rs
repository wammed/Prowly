//! GitHub API client service with rate-limit protection and multi-source auth detection.

use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

/// Error types occurring during GitHub API and CLI operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GitHubError {
    #[error("GitHub token not configured")]
    NoToken,

    #[error("GitHub rate limit exceeded")]
    RateLimited,

    #[error("GitHub resource not found: {0}")]
    NotFound(String),

    #[error("GitHub API error: {0}")]
    Api(String),

    #[error("Git CLI error: {0}")]
    Cli(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("{0}")]
    Other(String),
}

impl From<octocrab::Error> for GitHubError {
    fn from(err: octocrab::Error) -> Self {
        match &err {
            octocrab::Error::GitHub { source, .. } => {
                if source.status_code.as_u16() == 404 {
                    GitHubError::NotFound(source.message.clone())
                } else if source.status_code.as_u16() == 403
                    && source.message.to_lowercase().contains("rate limit")
                {
                    GitHubError::RateLimited
                } else {
                    GitHubError::Api(source.message.clone())
                }
            }
            octocrab::Error::Http { source, .. } => GitHubError::Network(source.to_string()),
            octocrab::Error::Json { source, .. } => GitHubError::Serialization(source.to_string()),
            other => GitHubError::Other(other.to_string()),
        }
    }
}

impl From<GitHubError> for String {
    fn from(err: GitHubError) -> Self {
        err.to_string()
    }
}

/// Target GitHub repository (owner/name).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GitHubRepoTarget {
    pub owner: String,
    pub repo: String,
}

impl GitHubRepoTarget {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
        }
    }

    /// Parse owner and repo name from Git remote URL.
    /// Handles SSH (`git@github.com:owner/repo.git`, `ssh://git@github.com/owner/repo.git`) and HTTPS (`https://github.com/owner/repo.git`).
    pub fn from_remote_url(url: &str) -> Option<Self> {
        let trimmed = url.trim().trim_end_matches(".git");
        if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
            let mut parts = rest.split('/');
            let owner = parts.next()?;
            let repo = parts.next()?;
            if !owner.is_empty() && !repo.is_empty() {
                return Some(Self::new(owner, repo));
            }
        } else if let Some(rest) = trimmed
            .strip_prefix("https://github.com/")
            .or_else(|| trimmed.strip_prefix("http://github.com/"))
            .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
        {
            let mut parts = rest.split('/');
            let owner = parts.next()?;
            let repo = parts.next()?;
            if !owner.is_empty() && !repo.is_empty() {
                return Some(Self::new(owner, repo));
            }
        }
        None
    }
}

/// Pull request representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestItem {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub state: String, // "open", "closed", "merged"
    pub is_draft: bool,
    pub head_branch: String,
    pub base_branch: String,
    pub body: Option<String>,
    pub comments_count: u32,
    pub html_url: String,
    pub updated_at: String,
}

/// GitHub Issue item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueItem {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub state: String, // "open", "closed"
    pub body: Option<String>,
    pub comments_count: u32,
    pub labels: Vec<String>,
    pub html_url: String,
    pub updated_at: String,
}

/// GitHub Actions workflow run item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunItem {
    pub id: u64,
    pub name: String,
    pub head_branch: String,
    pub event: String,
    pub status: String,             // "completed", "in_progress", "queued"
    pub conclusion: Option<String>, // "success", "failure", "cancelled"
    pub html_url: String,
    pub created_at: String,
    pub logs: Option<String>,
}

/// Notification / Alert item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertItem {
    pub id: String,
    pub title: String,
    pub repository: String,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
    pub url: String,
}

/// Repository metadata representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMetadata {
    pub is_private: bool,
    pub description: Option<String>,
}

/// Rate limit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitHubRateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset_timestamp: i64,
}

impl Default for GitHubRateLimit {
    fn default() -> Self {
        Self {
            limit: 5000,
            remaining: 5000,
            reset_timestamp: 0,
        }
    }
}

/// Per-repository cached data container.
#[derive(Default, Clone)]
struct CachedRepoData {
    repo_meta: Option<(Instant, RepoMetadata)>,
    prs: Option<(Instant, Vec<PullRequestItem>)>,
    issues: Option<(Instant, Vec<IssueItem>)>,
    runs: Option<(Instant, Vec<WorkflowRunItem>)>,
}

/// Internal global cached data container.
#[derive(Default)]
struct CachedData {
    repos: std::collections::HashMap<(String, String), CachedRepoData>,
    mock_defaults: CachedRepoData,
    alerts: Option<(Instant, Vec<AlertItem>)>,
    rate_limit: GitHubRateLimit,
}

/// High-level GitHub service with caching and rate-limit guard.
#[derive(Clone)]
pub struct GitHubService {
    token: Option<String>,
    octo: Arc<Mutex<Option<octocrab::Octocrab>>>,
    cache: Arc<Mutex<CachedData>>,
    cache_ttl: Duration,
    mock_mode: bool,
}

impl GitHubService {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            octo: Arc::new(Mutex::new(None)),
            cache: Arc::new(Mutex::new(CachedData::default())),
            cache_ttl: Duration::from_secs(60),
            mock_mode: false,
        }
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    async fn get_octocrab(&self) -> Option<octocrab::Octocrab> {
        let mut guard = self.octo.lock().await;
        if let Some(ref o) = *guard {
            return Some(o.clone());
        }
        if let Some(ref t) = self.token {
            if let Ok(o) = octocrab::OctocrabBuilder::new()
                .personal_token(t.clone())
                .build()
            {
                *guard = Some(o.clone());
                return Some(o);
            }
        }
        None
    }

    /// Automatically discover authentication token from `gh auth token` CLI,
    /// environment variable `GITHUB_TOKEN` / `GH_TOKEN`, or return None.
    pub fn detect_token() -> Option<String> {
        if let Ok(tok) = std::env::var("GITHUB_TOKEN") {
            if !tok.trim().is_empty() {
                return Some(tok.trim().to_string());
            }
        }
        if let Ok(tok) = std::env::var("GH_TOKEN") {
            if !tok.trim().is_empty() {
                return Some(tok.trim().to_string());
            }
        }

        // Try `gh auth token`
        if let Ok(output) = Command::new("gh").args(["auth", "token"]).output() {
            if output.status.success() {
                let tok = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !tok.is_empty() {
                    return Some(tok);
                }
            }
        }

        None
    }

    /// Enable mock mode for testing and offline environments.
    pub fn enable_mock_mode(&mut self) {
        self.mock_mode = true;
    }

    pub fn is_mock_mode(&self) -> bool {
        self.mock_mode
    }

    /// Set mock dataset.
    pub async fn set_mock_data(
        &self,
        prs: Vec<PullRequestItem>,
        runs: Vec<WorkflowRunItem>,
        alerts: Vec<AlertItem>,
    ) {
        let mut guard = self.cache.lock().await;
        let now = Instant::now();
        guard.mock_defaults.prs = Some((now, prs.clone()));
        guard.mock_defaults.runs = Some((now, runs.clone()));
        guard.alerts = Some((now, alerts));

        for repo_entry in guard.repos.values_mut() {
            repo_entry.prs = Some((now, prs.clone()));
            repo_entry.runs = Some((now, runs.clone()));
        }
    }

    /// Set mock issues dataset.
    pub async fn set_mock_issues(&self, issues: Vec<IssueItem>) {
        let mut guard = self.cache.lock().await;
        let now = Instant::now();
        guard.mock_defaults.issues = Some((now, issues.clone()));
        for repo_entry in guard.repos.values_mut() {
            repo_entry.issues = Some((now, issues.clone()));
        }
    }

    /// Set mock repository metadata.
    pub async fn set_mock_repo_metadata(&self, is_private: bool) {
        let mut guard = self.cache.lock().await;
        let now = Instant::now();
        let meta = RepoMetadata {
            is_private,
            description: None,
        };
        guard.mock_defaults.repo_meta = Some((now, meta.clone()));
        for repo_entry in guard.repos.values_mut() {
            repo_entry.repo_meta = Some((now, meta.clone()));
        }
    }

    /// Retrieve active rate limit status.
    pub async fn get_rate_limit(&self) -> GitHubRateLimit {
        let guard = self.cache.lock().await;
        guard.rate_limit
    }

    /// Retrieve repository metadata (visibility/privacy, etc.).
    pub async fn get_repo_metadata(
        &self,
        target: &GitHubRepoTarget,
        force_refresh: bool,
    ) -> Result<RepoMetadata, GitHubError> {
        let repo_key = (target.owner.clone(), target.repo.clone());
        {
            let guard = self.cache.lock().await;
            if !force_refresh {
                let cached = guard
                    .repos
                    .get(&repo_key)
                    .and_then(|r| r.repo_meta.as_ref())
                    .or_else(|| guard.mock_defaults.repo_meta.as_ref());
                if let Some((ts, ref meta)) = cached {
                    if ts.elapsed() < self.cache_ttl {
                        return Ok(meta.clone());
                    }
                }
            }

            if self.mock_mode {
                return Ok(guard
                    .repos
                    .get(&repo_key)
                    .and_then(|r| r.repo_meta.as_ref())
                    .or_else(|| guard.mock_defaults.repo_meta.as_ref())
                    .map(|(_, m)| m.clone())
                    .unwrap_or(RepoMetadata {
                        is_private: false,
                        description: None,
                    }));
            }
        }

        // 1. Try `gh repo view <owner>/<repo> --json isPrivate` CLI asynchronously without holding Mutex
        let owner_repo = format!("{}/{}", target.owner, target.repo);
        let gh_output = tokio::task::spawn_blocking(move || {
            Command::new("gh")
                .args(["repo", "view", &owner_repo, "--json", "isPrivate"])
                .output()
        })
        .await
        .ok()
        .and_then(|res| res.ok());

        if let Some(output) = gh_output {
            if output.status.success() {
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    if let Some(is_private) = val.get("isPrivate").and_then(|v| v.as_bool()) {
                        let meta = RepoMetadata {
                            is_private,
                            description: None,
                        };
                        let mut guard = self.cache.lock().await;
                        guard.repos.entry(repo_key).or_default().repo_meta =
                            Some((Instant::now(), meta.clone()));
                        return Ok(meta);
                    }
                }
            }
        }

        // 2. Fallback to Octocrab API
        let octo = match self.get_octocrab().await {
            Some(o) => o,
            None => match octocrab::OctocrabBuilder::new().build() {
                Ok(o) => o,
                Err(_) => (*octocrab::instance()).clone(),
            },
        };

        match octo.repos(&target.owner, &target.repo).get().await {
            Ok(repo) => {
                let is_private = repo.private.unwrap_or(false);
                let meta = RepoMetadata {
                    is_private,
                    description: repo.description,
                };
                let mut guard = self.cache.lock().await;
                guard.repos.entry(repo_key).or_default().repo_meta =
                    Some((Instant::now(), meta.clone()));
                if guard.rate_limit.remaining > 0 {
                    guard.rate_limit.remaining -= 1;
                }
                Ok(meta)
            }
            Err(e) => Err(GitHubError::from(e)),
        }
    }

    /// List pull requests with caching.
    pub async fn list_pull_requests(
        &self,
        target: &GitHubRepoTarget,
        force_refresh: bool,
    ) -> Result<Vec<PullRequestItem>, GitHubError> {
        let repo_key = (target.owner.clone(), target.repo.clone());
        let mut guard = self.cache.lock().await;
        if !force_refresh {
            let cached = guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.prs.as_ref())
                .or_else(|| guard.mock_defaults.prs.as_ref());
            if let Some((ts, ref prs)) = cached {
                if ts.elapsed() < self.cache_ttl {
                    return Ok(prs.clone());
                }
            }
        }

        if self.mock_mode {
            return Ok(guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.prs.as_ref())
                .or_else(|| guard.mock_defaults.prs.as_ref())
                .map(|(_, p)| p.clone())
                .unwrap_or_default());
        }

        let octo = match self.get_octocrab().await {
            Some(o) => o,
            None => {
                return Ok(guard
                    .repos
                    .get(&repo_key)
                    .and_then(|r| r.prs.as_ref())
                    .or_else(|| guard.mock_defaults.prs.as_ref())
                    .map(|(_, p)| p.clone())
                    .unwrap_or_default());
            }
        };

        // Query PRs via octocrab
        match octo
            .pulls(&target.owner, &target.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(30)
            .send()
            .await
        {
            Ok(page) => {
                let items: Vec<PullRequestItem> = page
                    .into_iter()
                    .map(|pr| {
                        let state_str = if pr.merged_at.is_some() {
                            "merged".to_string()
                        } else {
                            match pr.state {
                                Some(octocrab::models::IssueState::Open) => "open".to_string(),
                                Some(octocrab::models::IssueState::Closed) => "closed".to_string(),
                                _ => "open".to_string(),
                            }
                        };
                        PullRequestItem {
                            number: pr.number,
                            title: pr.title.unwrap_or_default(),
                            author: pr
                                .user
                                .map(|u| u.login)
                                .unwrap_or_else(|| "unknown".to_string()),
                            state: state_str,
                            is_draft: pr.draft.unwrap_or(false),
                            head_branch: pr.head.ref_field,
                            base_branch: pr.base.ref_field,
                            body: pr.body,
                            comments_count: pr.comments.unwrap_or(0) as u32,
                            html_url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
                            updated_at: pr.updated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
                        }
                    })
                    .collect();

                guard.repos.entry(repo_key).or_default().prs =
                    Some((Instant::now(), items.clone()));
                if guard.rate_limit.remaining > 0 {
                    guard.rate_limit.remaining -= 1;
                }
                Ok(items)
            }
            Err(e) => Err(GitHubError::from(e)),
        }
    }

    /// List repository issues (excluding pull requests) with caching.
    pub async fn list_issues(
        &self,
        target: &GitHubRepoTarget,
        force_refresh: bool,
    ) -> Result<Vec<IssueItem>, GitHubError> {
        let repo_key = (target.owner.clone(), target.repo.clone());
        let mut guard = self.cache.lock().await;
        if !force_refresh {
            let cached = guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.issues.as_ref())
                .or_else(|| guard.mock_defaults.issues.as_ref());
            if let Some((ts, ref issues)) = cached {
                if ts.elapsed() < self.cache_ttl {
                    return Ok(issues.clone());
                }
            }
        }

        if self.mock_mode {
            return Ok(guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.issues.as_ref())
                .or_else(|| guard.mock_defaults.issues.as_ref())
                .map(|(_, p)| p.clone())
                .unwrap_or_default());
        }

        let octo = match self.get_octocrab().await {
            Some(o) => o,
            None => {
                return Ok(guard
                    .repos
                    .get(&repo_key)
                    .and_then(|r| r.issues.as_ref())
                    .or_else(|| guard.mock_defaults.issues.as_ref())
                    .map(|(_, p)| p.clone())
                    .unwrap_or_default());
            }
        };

        // Query issues via octocrab
        match octo
            .issues(&target.owner, &target.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(50)
            .send()
            .await
        {
            Ok(page) => {
                let items: Vec<IssueItem> = page
                    .into_iter()
                    .filter(|issue| issue.pull_request.is_none())
                    .map(|issue| {
                        let state_str = match issue.state {
                            octocrab::models::IssueState::Open => "open".to_string(),
                            octocrab::models::IssueState::Closed => "closed".to_string(),
                            _ => "open".to_string(),
                        };
                        let labels = issue.labels.into_iter().map(|l| l.name).collect();
                        IssueItem {
                            number: issue.number,
                            title: issue.title,
                            author: issue.user.login,
                            state: state_str,
                            body: issue.body,
                            comments_count: issue.comments,
                            labels,
                            html_url: issue.html_url.to_string(),
                            updated_at: issue.updated_at.to_rfc3339(),
                        }
                    })
                    .collect();

                guard.repos.entry(repo_key.clone()).or_default().issues =
                    Some((Instant::now(), items.clone()));
                if guard.rate_limit.remaining > 0 {
                    guard.rate_limit.remaining -= 1;
                }
                Ok(items)
            }
            Err(e) => {
                if let Some(repo_entry) = guard.repos.get(&repo_key) {
                    if let Some((_, ref issues)) = repo_entry.issues {
                        return Ok(issues.clone());
                    }
                }
                Err(GitHubError::from(e))
            }
        }
    }

    /// List Actions workflow runs with caching.
    pub async fn list_workflow_runs(
        &self,
        target: &GitHubRepoTarget,
        force_refresh: bool,
    ) -> Result<Vec<WorkflowRunItem>, GitHubError> {
        let repo_key = (target.owner.clone(), target.repo.clone());
        let mut guard = self.cache.lock().await;
        if !force_refresh {
            let cached = guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.runs.as_ref())
                .or_else(|| guard.mock_defaults.runs.as_ref());
            if let Some((ts, ref runs)) = cached {
                if ts.elapsed() < self.cache_ttl {
                    return Ok(runs.clone());
                }
            }
        }

        if self.mock_mode {
            return Ok(guard
                .repos
                .get(&repo_key)
                .and_then(|r| r.runs.as_ref())
                .or_else(|| guard.mock_defaults.runs.as_ref())
                .map(|(_, r)| r.clone())
                .unwrap_or_default());
        }

        let octo = match self.get_octocrab().await {
            Some(o) => o,
            None => {
                return Ok(guard
                    .repos
                    .get(&repo_key)
                    .and_then(|r| r.runs.as_ref())
                    .or_else(|| guard.mock_defaults.runs.as_ref())
                    .map(|(_, r)| r.clone())
                    .unwrap_or_default());
            }
        };

        match octo
            .workflows(&target.owner, &target.repo)
            .list_all_runs()
            .per_page(30)
            .send()
            .await
        {
            Ok(page) => {
                let items: Vec<WorkflowRunItem> = page
                    .into_iter()
                    .map(|run| WorkflowRunItem {
                        id: run.id.into_inner(),
                        name: run.name,
                        head_branch: run.head_branch,
                        event: run.event,
                        status: run.status,
                        conclusion: run.conclusion,
                        html_url: run.html_url.to_string(),
                        created_at: run.created_at.to_rfc3339(),
                        logs: None,
                    })
                    .collect();

                guard.repos.entry(repo_key).or_default().runs =
                    Some((Instant::now(), items.clone()));
                if guard.rate_limit.remaining > 0 {
                    guard.rate_limit.remaining -= 1;
                }
                Ok(items)
            }
            Err(e) => Err(GitHubError::from(e)),
        }
    }

    /// List notifications / alerts.
    pub async fn list_alerts(&self, force_refresh: bool) -> Result<Vec<AlertItem>, GitHubError> {
        let mut guard = self.cache.lock().await;
        if !force_refresh {
            if let Some((ts, ref alerts)) = guard.alerts {
                if ts.elapsed() < self.cache_ttl {
                    return Ok(alerts.clone());
                }
            }
        }

        if self.mock_mode {
            return Ok(guard
                .alerts
                .as_ref()
                .map(|(_, a)| a.clone())
                .unwrap_or_default());
        }

        let octo = match self.get_octocrab().await {
            Some(o) => o,
            None => {
                return Ok(guard
                    .alerts
                    .as_ref()
                    .map(|(_, a)| a.clone())
                    .unwrap_or_default());
            }
        };

        match octo.activity().notifications().list().send().await {
            Ok(page) => {
                let items: Vec<AlertItem> = page
                    .into_iter()
                    .map(|n| AlertItem {
                        id: n.id.to_string(),
                        title: n.subject.title,
                        repository: n.repository.full_name.unwrap_or_default(),
                        reason: n.reason,
                        unread: n.unread,
                        updated_at: n.updated_at.to_rfc3339(),
                        url: n.subject.url.map(|u| u.to_string()).unwrap_or_default(),
                    })
                    .collect();

                guard.alerts = Some((Instant::now(), items.clone()));
                if guard.rate_limit.remaining > 0 {
                    guard.rate_limit.remaining -= 1;
                }
                Ok(items)
            }
            Err(e) => Err(GitHubError::from(e)),
        }
    }

    /// Mark a notification as read.
    pub async fn mark_notification_read(&self, thread_id: &str) -> Result<(), GitHubError> {
        let mut guard = self.cache.lock().await;
        if let Some((_, ref mut alerts)) = guard.alerts {
            if let Some(item) = alerts.iter_mut().find(|a| a.id == thread_id) {
                item.unread = false;
            }
        }

        if self.mock_mode {
            return Ok(());
        }

        if let Ok(id_u64) = thread_id.parse::<u64>() {
            if let Some(octo) = self.get_octocrab().await {
                if let Err(e) = octo
                    .activity()
                    .notifications()
                    .mark_as_read(id_u64.into())
                    .await
                {
                    return Err(GitHubError::from(e));
                }
            }
        }

        Ok(())
    }

    /// Rerun a failed or completed workflow run.
    pub async fn rerun_workflow_run(
        &self,
        target: &GitHubRepoTarget,
        run_id: u64,
    ) -> Result<String, GitHubError> {
        if self.mock_mode {
            let mut guard = self.cache.lock().await;
            let target_key = (target.owner.clone(), target.repo.clone());
            let runs_opt = if let Some(repo) = guard.repos.get_mut(&target_key) {
                repo.runs.as_mut()
            } else {
                guard.mock_defaults.runs.as_mut()
            };
            if let Some((_, ref mut runs)) = runs_opt {
                if let Some(r) = runs.iter_mut().find(|r| r.id == run_id) {
                    r.status = "in_progress".to_string();
                    r.conclusion = None;
                }
            }
            return Ok(format!("Triggered rerun for workflow #{}", run_id));
        }

        // Try using `gh run rerun <id>` first
        if let Ok(output) = Command::new("gh")
            .args([
                "run",
                "rerun",
                &run_id.to_string(),
                "-R",
                &format!("{}/{}", target.owner, target.repo),
            ])
            .output()
        {
            if output.status.success() {
                return Ok(format!("Triggered rerun for workflow #{}", run_id));
            }
        }

        // Try octocrab post
        if let Some(octo) = self.get_octocrab().await {
            let route = format!(
                "/repos/{}/{}/actions/runs/{}/rerun",
                target.owner, target.repo, run_id
            );
            match octo.post::<_, serde_json::Value>(&route, None::<&()>).await {
                Ok(_) => Ok(format!("Triggered rerun for workflow #{}", run_id)),
                Err(e) => Err(GitHubError::from(e)),
            }
        } else {
            Err(GitHubError::NoToken)
        }
    }

    /// Checkout a PR branch locally into the working directory.
    pub fn checkout_pr_locally(
        &self,
        target: &GitHubRepoTarget,
        pr_number: u64,
        head_branch: &str,
        workdir: &std::path::Path,
    ) -> Result<String, GitHubError> {
        // Try `gh pr checkout <number>`
        if let Ok(output) = Command::new("gh")
            .current_dir(workdir)
            .args([
                "pr",
                "checkout",
                &pr_number.to_string(),
                "-R",
                &format!("{}/{}", target.owner, target.repo),
            ])
            .output()
        {
            if output.status.success() {
                return Ok(format!(
                    "Checked out PR #{} ({}) via gh CLI",
                    pr_number, head_branch
                ));
            }
        }

        // Fallback to git fetch and checkout with timeout guard
        let fetch_ref = format!("pull/{}/head:pr-{}", pr_number, pr_number);
        let fetch_res = crate::git::clean_git_run(workdir, &["fetch", "origin", &fetch_ref]);

        match fetch_res {
            Ok(fetch_out) if fetch_out.status.success() => {
                let branch_name = format!("pr-{}", pr_number);
                let co = crate::git::clean_git_run(workdir, &["checkout", &branch_name]);

                match co {
                    Ok(out) if out.status.success() => Ok(format!(
                        "Checked out PR #{} as branch '{}'",
                        pr_number, branch_name
                    )),
                    Ok(out) => Err(GitHubError::Cli(
                        String::from_utf8_lossy(&out.stderr).trim().to_string(),
                    )),
                    Err(e) => Err(GitHubError::Cli(e.to_string())),
                }
            }
            Ok(fetch_out) => {
                let err = String::from_utf8_lossy(&fetch_out.stderr)
                    .trim()
                    .to_string();
                Err(GitHubError::Cli(format!(
                    "Failed to fetch PR #{} from origin: {}",
                    pr_number,
                    if err.is_empty() {
                        "remote reference not found or network error".to_string()
                    } else {
                        err
                    }
                )))
            }
            Err(e) => Err(GitHubError::Cli(format!(
                "Failed to execute git fetch for PR #{}: {}",
                pr_number, e
            ))),
        }
    }
}
