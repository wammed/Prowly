use prowly::git::GitService;
use prowly::services::git_runner::SafeGitRunner;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_ai_command_security_deny_list() {
    // Prohibited: rebase --exec
    assert!(SafeGitRunner::parse_ai_command("git rebase --exec 'touch /tmp/pwn'").is_err());
    assert!(SafeGitRunner::parse_ai_command("git rebase -x 'ls'").is_err());
    assert!(SafeGitRunner::parse_ai_command("git rebase --exec=whoami").is_err());

    // Prohibited: arbitrary file write
    assert!(SafeGitRunner::parse_ai_command("git diff --output=/etc/passwd").is_err());
    assert!(SafeGitRunner::parse_ai_command("git log --output=~/.bashrc").is_err());

    // Prohibited: custom pack helpers
    assert!(SafeGitRunner::parse_ai_command("git fetch --upload-pack=evil origin").is_err());
    assert!(SafeGitRunner::parse_ai_command("git push --receive-pack=evil origin").is_err());

    // Prohibited: external diff driver
    assert!(SafeGitRunner::parse_ai_command("git diff --ext-diff").is_err());

    // Safe commands are allowed
    assert!(SafeGitRunner::parse_ai_command("git status").is_ok());
    assert!(SafeGitRunner::parse_ai_command("git commit -m 'feat: test'").is_ok());
    assert!(SafeGitRunner::parse_ai_command("git branch -a").is_ok());
}

#[test]
fn test_commit_executes_pre_commit_hook() {
    let dir = tempdir().unwrap();
    let p = dir.path();

    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(p)
        .output()
        .unwrap();

    // Create an initial commit
    fs::write(p.join("init.txt"), "init\n").unwrap();
    Command::new("git")
        .args(["add", "init.txt"])
        .current_dir(p)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(p)
        .output()
        .unwrap();

    // Install a rejecting pre-commit hook
    let hooks_dir = p.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir).unwrap();
    let hook_file = hooks_dir.join("pre-commit");
    fs::write(
        &hook_file,
        "#!/bin/sh\necho 'pre-commit hook blocked'\nexit 1\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook_file, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let git = GitService::open(p).unwrap();
    fs::write(p.join("change.txt"), "hello\n").unwrap();
    git.stage_file("change.txt").unwrap();

    // The commit MUST be rejected by the pre-commit hook!
    let res = git.commit("attempt blocked commit");
    assert!(
        res.is_err(),
        "Commit should fail because pre-commit hook exits 1"
    );
    let err_msg = format!("{:?}", res.err());
    assert!(err_msg.contains("pre-commit hook blocked"));
}
