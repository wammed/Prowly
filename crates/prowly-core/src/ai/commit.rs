//! AI-assisted Conventional Commits generator and pre-commit self code review.

use crate::ai::{AiClient, AiError, AiPrompt};
use crate::i18n::Language;

/// Generated commit message separating short summary and multi-line body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedCommit {
    pub summary: String,
    pub description: Option<String>,
}

/// Default maximum characters of diff passed to AI context to prevent overflow (approx 3,000 tokens).
pub const DEFAULT_MAX_DIFF_CHARS: usize = 12_000;

/// Safely truncate and filter a git diff to fit within model context windows.
/// Filters out verbose lockfile diffs while preserving commit-critical change headers and structure.
pub fn truncate_diff(diff: &str, max_chars: usize) -> String {
    let mut out = String::with_capacity(max_chars.min(diff.len()));
    let mut in_lockfile = false;
    let mut omitted_lines = 0usize;
    let mut total_lines = 0usize;

    for line in diff.lines() {
        total_lines += 1;

        // Detect file headers to check for lockfiles
        if line.starts_with("diff --git ")
            || line.starts_with("--- a/")
            || line.starts_with("+++ b/")
        {
            let lower = line.to_lowercase();
            in_lockfile =
                lower.contains("lock") || lower.contains(".lock") || lower.ends_with("-lock.json");
            if in_lockfile && line.starts_with("diff --git ") {
                out.push_str(line);
                out.push_str("\n[Lockfile changes omitted to preserve context]\n");
                continue;
            }
        }

        if in_lockfile && !line.starts_with("diff --git ") {
            continue;
        }

        // Check if adding this line would exceed max_chars budget
        if out.len() + line.len() + 1 > max_chars {
            omitted_lines += 1;
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    if omitted_lines > 0 {
        out.push_str(&format!(
            "\n[... Diff truncated: context limit reached ({} lines omitted out of {}) ...]\n",
            omitted_lines, total_lines
        ));
    }

    out
}

/// Ask AI to generate a Conventional Commit message from staged diff.
pub async fn generate_commit_message(
    diff: &str,
    client: &dyn AiClient,
    lang: Language,
) -> Result<GeneratedCommit, AiError> {
    let system = match lang {
        Language::Ja => {
            "あなたはプロフェッショナルなGitコミットメッセージ生成アシスタントです。\n\
提供された差分（Diff）を分析し、Conventional Commits形式（例: feat(auth): add login flow または fix(ui): resolve layout overflow）でコミットメッセージを作成してください。\n\
1行目は要約（50文字以内）、2行目は空行、3行目以降は変更理由や詳細を簡潔に記述してください。\n\
コードブロック等の余計な装飾は含めず、コミットメッセージ本文のみを出力してください。"
        }
        Language::En => {
            "You are a professional Git commit message assistant.\n\
Analyze the provided diff and generate a Conventional Commits message (e.g. feat(auth): add login flow or fix(ui): resolve layout overflow).\n\
Line 1 must be a concise summary under 50 chars, followed by an empty line, then a brief description if needed.\n\
Output ONLY the raw commit message text without markdown code blocks."
        }
    };

    let sanitized_diff = crate::ai::mask_secrets(diff);
    let truncated_diff = truncate_diff(&sanitized_diff, DEFAULT_MAX_DIFF_CHARS);
    let user = format!("Git Diff:\n```diff\n{}\n```", truncated_diff);
    let prompt = AiPrompt::system_and_user(system, user);
    let res = client.complete(&prompt).await?;

    let lines: Vec<&str> = res.content.trim().lines().collect();
    if lines.is_empty() {
        return Ok(GeneratedCommit {
            summary: "chore: update repository".to_string(),
            description: None,
        });
    }

    let summary = lines[0].trim().to_string();
    let description = if lines.len() > 2 {
        Some(lines[2..].join("\n").trim().to_string())
    } else {
        None
    };

    Ok(GeneratedCommit {
        summary,
        description,
    })
}

/// Ask AI to perform a pre-commit self code review on changes.
pub async fn review_changes(
    diff: &str,
    client: &dyn AiClient,
    lang: Language,
) -> Result<String, AiError> {
    let system = match lang {
        Language::Ja => {
            "あなたはシニアエンジニアです。コミット前の差分（Diff）をレビューしてください。\n\
以下の観点検知に特化してください:\n\
1. 潜在的なバグ・エッジケース抜け\n\
2. デバッグ用コード（dbg!, println!, console.log等）の消し忘れ\n\
3. セキュリティ・秘密情報の露出リスク\n\
4. パフォーマンス上の注意点\n\
問題がなければ「LGTM! 顕著な問題は見当たりません」と簡潔に答えてください。"
        }
        Language::En => {
            "You are a senior staff engineer performing a pre-commit code review on git changes.\n\
Check specifically for:\n\
1. Potential bugs or unhandled edge cases\n\
2. Leftover debug code (dbg!, println!, console.log)\n\
3. Security issues or exposed secrets\n\
4. Performance regressions\n\
If everything looks good, provide a brief LGTM."
        }
    };

    let sanitized_diff = crate::ai::mask_secrets(diff);
    let truncated_diff = truncate_diff(&sanitized_diff, DEFAULT_MAX_DIFF_CHARS);
    let user = format!("Git Diff to review:\n```diff\n{}\n```", truncated_diff);
    let prompt = AiPrompt::system_and_user(system, user);
    let res = client.complete(&prompt).await?;
    Ok(res.content)
}
