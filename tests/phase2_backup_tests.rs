use prowly::git::GitService;
use prowly::services::BackupService;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_backup_service_direct() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();

    let backup_service = BackupService::with_root(backup_dir.path().to_path_buf());
    let target_file = temp_dir.path().join("code.rs");

    // Write original contents
    fs::write(&target_file, "fn original() {}\n").unwrap();

    // Create backup
    let record = backup_service.backup_file(&target_file).unwrap();
    assert!(record.backup_path.exists());

    // Modify file (simulating edits that would be discarded)
    fs::write(&target_file, "fn modified() {}\n").unwrap();
    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        "fn modified() {}\n"
    );

    // Restore backup
    backup_service.restore_backup(&record).unwrap();
    assert_eq!(
        fs::read_to_string(&target_file).unwrap(),
        "fn original() {}\n"
    );
}

#[test]
fn test_soft_discard_via_git_service() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = TempDir::new().unwrap();

    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).unwrap();

    // User setup
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Tester").unwrap();
    config.set_str("user.email", "tester@test.com").unwrap();

    // Initial commit
    let test_file = repo_path.join("file.txt");
    fs::write(&test_file, "Version 1\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file.txt")).unwrap();
    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Init", &tree, &[])
        .unwrap();

    let git = GitService::open(repo_path).unwrap();
    let backup_service = BackupService::with_root(backup_dir.path().to_path_buf());

    // Edit file to Version 2
    fs::write(&test_file, "Version 2 (Discard me)\n").unwrap();

    // Perform soft discard
    let record = git.soft_discard("file.txt", &backup_service).unwrap();

    // Worktree file should be reverted to Version 1
    assert_eq!(fs::read_to_string(&test_file).unwrap(), "Version 1\n");

    // The backup should hold Version 2
    let backed_up_content = fs::read_to_string(&record.backup_path).unwrap();
    assert_eq!(backed_up_content, "Version 2 (Discard me)\n");
}
