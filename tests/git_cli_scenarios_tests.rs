//! Automated scenario tests for Git CLI execution, synchronization, conflict abort, and staging.

mod common;
use common::GitTestSandbox;
use prowly::git::RepoOperationState;
use prowly::services::SafeGitRunner;

#[test]
fn test_scenario_branch_checkout_commit_and_push() {
    let sandbox = GitTestSandbox::new();
    let service = sandbox.service();

    // 1. Create and switch to new branch
    sandbox.git_success(&["checkout", "-b", "feature/auth-layer"]);

    // 2. Add new file and stage it
    sandbox.write_file("auth.rs", "pub fn verify() -> bool { true }\n");
    service.stage_file("auth.rs").expect("Failed to stage file");

    let status = service.get_status().expect("Failed to get status");
    assert_eq!(status.files.len(), 1);
    assert!(status.files[0].staged);

    // 3. Commit staged changes
    sandbox.git_success(&["commit", "-m", "feat: add auth layer"]);

    // 4. Push via cli_push with automatic upstream tracking (-u origin feature/auth-layer)
    let push_result = service.cli_push();
    assert!(
        push_result.is_ok(),
        "cli_push should automatically set upstream on new branch: {:?}",
        push_result
    );

    // 5. Verify branch exists on remote
    let remote_branches = sandbox.git_success(&["branch", "-r"]);
    assert!(remote_branches.contains("origin/feature/auth-layer"));
}

#[test]
fn test_scenario_multi_client_pull_sync() {
    let sandbox1 = GitTestSandbox::new();
    let sandbox2 = sandbox1.clone_client();

    let service1 = sandbox1.service();
    let service2 = sandbox2.service();

    // 1. Client 1 makes and pushes a commit to main
    sandbox1.write_file("shared_doc.txt", "First paragraph by Client 1\n");
    service1.stage_file("shared_doc.txt").unwrap();
    sandbox1.git_success(&["commit", "-m", "docs: update shared doc"]);
    service1.cli_push().expect("Client 1 push failed");

    // 2. Client 2 pulls the changes using cli_pull
    let pull_result = service2.cli_pull();
    assert!(
        pull_result.is_ok(),
        "Client 2 pull failed: {:?}",
        pull_result
    );

    // 3. Verify content synchronized to Client 2
    let content = sandbox2.read_file("shared_doc.txt");
    assert_eq!(content, "First paragraph by Client 1\n");
}

#[test]
fn test_scenario_rebase_conflict_detection_and_abort() {
    let sandbox1 = GitTestSandbox::new();
    let sandbox2 = sandbox1.clone_client();

    let service1 = sandbox1.service();
    let service2 = sandbox2.service();

    // 1. Client 1 edits line 2 of README.md and pushes
    sandbox1.write_file("README.md", "# Test Repo\nClient 1 version\n");
    sandbox1.git_success(&["commit", "-am", "chore: client 1 edit"]);
    service1.cli_push().unwrap();

    // 2. Client 2 edits line 2 differently and commits locally
    sandbox2.write_file("README.md", "# Test Repo\nClient 2 version\n");
    sandbox2.git_success(&["commit", "-am", "chore: client 2 edit"]);

    // 3. Client 2 fetches and attempts rebase -> triggers conflict
    sandbox2.git_success(&["fetch", "origin"]);
    let rebase_out = sandbox2.git_exec(&["rebase", "origin/main"]);
    assert!(!rebase_out.status.success(), "Rebase should conflict");

    // 4. Verify Prowly detects RebaseInProgress
    assert_eq!(
        service2.get_operation_state(),
        RepoOperationState::RebaseInProgress
    );

    // 5. Abort the in-progress rebase
    let abort_res = service2.abort_current_operation();
    assert!(
        abort_res.is_ok(),
        "Abort rebase should succeed: {:?}",
        abort_res
    );

    // 6. Verify state returned to clean None and file restored to Client 2's commit
    assert_eq!(service2.get_operation_state(), RepoOperationState::None);
    assert_eq!(
        sandbox2.read_file("README.md"),
        "# Test Repo\nClient 2 version\n"
    );
}

