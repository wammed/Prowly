use prowly::git::{FileStatus, GitService};
use std::fs;
use tempfile::TempDir;

fn setup_temp_repo() -> (TempDir, GitService) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Initialize repo via libgit2
    let repo = git2::Repository::init(repo_path).unwrap();

    // Configure test user for commits
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Prowly Tester").unwrap();
    config.set_str("user.email", "tester@prowly.local").unwrap();

    // Create an initial commit
    let init_file = repo_path.join("README.md");
    fs::write(&init_file, "# Test Repo\nInitial line\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    let tree_oid = index.write_tree().unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    let service = GitService::open(repo_path).unwrap();
    (temp_dir, service)
}

#[test]
fn test_git_status_and_staging_lifecycle() {
    let (_temp_dir, git) = setup_temp_repo();

    // 1. Initial status should be clean
    let status = git.get_status().unwrap();
    assert!(status.files.is_empty());

    // 2. Modify existing file
    let readme_path = git.workdir().join("README.md");
    fs::write(&readme_path, "# Test Repo\nInitial line\nSecond line\n").unwrap();

    let status = git.get_status().unwrap();
    assert_eq!(status.files.len(), 1);
    assert_eq!(status.files[0].path, "README.md");
    assert_eq!(status.files[0].status, FileStatus::Modified);
    assert!(!status.files[0].staged);

    // 3. Stage the modified file
    git.stage_file("README.md").unwrap();
    let status = git.get_status().unwrap();
    assert_eq!(status.files.len(), 1);
    assert!(status.files[0].staged);

    // 4. Unstage the file
    git.unstage_file("README.md").unwrap();
    let status = git.get_status().unwrap();
    assert_eq!(status.files.len(), 1);
    assert!(!status.files[0].staged);

    // 5. Add a new untracked file
    let new_file = git.workdir().join("src/lib.rs");
    fs::create_dir_all(new_file.parent().unwrap()).unwrap();
    fs::write(&new_file, "pub fn hello() {}\n").unwrap();

    let status = git.get_status().unwrap();
    assert_eq!(status.files.len(), 2);

    // 6. Stage all
    git.stage_all().unwrap();
    let status = git.get_status().unwrap();
    assert_eq!(status.files.len(), 2);
    assert!(status.files.iter().all(|f| f.staged));

    // 7. Commit changes
    let oid = git.commit("feat: add second line and lib.rs").unwrap();
    assert!(!oid.to_string().is_empty());

    // Status should be clean after commit
    let status = git.get_status().unwrap();
    assert!(status.files.is_empty());
}

#[test]
fn test_git_diff_generation() {
    let (_temp_dir, git) = setup_temp_repo();

    let readme_path = git.workdir().join("README.md");
    fs::write(&readme_path, "# Test Repo\nNew Line Added\n").unwrap();

    // Query unstaged diff
    let diff = git.get_diff(Some("README.md"), false).unwrap();
    assert_eq!(diff.file_path, "README.md");
    assert!(!diff.hunks.is_empty());
    assert!(diff.stats_added > 0 || diff.stats_deleted > 0);
}

#[test]
fn test_git_cli_push_and_ahead_status() {
    let temp_dir = TempDir::new().unwrap();
    let remote_dir = TempDir::new().unwrap();

    // 1. Create bare remote repo
    git2::Repository::init_bare(remote_dir.path()).unwrap();

    // 2. Init local repo
    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.local").unwrap();

    // Commit a file
    let file = repo_path.join("file.txt");
    fs::write(&file, "hello").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    let tree_oid = index.write_tree().unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    // Add remote
    repo.remote("origin", remote_dir.path().to_str().unwrap()).unwrap();

    let git = GitService::open(repo_path).unwrap();

    // First push via CLI and set upstream
    std::process::Command::new("git")
        .current_dir(repo_path)
        .args(&["push", "-u", "origin", "main"])
        .output()
        .unwrap();

    let status = git.get_status().unwrap();
    println!("Status after initial push: ahead={}, behind={}", status.ahead, status.behind);

    // Make new commit
    fs::write(&file, "hello 2").unwrap();
    git.stage_file("file.txt").unwrap();
    git.commit("second commit").unwrap();

    let status_before_push = git.get_status().unwrap();
    println!("Status before push: ahead={}, behind={}", status_before_push.ahead, status_before_push.behind);
    assert_eq!(status_before_push.ahead, 1);

    // Now call cli_push()
    let push_res = git.cli_push();
    assert!(push_res.is_ok());

    let status_after_push = git.get_status().unwrap();
    assert_eq!(status_after_push.ahead, 0);

    // Verify empty commit prevention
    let empty_commit_res = git.commit("should fail empty commit");
    assert!(empty_commit_res.is_err());
}

