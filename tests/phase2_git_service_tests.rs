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


