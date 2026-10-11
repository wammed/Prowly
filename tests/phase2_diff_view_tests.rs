use prowly::git::{DiffHunk, DiffLine, DiffLineType, FileDiff};
use prowly::ui::views::diff::DiffView;

#[test]
fn test_flatten_diff_lines_structure() {
    let diff = FileDiff {
        file_path: "src/main.rs".to_string(),
        is_binary: false,
        stats_added: 2,
        stats_deleted: 1,
        hunks: vec![DiffHunk {
            header: "@@ -1,3 +1,4 @@".to_string(),
            lines: vec![
                DiffLine {
                    origin: ' ',
                    content: "fn main() {".to_string(),
                    old_lineno: Some(1),
                    new_lineno: Some(1),
                    line_type: DiffLineType::Context,
                },
                DiffLine {
                    origin: '-',
                    content: "    println!(\"old\");".to_string(),
                    old_lineno: Some(2),
                    new_lineno: None,
                    line_type: DiffLineType::Deleted,
                },
                DiffLine {
                    origin: '+',
                    content: "    println!(\"new\");".to_string(),
                    old_lineno: None,
                    new_lineno: Some(2),
                    line_type: DiffLineType::Added,
                },
            ],
        }],
    };

    let flat = DiffView::flatten_diff_lines(&diff);

    // 1 header + 3 lines = 4 total lines
    assert_eq!(flat.len(), 4);

    // First line must be hunk header
    assert_eq!(flat[0].3, DiffLineType::HunkHeader);
    assert_eq!(flat[0].4, "@@ -1,3 +1,4 @@");

    // Second line must be context
    assert_eq!(flat[1].3, DiffLineType::Context);
    assert_eq!(flat[1].0, Some(1));
    assert_eq!(flat[1].1, Some(1));

    // Third line must be deleted
    assert_eq!(flat[2].3, DiffLineType::Deleted);
    assert_eq!(flat[2].0, Some(2));
    assert_eq!(flat[2].1, None);

    // Fourth line must be added
    assert_eq!(flat[3].3, DiffLineType::Added);
    assert_eq!(flat[3].0, None);
    assert_eq!(flat[3].1, Some(2));
}
