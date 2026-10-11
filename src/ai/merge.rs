//! AI-assisted merge conflict resolution.

use crate::ai::{AiClient, AiError, AiPrompt};
use crate::git::conflict::ConflictHunk;
use crate::i18n::Language;

/// Perform an intelligent merge of a conflict hunk using AI.
pub async fn smart_merge_conflict(
    file_path: &str,
    hunk: &ConflictHunk,
    client: &dyn AiClient,
    lang: Language,
) -> Result<String, AiError> {
    let system = match lang {
        Language::Ja => {
            "あなたはGitマージコンフリクト解決の専門家です。\n\
Current(現在)のブランチとIncoming(取り込み元)のブランチのコード意図を正確に分析し、両方の要件を満たす最適なマージ後コードを出力してください。\n\
注意: コードブロックやMarkdownの装飾(```)、解説テキストは含めず、解決後のソースコード行のみをそのまま返してください。"
        }
        Language::En => {
            "You are an expert at resolving Git merge conflicts.\n\
Analyze the intents of both Current and Incoming branches and output the optimal merged code.\n\
Important: Return ONLY the resolved source code lines. Do NOT include markdown code fences (```) or conversational explanations."
        }
    };

    let user = format!(
        "File: {}\n\n<<<<<<< Current ({})\n{}\n=======\n{}\n>>>>>>> Incoming ({})",
        file_path,
        hunk.current_branch_name,
        hunk.current_lines.join("\n"),
        hunk.incoming_lines.join("\n"),
        hunk.incoming_branch_name
    );

    let prompt = AiPrompt::system_and_user(system, user);
    let res = client.complete(&prompt).await?;
    let content = res.content.trim();

    // Clean any accidental markdown fences if returned
    let cleaned = content
        .trim_start_matches("```rust")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    Ok(cleaned.to_string())
}
