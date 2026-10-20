//! Comprehensive matrix test suite covering all implemented and cataloged Git subcommands.

mod common;
use common::GitTestSandbox;
use prowly::git::{reword_head_commit, RepoOperationState};
use prowly::services::SafeGitRunner;
use prowly::ui::modals::GitSubcommandDef;
use std::collections::HashSet;
use std::process::Command;

#[test]
fn test_matrix_catalog_all_subcommands_syntax_and_safety() {
    let all_subs = GitSubcommandDef::ALL;
    assert!(
        all_subs.len() >= 50,
        "Subcommand catalog should have extensive coverage (currently {})",
        all_subs.len()
    );

    let mut seen_names = HashSet::new();

    for sub in all_subs {
        // 1. Uniqueness
        assert!(
            seen_names.insert(sub.name),
            "Duplicate subcommand in catalog: '{}'",
            sub.name
        );

        // 2. Naming safety (no spaces, shell operators, only alphanumeric + hyphen)
        assert!(
            SafeGitRunner::is_safe_subcommand(sub.name),
            "Subcommand name '{}' failed is_safe_subcommand check",
            sub.name
        );
        assert_eq!(
            sub.name,
            sub.name.to_lowercase(),
            "Subcommand '{}' should be lowercase",
            sub.name
        );

        // 3. Category validity
        assert!(
            matches!(sub.category, "Main" | "Ancillary" | "Plumbing"),
            "Invalid category '{}' for subcommand '{}'",
            sub.category,
            sub.name
        );

        // 4. Description presence
        assert!(
            !sub.description.trim().is_empty(),
            "Subcommand '{}' missing description",
            sub.name
        );

        // 5. Flags syntax validation (no unclosed quotes or trailing escapes)
        for flag in sub.common_flags {
            assert!(
                SafeGitRunner::tokenize_args_strict(flag).is_ok(),
                "Subcommand '{}' has invalid flag syntax: '{}'",
                sub.name,
                flag
            );
        }

        // 6. Subcommand lookup methods consistency
        assert_eq!(
            GitSubcommandDef::find(sub.name).map(|s| s.name),
            Some(sub.name)
        );
        assert!(GitSubcommandDef::is_known_subcommand(sub.name));
    }
}

#[test]
fn test_matrix_all_subcommands_recognized_by_git_binary() {
    // Retrieve system Git supported commands
    let output = Command::new("git")
        .args(["--list-cmds=main,others"])
        .output()
        .expect("Failed to execute git --list-cmds");
    let system_cmds: HashSet<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // Optional subcommands that may be split into separate packages by some distributions
    // (e.g. git-email for 'send-email') and may not be installed on headless minimal environments.
    const OPTIONAL_DISTRO_SUBCOMMANDS: &[&str] = &["send-email"];

    // Verify each cataloged command in Prowly is recognized by the system Git
    for sub in GitSubcommandDef::ALL {
        if system_cmds.contains(sub.name) {
            continue;
        }

        // Fallback check via git -h or git help
        let h_out = Command::new("git").args([sub.name, "-h"]).output();
        if let Ok(out) = h_out {
            let code = out.status.code().unwrap_or(-1);
            // 0 (success) or 129 (usage) means recognized git command
            if code == 0 || code == 129 {
                continue;
            }
        }

        // Gracefully allow known optional distribution-modular subcommands
        if OPTIONAL_DISTRO_SUBCOMMANDS.contains(&sub.name) {
            continue;
        }

        panic!(
            "Cataloged subcommand '{}' is not recognized by system Git binary",
            sub.name
        );
    }
}

#[test]
fn test_matrix_safety_guard_destructive_flags_coverage() {
    // Exhaustive matrix of destructive commands and flags that MUST be guarded
    let destructive_matrix = [
        // Force Push variants
        ("push", "--force"),
        ("push", "-f"),
        ("push", "-f origin main"),
        ("push", "--force-with-lease"),
        ("push", "--force-with-lease=main"),
        ("push", "--force-if-includes"),
        // Hard / Merge Reset variants
        ("reset", "--hard"),
        ("reset", "--hard HEAD~1"),
        ("reset", "--hard origin/main"),
        ("reset", "--merge"),
        ("reset", "--keep"),
        // Clean force variants
        ("clean", "-f"),
        ("clean", "-fd"),
        ("clean", "-fdx"),
        ("clean", "-f -d -x"),
        ("clean", "--force"),
        // Branch force delete variants
        ("branch", "-D feature"),
        ("branch", "-D old-branch"),
        ("branch", "-d feature --force"),
        // Checkout force variants
        ("checkout", "-f"),
        ("checkout", "--force"),
        ("checkout", "-f other-branch"),
        ("checkout", "-B new-branch"),
        ("checkout", "-C new-branch"),
        // Switch force variants
        ("switch", "-f"),
        ("switch", "--discard-changes"),
        ("switch", "-C other"),
        // Restore dangerous variants (touching worktree with source)
        ("restore", "-w -s HEAD~1 file.rs"),
        ("restore", "--worktree --source=HEAD~1 ."),
        ("restore", "--staged --worktree ."),
    ];

    for (sub, args) in destructive_matrix {
        let warning = SafeGitRunner::check_dangerous_git_str(sub, args);
        assert!(
            warning.is_some(),
            "SAFETY VIOLATION: Command 'git {} {}' was NOT flagged by check_dangerous_git_str",
            sub,
            args
        );
    }

    // Verify safe variants are NOT flagged
    let safe_matrix = [
        ("push", "origin main"),
        ("push", "-u origin feature"),
        ("reset", "--soft HEAD~1"),
        ("reset", "HEAD~1"),
        ("clean", "-n -d"),
        ("clean", "--dry-run --force"),
        ("branch", "-d merged-feature"),
        ("branch", "-a"),
        ("checkout", "-b new-feature"),
        ("checkout", "main"),
        ("switch", "main"),
        ("switch", "-c new-feature"),
        ("restore", "--staged file.rs"),
    ];

    for (sub, args) in safe_matrix {
        let warning = SafeGitRunner::check_dangerous_git_str(sub, args);
        assert!(
            warning.is_none(),
            "FALSE POSITIVE: Safe command 'git {} {}' was unexpectedly flagged: {:?}",
            sub,
            args,
            warning
        );
    }
}

