use prowly::ai::ci::analyze_ci_failure;
use prowly::ai::client::MockAiClient;
use prowly::ai::pr::generate_pr_draft;
use prowly::i18n::Language;

#[tokio::test]
async fn test_ai_generate_pr_draft() {
    let mock_response = "feat(auth): add OAuth2 refresh token flow\n\n## 概要\nOAuth2のアクセストークン自動更新機能を実装しました。\n\n## 変更点\n• トークンリフレッシュエンドポイントの追加\n• セッション期限切れハンドラの追加";
    let client = MockAiClient::new(mock_response);

    let draft = generate_pr_draft("feature-oauth", "diff content here", &client, Language::Ja)
        .await
        .expect("generate_pr_draft failed");

    assert_eq!(draft.title, "feat(auth): add OAuth2 refresh token flow");
    assert!(draft.body.contains("## 概要"));
    assert!(draft.body.contains("OAuth2"));
}

#[tokio::test]
async fn test_ai_analyze_ci_failure() {
    let mock_response = "原因: 型の不一致 (expected u64, found String)\n対策: .parse::<u64>() で数値へ変換してください\n詳細:\nCargo check でコンパイルエラーが発生しています。";
    let client = MockAiClient::new(mock_response);

    let log = "error[E0308]: mismatched types\n --> src/main.rs:15:9\n expected u64, found String";

    let analysis = analyze_ci_failure("Build & Test", log, &client, Language::Ja)
        .await
        .expect("analyze_ci_failure failed");

    assert!(analysis.root_cause.contains("型の不一致"));
    assert!(analysis.suggested_fix.contains("parse::<u64>()"));
    assert!(analysis.full_advice.contains("Cargo check"));
}