#[test]
fn test_scenario_merge_conflict_detection_and_abort() {
    let sandbox = GitTestSandbox::new();
    let service = sandbox.service();

    // 1. Branch A: commit change
    sandbox.git_success(&["checkout", "-b", "topic/feature-x"]);
    sandbox.write_file("module.rs", "fn calc() -> i32 { 10 }\n");
    sandbox.git_success(&["add", "module.rs"]);
    sandbox.git_success(&["commit", "-m", "feat: feature-x implementation"]);

    // 2. Main: commit conflicting change
    sandbox.git_success(&["checkout", "main"]);
    sandbox.write_file("module.rs", "fn calc() -> i32 { 99 }\n");
    sandbox.git_success(&["add", "module.rs"]);
    sandbox.git_success(&["commit", "-m", "feat: main implementation"]);

    // 3. Merge topic into main -> triggers conflict
    let merge_out = sandbox.git_exec(&["merge", "topic/feature-x"]);
    assert!(!merge_out.status.success(), "Merge should conflict");

    // 4. Verify Prowly detects MergeInProgress
    assert_eq!(
        service.get_operation_state(),
        RepoOperationState::MergeInProgress
    );

    // 5. Abort in-progress merge
    let abort_res = service.abort_current_operation();
    assert!(
        abort_res.is_ok(),
        "Abort merge should succeed: {:?}",
        abort_res
    );

    // 6. Verify state restored to clean None and file restored to Main's commit
    assert_eq!(service.get_operation_state(), RepoOperationState::None);
    assert_eq!(sandbox.read_file("module.rs"), "fn calc() -> i32 { 99 }\n");
}

#[test]
fn test_scenario_stash_save_and_pop_lifecycle() {
    let sandbox = GitTestSandbox::new();
    let service = sandbox.service();

    // 1. Create dirty working tree modification
    sandbox.write_file("README.md", "Temporary uncommitted notes\n");

    let status = service.get_status().unwrap();
    assert!(!status.files.is_empty(), "Working tree should be dirty");

    // 2. Stash changes via service
    let stash_msg = service
        .cli_stash_save("WIP test stash")
        .expect("cli_stash_save failed");
    assert!(!stash_msg.is_empty());

    // 3. Working tree is clean and file reset to HEAD
    let status_clean = service.get_status().unwrap();
    assert!(status_clean.files.is_empty());
    assert_eq!(
        sandbox.read_file("README.md"),
        "# Test Repo\nInitial content\n"
    );

    // 4. Pop stash back
    let pop_res = service.cli_stash_pop();
    assert!(pop_res.is_ok(), "cli_stash_pop failed: {:?}", pop_res);

    // 5. Verify dirty changes restored
    assert_eq!(
        sandbox.read_file("README.md"),
        "Temporary uncommitted notes\n"
    );
}

#[test]
fn test_scenario_safe_git_runner_execution() {
    let sandbox = GitTestSandbox::new();

    // Verify SafeGitRunner can execute status and rev-parse without shell
    let status_out =
        SafeGitRunner::execute_git(sandbox.path(), "status", &["--porcelain".to_string()])
            .expect("SafeGitRunner failed");
    assert!(status_out.status.success());
    assert!(String::from_utf8_lossy(&status_out.stdout)
        .trim()
        .is_empty());

    let branch_out = SafeGitRunner::execute_git(
        sandbox.path(),
        "rev-parse",
        &["--abbrev-ref".to_string(), "HEAD".to_string()],
    )
    .expect("SafeGitRunner failed");
    assert!(branch_out.status.success());
    assert_eq!(String::from_utf8_lossy(&branch_out.stdout).trim(), "main");
}
