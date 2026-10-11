use prowly::git::conflict::{apply_resolutions, parse_conflicts, ConflictChoice};

#[test]
fn test_parse_conflicts_and_resolve_current() {
    let conflicted = r#"fn main() {
<<<<<<< HEAD
    println!("Hello from current");
=======
    println!("Hello from incoming");
>>>>>>> feature/test
}
"#;

    let (mut hunks, non_conflict) = parse_conflicts(conflicted);
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0].current_branch_name, "HEAD");
    assert_eq!(hunks[0].incoming_branch_name, "feature/test");
    assert_eq!(
        hunks[0].current_lines,
        vec!["    println!(\"Hello from current\");"]
    );
    assert_eq!(
        hunks[0].incoming_lines,
        vec!["    println!(\"Hello from incoming\");"]
    );
    assert!(non_conflict.contains(&"fn main() {".to_string()));

    // Resolve as Current
    hunks[0].choice = Some(ConflictChoice::Current);
    let resolved = apply_resolutions(conflicted, &hunks);
    assert!(resolved.contains("Hello from current"));
    assert!(!resolved.contains("Hello from incoming"));
    assert!(!resolved.contains("<<<<<<<"));
    assert!(!resolved.contains("======="));
    assert!(!resolved.contains(">>>>>>>"));
}

#[test]
fn test_resolve_incoming() {
    let conflicted = r#"Line 1
<<<<<<< HEAD
Current Line
=======
Incoming Line
>>>>>>> branch-a
Line 3
"#;

    let (mut hunks, _) = parse_conflicts(conflicted);
    assert_eq!(hunks.len(), 1);

    hunks[0].choice = Some(ConflictChoice::Incoming);
    let resolved = apply_resolutions(conflicted, &hunks);
    assert!(resolved.contains("Incoming Line"));
    assert!(!resolved.contains("Current Line"));
    assert!(!resolved.contains("<<<<<<<"));
}

#[test]
fn test_resolve_both_and_custom() {
    let conflicted = r#"<<<<<<< HEAD
First
=======
Second
>>>>>>> feature
"#;

    let (mut hunks, _) = parse_conflicts(conflicted);
    assert_eq!(hunks.len(), 1);

    // Test Both
    hunks[0].choice = Some(ConflictChoice::Both);
    let resolved_both = apply_resolutions(conflicted, &hunks);
    assert_eq!(resolved_both, "First\nSecond");

    // Test Custom (e.g. from AI smart merge)
    hunks[0].custom_resolution = Some("Merged Custom Line".to_string());
    let resolved_custom = apply_resolutions(conflicted, &hunks);
    assert_eq!(resolved_custom, "Merged Custom Line");
}
