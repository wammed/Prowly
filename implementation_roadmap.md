# Prowly 実装ロードマップ (拡張版)

本書は [Design.md](file:///home/susie/git/wammed/Prowly/Design.md) および追加要件に基づき、TUI Git/GitHubクライアント「**Prowly**」を実装するための詳細なロードマップです。

---

## 1. 追加要件 & 設計方針

| 項目 | 設計方針 |
| :--- | :--- |
| **Nerd Font & リッチUI** | Nerd Fontグリフ（ファイル種別、Gitステータス、ブランチ、PR、Actions、AI等）をフル活用した圧倒的に美しいTUI。<br>フォント自体は同梱せず、ユーザーのターミナルフォント環境を活用。境界線（Borders）、グラデーション、ステータスバッジの緻密なスタイリング。 |
| **20種類のテーマカラー** | 人気の配色スキーム（Catppuccin 4種, Tokyo Night 3種, Nord, Dracula, Gruvbox 2種, Solarized 2種, Rosé Pine 3種, One Dark, Monokai Pro, Synthwave '84, Everforest 等）を20種プリセット実装。アプリ内から即時切り替え可能。 |
| **完全な多言語対応 (i18n)** | 全UI・メニュー・ヘルプ・ポップアップ・通知・ドキュメントを**英語 (EN)** および **日本語 (JA)** の両方で提供。<br>キーバインドまたはコマンドパレットからアプリ再起動なしでリアルタイム切り替え。 |
| **充実した自動テスト** | ユニットテスト、Git操作結合テスト（一時Gitリポジトリ生成）、UIステート遷移テスト、キー/マウスイベントテスト、テーマ/i18n完全性テスト（未翻訳キー検知）を網羅。 |

---

## 2. 全体アーキテクチャ & モジュール構成

```mermaid
graph TD
    subgraph Presentation [プレゼンテーション層 (Ratatui / Crossterm)]
        UI_Components[UIコンポーネント (Header, Tree, Diff, Modals...)]
        Theme_Engine[テーマエンジン (20種カラーパレット)]
        I18n_Engine[i18n エンジン (日英即時切替)]
        Icon_Module[Nerd Font アイコンモジュール]
    end

    subgraph State_Event [状態管理 & イベントループ]
        AppState[App State / Store]
        Event_Bus[Tokio MPSC 非同期イベントバス]
        Action_Dispatcher[アクション・ディスパッチャ]
    end

    subgraph Core_Services [コアサービス層]
        Git_Service[Git サービス (git2 / gix)]
        GitHub_Service[GitHub サービス (octocrab)]
        AI_Service[黒子AI サービス (BYOK)]
        External_Process[外部プロセス連携 (エディタ/シェル)]
    end

    subgraph Test_Suite [自動テストスイート]
        Unit_Tests[ユニットテスト]
        Git_Integration[一時リポジトリ結合テスト]
        UI_State_Tests[状態遷移 & イベントテスト]
        I18n_Lint[未翻訳キー・構文検証]
    end

    UI_Components --> Theme_Engine
    UI_Components --> I18n_Engine
    UI_Components --> Icon_Module
    UI_Components --> AppState

    Event_Bus --> Action_Dispatcher --> AppState
    AppState --> Git_Service
    AppState --> GitHub_Service
    AppState --> AI_Service
    AppState --> External_Process
```

---

## 3. 20種類のプリセットカラーテーマ一覧

1. **Catppuccin Mocha** (ダーク・パステル高コントラスト)
2. **Catppuccin Macchiato** (ミディアムダーク)
3. **Catppuccin Frappe** (ソフトダーク)
4. **Catppuccin Latte** (クリーンライト)
5. **Tokyo Night** (洗練された深いネイビー・ネオン)
6. **Tokyo Night Storm** (落ち着いた青灰色ダーク)
7. **Tokyo Night Light** (明るくモダンなライト)
8. **Nord** (北欧風の氷雪・アークティックブルー)
9. **Dracula** (鮮やかなパープル・ピンク系ダーク)
10. **Gruvbox Dark** (温かみのあるレトロダーク)
11. **Gruvbox Light** (温かみのあるレトロライト)
12. **Solarized Dark** (目に優しい黄緑・シアン系ダーク)
13. **Solarized Light** (伝統的で視認性の高いライト)
14. **One Dark (Atom)** (親しみやすいモダンダーク)
15. **One Light** (ミニマルで明るいライト)
16. **Rosé Pine** (くすみピンク・ヴィンテージダーク)
17. **Rosé Pine Moon** (クールダーク)
18. **Rosé Pine Dawn** (温かいソフトライト)
19. **Monokai Pro** (クラシックな開発者向け高コントラスト)
20. **Synthwave '84** (サイバーパンク・ネオン・レトロフューチャー)
21. **Everforest Dark** (自然と調和するナチュラルグリーンダーク)

---

## 4. 多言語対応 (i18n) アーキテクチャ

- **リソース管理**: `assets/locales/en.toml` および `assets/locales/ja.toml` によるキーベース管理（または Rust 型安全マクロ）。
- **完全性バリデーション**: テスト時に `en` と `ja` の全キーが 1:1 で一致しているかを自動検査（未翻訳の混入をCIで防止）。
- **リアルタイム反映**: ユーザーが設定またはショートカット（`Ctrl+L` やコマンドパレット）で言語を切り替えた瞬間、全UIウィジェットの描画テキストが再評価・即時更新。
- **ドキュメント・ヘルプ**: 
  - README / オンボーディングガイド（日英）
  - キーバインド一覧 & 操作ガイド（日英）
  - 各種エラーメッセージ・確認ダイアログ（日英）

---

## 5. フェーズ別実装ロードマップ

### Phase 0: プロジェクト基盤、テーマ・i18n・アイコン基盤
> **目標**: 堅牢なプロジェクト設計、デザインシステム（20種テーマ＋Nerd Font）、i18nエンジンの構築

- [ ] **プロジェクト初期化 & 依存選定**:
  - `ratatui`, `crossterm`, `tokio`, `git2`, `octocrab`, `syntect`, `termimad`, `serde`, `toml`
- [ ] **Nerd Font アイコンモジュール (`ui/icons.rs`)**:
  - Git/GitHub/ファイルタイプ/UI装飾用の高解像度Nerd Font定数定義
- [ ] **20種類テーマエンジン (`ui/theme.rs`)**:
  - テーマカラー構造体定義（Background, Foreground, Accent, DiffAdd, DiffRemove, Border, etc.）
  - 20種類以上のテーマプリセット実装 & 実行時切り替え機構
- [ ] **i18n エンジン (`i18n/mod.rs`)**:
  - 日英辞書定義、翻訳マクロ `t!("key")`、動的言語切替機構
  - CI用: 日英キー一致チェックテスト
- [ ] **設定管理 (`config/mod.rs`)**:
  - `config.toml` の読み書き（テーマ設定、ロケール設定、エディタ設定）

---

### Phase 1: TUI エンジン、レイアウト、マウス/キー入力 & テスト
> **目標**: リッチな外観とマウス/キー完全対応のUIシェル、自動テスト基盤

- [ ] **非同期イベントループ**:
  - `crossterm` 入力（キー / マウス左クリック / 右クリック / ドラッグ / ホイール）
  - UI描画レート（Tick）とバックグラウンドタスク通信
- [ ] **画面分割レイアウト (`ui/layout.rs`)**:
  - ヘッダー（アイコン付きリポジトリ情報・ステータス）
  - 左ペイン（ディレクトリマップ）
  - 右ペイン（メインタブ: Changes, PRs, Actions, Alerts）
  - フッター（コンテキストヘルプ、言語・テーマ状態）
- [ ] **UIコンポーネント基盤**:
  - ボタン（クリック可能エリア判定）、チェックボックス、タブバー
  - モーダル / ポップアップ（Clear重ね合わせ描画）
- [ ] **テーマ / 言語セレクター UI**:
  - アプリ内ポップアップでテーマ20種プレビュー切り替え
  - 英語 ⇄ 日本語ワンタッチ切り替え
- [ ] **テスト項目**:
  - キーイベントディスパッチテスト
  - マウスクリックヒットテスト（ペイン境界、ボタン座標判定）
  - テーマカラーコントラスト & ロードテスト

---

### Phase 2: ローカル Git コア機能 & Changes ビュー
> **目標**: 日常の開発フロー（差分確認、ステージング、コミット、Stash）の実装

- [ ] **Git サービス層 (`git/service.rs`)**:
  - 一時Gitリポジトリを用いたテスト可能な抽象化
  - ステータス一覧（M, U, D, R, C）、ブランチ情報取得
- [ ] **Changes ビュー (`ui/views/changes.rs`)**:
  - ファイルリスト（チェックボックスで個別/一括ステージング）
  - コミットメッセージ入力ボックス
  - [ Commit & Sync ] アクション（Commit + Push）
  - [ Stash All ] / [ Pop Stash ]
- [ ] **リッチ Diff ビュー (`ui/views/diff.rs`)**:
  - `syntect` による言語自動判別シンタックスハイライト
  - 差分追加行（薄緑）、削除行（薄赤）の背景ハイライト
  - マウスホイールによる快適スクロール
- [ ] **テスト項目**:
  - 一時リポジトリでのステージ/アンステージ動作テスト
  - コミット作成 & ブランチ切り替えテスト
  - Diffパースエンジンのエッジケーステスト

---

### Phase 3: ディレクトリマップ & ビジュアル操作強化
> **目標**: 変更密度がひと目でわかるディレクトリマップと、右クリックメニュー等の実装

- [ ] **ディレクトリマップ (左ペイン) (`ui/views/dir_map.rs`)**:
  - 変更ファイルを含むディレクトリのみをツリー展開
  - フォルダ横の変更件数バッジ（`[2]` など）
  - フォルダクリックによる右ペインファイル一覧の絞り込み
  - 非Gitディレクトリ時の案内（`[ Init ]`, `[ Clone ]`）
- [ ] **ヘッダーインタラクション**:
  - ブランチ名クリックでファジーブランチ切替ポップアップ
  - 未Push/未Pull（`↑1 ↓0`）クリックでコミット一覧ドロップダウン
- [ ] **右クリックコンテキストメニュー (`ui/modals/context_menu.rs`)**:
  - 右クリック または `m` / `F10` で起動
  - [ Open in Editor ], [ Reveal in File Manager ], [ Discard changes ]
- [ ] **テスト項目**:
  - ディレクトリツリー構築ロジックの単体テスト（深い階層、空ディレクトリ）
  - フィルタリング連動テスト

---

### Phase 4: GitHub API 統合 (PRs / Actions / Alerts)
> **目標**: リモート GitHub の全機能をターミナル内で快適に操作

- [ ] **GitHub 認証 & API クライアント (`github/client.rs`)**:
  - `gh auth token` 自動検出 ＆ PAT設定フォールバック
- [ ] **PRs タブ (`ui/views/prs.rs`)**:
  - PR一覧（レビュー待ち、担当PR）
  - PR詳細・Markdownリッチレンダリング（`termimad`）
  - [ Checkout locally ]（作業ツリー自動Stash保護 → PRブランチチェックアウト）
- [ ] **Actions タブ (`ui/views/actions.rs`)**:
  - ワークフロー実行履歴・ステータス
  - ジョブ別ログストリーミング閲覧 & ログ内検索（`/`）
  - [ Rerun Failed Jobs ]（`r` キー）
- [ ] **Alerts タブ (`ui/views/alerts.rs`)**:
  - 通知一覧 & ワンクリック既読化（`d` キー）
- [ ] **テスト項目**:
  - GitHub APIレスポンスパースのモックテスト
  - PRチェックアウト時の安全なStash復元テスト

---

### Phase 5: 高度機能 & エスケープハッチ
> **目標**: ビジュアルコンフリクト解消、コマンドパレット、ターミナル一時脱出

- [ ] **ビジュアル・コンフリクト解決モード**:
  - コンフリクト検出（`⚠ CONFLICT` バッジ）
  - 対比ビュー（Current vs Incoming）
  - [ Accept Current ], [ Accept Incoming ], [ Accept Both ] ワンクリック解決
- [ ] **コマンドパレット (`:` / `Ctrl+P`)**:
  - 全アクション・Git/GitHubコマンドのファジー検索（日英両言語での検索対応）
  - コマンド実行結果のインライン表示
- [ ] **ターミナル一時脱出 (Suspend / Resume)**:
  - 外部エディタ起動時および対話型コマンド実行時のRawモード解除と復帰
- [ ] **テスト項目**:
  - コンフリクトマーカーパース & マージ解決ロジックテスト
  - コマンドパレットのファジーマッチングテスト

---

### Phase 6: 全面統合 黒子AI & 総合テスト・仕上げ
> **目標**: 画面文脈を自動注入するAIアシスタント、全テスト通過、ドキュメント日英完備

- [ ] **黒子AI アシスタント (`ai/mod.rs`)**:
  - 画面文脈（Diff, Actionsログ, PR, コンフリクト）の自動抽出 & プロンプト注入
  - BYOK (OpenAI, Anthropic, Gemini, Ollama) 対応
  - 安全承認制（`[ Apply ]` ボタンで変更反映）
- [ ] **多言語ドキュメント整備**:
  - README (英語・日本語)
  - 操作マニュアル & ヘルプモーダル (英語・日本語)
- [ ] **自動テストスイートの総合拡充**:
  - 全モジュールの単体テストカバレッジ向上
  - 未翻訳キー検知テスト（CI自動検証）
  - クロスプラットフォーム検証（パス処理、改行コード処理）
