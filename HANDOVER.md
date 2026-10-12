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

- **Current Status**: **All Phases (Phase 0 through Phase 6) Completed Successfully**.
- **Automated Test Suite**: **66 Automated Tests Passing (100% Pass Rate)** across all unit and integration test suites.
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
    3. Floating right-click context menu modal (`src/ui/modals/context_menu.rs`).
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
