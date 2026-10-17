//! Terminal escape hatch: suspend TUI, launch external editor/tool, and resume gracefully.

use crossterm::{
    cursor::{Hide, Show},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{self, stdout, Write};
use std::path::Path;
use std::process::Command;

/// Open an external editor targeting a file by temporarily suspending raw TUI mode.
/// Follows resolution order: Config preference -> $VISUAL -> $EDITOR -> OS default.
pub fn open_external_editor(file_path: &Path) -> io::Result<()> {
    open_external_editor_with_preference(file_path, None)
}

/// Open external editor with optional explicit preference from config.
pub fn open_external_editor_with_preference(
    file_path: &Path,
    preferred_cmd: Option<&str>,
) -> io::Result<()> {
    let editor = preferred_cmd
        .map(|s| s.to_string())
        .or_else(|| std::env::var("VISUAL").ok())
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "nano".to_string()
            }
        });

    let tokens = crate::services::SafeGitRunner::tokenize_args(&editor);
    if tokens.is_empty() {
        return Err(io::Error::other("Editor command is empty"));
    }
    let cmd = &tokens[0];
    let mut args: Vec<&str> = tokens[1..].iter().map(|s| s.as_str()).collect();
    let file_str = file_path.to_string_lossy();
    args.push(file_str.as_ref());

    run_interactive_command(cmd, &args)
}

/// Reveal a file or directory in the system file manager (e.g. xdg-open, open, explorer).
pub fn reveal_in_file_manager(path: &Path) -> io::Result<()> {
    let target = if path.is_dir() {
        path.to_path_buf()
    } else if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            parent.to_path_buf()
        }
    } else {
        std::path::PathBuf::from(".")
    };

    let target_str = target.to_str().unwrap_or(".");

    let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("explorer", vec![target_str])
    } else if cfg!(target_os = "macos") {
        ("open", vec![target_str])
    } else {
        ("xdg-open", vec![target_str])
    };

    Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| io::Error::other(format!("Failed to open file manager with '{}': {}", cmd, e)))
}

/// Execute an interactive command that needs terminal control by temporarily leaving raw TUI mode.
pub fn run_interactive_command(command: &str, args: &[&str]) -> io::Result<()> {
    // 1. Leave TUI & restore cooked terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show)?;
    let _ = stdout().flush();

    // 2. Run child process with inherited standard streams
    let status = Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    // 3. Re-enter TUI alternate screen & enable raw mode
    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        Hide,
        Clear(ClearType::All)
    )?;
    let _ = stdout().flush();

    match status {
        Ok(s) => {
            if s.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "Command '{}' exited with code {:?}",
                    command,
                    s.code()
                )))
            }
        }
        Err(e) => Err(io::Error::other(format!(
            "Failed to execute command '{}': {}",
            command, e
        ))),
    }
}
