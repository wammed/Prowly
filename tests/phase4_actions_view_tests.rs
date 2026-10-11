use prowly::github::WorkflowRunItem;
use prowly::ui::views::{ActionsFocus, ActionsView};

#[test]
fn test_actions_view_navigation_and_focus() {
    let mut view = ActionsView::new();
    assert_eq!(view.focus, ActionsFocus::RunsList);

    let runs = vec![
        WorkflowRunItem {
            id: 101,
            name: "Build & Test".to_string(),
            head_branch: "main".to_string(),
            event: "push".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            html_url: "url1".to_string(),
            created_at: "2026-10-10".to_string(),
            logs: Some("line1\nline2".to_string()),
        },
        WorkflowRunItem {
            id: 102,
            name: "Lint".to_string(),
            head_branch: "feature".to_string(),
            event: "pull_request".to_string(),
            status: "completed".to_string(),
            conclusion: Some("failure".to_string()),
            html_url: "url2".to_string(),
            created_at: "2026-10-11".to_string(),
            logs: Some("error on line 5".to_string()),
        },
    ];

    view.set_runs(runs);
    assert_eq!(view.selected_run().unwrap().id, 101);

    view.next();
    assert_eq!(view.selected_run().unwrap().id, 102);

    view.previous();
    assert_eq!(view.selected_run().unwrap().id, 101);

    view.toggle_focus();
    assert_eq!(view.focus, ActionsFocus::LogView);

    view.toggle_focus();
    assert_eq!(view.focus, ActionsFocus::RunsList);
}
