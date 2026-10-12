//! Icons module with high-fidelity Nerd Font glyphs and ASCII/Unicode fallback definitions.

use serde::{Deserialize, Serialize};

/// High-level icon identifiers used across the Prowly UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IconId {
    // Git Icons
    Branch,
    Commit,
    Stash,
    Worktree,
    Tag,
    Remote,
    Merge,
    Conflict,
    DiffAdded,
    DiffRemoved,
    DiffModified,

    // GitHub & Collaboration
    PrOpen,
    PrClosed,
    PrMerged,
    PrDraft,
    IssueOpen,
    IssueClosed,

    // Status & CI
    CheckSuccess,
    CheckFailed,
    CheckRunning,
    CheckWarning,
    CheckSkipped,

    // Signatures
    Verified,
    Unverified,

    // UI & Navigation
    AiSparkle,
    FolderOpen,
    FolderClosed,
    File,
    Sync,
    ArrowUp,
    ArrowDown,
    LockPrivate,
    GlobePublic,
    Help,
    Alert,
    Edit,
    Trash,
    Copy,
    CheckboxChecked,
    CheckboxUnchecked,
    Settings,
}

/// Collection of glyph strings for the active display mode.
#[derive(Debug, Clone)]
pub struct Icons {
    pub use_nerd_font: bool,
}

impl Icons {
    /// Create an icon provider configured for Nerd Font or fallback.
    pub const fn new(use_nerd_font: bool) -> Self {
        Self { use_nerd_font }
    }

    /// Default Nerd Font icon provider.
    pub const fn nerd_font() -> Self {
        Self {
            use_nerd_font: true,
        }
    }

    /// Fallback ASCII/Unicode icon provider.
    pub const fn fallback() -> Self {
        Self {
            use_nerd_font: false,
        }
    }

