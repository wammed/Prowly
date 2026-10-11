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
use std::io::{self, stdout};
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

    run_interactive_command(&editor, &[file_path.to_string_lossy().as_ref()])
}

/// Reveal a file or directory in the system file manager (e.g. xdg-open, open, explorer).
pub fn reveal_in_file_manager(path: &Path) -> io::Result<()> {
    let target = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    let (cmd, args): (&str, Vec<&str>) = if cfg!(windows) {
        ("explorer", vec![target.to_str().unwrap_or(".")])
    } else if cfg!(target_os = "macos") {
        ("open", vec![target.to_str().unwrap_or(".")])
    } else {
        ("xdg-open", vec![target.to_str().unwrap_or(".")])
    };

    Command::new(cmd)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| io::Error::other(format!("Failed to open file manager with '{}': {}", cmd, e)))
}

/// Execute an interactive command that needs terminal control by temporarily leaving raw TUI mode.
pub fn run_interactive_command(command: &str, args: &[&str]) -> io::Result<()> {
    // 1. Leave TUI & restore cooked terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show)?;

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
