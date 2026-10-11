//! Internationalization (i18n) module for Prowly.
//! Supports English (EN) and Japanese (JA), variable interpolation, and pluralization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;

/// Supported languages in Prowly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ja,
}

impl Language {
    pub const ALL: &'static [Language] = &[Language::En, Language::Ja];

    pub const fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }

    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Ja => "日本語",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" | "en_us" | "en-us" => Some(Self::En),
            "ja" | "ja_jp" | "ja-jp" => Some(Self::Ja),
            _ => None,
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// An entry in the translation table, either singular string or count-based plural forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslationEntry {
    Plain(&'static str),
    Plural {
        one: &'static str,
        other: &'static str,
    },
}

impl TranslationEntry {
    pub const fn plain(s: &'static str) -> Self {
        Self::Plain(s)
    }

    pub const fn plural(one: &'static str, other: &'static str) -> Self {
        Self::Plural { one, other }
    }

    pub fn resolve(&self, lang: Language, count: Option<usize>) -> &'static str {
        match self {
            Self::Plain(s) => s,
            Self::Plural { one, other } => {
                match lang {
                    Language::En => {
                        if count == Some(1) {
                            one
                        } else {
                            other
                        }
                    }
                    Language::Ja => {
                        // Japanese typically has no grammatical plural distinctions; use `other`
                        other
                    }
                }
            }
        }
    }
}

/// Thread-safe Internationalization manager.
pub struct I18n {
    current_lang: RwLock<Language>,
}

impl I18n {
    /// Create new i18n manager with the given initial language.
    pub fn new(lang: Language) -> Self {
        Self {
            current_lang: RwLock::new(lang),
        }
    }

    /// Retrieve the currently selected language.
    pub fn language(&self) -> Language {
        *self.current_lang.read().unwrap()
    }

    /// Change the active language.
    pub fn set_language(&self, lang: Language) {
        let mut l = self.current_lang.write().unwrap();
        *l = lang;
    }

    /// Toggle between English and Japanese.
    pub fn toggle_language(&self) -> Language {
        let mut l = self.current_lang.write().unwrap();
        let new_lang = match *l {
            Language::En => Language::Ja,
            Language::Ja => Language::En,
        };
        *l = new_lang;
        new_lang
    }

    /// Translate a key using the active language.
    pub fn t(&self, key: &str) -> String {
        self.translate(key, None, &[])
    }

    /// Translate a key with variable arguments.
    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.translate(key, None, args)
    }

    /// Translate a pluralized key with a count and optional args.
    pub fn t_plural(&self, key: &str, count: usize, args: &[(&str, &str)]) -> String {
        let count_str = count.to_string();
        let mut merged_args = Vec::with_capacity(args.len() + 1);
        merged_args.push(("count", count_str.as_str()));
        for (k, v) in args {
            if *k != "count" {
                merged_args.push((*k, *v));
            }
        }
        self.translate(key, Some(count), &merged_args)
    }

    /// Core translation resolver with variable replacement.
    pub fn translate(&self, key: &str, count: Option<usize>, args: &[(&str, &str)]) -> String {
        let lang = self.language();
        let template = lookup_template(lang, key, count).unwrap_or(key);
        interpolate(template, args)
    }
}

/// Interpolates `{var}` parameters in a template string.
pub fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, val) in args {
        let pattern = format!("{{{}}}", key);
        result = result.replace(&pattern, val);
    }
    result
}

/// Lookup a translation template by language and key.
pub fn lookup_template(lang: Language, key: &str, count: Option<usize>) -> Option<&'static str> {
    let table = match lang {
        Language::En => &EN_DICTIONARY,
        Language::Ja => &JA_DICTIONARY,
    };
    table.get(key).map(|entry| entry.resolve(lang, count))
}

/// Global lazy dictionary instance getters for verification tests.
pub fn get_dictionary(lang: Language) -> &'static HashMap<&'static str, TranslationEntry> {
    match lang {
        Language::En => &EN_DICTIONARY,
        Language::Ja => &JA_DICTIONARY,
    }
}

macro_rules! define_dictionary {
    ($( $key:literal => $val:expr ),* $(,)?) => {{
        let mut m = HashMap::new();
        $(
            m.insert($key, $val);
        )*
        m
    }};
}

use std::sync::LazyLock;

