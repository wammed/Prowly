//! Screen context collection and prompt assembly for the Omnipresent AI engine.

use crate::i18n::Language;
use serde::{Deserialize, Serialize};

/// Context snippet for a Pull Request when viewing PRs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PrContext {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub base_branch: String,
    pub head_branch: String,
    pub body: Option<String>,
    pub review_comments: Vec<String>,
}

/// Information about a CI workflow run failure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CiContext {
    pub workflow_name: String,
    pub job_name: String,
    pub failed_step: Option<String>,
    pub error_log_tail: String,
}

/// Information about a Git conflict in a file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConflictContext {
    pub file_path: String,
    pub conflict_markers_snippet: String,
    pub our_branch: String,
    pub their_branch: String,
}

/// Aggregated screen context gathered from the active UI state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiContext {
    pub active_view: String,
    pub repo_name: Option<String>,
    pub repo_path: Option<String>,
    pub current_branch: Option<String>,
    pub upstream_ahead: usize,
    pub upstream_behind: usize,

    // File / Diff context
    pub selected_file: Option<String>,
    pub is_staged: Option<bool>,
    pub diff_snippet: Option<String>,
    pub cursor_line: Option<usize>,

    // PR, CI, Conflict, Error context
    pub pr: Option<PrContext>,
    pub ci: Option<CiContext>,
    pub conflict: Option<ConflictContext>,
    pub last_git_error: Option<String>,
}

impl AiContext {
    /// Create an empty context.
    pub fn new(active_view: impl Into<String>) -> Self {
        Self {
            active_view: active_view.into(),
            ..Default::default()
        }
    }

    /// Short single-line description of the captured context for display in the AI overlay.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("View: {}", self.active_view));
        if let Some(branch) = &self.current_branch {
            parts.push(format!("Branch: {}", branch));
        }
        if let Some(file) = &self.selected_file {
            parts.push(format!("File: {}", file));
        }
        if let Some(pr) = &self.pr {
            parts.push(format!("PR #{}", pr.number));
        }
        if let Some(ci) = &self.ci {
            parts.push(format!("CI: {}", ci.job_name));
        }
        if self.conflict.is_some() {
            parts.push("Conflict active".to_string());
        }
        if self.last_git_error.is_some() {
            parts.push("Git error".to_string());
        }
        parts.join(" | ")
    }

    /// Assemble the system prompt tailored to the user's language and Prowly's assistant role.
    pub fn format_system_prompt(&self, lang: Language) -> String {
        match lang {
            Language::Ja => indoc_ja(),
            Language::En => indoc_en(),
        }
    }

    /// Assemble user prompt including context payload and user query.
    pub fn format_user_prompt(&self, user_query: &str) -> String {
        let mut sections = Vec::new();

        sections.push(format!("### Current UI Context: [{}]", self.active_view));

        if let (Some(repo), Some(branch)) = (&self.repo_name, &self.current_branch) {
            sections.push(format!(
                "Repository: {} (Branch: {}, Ahead: {}, Behind: {})",
                repo, branch, self.upstream_ahead, self.upstream_behind
            ));
        }

        if let Some(file) = &self.selected_file {
            let staged_str = match self.is_staged {
                Some(true) => "Staged",
                Some(false) => "Unstaged",
                None => "Unknown",
            };
            sections.push(format!("Selected File: {} ({})", file, staged_str));
        }

        if let Some(diff) = &self.diff_snippet {
            sections.push(format!("```diff\n{}\n```", diff));
        }

        if let Some(pr) = &self.pr {
            sections.push(format!(
                "PR #{}: {}\nBase: {} <- Head: {}\nAuthor: {}\nDescription:\n{}",
                pr.number,
                pr.title,
                pr.base_branch,
                pr.head_branch,
                pr.author,
                pr.body.as_deref().unwrap_or("(none)")
            ));
            if !pr.review_comments.is_empty() {
                sections.push(format!(
                    "Review Comments:\n{}",
                    pr.review_comments.join("\n")
                ));
            }
        }

        if let Some(ci) = &self.ci {
            sections.push(format!(
                "CI Failure: Workflow '{}', Job '{}', Step: '{}'\nLog Output:\n```\n{}\n```",
                ci.workflow_name,
                ci.job_name,
                ci.failed_step.as_deref().unwrap_or("unknown"),
                ci.error_log_tail
            ));
        }

        if let Some(conflict) = &self.conflict {
            sections.push(format!(
                "Merge Conflict in {}:\nOur branch: {}\nTheir branch: {}\nSnippet:\n```\n{}\n```",
                conflict.file_path,
                conflict.our_branch,
                conflict.their_branch,
                conflict.conflict_markers_snippet
            ));
        }

        if let Some(err) = &self.last_git_error {
            sections.push(format!("Recent Git Error Output:\n```\n{}\n```", err));
        }

        sections.push("### User Request / Question:".to_string());
        sections.push(user_query.to_string());

        sections.join("\n\n")
    }
}

fn indoc_ja() -> String {
    "あなたはターミナルGitクライアント「Prowly」に統合された専属AIアシスタント（黒子AI）です。\n\
ユーザーの画面コンテキスト（ブランチ、差分、CIエラーログ、PR情報、コンフリクト等）を正確に把握し、\n\
的確で簡潔かつ安全な助言を提供してください。\n\
変更パッチやGitコマンドを提案する際は、ユーザーが承認（Apply）しやすいよう明確にコードブロックで提示してください。\n\
破壊的な操作（force push, hard reset等）を提案する場合は必ず警告を添えてください。"
        .to_string()
}

fn indoc_en() -> String {
    "You are the embedded AI Assistant in Prowly, a modern terminal Git & GitHub client.\n\
You have access to the user's active UI context (current branch, diff snippet, CI logs, PRs, conflicts).\n\
Provide concise, actionable, and safe advice.\n\
When suggesting code diffs or Git commands, format them clearly in markdown code blocks so the user can easily review and apply them.\n\
Always warn before suggesting destructive Git actions (such as git reset --hard or git push --force)."
        .to_string()
}

/// Trait implemented by views and controllers to supply current screen context to the AI assistant.
pub trait ContextProvider {
    fn capture_ai_context(&self) -> AiContext;
}