#[test]
fn test_git_cli_push_auto_upstream_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let remote_dir = TempDir::new().unwrap();

    // 1. Create bare remote repo
    git2::Repository::init_bare(remote_dir.path()).unwrap();

    // 2. Init local repo
    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.local").unwrap();

    // Commit a file
    let file = repo_path.join("file.txt");
    fs::write(&file, "hello branch").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    let tree_oid = index.write_tree().unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

    // Add remote
    repo.remote("origin", remote_dir.path().to_str().unwrap()).unwrap();

    let git = GitService::open(repo_path).unwrap();

    // Call cli_push() without upstream configured! Should auto fallback to -u origin <branch>
    let push_res = git.cli_push();
    assert!(push_res.is_ok(), "Push without upstream should succeed via auto-fallback: {:?}", push_res);

    let status = git.get_status().unwrap();
    assert_eq!(status.ahead, 0);
}

#[test]
fn test_git_ahead_and_behind_sync() {
    let remote_dir = TempDir::new().unwrap();
    let local1_dir = TempDir::new().unwrap();
    let local2_dir = TempDir::new().unwrap();

    // 1. Bare remote
    git2::Repository::init_bare(remote_dir.path()).unwrap();

    // 2. Local 1 init and push initial commit
    let repo1 = git2::Repository::init(local1_dir.path()).unwrap();
    let mut config1 = repo1.config().unwrap();
    config1.set_str("user.name", "Tester1").unwrap();
    config1.set_str("user.email", "tester1@test.local").unwrap();
    fs::write(local1_dir.path().join("base.txt"), "base content\n").unwrap();
    let mut index1 = repo1.index().unwrap();
    index1.add_path(std::path::Path::new("base.txt")).unwrap();
    let tree_oid1 = index1.write_tree().unwrap();
    index1.write().unwrap();
    let tree1 = repo1.find_tree(tree_oid1).unwrap();
    let sig1 = repo1.signature().unwrap();
    repo1.commit(Some("HEAD"), &sig1, &sig1, "Base commit", &tree1, &[]).unwrap();
    repo1.remote("origin", remote_dir.path().to_str().unwrap()).unwrap();

    let git1 = GitService::open(local1_dir.path()).unwrap();
    git1.cli_push().unwrap();

    // 3. Local 2 clones from remote, adds a remote commit, and pushes
    let repo2 = git2::Repository::clone(remote_dir.path().to_str().unwrap(), local2_dir.path()).unwrap();
    let mut config2 = repo2.config().unwrap();
    config2.set_str("user.name", "Tester2").unwrap();
    config2.set_str("user.email", "tester2@test.local").unwrap();
    let git2 = GitService::open(local2_dir.path()).unwrap();
    fs::write(local2_dir.path().join("from_tester2.txt"), "tester2\n").unwrap();
    git2.stage_all().unwrap();
    git2.commit("Commit from remote user").unwrap();
    git2.cli_push().unwrap();

    // 4. Local 1 creates a local commit without pushing (should be ahead: 1)
    fs::write(local1_dir.path().join("local_only.txt"), "local commit\n").unwrap();
    git1.stage_all().unwrap();
    git1.commit("Local unpushed commit").unwrap();

    let status_before_fetch = git1.get_status().unwrap();
    assert_eq!(status_before_fetch.ahead, 1);
    // Before fetch, local1's refs/remotes/origin does not have tester2's commit yet, so behind is 0
    assert_eq!(status_before_fetch.behind, 0);

    // 5. Run cli_fetch() in Local 1
    let fetch_res = git1.cli_fetch();
    assert!(fetch_res.is_ok(), "Fetch should succeed: {:?}", fetch_res);

    // 6. After fetch, local 1 should reflect ahead: 1, behind: 1
    let status_after_fetch = git1.get_status().unwrap();
    assert_eq!(status_after_fetch.ahead, 1);
    assert_eq!(status_after_fetch.behind, 1);

    // 7. Verify unpushed and unpulled commit subjects
    let (unpushed, unpulled) = git1.get_unpushed_unpulled_commits().unwrap();
    assert_eq!(unpushed.len(), 1);
    assert!(unpushed[0].contains("Local unpushed commit"));
    assert_eq!(unpulled.len(), 1);
    assert!(unpulled[0].contains("Commit from remote user"));
}

