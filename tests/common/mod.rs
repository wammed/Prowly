//! Shared test fixtures and helpers for Git integration testing.

use prowly::git::GitService;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[allow(dead_code)]
pub struct GitTestSandbox {
    pub remote_dir: Option<TempDir>,
    pub work_dir: TempDir,
}

#[allow(dead_code)]
impl GitTestSandbox {
    /// Initialize a new isolated sandbox with a bare remote repository and a cloned working tree.
    pub fn new() -> Self {
        let remote_dir = TempDir::new().expect("Failed to create remote temp dir");
        let work_dir = TempDir::new().expect("Failed to create work temp dir");

        // 1. Initialize bare remote with main branch
        let status = Command::new("git")
            .current_dir(remote_dir.path())
            .args(["init", "--bare", "-b", "main"])
            .output()
            .expect("Failed to init bare remote");
        assert!(status.status.success(), "git init --bare failed");

        let remote_path_str = remote_dir
            .path()
            .to_str()
            .expect("Valid UTF-8 path")
            .to_string();

        // 2. Initialize working repository
        let sandbox = Self {
            remote_dir: Some(remote_dir),
            work_dir,
        };

        sandbox.git_success(&["init", "-b", "main"]);
        sandbox.git_success(&["config", "user.name", "Prowly Tester"]);
        sandbox.git_success(&["config", "user.email", "tester@prowly.local"]);
        sandbox.git_success(&["remote", "add", "origin", &remote_path_str]);

        // 3. Initial commit and push
        sandbox.write_file("README.md", "# Test Repo\nInitial content\n");
        sandbox.git_success(&["add", "README.md"]);
        sandbox.git_success(&["commit", "-m", "chore: initial commit"]);
        sandbox.git_success(&["push", "-u", "origin", "main"]);

        sandbox
    }

    /// Clone another independent working repository from the same remote (for multi-client tests).
    pub fn clone_client(&self) -> GitTestSandbox {
        let remote = self
            .remote_dir
            .as_ref()
            .expect("Remote dir must exist to clone a client");
        let remote_path_str = remote.path().to_str().expect("Valid UTF-8 path");

        let work_dir = TempDir::new().expect("Failed to create client work temp dir");
        let work_path_str = work_dir.path().to_str().expect("Valid UTF-8 path");

        let output = Command::new("git")
            .args(["clone", remote_path_str, work_path_str])
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to clone client");
        assert!(output.status.success(), "git clone failed: {:?}", output);

        let client = GitTestSandbox {
            remote_dir: None,
            work_dir,
        };
        client.git_success(&["config", "user.name", "Second Client"]);
        client.git_success(&["config", "user.email", "client2@prowly.local"]);

        client
    }

    /// Execute a git command in this sandbox, asserting that it succeeds.
    pub fn git_success(&self, args: &[&str]) -> String {
        let output = self.git_exec(args);
        assert!(
            output.status.success(),
            "Git command failed: git {}\nStderr: {}\nStdout: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Execute a git command returning raw Output (allowing assertions on failure/stderr).
    pub fn git_exec(&self, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .current_dir(self.work_dir.path())
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()
            .expect("Failed to execute git")
    }

    /// Write text to a file inside the working directory.
    pub fn write_file(&self, rel_path: &str, content: &str) {
        let target = self.work_dir.path().join(rel_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dirs");
        }
        fs::write(target, content).expect("Failed to write file");
    }

    /// Read text from a file inside the working directory.
    pub fn read_file(&self, rel_path: &str) -> String {
        let target = self.work_dir.path().join(rel_path);
        fs::read_to_string(target).expect("Failed to read file")
    }

    /// Return path to the working directory.
    pub fn path(&self) -> &Path {
        self.work_dir.path()
    }

    /// Open a Prowly GitService connected to this working directory.
    pub fn service(&self) -> GitService {
        GitService::open(self.path()).expect("Failed to open GitService on sandbox")
    }
}
