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
