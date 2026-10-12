use prowly::ai::{generate_commit_message, review_changes, MockAiClient};
use prowly::i18n::Language;

#[tokio::test]
async fn test_ai_commit_message_generation() {
    let mock = MockAiClient::new(
        "feat(tui): integrate hybrid git engine\n\n- Connect libgit2 for in-process fast queries\n- Provide soft-discard safety backup",
    );

    let diff_text = "+pub fn soft_discard() {}";
    let gen = generate_commit_message(diff_text, &mock, Language::En)
        .await
        .unwrap();

    assert_eq!(gen.summary, "feat(tui): integrate hybrid git engine");
    assert!(gen.description.is_some());
    assert!(gen
        .description
        .as_ref()
        .unwrap()
        .contains("Connect libgit2"));
}

#[tokio::test]
async fn test_ai_pre_commit_review() {
    let mock = MockAiClient::new(
        "LGTM! The changes include proper error handling and clean safety backups.",
    );

    let diff_text = "+pub fn soft_discard() {}";
    let review = review_changes(diff_text, &mock, Language::Ja)
        .await
        .unwrap();

    assert!(review.contains("LGTM"));
}

#[test]
fn test_truncate_diff_small_diff_unchanged() {
    let diff = "diff --git a/src/main.rs b/src/main.rs\n+println!(\"hello\");\n";
    let truncated = prowly::ai::truncate_diff(diff, 1000);
    assert_eq!(diff, truncated);
}

#[test]
fn test_truncate_diff_large_diff_truncated() {
    let mut large_diff = String::new();
    large_diff.push_str("diff --git a/big.rs b/big.rs\n");
    for i in 0..500 {
        large_diff.push_str(&format!("+line {} of very large diff text content here\n", i));
    }

    let truncated = prowly::ai::truncate_diff(&large_diff, 1000);
    assert!(truncated.len() < large_diff.len());
    assert!(truncated.contains("Diff truncated"));
    assert!(truncated.contains("lines omitted"));
}

#[test]
fn test_truncate_diff_lockfile_omission() {
    let diff = "diff --git a/Cargo.lock b/Cargo.lock\n+[[package]]\n+name = \"foo\"\n+version = \"1.0.0\"\ndiff --git a/src/lib.rs b/src/lib.rs\n+pub fn lib() {}\n";
    let truncated = prowly::ai::truncate_diff(diff, 10_000);
    assert!(truncated.contains("Lockfile changes omitted"));
    assert!(!truncated.contains("+[[package]]"));
    assert!(truncated.contains("pub fn lib()"));
}

#[test]
fn test_ollama_client_num_ctx_config() {
    let mut cfg = prowly::ai::AiConfig::default();
    cfg.provider = prowly::ai::AiProvider::Ollama;
    assert_eq!(cfg.num_ctx, Some(8192));

    cfg.num_ctx = Some(16384);
    let client = prowly::ai::OllamaClient::new(cfg);
    assert_eq!(client.chat_url(), "http://localhost:11434/api/chat");
}