#[test]
fn test_external_cli_commit_detection() {
    let remote_dir = TempDir::new().unwrap();
    let local_dir = TempDir::new().unwrap();

    git2::Repository::init_bare(remote_dir.path()).unwrap();

    let repo = git2::Repository::init(local_dir.path()).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.local").unwrap();
    fs::write(local_dir.path().join("base.txt"), "base\n").unwrap();
    let git = GitService::open(local_dir.path()).unwrap();
    git.stage_all().unwrap();
    git.commit("Initial commit").unwrap();
    repo.remote("origin", remote_dir.path().to_str().unwrap()).unwrap();
    git.cli_push().unwrap();

    assert_eq!(git.get_status().unwrap().ahead, 0);

    // Commit via external git CLI command!
    fs::write(local_dir.path().join("external.txt"), "external\n").unwrap();
    let _ = std::process::Command::new("git")
        .current_dir(local_dir.path())
        .args(["add", "."])
        .output()
        .unwrap();
    let _ = std::process::Command::new("git")
        .current_dir(local_dir.path())
        .args(["commit", "-m", "External commit from terminal"])
        .output()
        .unwrap();

    let status = git.get_status().unwrap();
    assert_eq!(status.ahead, 1, "Status should detect commit made by external git CLI");
}

#[test]
fn test_cli_pull_updates_local_branch() {
    let remote_dir = TempDir::new().unwrap();
    let local1_dir = TempDir::new().unwrap();
    let local2_dir = TempDir::new().unwrap();

    git2::Repository::init_bare(remote_dir.path()).unwrap();

    let repo1 = git2::Repository::init(local1_dir.path()).unwrap();
    let mut config1 = repo1.config().unwrap();
    config1.set_str("user.name", "Tester1").unwrap();
    config1.set_str("user.email", "tester1@test.local").unwrap();
    fs::write(local1_dir.path().join("base.txt"), "base\n").unwrap();
    let git1 = GitService::open(local1_dir.path()).unwrap();
    git1.stage_all().unwrap();
    git1.commit("Initial commit").unwrap();
    repo1.remote("origin", remote_dir.path().to_str().unwrap()).unwrap();
    git1.cli_push().unwrap();

    // Local 2 clones, creates commit, and pushes
    let repo2 = git2::Repository::clone(remote_dir.path().to_str().unwrap(), local2_dir.path()).unwrap();
    let mut config2 = repo2.config().unwrap();
    config2.set_str("user.name", "Tester2").unwrap();
    config2.set_str("user.email", "tester2@test.local").unwrap();
    fs::write(local2_dir.path().join("file2.txt"), "content2\n").unwrap();
    let git2 = GitService::open(local2_dir.path()).unwrap();
    git2.stage_all().unwrap();
    git2.commit("Commit from user 2").unwrap();
    git2.cli_push().unwrap();

    // Local 1 fetches and sees behind: 1
    git1.cli_fetch().unwrap();
    let status_before = git1.get_status().unwrap();
    assert_eq!(status_before.behind, 1);

    // Local 1 pulls
    let pull_res = git1.cli_pull();
    assert!(pull_res.is_ok(), "Pull should succeed: {:?}", pull_res);

    // After pull, behind should be 0, and file2.txt should exist locally
    let status_after = git1.get_status().unwrap();
    assert_eq!(status_after.behind, 0);
    assert!(local1_dir.path().join("file2.txt").exists());
}





