//! AI-powered repository status diagnostics and next action recommendations.

use crate::ai::{AiClient, AiError, AiPrompt};
use crate::git::GitRepoStatus;
use crate::i18n::Language;
use serde::{Deserialize, Serialize};

/// High-level health status of a Git repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoHealth {
    Clean,
    NeedsCommit,
    NeedsPush,
    NeedsPull,
    Conflicted,
}

/// Structured diagnostic result returned by the AI assistant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoDiagnosis {
    pub health: RepoHealth,
    pub headline: String,
    pub recommended_actions: Vec<String>,
    pub explanation: String,
}

/// Perform an AI diagnostic scan of the current repository state.
pub async fn diagnose_repository(
    status: &GitRepoStatus,
    client: &dyn AiClient,
    lang: Language,
) -> Result<RepoDiagnosis, AiError> {
    let staged_count = status.files.iter().filter(|f| f.staged).count();
    let unstaged_count = status.files.len().saturating_sub(staged_count);
    let conflict_count = status
        .files
        .iter()
        .filter(|f| f.status == crate::git::FileStatus::Conflicted)
        .count();

    let system = match lang {
        Language::Ja => {
            "あなたはGitの熟練アーキテクトです。現在のリポジトリ状態を診断し、開発者が次にとるべき最善のアクションを提案してください。\n\
回答フォーマット:\n\
1行目: [Clean | NeedsCommit | NeedsPush | NeedsPull | Conflicted] のいずれか1単語\n\
2行目: 簡潔な要約見出し (例: ステージされた変更があります。コミットを推奨します)\n\
3行目以降: 推奨アクション（箇条書き • で3点以内）および簡単な解説"
        }
        Language::En => {
            "You are an expert Git architect. Diagnose the repository state and recommend the best next actions.\n\
Format:\n\
Line 1: One word out of [Clean | NeedsCommit | NeedsPush | NeedsPull | Conflicted]\n\
Line 2: Brief headline summary\n\
Line 3+: Recommended actions (bullet points with •) and short explanation"
        }
    };

    let user = format!(
        "Repository Status:\n\
Branch: {}\n\
Upstream: {}\n\
Ahead: {}, Behind: {}\n\
Staged Files: {}\n\
Unstaged Files: {}\n\
Conflicts: {}\n\
Files:\n{}",
        status.branch,
        status.upstream.as_deref().unwrap_or("none"),
        status.ahead,
        status.behind,
        staged_count,
        unstaged_count,
        conflict_count,
        status
            .files
            .iter()
            .take(10)
            .map(|f| format!("- [{}] {}", f.status.badge(), f.path))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let prompt = AiPrompt::system_and_user(system, user);
    let res = client.complete(&prompt).await?;

    let lines: Vec<&str> = res.content.trim().lines().collect();

    let health = if conflict_count > 0 {
        RepoHealth::Conflicted
    } else if let Some(first) = lines.first() {
        match first.to_lowercase().trim() {
            s if s.contains("clean") => RepoHealth::Clean,
            s if s.contains("push") => RepoHealth::NeedsPush,
            s if s.contains("pull") => RepoHealth::NeedsPull,
            s if s.contains("conflict") => RepoHealth::Conflicted,
            _ => RepoHealth::NeedsCommit,
        }
    } else {
        RepoHealth::NeedsCommit
    };

    let headline = if lines.len() > 1 {
        lines[1].trim().to_string()
    } else {
        "Repository diagnosis ready".to_string()
    };

    let mut recommended_actions = Vec::new();
    let mut explanation_lines = Vec::new();

    for line in lines.iter().skip(2) {
        let trimmed = line.trim();
        if trimmed.starts_with('•') || trimmed.starts_with('-') || trimmed.starts_with('*') {
            recommended_actions.push(trimmed.trim_start_matches(['•', '-', '*', ' ']).to_string());
        } else if !trimmed.is_empty() {
            explanation_lines.push(trimmed.to_string());
        }
    }

    if recommended_actions.is_empty() {
        if staged_count > 0 {
            recommended_actions.push("Commit staged changes (c)".to_string());
        }
        if unstaged_count > 0 {
            recommended_actions.push("Stage modified files (a)".to_string());
        }
        if status.ahead > 0 {
            recommended_actions.push("Push commits to upstream".to_string());
        }
    }

    Ok(RepoDiagnosis {
        health,
        headline,
        recommended_actions,
        explanation: explanation_lines.join("\n"),
    })
}
