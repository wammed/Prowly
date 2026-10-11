use prowly::services::ClipboardService;

#[test]
fn test_clipboard_service_creation() {
    let cb = ClipboardService::new();
    // In headless Linux CI without X11 or Wayland, copy_text might return Err,
    // but the service must never panic and must gracefully handle either condition.
    match cb.copy_text("git checkout -b feat/tui") {
        Ok(()) => {
            if let Ok(retrieved) = cb.get_text() {
                assert_eq!(retrieved, "git checkout -b feat/tui");
            }
        }
        Err(err) => {
            assert!(!err.is_empty());
        }
    }
}