#[test]
fn test_matrix_safe_subcommand_dryrun_dispatch() {
    let sandbox = GitTestSandbox::new();

    // Matrix of safe read-only or dry-run commands to execute inside sandbox
    let commands_to_test: &[(&str, &[&str])] = &[
        ("status", &["--porcelain"]),
        ("log", &["-n", "1", "--oneline"]),
        ("diff", &["HEAD"]),
        ("branch", &["--list"]),
        ("tag", &["--list"]),
        ("remote", &["-v"]),
        ("rev-parse", &["HEAD"]),
        ("show", &["--stat", "HEAD"]),
        ("stash", &["list"]),
        ("clean", &["-n"]),
        ("add", &["-n", "."]),
        ("fetch", &["--dry-run"]),
        ("archive", &["--list"]),
        ("check-ignore", &["-v", "-n", "README.md"]),
        ("check-ref-format", &["refs/heads/main"]),
        ("config", &["--list"]),
        ("describe", &["--always"]),
        ("shortlog", &["-s", "HEAD"]),
        ("var", &["GIT_AUTHOR_IDENT"]),
        ("version", &["--build-options"]),
    ];

    for (subcommand, args) in commands_to_test {
        let string_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let output = SafeGitRunner::execute_git(sandbox.path(), subcommand, &string_args);
        assert!(
            output.is_ok(),
            "SafeGitRunner failed to spawn git {} {:?}",
            subcommand,
            args
        );
        let out = output.unwrap();
        let code = out.status.code().unwrap_or(-1);
        let is_ok = if *subcommand == "check-ignore" {
            code == 0 || code == 1
        } else {
            out.status.success()
        };
        assert!(
            is_ok,
            "git {} {:?} exited with error:\nStderr: {}\nStdout: {}",
            subcommand,
            args,
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        );
    }
}

#[test]
fn test_matrix_full_porcelain_lifecycle() {
    let sandbox = GitTestSandbox::new();
    let service = sandbox.service();

    // 1. Status clean
    let status = service.get_status().unwrap();
    assert_eq!(status.operation_state, RepoOperationState::None);
    assert!(status.files.is_empty());

    // 2. Stage single file
    sandbox.write_file("file1.txt", "line 1\n");
    service.stage_file("file1.txt").unwrap();
    let status = service.get_status().unwrap();
    assert_eq!(status.files.len(), 1);
    assert!(status.files[0].staged);

    // 3. Unstage single file
    service.unstage_file("file1.txt").unwrap();
    let status = service.get_status().unwrap();
    assert_eq!(status.files.len(), 1);
    assert!(!status.files[0].staged);

    // 4. Stage all files
    sandbox.write_file("file2.txt", "line 2\n");
    service.stage_all().unwrap();
    let status = service.get_status().unwrap();
    assert_eq!(status.files.len(), 2);
    assert!(status.files.iter().all(|f| f.staged));

    // 5. Unstage all files
    service.unstage_all().unwrap();
    let status = service.get_status().unwrap();
    assert_eq!(status.files.len(), 2);
    assert!(status.files.iter().all(|f| !f.staged));

    // 6. Commit staged files
    service.stage_file("file1.txt").unwrap();
    sandbox.git_success(&["commit", "-m", "feat: commit file 1"]);

    // 7. Reword head commit
    reword_head_commit(sandbox.path(), "feat: reworded commit message").unwrap();
    let log_msg = sandbox.git_success(&["log", "-1", "--pretty=%s"]);
    assert_eq!(log_msg, "feat: reworded commit message");

    // 8. Remote operations: Push, Fetch, Pull
    service.cli_push().unwrap();
    service.cli_fetch().unwrap();
    service.cli_pull().unwrap();

    // 9. Stash Save and Pop (modify tracked file)
    sandbox.write_file("file1.txt", "uncommitted stash modification\n");
    let save_msg = service.cli_stash_save("test stash msg").unwrap();
    assert!(!save_msg.is_empty());
    assert!(service.cli_stash_pop().is_ok());
    assert_eq!(
        sandbox.read_file("file1.txt"),
        "uncommitted stash modification\n"
    );

    // 10. Operation State & Abort on Rebase
    let client2 = sandbox.clone_client();
    let service2 = client2.service();
    sandbox.write_file("file1.txt", "conflict A\n");
    sandbox.git_success(&["commit", "-am", "conflict A"]);
    service.cli_push().unwrap();

    client2.write_file("file1.txt", "conflict B\n");
    client2.git_success(&["commit", "-am", "conflict B"]);
    client2.git_success(&["fetch", "origin"]);
    let _ = client2.git_exec(&["rebase", "origin/main"]);

    assert_eq!(
        service2.get_operation_state(),
        RepoOperationState::RebaseInProgress
    );
    assert!(service2.abort_current_operation().is_ok());
    assert_eq!(service2.get_operation_state(), RepoOperationState::None);
}
