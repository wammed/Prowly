# Project Handover Document (HANDOVER) - Prowly

Last Updated: 2026-10-12

---

## 1. Project Overview & Concept

- **Project Name**: Prowly
- **Core Concept**: "A terminal-based superior alternative to GitHub Desktop, with an omnipresent and intelligent AI co-pilot."
- **Dual-Input UX**: Fully functional using only the mouse, blazingly fast using the keyboard.
- **Language & Platform**: Rust (Cross-platform: Linux / macOS / Windows)
- **Primary Design Documents**:
  - Specification: [Design.md](file:///home/susie/git/wammed/Prowly/Design.md)
  - Implementation Roadmap: [implementation_roadmap.md](file:///home/susie/git/wammed/Prowly/implementation_roadmap.md)
  - User Documentation: [README.md](file:///home/susie/git/wammed/Prowly/README.md) / [README.ja.md](file:///home/susie/git/wammed/Prowly/README.ja.md)

---

## 2. Established System Architecture & Design Decisions

| Category | Decision & Implementation Approach |
| :--- | :--- |
| **UI Rendering & Input** | `ratatui` + `crossterm`. Seamless combination of full mouse support (click, right-click, drag, scroll wheel) and keyboard shortcuts. |
| **Async Engine** | `tokio`. UI rendering thread is cleanly decoupled from Git/GitHub API/AI background tasks using mpsc channels. |
| **Git Engine** | **Hybrid Architecture (`git2` + local `git` CLI)**.<br>Status inspection, diff parsing, and tree traversal are handled in-process for peak speed.<br>SSH agent auth, credential helpers, GPG signing, and push/pull are safely delegated to the local `git` CLI to eliminate authentication hurdles. |
| **Performance** | **Virtual Scrolling & LRU Cache**.<br>Even with tens of thousands of diff/CI log lines, only the lines within the viewport are highlighted on-demand via `syntect` and rendered, maintaining a fluid 60 FPS. |
| **Omnipresent AI** | **First-Class Citizen throughout the application**.<br>Accessible everywhere (Diff, PR, CI logs, Conflicts, Command Palette, Header, Errors) via `Ctrl+A` / `[ 󱚣 Ask AI ]`.<br>Automatically injects screen context (selection, diffs, raw logs) and requires safety confirmation (`[ Apply ]`).<br>BYOK (OpenAI, Anthropic, Gemini, Ollama, Custom) multi-provider support. Local Ollama needs no API key and communicates directly with native `/api/chat` (auto-normalizing port-only URLs). |
| **Safety Net** | **Soft-Discard Mechanism**. Automatically creates hidden stashes / temporary backups before executing changes discard to prevent accidental loss of code. |
| **Git Extensions** | Git Worktree management, GPG/SSH commit signature verification badges, visual conflict resolution, and quick interactive rebase (Fixup / Squash / Reword). |
| **Design System** | High-definition Nerd Font glyphs with an automatic ASCII/simple-Unicode fallback for non-Nerd Font environments. 21 preconfigured rich color themes. |
| **Internationalization (i18n)** | Full English (EN) and Japanese (JA) coverage across all UI, menus, help, popups, notifications, AI prompts/responses, and docs. Supports pluralization. |
| **Clipboard Integration** | Native OS clipboard integration via the `arboard` crate (`y` key). |
| **Desktop Launcher Integration** | FreeDesktop `.desktop` entry with `Terminal=true` + scalable icon (`images/Prowly-Icon.svg`) automatically launching inside the system default terminal emulator. |
| **API Rate-Limit Protection** | Exponential backoff, HTTP `ETag` caching, and explicit manual refresh (`R` key). |

---

## 3. Project Workflow & Operational Rules

1. **Continuous Handover Updates**:
   - Whenever specifications change, milestones advance, or major architectural decisions are made, this handover document (`HANDOVER.md` and `HANDOVER.ja.md`) must be kept up to date.
2. **Bilingual Documentation**:
   - All documentation (README, specifications, manuals, handover notes, etc.) must be provided in both **English and an identical Japanese version**.
3. **Proactive Proposals**:
   - As implementation proceeds, any potential improvements, architecture refinements, or recommended features discovered should be actively proposed to the user for decision.
4. **Comprehensive Automated Testing**:
   - Unit tests, integration tests with temporary Git repositories, UI state machine tests, and untranslated i18n key detection tests must be maintained at each phase.

---

## 4. Current Status & Milestone Achievements

- **Current Status**: **All Phases (Phase 0 through Phase 8) Completed Successfully**.
- **Automated Test Suite**: **70 Automated Tests Passing (100% Pass Rate)** across all unit and integration test suites.
- **Architectural Note on Async Runtime**: Background GitHub and AI operations are fully non-blocking and decoupled via `AppEvent` channels and `tokio::spawn`, eliminating runtime nesting panics (`block_on` inside Tokio).
- **Phase-by-Phase Achievements**:
  - **Phase 0 (Foundations)**:
    1. Dependencies in `Cargo.toml`.
    2. 21 Color Themes & dynamic cycling engine (`src/ui/theme.rs`).
    3. Nerd Font & ASCII/Unicode fallback icon engine (`src/ui/icons.rs`).
    4. Bilingual i18n engine with pluralization & placeholder parity (`src/i18n/mod.rs`).
    5. BYOK AI multi-provider client abstraction (`src/ai/mod.rs`).
    6. TOML Configuration persistence (`src/config/mod.rs`).
  - **Phase 1 (Core TUI & Virtual Scrolling)**:
    1. Async event loop with keyboard and full mouse hit testing (`src/event.rs`, `src/ui/layout.rs`).
    2. High-performance virtual scrolling with LRU highlight cache (`src/ui/virtual_scroll.rs`).
    3. Omnipresent AI Assistant modal overlay (`src/ui/modals/ai_assistant.rs`).
    4. OS Clipboard integration (`src/services/clipboard.rs`).
    5. Unified application state (`src/app.rs`).
  - **Phase 2 (Changes, Safety & AI Review)**:
    1. Soft-discard safety net (`src/services/backup.rs`): automatic timestamped backup before discarding.
    2. Git hybrid service (`src/git/service.rs`).
    3. Changes view with staging checkboxes & commit message editor (`src/ui/views/changes.rs`).
    4. Syntect-powered diff view (`src/ui/views/diff.rs`).
    5. AI Conventional Commit generation (`Ctrl+G`) & pre-commit code review (`Ctrl+R`) (`src/ai/commit.rs`).
  - **Phase 3 (Directory Map, Worktrees & AI Diagnostic)**:
    1. Directory Map component with tree aggregation & count badges (`src/ui/views/dir_map.rs`).
    2. Native Git Worktree manager & interactive view (`src/git/worktree.rs`, `src/ui/views/worktrees.rs`).
    3. Floating context menu modal with terminal compatibility design (includes left-click `[ 󰍜 Menu ▾ ]` button and `m` key fallback for 100% operation in GNOME Terminal and desktop environments that intercept right-clicks) (`src/ui/modals/context_menu.rs`).
    4. Repository AI Health Diagnostic (`Clean`, `NeedsCommit`, `Conflicted`) (`src/ai/diagnostic.rs`).
  - **Phase 4 (GitHub Deep Integration)**:
    1. GitHub Client with TTL cache, rate-limit protection, and mock support (`src/github/client.rs`).
    2. Pull Requests view with Markdown descriptions & local checkout (`src/ui/views/prs.rs`).
    3. Actions view with workflow status & step-by-step logs (`src/ui/views/actions.rs`).
    4. Alerts & Notifications view (`src/ui/views/alerts.rs`).
    5. AI PR draft generation (`src/ai/pr.rs`) and AI CI failure root cause analysis (`src/ai/ci.rs`).
  - **Phase 5 (Advanced Features, Conflicts, Rebase & Command Palette)**:
    1. Visual Conflict Resolution modal with Current vs Incoming vs AI Smart Merge (`src/git/conflict.rs`, `src/ui/modals/conflict_resolver.rs`, `src/ai/merge.rs`).
    2. Quick Interactive Rebase with squash, reword, and fixup (`src/git/rebase.rs`).
    3. Fuzzy Command Palette with natural language to Git command translation (`src/ui/modals/command_palette.rs`, `src/ai/command.rs`).
    4. Terminal Suspend / Resume escape hatch for `$EDITOR` (`src/services/terminal.rs`).
  - **Phase 6 (Installers, Desktop Launcher, Settings Modal Improvements & Bilingual Documentation)**:
    1. Automated installation script (`scripts/install.sh`) building release binary, registering scalable SVG icon, and generating FreeDesktop `.desktop` entry with `Terminal=true` for default system terminal launch.
    2. Uninstallation script (`scripts/uninstall.sh`) and Windows PowerShell script (`scripts/install.ps1`).
    3. Generous responsive sizing (up to 96 columns x 34 rows) with mouse wheel and keyboard scrolling for the Settings modal (`src/ui/modals/settings.rs`).
    4. Full bilingual documentation (`README.md` & `README.ja.md`).
    5. Complete automated test suite coverage with 66 tests passing.
  - **Phase 7 (Modern GUI-Like Rich & Luxurious Aesthetic Elevation)**:
    1. Capsule and pill badge rendering engine leveraging rounded Nerd Font powerline glyphs (`` / ``) with fallback (`src/ui/capsule.rs`).
    2. Fully encapsulated top navigation bar (`PROWLY` brand badge, repository breadcrumb, branch & sync status split pill, AI status badge, desktop button capsules).
    3. macOS Segmented Control-style tab bar with vibrant highlighted active pill and muted inactive capsules.
    4. Pane focus accent and glowing corner indicator (`● ACTIVE`) on rounded window frames.
    5. GitHub Desktop-style visual diff addition/deletion ratio gauge (`+N -M ■■■■□`), file status pills (`[M]`, `[+]`, `[-]`), and active row cursor bar (`▎`).
    6. Native desktop-style bottom status bar with `NORMAL` mode badge, rounded keycap shortcuts (`[Tab] Panes`, `[Space] Stage`), AI co-pilot status, theme, and language chips.
  - **Phase 8 (3-Layer Hierarchical Command Palette, 110+ Git Catalog, Tab Flag Completion & Safety Guard)**:
    1. 3-layer hierarchical architecture (Layer 1: Prowly Native Actions, Layer 2: 110+ Standard Git CLI Subcommands Catalog, Layer 3: Direct Arguments Input with Flag Previews) (`src/ui/modals/git_subcommands.rs`, `src/ui/modals/command_palette.rs`).
    2. Virtual scrolling beyond 13 items with header button `[ 󰘳 Commands ]`.
    3. Breadcrumbs title (`Command Palette ❯ git ❯ <subcommand>`) and dynamic prompt context indicators.
    4. Hierarchical navigation: Esc key and empty-query Backspace parent layer rollback.
    5. Direct shortcut jumps: Direct input of `git cherry-pick --continue` or `cherry-pick` from Layer 1.
    6. Safety Guard: Detection and warning on destructive operations (`push --force` / `-f`, `reset --hard`, `clean -fdx`, `branch -D`) requiring explicit 2-step Enter confirmation.
    7. Tab key flag autocompletion in Layer 3.
    8. Comprehensive manual test checklist (`MANUAL_TEST_CHECKLIST.md`) and all 70 automated tests passing.

---

## 5. Manual Testing & Quality Assurance (QA)

- **Comprehensive Checklist**: [MANUAL_TEST_CHECKLIST.md](file:///home/susie/git/wammed/Prowly/MANUAL_TEST_CHECKLIST.md)
  - Covers 61 test scenarios across 15 operational categories.
  - **Key Focus Areas**:
    1. **External Editor & Terminal Suspend/Resume**:
       - Verify that files open correctly with full contents in external editors (VS Code `code -w`, Cursor, Neovim, Nano).
       - Ensure terminal RAW mode, mouse capture, and UI layout restore with zero rendering corruption upon editor exit.
    2. **Terminal Interception Compatibility**:
       - Ensure 100% full controllability via left-click `[ 󰍜 Menu ▾ ]` button and `m` / `F10` key in environments where terminals intercept right-clicks (GNOME Terminal, default Windows console).
    3. **3-Layer Command Palette Transitions & Safety**:
       - Validate Layer 1 (Root) ⇄ Layer 2 (110+ Git Catalog) ⇄ Layer 3 (Args Input & Tab completion) transitions and Esc/Backspace rollbacks.
       - Enforce safety guard warnings and two-step Enter confirmations for destructive operations (`push --force`, `reset --hard`, `clean -fdx`, `branch -D`).
    4. **Directory Map & Staging Aggregation**:
       - Verify modification count badges, tree auto-expansion, and folder click filtering.
    5. **Omnipresent AI Assistant**:
       - Validate context auto-injection across all tabs and interactive patch application.
- **Current Quality Status**:
  - Automated Tests: **All 70 tests passing (`cargo test`)**.
  - Manual Tests: Checklist established; manual verification active prior to release.

---

## 6. Future Roadmap: GUI Wrapper (Desktop Prowly)

### 6.1 Motivation & Vision
Once testing and feature additions stabilize, the next major evolution is creating a **Desktop GUI Wrapper** to realize the core vision of "surpassing GitHub Desktop" with a modern native desktop window.

### 6.2 Architectural Advantages of Current Codebase
Prowly's business logic (`src/git/`, `src/github/`, `src/ai/`, `src/config/`, `src/i18n/`) is decoupled from the Ratatui terminal rendering layer, allowing the engine to be directly shared with a GUI frontend.

### 6.3 Recommended Technology: Tauri 2.0 (Rust + Web)
* **Primary Recommendation: Tauri 2.0**
  * **Key Benefits**:
    - **Ultra-lightweight & performant**: Minimal memory footprint (tens of MBs vs hundreds in Electron), instant startup.
    - **Premium visual design**: Modern CSS, glassmorphism, fluid animations, and rich HTML/Canvas diff & markdown renderers.
    - **Zero-overhead Rust IPC**: Direct invocation of core Git, GitHub, and AI services.
  * **Frontend Options**: Svelte 5 / React / Vue 3 with TailwindCSS or Vanilla CSS.
* **Alternative Considered**: Pure-Rust GUI frameworks (**Slint**, **Iced**, or **egui**) — evaluated for single-binary benefits against the effort of custom rich diff/markdown layouting.

### 6.4 Multi-Crate Workspace Roadmap
1. **Step 1: Workspace Refactoring & Core Decoupling**
   - `crates/prowly-core/`: Shared Git engine, GitHub client, AI services, config, and i18n logic.
   - `crates/prowly-tui/`: Current Ratatui terminal application (maintained as high-speed CLI tool).
   - `crates/prowly-gui/`: New Tauri desktop GUI application.
2. **Step 2: Prototype Development**
   - Implement minimal GUI containing the Directory Map and Changes view (staging & diff view).
3. **Step 3: Feature Porting & Parity**
   - Integrate PRs, Actions logs, 3-layer Command Palette, and AI Co-Pilot into GUI components.
4. **Step 4: Cross-Platform Packaging**
   - Produce native installers for Linux (AppImage, deb), macOS (.dmg, .app), and Windows (.msi, .exe).


