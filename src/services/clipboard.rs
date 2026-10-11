//! OS Clipboard integration service via arboard.

use arboard::Clipboard;
use std::sync::Mutex;

/// Thread-safe clipboard manager.
pub struct ClipboardService {
    clipboard: Mutex<Option<Clipboard>>,
}

impl ClipboardService {
    /// Create a new clipboard service instance.
    pub fn new() -> Self {
        let cb = Clipboard::new().ok();
        Self {
            clipboard: Mutex::new(cb),
        }
    }

    /// Copy text to the OS system clipboard.
    pub fn copy_text(&self, text: &str) -> Result<(), String> {
        let mut guard = self.clipboard.lock().map_err(|e| e.to_string())?;
        if let Some(cb) = guard.as_mut() {
            cb.set_text(text).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            // Attempt to re-initialize clipboard if it wasn't available at startup
            match Clipboard::new() {
                Ok(mut cb) => {
                    cb.set_text(text).map_err(|e| e.to_string())?;
                    *guard = Some(cb);
                    Ok(())
                }
                Err(e) => Err(format!("Clipboard unavailable: {}", e)),
            }
        }
    }

    /// Retrieve text from the OS system clipboard.
    pub fn get_text(&self) -> Result<String, String> {
        let mut guard = self.clipboard.lock().map_err(|e| e.to_string())?;
        if let Some(cb) = guard.as_mut() {
            cb.get_text().map_err(|e| e.to_string())
        } else {
            Err("Clipboard unavailable".to_string())
        }
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}
