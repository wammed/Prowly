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
pub fn open_external_editor(file_path: &Path) -> io::Result<()> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "nano".to_string()
            }
        });

    // 1. Leave TUI & restore cooked terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture, Show)?;

    // 2. Run editor process
    let status = Command::new(&editor).arg(file_path).status();

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
        Ok(_) => Ok(()),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to execute external editor '{}': {}", editor, e),
        )),
    }
}
