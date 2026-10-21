//! AI GitHub Actions CI failure log analyzer and fix advisor.

use crate::ai::{AiClient, AiError, AiPrompt};
use crate::i18n::Language;

/// Result of analyzing a CI failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiFailureAnalysis {
    pub root_cause: String,
    pub suggested_fix: String,
    pub full_advice: String,
}

/// Analyze CI failure log and produce root cause explanation + fix recommendations.
pub async fn analyze_ci_failure(
    workflow_name: &str,
    log_snippet: &str,
    client: &dyn AiClient,
    lang: Language,
) -> Result<CiFailureAnalysis, AiError> {
    let system = match lang {
        Language::Ja => {
            "あなたはCI/CDおよびテスト自動化のエキスパートです。\n\
GitHub Actions の失敗ログを解析し、根本原因と修正方法を開発者に簡潔に解説してください。\n\
回答フォーマット:\n\
Line 1: 根本原因の一行要約 (例: 原因: 型の不一致 (expected u64, found String))\n\
Line 2: 修正提案の一行要約 (例: 対策: .parse::<u64>() を追加してください)\n\
Line 3以降: 詳細解説と修正コード例"
        }
        Language::En => {
            "You are an expert in CI/CD and automated testing.\n\
Analyze the failed GitHub Actions log, diagnose the root cause, and suggest a concise code fix.\n\
Format:\n\
Line 1: One-line root cause (e.g. Root Cause: Type mismatch expected u64, found String)\n\
Line 2: One-line fix action (e.g. Fix: Add .parse::<u64>() to convert string)\n\
Line 3+: Detailed explanation and code snippets"
        }
    };

    let masked_logs = crate::ai::mask_secrets(log_snippet);
    let user = format!(
        "Workflow Run: {}\n\nError Log:\n{}",
        workflow_name,
        masked_logs.chars().take(4000).collect::<String>()
    );

    let prompt = AiPrompt::system_and_user(system, user);
    let res = client.complete(&prompt).await?;
    let content = res.content.trim();

    let lines: Vec<&str> = content.lines().collect();
    let root_cause = lines
        .first()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "CI workflow failed".to_string());
    let suggested_fix = lines
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Check the step logs and verify dependencies".to_string());

    Ok(CiFailureAnalysis {
        root_cause,
        suggested_fix,
        full_advice: content.to_string(),
    })
}
