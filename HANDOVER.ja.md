# プロジェクト引き継ぎ書 (HANDOVER) - Prowly

最終更新日: 2026-10-12

---

## 1. プロジェクト概要・コンセプト

- **プロジェクト名**: Prowly
- **コアコンセプト**: 「全面統合された賢い AI コパイロットを常時備えた、GitHub Desktop を超えるターミナルクライアント」
- **デュアル入力 UX**: マウスのみでも完全操作可能、キーボードなら超高速。
- **言語・プラットフォーム**: Rust (クロスプラットフォーム: Linux / macOS / Windows)
- **主要設計ドキュメント**:
  - 仕様書: [Design.md](file:///home/susie/git/wammed/Prowly/Design.md)
  - 実装ロードマップ: [implementation_roadmap.md](file:///home/susie/git/wammed/Prowly/implementation_roadmap.md)
  - ユーザー向けドキュメント: [README.md](file:///home/susie/git/wammed/Prowly/README.md) / [README.ja.md](file:///home/susie/git/wammed/Prowly/README.ja.md)

---

## 2. 確定アーキテクチャ・設計決定事項

| 分類 | 決定方針・実装アプローチ |
| :--- | :--- |
| **UI 描画 & 入力** | `ratatui` + `crossterm`。マウス完全対応（クリック・右クリック・ドラッグ・ホイールスクロール）と軽快なキーボードショートカットの両立。 |
| **非同期エンジン** | `tokio`。UI 描画ループと Git/GitHub API/AI のバックグラウンドタスクを mpsc チャネルで分離。 |
| **Git エンジン** | **ハイブリッド構成 (`git2` + ローカル `git` CLI)**。<br>状態取得・差分パース・ツリー走査はインプロセスで最高速化。<br>SSH エージェント認証・Credential Helper・GPG 署名・Push/Pull はローカル `git` コマンドに委譲し認証トラブルを根絶。 |
| **描画パフォーマンス** | **仮想スクロール & LRU キャッシュ**。<br>数万行の差分や CI ログでも画面内のみを `syntect` で遅延ハイライト描画し、60 FPS を維持。 |
| **全面統合 AI** | **アプリ全体を貫くファーストクラス機能**。<br>全画面（差分、PR、CI ログ、競合、コマンドパレット、エラー時）から `Ctrl+A` / `[ 󱚣 Ask AI ]` で呼び出し可能。<br>画面コンテキスト（選択ファイル、差分、生ログ）を自動注入し、安全確認 (`[ Apply ]`) を経て反映。<br>BYOK (OpenAI, Anthropic, Gemini, Ollama, Custom) マルチプロバイダー対応。ローカル Ollama は API キー不要かつネイティブ API (`/api/chat`) を直叩き（ポート番号のみ指定でも自動正規化）。 |
| **安全ネット** | **ソフト破棄 (Soft-Discard)**。変更破棄の実行前に非表示スタッシュ/一時バックアップを自動退避し、誤操作によるコード消失を防止。 |
| **Git 拡張機能** | Git Worktree 管理、GPG/SSH コミット署名検証バッジ、ビジュアルコンフリクト解消、クイック・インタラクティブ・リベース（Fixup / Squash / Reword）。 |
| **デザインシステム** | 高精細 Nerd Font グリフと、非対応環境向け自動 ASCII/Unicode フォールバック。厳選された21色のプリセットカラーテーマ。 |
| **国際化 (i18n)** | 全 UI・メニュー・ヘルプ・ポップアップ・通知・AI プロンプト・ドキュメントで英語 (EN) / 日本語 (JA) 完全対応。複数形ルール対応。 |
| **クリップボード** | `arboard` クレートによる OS ネイティブクリップボード連携 (`y` キー)。 |
| **デスクトップランチャー統合** | FreeDesktop `.desktop` エントリ (`Terminal=true`) とスケーラブルアイコン (`images/Prowly-Icon.svg`) により、システムのデフォルトターミナルで自動起動。 |
| **API レートリミット保護** | 指数バックオフ、HTTP `ETag` キャッシュ、明示的手動リフレッシュ (`R` キー)。 |

---

## 3. プロジェクト運用ルール

1. **引き継ぎ書の継続的更新**:
   - 仕様変更やマイルストーン達成、設計判断時は本ドキュメント (`HANDOVER.md` および `HANDOVER.ja.md`) を必ず更新する。
2. **完全な日英併記ドキュメント**:
   - 仕様書・README・マニュアル等は常に**英語と日本語の同一内容**で維持する。
3. **プロアクティブな提案**:
   - 改善点や推奨機能の発見時は、積極的にユーザーへ提案して合意を得る。
4. **自動テストの網羅**:
   - 各フェーズで単体テスト、一時 Git リポジトリを用いた結合テスト、UI 状態遷移テスト、未翻訳キー検知テストを 100% 維持する。

---

## 4. 進捗状況 & マイルストーン達成サマリー

- **進捗状況**: **全フェーズ（Phase 0 から Phase 8 まで）完全完了**。
- **自動テスト通過実績**: **全70件の自動テストが 100% 合格**。
- **非同期アーキテクチャの注記**: GitHub API や AI などの外部非同期処理は `AppEvent` チャネルと `tokio::spawn` により完全非同期・非ブロッキングに分離され、Tokio ランタイム内でのネストパニック（`block_on` 競合）を根絶。
- **フェーズ別実装実績**:
  - **Phase 0 (基盤設計・依存関係)**:
    1. `Cargo.toml` の依存関係整備。
    2. 21色のカラーテーマパレット & 動的切り替えエンジン (`src/ui/theme.rs`)。
    3. Nerd Font & ASCII フォールバックアイコン機構 (`src/ui/icons.rs`)。
    4. 複数形・プレースホルダー整合性検証付き日英 i18n 機構 (`src/i18n/mod.rs`)。
    5. BYOK AI マルチプロバイダー抽象化クライアント (`src/ai/mod.rs`)。
    6. TOML 設定ファイルの読み書き・永続化 (`src/config/mod.rs`)。
  - **Phase 1 (コア TUI & 仮想スクロール)**:
    1. キーボード・マウス完全ヒットテスト付き非同期イベントループ (`src/event.rs`, `src/ui/layout.rs`)。
    2. LRU キャッシュ付き高性能仮想スクロールエンジン (`src/ui/virtual_scroll.rs`)。
    3. 全面統合 AI アシスタントモーダル (`src/ui/modals/ai_assistant.rs`)。
    4. OS クリップボード連携 (`src/services/clipboard.rs`)。
    5. 統合アプリケーション状態管理 (`src/app.rs`)。
  - **Phase 2 (変更ビュー、安全ネット & AIレビュー)**:
    1. ソフト破棄セーフティバックアップ (`src/services/backup.rs`)。
    2. Git ハイブリッドサービス (`src/git/service.rs`)。
    3. 変更一覧ビュー & コミットメッセージエディタ (`src/ui/views/changes.rs`)。
    4. Syntect 差分ハイライトビュー (`src/ui/views/diff.rs`)。
    5. AI コミットメッセージ自動生成 (`Ctrl+G`) & 事前コードレビュー (`Ctrl+R`) (`src/ai/commit.rs`)。
  - **Phase 3 (ディレクトリマップ、ワークツリー & AI診断)**:
    1. ディレクトリマップツリー集計 & 件数バッジ (`src/ui/views/dir_map.rs`)。
    2. Git ワークツリー管理 & インタラクティブ一覧ビュー (`src/git/worktree.rs`, `src/ui/views/worktrees.rs`)。
    3. 右クリックコンテキストメニュー & 端末互換設計（左クリック `[ 󰍜 Menu ▾ ]` ボタン / キー `m` 完備により、GNOME Terminal 等の右クリック横取り環境でも 100% 動作保証）(`src/ui/modals/context_menu.rs`)。
    4. リポジトリ健康診断 AI スキャン (`Clean`, `NeedsCommit`, `Conflicted`) (`src/ai/diagnostic.rs`)。
  - **Phase 4 (GitHub 深層統合)**:
    1. TTL キャッシュ & レートリミット保護付き GitHub クライアント (`src/github/client.rs`)。
    2. プルリクエスト一覧 & 詳細・ローカルチェックアウト (`src/ui/views/prs.rs`)。
    3. Actions ワークフロー履歴 & 仮想スクロールログ閲覧 (`src/ui/views/actions.rs`)。
    4. 通知・セキュリティアラート閲覧 (`src/ui/views/alerts.rs`)。
    5. AI PR ドラフト生成 (`src/ai/pr.rs`) & CI 失敗原因解析 (`src/ai/ci.rs`)。
  - **Phase 5 (高度機能、コンフリクト解消、リベース & コマンドパレット)**:
    1. ビジュアルコンフリクト解消モーダル（Current vs Incoming vs AI スマートマージ）(`src/git/conflict.rs`, `src/ui/modals/conflict_resolver.rs`, `src/ai/merge.rs`)。
    2. クイック・インタラクティブ・リベース（スカッシュ、リワード、フィックスアップ）(`src/git/rebase.rs`)。
    3. 自然言語 Git コマンド翻訳付きコマンドパレット (`src/ui/modals/command_palette.rs`, `src/ai/command.rs`)。
    4. 外部エディタ (`$EDITOR`) 起動用ターミナル一時脱出・復帰 (`src/services/terminal.rs`)。
  - **Phase 6 (インストーラー、デスクトップランチャー統合、設定メニュー改善 & 完全バイリンガルドキュメント)**:
    1. リリースビルド、SVG アイコン登録、FreeDesktop `.desktop` エントリ (`Terminal=true`) によるデフォルトターミナル自動起動インストーラー (`scripts/install.sh`)。
    2. クリーンなアンインストーラー (`scripts/uninstall.sh`) および Windows 向けスクリプト (`scripts/install.ps1`)。
    3. レスポンシブ拡大幅（最大96桁×34行）、マウスホイール・キーボードスクロール対応の快適な設定メニュー (`src/ui/modals/settings.rs`)。
    4. 完全な日英ドキュメント (`README.md` & `README.ja.md`)。
    5. 全66件の自動テストスイートによる品質検証完了。
  - **Phase 7 (モダンGUIアプリ風のリッチ＆ラグジュアリーなUI刷新)**:
    1. Nerd Font角丸ピル・カプセルグリフ (`` / ``) を活用したカプセルバッジエンジン (`src/ui/capsule.rs`) の導入。
    2. ヘッダーバーの完全カプセル化（`PROWLY` ブランドピル、リポジトリパス、ブランチ・同期差分スプリットピル、AIステータス、アクションボタン）。
    3. macOSセグメントコントロール風のタブバー（選択中タブのハイライトピルと非アクティブタブの上品なカプセルデザイン）。
    4. アクティブペインの発光インジケータ（`● ACTIVE`）と丸角ボーダー。
    5. GitHub Desktop風のDiff増減比率ゲージバー（`+N -M ■■■■□`）およびファイル状態カラーピル (`[M]`, `[+]`, `[-]`)、選択中カーソルバー (`▎`)。
    6. デスクトップアプリ風の最下部ステータスバー（`NORMAL` モードピル、キーボードショートカット用キーキャップバッジ `[Tab] Panes`, `[Space] Stage`、AI・テーマ・言語バッジ）。
  - **Phase 8 (3層積層コマンドパレット、110+ Gitカタログ、Tabフラグ補完 & セーフティガード)**:
    1. 3層積層アーキテクチャの導入（1層: Prowly内蔵機能、2層: 110+標準Git CLIサブコマンドカタログ、3層: 引数直接入力 & 代表フラグプレビュー）(`src/ui/modals/git_subcommands.rs`, `src/ui/modals/command_palette.rs`)。
    2. 13件制限のない全件仮想スクロールおよびヘッダーボタン `[ 󰘳 Commands ]` の新設。
    3. パンくずリスト表示（`Command Palette ❯ git ❯ <subcommand>`）およびプロンプトコンテキスト表示。
    4. 階層ナビゲーション: Escキーおよび空入力状態でのBackspaceによる親階層ロールバック。
    5. ダイレクトショートカット: 第1層からの `git cherry-pick --continue` や `cherry-pick` 直打ちジャンプ。
    6. セーフティガード: `push --force` / `-f`、`reset --hard`、`clean -fdx`、`branch -D` 等の破壊的操作に対する警告および2段階Enter確認プロテクション。
    7. 第3層におけるTabキーによる代表フラグの巡回自動補完。
    8. 全機能網羅の手動テストチェックリスト作成 (`MANUAL_TEST_CHECKLIST.md`) および全70件の自動テストスイートによる品質検証完了。

---

## 5. 手動テスト計画・品質検証 (Manual Testing & QA)

- **手動テストチェックリスト**: [MANUAL_TEST_CHECKLIST.md](file:///home/susie/git/wammed/Prowly/MANUAL_TEST_CHECKLIST.md)
  - 全15カテゴリ・61大項目にわたる網羅的なテストケースを策定。
  - **重点検証項目**:
    1. **外部エディタ連携 & ターミナル一時脱出・復帰**:
       - VS Code (`code -w`), Cursor, Neovim, Nano 等でファイルを開いた際、本文が正常に表示されること。
       - エディタ終了後にProwlyへ戻った際、端末のRAWモード・マウスキャプチャ・画面描画が100%崩れず復帰すること。
    2. **端末互換性 (Terminal Compatibility)**:
       - GNOME TerminalやWindows既定コンソール等、端末側で右クリックが横取りされる環境でも、左クリック `[ 󰍜 Menu ▾ ]` ボタンおよびキーボード `m` / `F10` で確実に操作が完結すること。
    3. **3層積層コマンドパレットの階層遷移 & 安全性**:
       - 1層（Prowly操作）⇄ 2層（110+ Gitカタログ）⇄ 3層（引数入力 & Tabフラグ補完）のEsc / Backspaceロールバック。
       - 破壊的操作 (`push --force`, `reset --hard`, `clean -fdx`, `branch -D`) に対するセーフティガード警告と2段階Enter確認。
    4. **ディレクトリマップ & 一括ステージング**:
       - 変更フォルダの自動展開、変更数バッジ集計、フォルダクリックによるファイル一覧絞り込み。
    5. **AI コパイロット連携**:
       - 各画面（Changes, PRs, Actions, Conflict）におけるコンテキスト自動注入と提案コードのパッチ適用。
- **現在のテストステータス**:
  - 自動テスト（ユニット・結合・UI遷移テスト）: **70件全件合格 (`cargo test`)**。
  - 手動テスト: チェックリスト作成完了。リリース前の実機検証を順次実施中。

---

## 6. 次期開発構想：GUI Wrapper (デスクトップ版 Prowly)

### 6.1 背景と狙い
テスト完走および機能追加の安定化後、Prowlyのコアコンセプトである「GitHub Desktop の上位互換」をさらに推し進めるため、**デスクトップGUIラッパー（GUI版 Prowly）**の作成に着手する計画。

### 6.2 既存アーキテクチャの強み
Prowlyはすでにビジネスロジック（`src/git/`, `src/github/`, `src/ai/`, `src/config/`, `src/i18n/`）がUI描画部（Ratatui/Crossterm）と綺麗に分離されているため、コアエンジンを共有したまま新規GUIフロントエンドを構築可能。

### 6.3 技術選定と推奨アーキテクチャ
* **第一推奨: Tauri 2.0 (Rust + Webフロントエンド)**
  * **選定理由**:
    - **軽量・高速**: Electronと異なりメモリ消費が数十MBと極小、起動が爆速。
    - **最高峰のデザイン性**: モダンCSS、グラスモーフィズム、滑らかなアニメーション、Monaco EditorやリッチなHTML/Canvas Diffビューアの活用が可能。
    - **Rustネイティブ連携**: 既存のRust製コアロジックをTauriコマンド（IPC）経由でゼロオーバーヘッド連携。
  * **フロントエンド候補**: Svelte 5 / React / Vue 3 + TailwindCSS / Vanilla CSS。
* **オルタナティブ（検討対象）**:
  - ピュアRust GUI（**Slint** または **Iced** / **egui**）: 単一バイナリ・Web技術不要の利点があるが、リッチなDiffハイライトやMarkdownのレイアウト表現コストを比較検討。

### 6.4 マルチクレート・ワークスペース移行ロードマップ
1. **ステップ 1: ワークスペース化 & コアロジック分離**
   - `crates/prowly-core/`: Gitエンジン、GitHub API、AIクライアント、設定、国際化ロジック
   - `crates/prowly-tui/`: 現在のRatatuiターミナル版（CLIツールとして継続提供）
   - `crates/prowly-gui/`: 新規Tauriデスクトップアプリ版
2. **ステップ 2: GUIプロトタイプ開発**
   - ディレクトリマップ ＋ Changesビュー（差分表示 & ステージング）の最小構成GUIを実装。
3. **ステップ 3: 全機能のGUI移植**
   - PR、Actions、3層コマンドパレット、AIアシスタントのGUIコンポーネント化。
4. **ステップ 4: クロスプラットフォーム配布**
   - Linux (AppImage, deb), macOS (.dmg, .app), Windows (.msi, .exe) のインストーラーパッケージング。