    /// Resolve an icon glyph based on current font configuration.
    pub fn get(&self, id: IconId) -> &'static str {
        if self.use_nerd_font {
            Self::nerd_glyph(id)
        } else {
            Self::fallback_glyph(id)
        }
    }

    /// Get high-definition Nerd Font glyph for an icon ID.
    pub const fn nerd_glyph(id: IconId) -> &'static str {
        match id {
            // Git
            IconId::Branch => "",
            IconId::Commit => "󰊢",
            IconId::Stash => "󰆓",
            IconId::Worktree => "󰙅",
            IconId::Tag => "󰓹",
            IconId::Remote => "󰒍",
            IconId::Merge => "󰄲",
            IconId::Conflict => "",
            IconId::DiffAdded => "",
            IconId::DiffRemoved => "",
            IconId::DiffModified => "",

            // GitHub
            IconId::PrOpen => "",
            IconId::PrClosed => "",
            IconId::PrMerged => "",
            IconId::PrDraft => "",
            IconId::IssueOpen => "",
            IconId::IssueClosed => "",

            // CI & Status
            IconId::CheckSuccess => "",
            IconId::CheckFailed => "",
            IconId::CheckRunning => "",
            IconId::CheckWarning => "",
            IconId::CheckSkipped => "",

            // Signatures
            IconId::Verified => "󰄬",
            IconId::Unverified => "󰅚",

            // UI
            IconId::AiSparkle => "󱚣",
            IconId::FolderOpen => "",
            IconId::FolderClosed => "󰉋",
            IconId::File => "",
            IconId::Sync => "󰑐",
            IconId::ArrowUp => "",
            IconId::ArrowDown => "",
            IconId::LockPrivate => "",
            IconId::GlobePublic => "󰖟",
            IconId::Help => "",
            IconId::Alert => "",
            IconId::Edit => "󰏫",
            IconId::Trash => "",
            IconId::Copy => "󰆏",
            IconId::CheckboxChecked => "",
            IconId::CheckboxUnchecked => "",
            IconId::Settings => "",
        }
    }

    /// Get clean ASCII / Unicode fallback glyph for an icon ID.
    pub const fn fallback_glyph(id: IconId) -> &'static str {
        match id {
            // Git
            IconId::Branch => "*",
            IconId::Commit => "o",
            IconId::Stash => "$",
            IconId::Worktree => "#",
            IconId::Tag => "t",
            IconId::Remote => "@",
            IconId::Merge => "m",
            IconId::Conflict => "!",
            IconId::DiffAdded => "+",
            IconId::DiffRemoved => "-",
            IconId::DiffModified => "~",

            // GitHub
            IconId::PrOpen => "PR",
            IconId::PrClosed => "PR(x)",
            IconId::PrMerged => "PR(m)",
            IconId::PrDraft => "PR(d)",
            IconId::IssueOpen => "#",
            IconId::IssueClosed => "#(x)",

            // CI & Status
            IconId::CheckSuccess => "OK",
            IconId::CheckFailed => "ERR",
            IconId::CheckRunning => "...",
            IconId::CheckWarning => "WARN",
            IconId::CheckSkipped => "SKIP",

            // Signatures
            IconId::Verified => "[V]",
            IconId::Unverified => "[?]",

            // UI
            IconId::AiSparkle => "[AI]",
            IconId::FolderOpen => "[-]",
            IconId::FolderClosed => "[+]",
            IconId::File => " ",
            IconId::Sync => "<->",
            IconId::ArrowUp => "^",
            IconId::ArrowDown => "v",
            IconId::LockPrivate => "(P)",
            IconId::GlobePublic => "(O)",
            IconId::Help => "?",
            IconId::Alert => "!",
            IconId::Edit => "/",
            IconId::Trash => "x",
            IconId::Copy => "y",
            IconId::CheckboxChecked => "[x]",
            IconId::CheckboxUnchecked => "[ ]",
            IconId::Settings => "[*]",
        }
    }

    // Direct helper methods for commonly used icons
    pub fn branch(&self) -> &'static str {
        self.get(IconId::Branch)
    }
    pub fn commit(&self) -> &'static str {
        self.get(IconId::Commit)
    }
    pub fn stash(&self) -> &'static str {
        self.get(IconId::Stash)
    }
    pub fn worktree(&self) -> &'static str {
        self.get(IconId::Worktree)
    }
    pub fn conflict(&self) -> &'static str {
        self.get(IconId::Conflict)
    }
    pub fn pr_open(&self) -> &'static str {
        self.get(IconId::PrOpen)
    }
    pub fn check_success(&self) -> &'static str {
        self.get(IconId::CheckSuccess)
    }
    pub fn check_failed(&self) -> &'static str {
        self.get(IconId::CheckFailed)
    }
    pub fn check_running(&self) -> &'static str {
        self.get(IconId::CheckRunning)
    }
    pub fn verified(&self) -> &'static str {
        self.get(IconId::Verified)
    }
    pub fn unverified(&self) -> &'static str {
        self.get(IconId::Unverified)
    }
    pub fn ai_sparkle(&self) -> &'static str {
        self.get(IconId::AiSparkle)
    }
    pub fn folder_open(&self) -> &'static str {
        self.get(IconId::FolderOpen)
    }
    pub fn folder_closed(&self) -> &'static str {
        self.get(IconId::FolderClosed)
    }
    pub fn file(&self) -> &'static str {
        self.get(IconId::File)
    }
    pub fn sync(&self) -> &'static str {
        self.get(IconId::Sync)
    }
    pub fn arrow_up(&self) -> &'static str {
        self.get(IconId::ArrowUp)
    }
    pub fn arrow_down(&self) -> &'static str {
        self.get(IconId::ArrowDown)
    }
    pub fn checkbox_checked(&self) -> &'static str {
        self.get(IconId::CheckboxChecked)
    }
    pub fn checkbox_unchecked(&self) -> &'static str {
        self.get(IconId::CheckboxUnchecked)
    }
    pub fn settings(&self) -> &'static str {
        self.get(IconId::Settings)
    }
}
