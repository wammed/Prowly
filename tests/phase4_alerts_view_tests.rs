use prowly::github::AlertItem;
use prowly::ui::views::AlertsView;

#[test]
fn test_alerts_view_read_lifecycle() {
    let mut view = AlertsView::new();

    let alerts = vec![
        AlertItem {
            id: "1".to_string(),
            title: "Mentioned in #42".to_string(),
            repository: "wammed/Prowly".to_string(),
            reason: "mention".to_string(),
            unread: true,
            updated_at: "2026-10-10".to_string(),
            url: "url1".to_string(),
        },
        AlertItem {
            id: "2".to_string(),
            title: "Review requested on #43".to_string(),
            repository: "wammed/Prowly".to_string(),
            reason: "review_requested".to_string(),
            unread: true,
            updated_at: "2026-10-11".to_string(),
            url: "url2".to_string(),
        },
    ];

    view.set_alerts(alerts);
    assert_eq!(view.selected_alert().unwrap().id, "1");
    assert!(view.selected_alert().unwrap().unread);

    // Mark current selected as read
    let marked_id = view.mark_selected_read();
    assert_eq!(marked_id, Some("1".to_string()));
    assert!(!view.selected_alert().unwrap().unread);

    view.next();
    assert_eq!(view.selected_alert().unwrap().id, "2");
    assert!(view.selected_alert().unwrap().unread);
}
