use prowly::github::PullRequestItem;
use prowly::ui::views::{PrFocus, PrsView};

#[test]
fn test_prs_view_navigation_and_focus() {
    let mut view = PrsView::new();
    assert_eq!(view.focus, PrFocus::List);
    assert_eq!(view.selected_pr(), None);

    let prs = vec![
        PullRequestItem {
            number: 1,
            title: "First PR".to_string(),
            author: "alice".to_string(),
            state: "open".to_string(),
            is_draft: false,
            head_branch: "feature-1".to_string(),
            base_branch: "main".to_string(),
            body: Some("Description 1".to_string()),
            comments_count: 2,
            html_url: "url1".to_string(),
            updated_at: "2026-10-10".to_string(),
        },
        PullRequestItem {
            number: 2,
            title: "Second PR".to_string(),
            author: "bob".to_string(),
            state: "closed".to_string(),
            is_draft: true,
            head_branch: "feature-2".to_string(),
            base_branch: "main".to_string(),
            body: Some("Description 2".to_string()),
            comments_count: 0,
            html_url: "url2".to_string(),
            updated_at: "2026-10-11".to_string(),
        },
    ];

    view.set_prs(prs);
    assert_eq!(view.selected_pr().unwrap().number, 1);

    view.next();
    assert_eq!(view.selected_pr().unwrap().number, 2);

    view.previous();
    assert_eq!(view.selected_pr().unwrap().number, 1);

    view.toggle_focus();
    assert_eq!(view.focus, PrFocus::Detail);

    view.toggle_focus();
    assert_eq!(view.focus, PrFocus::List);
}