pub static EN_DICTIONARY: LazyLock<HashMap<&'static str, TranslationEntry>> = LazyLock::new(|| {
    define_dictionary!(
        // App & Navigation
        "app.title" => TranslationEntry::plain("Prowly"),
        "app.tagline" => TranslationEntry::plain("Terminal Git & GitHub client with Omnipresent AI"),

        // Header
        "header.synced" => TranslationEntry::plain("Synced"),
        "header.ahead_behind" => TranslationEntry::plain("↑{ahead} ↓{behind}"),
        "header.ci_passed" => TranslationEntry::plain("CI: Passed"),
        "header.ci_failed" => TranslationEntry::plain("CI: Failed"),
        "header.ci_running" => TranslationEntry::plain("CI: Running"),
        "header.alerts_unread" => TranslationEntry::plural("{count} unread alert", "{count} unread alerts"),
        "header.private" => TranslationEntry::plain("Private"),
        "header.public" => TranslationEntry::plain("Public"),

        // Tabs
        "tabs.changes" => TranslationEntry::plain("Changes"),
        "tabs.prs" => TranslationEntry::plain("Pull Requests"),
        "tabs.actions" => TranslationEntry::plain("Actions"),
        "tabs.alerts" => TranslationEntry::plain("Alerts"),
        "tabs.worktrees" => TranslationEntry::plain("Worktrees"),

        // Directory Map
        "dir_map.title" => TranslationEntry::plain("DIRECTORY MAP"),
        "dir_map.not_a_repo" => TranslationEntry::plain("Not a git repository"),
        "dir_map.init_repo" => TranslationEntry::plain("Init Repository"),
        "dir_map.clone_repo" => TranslationEntry::plain("Clone Repository"),

        // Buttons & Actions
        "btn.commit_and_sync" => TranslationEntry::plain("Commit & Sync"),
        "btn.commit" => TranslationEntry::plain("Commit"),
        "btn.stash_all" => TranslationEntry::plain("Stash All"),
        "btn.pop_stash" => TranslationEntry::plain("Pop Stash"),
        "btn.apply" => TranslationEntry::plain("Apply"),
        "btn.discard" => TranslationEntry::plain("Discard"),
        "btn.cancel" => TranslationEntry::plain("Cancel"),
        "btn.confirm" => TranslationEntry::plain("Confirm"),
        "btn.rerun_failed" => TranslationEntry::plain("Rerun Failed Jobs"),
        "btn.mark_as_done" => TranslationEntry::plain("Mark as Done"),
        "btn.checkout_pr" => TranslationEntry::plain("Checkout Locally"),
        "btn.ask_ai" => TranslationEntry::plain("Ask AI"),

        // Changes View
        "changes.files_changed" => TranslationEntry::plural("{count} file changed", "{count} files changed"),
        "changes.staged" => TranslationEntry::plain("Staged Changes"),
        "changes.unstaged" => TranslationEntry::plain("Changes"),
        "changes.commit_placeholder" => TranslationEntry::plain("Summary (commit message)..."),
        "changes.commit_desc_placeholder" => TranslationEntry::plain("Description (optional)..."),
        "changes.no_changes" => TranslationEntry::plain("No local changes"),
        "changes.stage_all" => TranslationEntry::plain("Stage All"),
        "changes.unstage_all" => TranslationEntry::plain("Unstage All"),

        // Diff View
        "diff.loading" => TranslationEntry::plain("Loading diff..."),
        "diff.binary_file" => TranslationEntry::plain("Binary file differs"),
        "diff.empty" => TranslationEntry::plain("No diff to display"),
        "diff.lines_added" => TranslationEntry::plural("+{count} line", "+{count} lines"),
        "diff.lines_removed" => TranslationEntry::plural("-{count} line", "-{count} lines"),

        // PRs View
        "prs.title" => TranslationEntry::plain("Pull Requests"),
        "prs.open" => TranslationEntry::plain("Open"),
        "prs.closed" => TranslationEntry::plain("Closed"),
        "prs.merged" => TranslationEntry::plain("Merged"),
        "prs.draft" => TranslationEntry::plain("Draft"),
        "prs.no_prs" => TranslationEntry::plain("No pull requests found"),
        "prs.reviews_pending" => TranslationEntry::plain("Review required"),

        // Actions / CI View
        "actions.title" => TranslationEntry::plain("Workflow Runs"),
        "actions.streaming_log" => TranslationEntry::plain("Streaming CI log..."),
        "actions.no_runs" => TranslationEntry::plain("No recent workflow runs"),

        // Alerts View
        "alerts.title" => TranslationEntry::plain("Notifications & Alerts"),
        "alerts.no_alerts" => TranslationEntry::plain("All clear! No unread notifications"),

        // Worktrees View
        "worktrees.title" => TranslationEntry::plain("Git Worktrees"),
        "worktrees.add" => TranslationEntry::plain("Create Worktree"),
        "worktrees.remove" => TranslationEntry::plain("Remove Worktree"),

        // Conflict Resolver
        "conflict.title" => TranslationEntry::plain("Conflict Resolver"),
        "conflict.accept_current" => TranslationEntry::plain("Accept Current (Ours)"),
        "conflict.accept_incoming" => TranslationEntry::plain("Accept Incoming (Theirs)"),
        "conflict.accept_both" => TranslationEntry::plain("Accept Both"),

        // Omnipresent AI Assistant
        "ai.title" => TranslationEntry::plain("Omnipresent AI Assistant"),
        "ai.ask_prompt" => TranslationEntry::plain("Ask AI about current screen context..."),
        "ai.thinking" => TranslationEntry::plain("AI is thinking..."),
        "ai.context_ready" => TranslationEntry::plain("Context automatically attached: {context}"),
        "ai.apply_confirmation" => TranslationEntry::plain("Apply this proposed change?"),

        // Safety & Soft Discard
        "safety.backup_created" => TranslationEntry::plain("Soft-discard: backup saved to {stash}"),
        "safety.backup_restored" => TranslationEntry::plain("Backup restored from {stash}"),
        "safety.discard_confirm" => TranslationEntry::plain("Discard changes? A safety backup will be saved"),

        // Footer & Shortcuts
        "footer.help" => TranslationEntry::plain("[?] Help"),
        "footer.commands" => TranslationEntry::plain("[:] Commands"),
        "footer.ask_ai" => TranslationEntry::plain("[Ctrl+A] Ask AI"),
        "footer.menu" => TranslationEntry::plain("[Right-Click] Menu"),
        "footer.theme" => TranslationEntry::plain("Theme: {name}"),
        "footer.lang" => TranslationEntry::plain("Lang: {lang}"),
        "footer.copied" => TranslationEntry::plain("Copied to clipboard: {text}"),
    )
});

