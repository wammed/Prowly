use prowly::ai::{
    smart_merge_conflict, translate_natural_language_to_git, CommandRiskLevel, MockAiClient,
};
use prowly::git::conflict::ConflictHunk;
use prowly::i18n::Language;

#[tokio::test]
async fn test_smart_merge_conflict() {
    let mock_response = "    println!(\"Combined: Current & Incoming\");\n";
    let client = MockAiClient::new(mock_response);

    let hunk = ConflictHunk {
        index: 0,
        current_branch_name: "main".to_string(),
        incoming_branch_name: "feature".to_string(),
        current_lines: vec!["    println!(\"Current\");".to_string()],
        incoming_lines: vec!["    println!(\"Incoming\");".to_string()],
        choice: None,
        custom_resolution: None,
    };

    let result = smart_merge_conflict("src/main.rs", &hunk, &client, Language::En)
        .await
        .unwrap();

    assert_eq!(result, "println!(\"Combined: Current & Incoming\");");
}

#[tokio::test]
async fn test_translate_natural_language_to_git_safe() {
    let mock_response = "git status\nSafe\nShows the working tree status.";
    let client = MockAiClient::new(mock_response);

    let suggestion =
        translate_natural_language_to_git("変更状態を確認したい", "main", &client, Language::Ja)
            .await
            .unwrap();

    assert_eq!(suggestion.command, "git status");
    assert_eq!(suggestion.risk_level, CommandRiskLevel::Safe);
    assert!(suggestion
        .explanation
        .contains("Shows the working tree status"));
}

#[tokio::test]
async fn test_translate_natural_language_to_git_dangerous() {
    let mock_response = "git reset --hard HEAD~1\nDangerous\nDiscards recent commit and working changes irreversibly.";
    let client = MockAiClient::new(mock_response);

    let suggestion = translate_natural_language_to_git(
        "直前のコミットを取り消して変更も全て破棄したい",
        "main",
        &client,
        Language::Ja,
    )
    .await
    .unwrap();

    assert_eq!(suggestion.command, "git reset --hard HEAD~1");
    assert_eq!(suggestion.risk_level, CommandRiskLevel::Dangerous);
}
