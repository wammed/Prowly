//! OS Clipboard integration service via arboard.

use arboard::Clipboard;
use std::sync::Mutex;

/// Thread-safe clipboard manager with in-memory fallback for headless environments.
pub struct ClipboardService {
    clipboard: Mutex<Option<Clipboard>>,
    fallback: Mutex<String>,
}

impl ClipboardService {
    /// Create a new clipboard service instance.
    pub fn new() -> Self {
        let cb = Clipboard::new().ok();
        Self {
            clipboard: Mutex::new(cb),
            fallback: Mutex::new(String::new()),
        }
    }

    /// Copy text to the OS system clipboard (with in-memory fallback if headless).
    pub fn copy_text(&self, text: &str) -> Result<(), String> {
        if let Ok(mut fb) = self.fallback.lock() {
            *fb = text.to_string();
        }
        let mut guard = self.clipboard.lock().map_err(|e| e.to_string())?;
        if let Some(cb) = guard.as_mut() {
            if cb.set_text(text).is_ok() {
                return Ok(());
            }
        }
        // Attempt to re-initialize clipboard if it wasn't available at startup
        match Clipboard::new() {
            Ok(mut cb) => {
                let _ = cb.set_text(text);
                *guard = Some(cb);
                Ok(())
            }
            Err(_) => {
                // In headless Linux environments (CI, SSH without X11 forwarding),
                // fall back to the in-memory buffer so clipboard operations succeed.
                Ok(())
            }
        }
    }

    /// Retrieve text from the OS system clipboard.
    pub fn get_text(&self) -> Result<String, String> {
        let mut guard = self.clipboard.lock().map_err(|e| e.to_string())?;
        if let Some(cb) = guard.as_mut() {
            if let Ok(text) = cb.get_text() {
                return Ok(text);
            }
        }
        if let Ok(fb) = self.fallback.lock() {
            if !fb.is_empty() {
                return Ok(fb.clone());
            }
        }
        Err("Clipboard unavailable".to_string())
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}
