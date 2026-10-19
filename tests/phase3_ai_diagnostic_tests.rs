use prowly::ai::client::MockAiClient;
use prowly::ai::diagnostic::{diagnose_repository, RepoHealth};
use prowly::git::{FileStatus, GitFileItem, GitRepoStatus};
use prowly::i18n::Language;

#[tokio::test]
async fn test_ai_diagnostic_needs_commit() {
    let mock_response = "NeedsCommit\nステージされた変更があります。コミットを推奨します\n• コミットメッセージを入力してコミットしてください\n• テストを実行して整合性を確認してください";
    let client = MockAiClient::new(mock_response);

    let status = GitRepoStatus {
        branch: "main".to_string(),
        upstream: Some("origin/main".to_string()),
        ahead: 0,
        behind: 0,
        files: vec![GitFileItem {
            path: "src/main.rs".to_string(),
            status: FileStatus::Modified,
            staged: true,
        }],
        operation_state: Default::default(),
    };

    let diagnosis = diagnose_repository(&status, &client, Language::Ja)
        .await
        .expect("diagnose_repository failed");

    assert_eq!(diagnosis.health, RepoHealth::NeedsCommit);
    assert!(diagnosis.headline.contains("コミットを推奨"));
    assert_eq!(diagnosis.recommended_actions.len(), 2);
}

#[tokio::test]
async fn test_ai_diagnostic_conflicted() {
    let mock_response = "Conflicted\n競合が発生しています\n• 競合マーカーを解消してください";
    let client = MockAiClient::new(mock_response);

    let status = GitRepoStatus {
        branch: "feature".to_string(),
        upstream: None,
        ahead: 0,
        behind: 0,
        files: vec![GitFileItem {
            path: "src/lib.rs".to_string(),
            status: FileStatus::Conflicted,
            staged: false,
        }],
        operation_state: Default::default(),
    };

    let diagnosis = diagnose_repository(&status, &client, Language::En)
        .await
        .expect("diagnose_repository failed");

    assert_eq!(diagnosis.health, RepoHealth::Conflicted);
}
