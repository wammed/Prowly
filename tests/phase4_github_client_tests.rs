use prowly::github::{
    AlertItem, GitHubRepoTarget, GitHubService, PullRequestItem, WorkflowRunItem,
};

#[test]
fn test_github_repo_target_from_remote_url() {
    // SSH format
    let target1 = GitHubRepoTarget::from_remote_url("git@github.com:wammed/Prowly.git");
    assert_eq!(target1, Some(GitHubRepoTarget::new("wammed", "Prowly")));

    // HTTPS format with .git
    let target2 = GitHubRepoTarget::from_remote_url("https://github.com/octocat/Hello-World.git");
    assert_eq!(
        target2,
        Some(GitHubRepoTarget::new("octocat", "Hello-World"))
    );

    // HTTPS format without .git
    let target3 = GitHubRepoTarget::from_remote_url("https://github.com/rust-lang/rust");
    assert_eq!(target3, Some(GitHubRepoTarget::new("rust-lang", "rust")));

    // Non-github url
    let target4 = GitHubRepoTarget::from_remote_url("git@gitlab.com:user/project.git");
    assert_eq!(target4, None);
}

#[tokio::test]
async fn test_github_service_mock_and_caching() {
    let mut service = GitHubService::new(None);
    service.enable_mock_mode();
    assert!(service.is_mock_mode());

    let prs = vec![PullRequestItem {
        number: 42,
        title: "feat: add ai co-pilot".to_string(),
        author: "susie".to_string(),
        state: "open".to_string(),
        is_draft: false,
        head_branch: "feat-ai".to_string(),
        base_branch: "main".to_string(),
        body: Some("Description".to_string()),
        comments_count: 5,
        html_url: "https://github.com/wammed/Prowly/pull/42".to_string(),
        updated_at: "2026-10-11T20:00:00Z".to_string(),
    }];

    let runs = vec![WorkflowRunItem {
        id: 101,
        name: "CI".to_string(),
        head_branch: "main".to_string(),
        event: "push".to_string(),
        status: "completed".to_string(),
        conclusion: Some("success".to_string()),
        html_url: "https://github.com/wammed/Prowly/actions/runs/101".to_string(),
        created_at: "2026-10-11T20:00:00Z".to_string(),
        logs: Some("All tests passed".to_string()),
    }];

    let alerts = vec![AlertItem {
        id: "alert-1".to_string(),
        title: "Security advisory".to_string(),
        repository: "wammed/Prowly".to_string(),
        reason: "mention".to_string(),
        unread: true,
        updated_at: "2026-10-11T20:00:00Z".to_string(),
        url: "https://github.com/wammed/Prowly/notifications".to_string(),
    }];

    service
        .set_mock_data(prs.clone(), runs.clone(), alerts.clone())
        .await;

    let target = GitHubRepoTarget::new("wammed", "Prowly");

    // Fetch PRs from mock/cache
    let fetched_prs = service.list_pull_requests(&target, false).await.unwrap();
    assert_eq!(fetched_prs.len(), 1);
    assert_eq!(fetched_prs[0].number, 42);

    // Fetch Runs from mock/cache
    let fetched_runs = service.list_workflow_runs(&target, false).await.unwrap();
    assert_eq!(fetched_runs.len(), 1);
    assert_eq!(fetched_runs[0].conclusion.as_deref(), Some("success"));

    // Fetch Alerts from mock/cache
    let fetched_alerts = service.list_alerts(false).await.unwrap();
    assert_eq!(fetched_alerts.len(), 1);
    assert!(fetched_alerts[0].unread);

    // Mark Alert as Read
    let mark_res = service.mark_notification_read("alert-1").await;
    assert!(mark_res.is_ok());

    let updated_alerts = service.list_alerts(false).await.unwrap();
    assert!(!updated_alerts[0].unread);

    // Test RepoMetadata visibility
    let meta_default = service.get_repo_metadata(&target, false).await.unwrap();
    assert!(!meta_default.is_private, "Default in mock mode should be public (is_private=false)");

    service.set_mock_repo_metadata(true).await;
    let meta_private = service.get_repo_metadata(&target, true).await.unwrap();
    assert!(meta_private.is_private, "Should return is_private=true after set_mock_repo_metadata(true)");

    service.set_mock_repo_metadata(false).await;
    let meta_public = service.get_repo_metadata(&target, true).await.unwrap();
    assert!(!meta_public.is_private, "Should return is_private=false after set_mock_repo_metadata(false)");
}