pub static JA_DICTIONARY: LazyLock<HashMap<&'static str, TranslationEntry>> = LazyLock::new(|| {
    define_dictionary!(
        // App & Navigation
        "app.title" => TranslationEntry::plain("Prowly"),
        "app.tagline" => TranslationEntry::plain("黒子AIと一体化したターミナルGit/GitHubクライアント"),

        // Header
        "header.synced" => TranslationEntry::plain("同期完了"),
        "header.ahead_behind" => TranslationEntry::plain("↑{ahead} ↓{behind}"),
        "header.ci_passed" => TranslationEntry::plain("CI: 成功"),
        "header.ci_failed" => TranslationEntry::plain("CI: 失敗"),
        "header.ci_running" => TranslationEntry::plain("CI: 実行中"),
        "header.alerts_unread" => TranslationEntry::plural("未読通知 {count}件", "未読通知 {count}件"),
        "header.private" => TranslationEntry::plain("非公開"),
        "header.public" => TranslationEntry::plain("公開"),

        // Tabs
        "tabs.changes" => TranslationEntry::plain("変更"),
        "tabs.prs" => TranslationEntry::plain("プルリクエスト"),
        "tabs.actions" => TranslationEntry::plain("アクション (CI)"),
        "tabs.alerts" => TranslationEntry::plain("通知"),
        "tabs.worktrees" => TranslationEntry::plain("ワークツリー"),

        // Directory Map
        "dir_map.title" => TranslationEntry::plain("ディレクトリマップ"),
        "dir_map.not_a_repo" => TranslationEntry::plain("Gitリポジトリではありません"),
        "dir_map.init_repo" => TranslationEntry::plain("リポジトリ初期化"),
        "dir_map.clone_repo" => TranslationEntry::plain("リポジトリをクローン"),

        // Buttons & Actions
        "btn.commit_and_sync" => TranslationEntry::plain("コミット＆同期"),
        "btn.commit" => TranslationEntry::plain("コミット"),
        "btn.stash_all" => TranslationEntry::plain("変更を退避 (Stash)"),
        "btn.pop_stash" => TranslationEntry::plain("退避を復元 (Pop)"),
        "btn.apply" => TranslationEntry::plain("反映する"),
        "btn.discard" => TranslationEntry::plain("破棄する"),
        "btn.cancel" => TranslationEntry::plain("キャンセル"),
        "btn.confirm" => TranslationEntry::plain("確認"),
        "btn.rerun_failed" => TranslationEntry::plain("失敗ジョブを再実行"),
        "btn.mark_as_done" => TranslationEntry::plain("既読にする"),
        "btn.checkout_pr" => TranslationEntry::plain("手元にチェックアウト"),
        "btn.ask_ai" => TranslationEntry::plain("AIに相談"),

        // Changes View
        "changes.files_changed" => TranslationEntry::plural("変更ファイル {count}件", "変更ファイル {count}件"),
        "changes.staged" => TranslationEntry::plain("ステージされた変更"),
        "changes.unstaged" => TranslationEntry::plain("変更ファイル"),
        "changes.commit_placeholder" => TranslationEntry::plain("コミットの要約を入力..."),
        "changes.commit_desc_placeholder" => TranslationEntry::plain("詳細な説明 (省略可能)..."),
        "changes.no_changes" => TranslationEntry::plain("変更はありません"),
        "changes.stage_all" => TranslationEntry::plain("すべてステージ"),
        "changes.unstage_all" => TranslationEntry::plain("すべてのステージを解除"),

        // Diff View
        "diff.loading" => TranslationEntry::plain("差分を読み込み中..."),
        "diff.binary_file" => TranslationEntry::plain("バイナリファイルのため差分非表示"),
        "diff.empty" => TranslationEntry::plain("表示する差分がありません"),
        "diff.lines_added" => TranslationEntry::plural("+{count}行", "+{count}行"),
        "diff.lines_removed" => TranslationEntry::plural("-{count}行", "-{count}行"),

        // PRs View
        "prs.title" => TranslationEntry::plain("プルリクエスト"),
        "prs.open" => TranslationEntry::plain("オープン"),
        "prs.closed" => TranslationEntry::plain("クローズ"),
        "prs.merged" => TranslationEntry::plain("マージ済み"),
        "prs.draft" => TranslationEntry::plain("下書き"),
        "prs.no_prs" => TranslationEntry::plain("プルリクエストはありません"),
        "prs.reviews_pending" => TranslationEntry::plain("レビュー待ち"),

        // Actions / CI View
        "actions.title" => TranslationEntry::plain("ワークフロー履歴"),
        "actions.streaming_log" => TranslationEntry::plain("CIログを受信中..."),
        "actions.no_runs" => TranslationEntry::plain("直近のワークフロー履歴はありません"),

        // Alerts View
        "alerts.title" => TranslationEntry::plain("通知とアラート"),
        "alerts.no_alerts" => TranslationEntry::plain("未読の通知はありません"),

        // Worktrees View
        "worktrees.title" => TranslationEntry::plain("Git ワークツリー"),
        "worktrees.add" => TranslationEntry::plain("ワークツリー作成"),
        "worktrees.remove" => TranslationEntry::plain("ワークツリー削除"),

        // Conflict Resolver
        "conflict.title" => TranslationEntry::plain("コンフリクト解消モード"),
        "conflict.accept_current" => TranslationEntry::plain("現在の変更 (自分側) を採用"),
        "conflict.accept_incoming" => TranslationEntry::plain("相手の変更 (相手側) を採用"),
        "conflict.accept_both" => TranslationEntry::plain("両方の変更を残す"),

        // Omnipresent AI Assistant
        "ai.title" => TranslationEntry::plain("全面統合 AI アシスタント"),
        "ai.ask_prompt" => TranslationEntry::plain("現在の画面コンテキストについてAIに指示..."),
        "ai.thinking" => TranslationEntry::plain("AIが思考中..."),
        "ai.context_ready" => TranslationEntry::plain("自動注入されたコンテキスト: {context}"),
        "ai.apply_confirmation" => TranslationEntry::plain("このAI提案を適用しますか？"),

        // Safety & Soft Discard
        "safety.backup_created" => TranslationEntry::plain("ソフト破棄: バックアップを一時退避しました ({stash})"),
        "safety.backup_restored" => TranslationEntry::plain("バックアップを復元しました ({stash})"),
        "safety.discard_confirm" => TranslationEntry::plain("変更を破棄しますか？(安全のためバックアップが退避されます)"),

        // Footer & Shortcuts
        "footer.help" => TranslationEntry::plain("[?] ヘルプ"),
        "footer.commands" => TranslationEntry::plain("[:] コマンド"),
        "footer.ask_ai" => TranslationEntry::plain("[Ctrl+A] AI相談"),
        "footer.menu" => TranslationEntry::plain("[右クリック] メニュー"),
        "footer.theme" => TranslationEntry::plain("テーマ: {name}"),
        "footer.lang" => TranslationEntry::plain("言語: {lang}"),
        "footer.copied" => TranslationEntry::plain("クリップボードにコピー: {text}"),
    )
});
