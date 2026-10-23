//! Services provided by the TUI crate, re-exporting headless core services.

pub use prowly_core::services::*;

pub mod terminal;
pub use terminal::{
    open_external_editor, open_external_editor_with_preference, reveal_in_file_manager,
    run_interactive_command,
};
