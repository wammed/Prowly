use prowly::ai::{
    AiClient, AiConfig, AiContext, AiPrompt, AiProvider, CiContext, ConflictContext, MockAiClient,
    OllamaClient, PrContext,
};
use prowly::i18n::Language;

#[test]
fn test_ollama_chat_url_resolution() {
    let mut cfg = AiConfig {
        provider: AiProvider::Ollama,
        api_base: None,
        ..Default::default()
    };

    // 1. Default (None) -> http://localhost:11434/api/chat
    let client = OllamaClient::new(cfg.clone());
    assert_eq!(client.chat_url(), "http://localhost:11434/api/chat");

    // 2. Host with port without /api
    cfg.api_base = Some("http://127.0.0.1:11434".to_string());
    let client = OllamaClient::new(cfg.clone());
    assert_eq!(client.chat_url(), "http://127.0.0.1:11434/api/chat");

    // 3. Host with trailing slash
    cfg.api_base = Some("http://127.0.0.1:11434/".to_string());
    let client = OllamaClient::new(cfg.clone());
    assert_eq!(client.chat_url(), "http://127.0.0.1:11434/api/chat");

    // 4. Host explicitly with /api
    cfg.api_base = Some("http://127.0.0.1:11434/api".to_string());
    let client = OllamaClient::new(cfg.clone());
    assert_eq!(client.chat_url(), "http://127.0.0.1:11434/api/chat");

    // 5. Host explicitly with /api/
    cfg.api_base = Some("http://127.0.0.1:11434/api/".to_string());
    let client = OllamaClient::new(cfg);
    assert_eq!(client.chat_url(), "http://127.0.0.1:11434/api/chat");
}

#[tokio::test]
async fn test_mock_ai_client() {
    let mock = MockAiClient::new("Here is your Git commit message: feat(auth): add login flow");
    let prompt = AiPrompt::system_and_user("System instructions", "Generate commit message");
    let res = mock.complete(&prompt).await.unwrap();

    assert_eq!(
        res.content,
        "Here is your Git commit message: feat(auth): add login flow"
    );
    assert_eq!(res.model, "mock-model");
    assert_eq!(res.prompt_tokens, Some(10));
}

#[test]
fn test_ai_context_formatting() {
    let mut ctx = AiContext::new("Changes");
    ctx.repo_name = Some("prowly".to_string());
    ctx.current_branch = Some("feat/ai-phase0".to_string());
    ctx.upstream_ahead = 1;
    ctx.upstream_behind = 0;
    ctx.selected_file = Some("src/main.rs".to_string());
    ctx.is_staged = Some(true);
    ctx.diff_snippet = Some("+println!(\"Hello World\");".to_string());

    let summary = ctx.summary();
    assert!(summary.contains("View: Changes"));
    assert!(summary.contains("Branch: feat/ai-phase0"));
    assert!(summary.contains("File: src/main.rs"));

    let user_prompt = ctx.format_user_prompt("Suggest a conventional commit message");
    assert!(user_prompt.contains("Repository: prowly"));
    assert!(user_prompt.contains("Selected File: src/main.rs (Staged)"));
    assert!(user_prompt.contains("+println!(\"Hello World\");"));
    assert!(user_prompt.contains("Suggest a conventional commit message"));

    let sys_en = ctx.format_system_prompt(Language::En);
    assert!(sys_en.contains("Prowly"));
    assert!(sys_en.contains("destructive"));

    let sys_ja = ctx.format_system_prompt(Language::Ja);
    assert!(sys_ja.contains("黒子AI"));
    assert!(sys_ja.contains("破壊的"));
}

#[test]
fn test_ai_context_with_ci_and_pr() {
    let mut ctx = AiContext::new("Actions");
    ctx.ci = Some(CiContext {
        workflow_name: "CI Build".to_string(),
        job_name: "test".to_string(),
        failed_step: Some("cargo test".to_string()),
        error_log_tail: "FAILED test_something".to_string(),
    });
    ctx.pr = Some(PrContext {
        number: 42,
        title: "Add AI overlay".to_string(),
        author: "alice".to_string(),
        base_branch: "main".to_string(),
        head_branch: "feat/ai".to_string(),
        body: Some("Implements omnipresent AI".to_string()),
        review_comments: vec!["Looks good, check formatting".to_string()],
    });
    ctx.conflict = Some(ConflictContext {
        file_path: "src/lib.rs".to_string(),
        conflict_markers_snippet: "<<<<<<< HEAD\nfoo\n=======\nbar\n>>>>>>> branch".to_string(),
        our_branch: "HEAD".to_string(),
        their_branch: "branch".to_string(),
    });

    let prompt = ctx.format_user_prompt("Explain what failed");
    assert!(prompt.contains("CI Build"));
    assert!(prompt.contains("cargo test"));
    assert!(prompt.contains("PR #42: Add AI overlay"));
    assert!(prompt.contains("src/lib.rs"));
    assert!(prompt.contains("Explain what failed"));
}
