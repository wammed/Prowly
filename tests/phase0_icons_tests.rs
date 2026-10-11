use prowly::ui::icons::{IconId, Icons};

#[test]
fn test_all_icons_have_nerd_and_fallback_glyphs() {
    let all_icons = [
        IconId::Branch,
        IconId::Commit,
        IconId::Stash,
        IconId::Worktree,
        IconId::Tag,
        IconId::Remote,
        IconId::Merge,
        IconId::Conflict,
        IconId::DiffAdded,
        IconId::DiffRemoved,
        IconId::DiffModified,
        IconId::PrOpen,
        IconId::PrClosed,
        IconId::PrMerged,
        IconId::PrDraft,
        IconId::IssueOpen,
        IconId::IssueClosed,
        IconId::CheckSuccess,
        IconId::CheckFailed,
        IconId::CheckRunning,
        IconId::CheckWarning,
        IconId::CheckSkipped,
        IconId::Verified,
        IconId::Unverified,
        IconId::AiSparkle,
        IconId::FolderOpen,
        IconId::FolderClosed,
        IconId::File,
        IconId::Sync,
        IconId::ArrowUp,
        IconId::ArrowDown,
        IconId::LockPrivate,
        IconId::GlobePublic,
        IconId::Help,
        IconId::Alert,
        IconId::Edit,
        IconId::Trash,
        IconId::Copy,
        IconId::CheckboxChecked,
        IconId::CheckboxUnchecked,
    ];

    let nerd = Icons::nerd_font();
    let fallback = Icons::fallback();

    for id in all_icons {
        let n_glyph = nerd.get(id);
        let f_glyph = fallback.get(id);

        assert!(!n_glyph.is_empty(), "Nerd glyph for {:?} is empty", id);
        assert!(!f_glyph.is_empty(), "Fallback glyph for {:?} is empty", id);

        // Fallback glyphs should be ASCII or basic punctuation
        assert!(
            f_glyph.is_ascii(),
            "Fallback glyph '{f_glyph}' for {:?} should be ASCII",
            id
        );
    }
}

#[test]
fn test_convenience_methods() {
    let nerd = Icons::nerd_font();
    assert_eq!(nerd.branch(), nerd.get(IconId::Branch));
    assert_eq!(nerd.commit(), nerd.get(IconId::Commit));
    assert_eq!(nerd.stash(), nerd.get(IconId::Stash));
    assert_eq!(nerd.worktree(), nerd.get(IconId::Worktree));
    assert_eq!(nerd.conflict(), nerd.get(IconId::Conflict));
    assert_eq!(nerd.pr_open(), nerd.get(IconId::PrOpen));
    assert_eq!(nerd.check_success(), nerd.get(IconId::CheckSuccess));
    assert_eq!(nerd.check_failed(), nerd.get(IconId::CheckFailed));
    assert_eq!(nerd.ai_sparkle(), nerd.get(IconId::AiSparkle));
}
