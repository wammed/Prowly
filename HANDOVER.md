# Project Handover Document (HANDOVER) - Prowly

Last Updated: 2026-10-11

---

## 1. Project Overview & Concept

- **Project Name**: Prowly
- **Core Concept**: "A terminal-based superior alternative to GitHub Desktop, with an omnipresent and intelligent AI co-pilot."
- **Dual-Input UX**: Fully functional using only the mouse, blazingly fast using the keyboard.
- **Language & Platform**: Rust (Cross-platform: Linux / macOS / Windows)
- **Primary Design Documents**:
  - Specification: [Design.md](file:///home/susie/git/wammed/Prowly/Design.md)
  - Implementation Roadmap: [implementation_roadmap.md](file:///home/susie/git/wammed/Prowly/implementation_roadmap.md)

---

## 2. Established System Architecture & Design Decisions

| Category | Decision & Implementation Approach |
| :--- | :--- |
| **UI Rendering & Input** | `ratatui` + `crossterm`. Seamless combination of full mouse support (click, right-click, drag, scroll wheel) and keyboard shortcuts. |
| **Async Engine** | `tokio`. UI rendering thread is cleanly decoupled from Git/GitHub API/AI background tasks using mpsc channels. |
| **Git Engine** | **Hybrid Architecture (`git2` + local `git` CLI)**.<br>Status inspection, diff parsing, and tree traversal are handled in-process for peak speed.<br>SSH agent auth, credential helpers, GPG signing, and push/pull are safely delegated to the local `git` CLI to eliminate authentication hurdles. |
| **Performance** | **Virtual Scrolling & LRU Cache**.<br>Even with tens of thousands of diff/CI log lines, only the lines within the viewport are highlighted on-demand via `syntect` and rendered, maintaining a fluid 60 FPS. |
| **Omnipresent AI** | **First-Class Citizen throughout the application**.<br>Accessible everywhere (Diff, PR, CI logs, Conflicts, Command Palette, Header, Errors) via `?` / `Ctrl+A` / `[ 󱚣 Ask AI ]`.<br>Automatically injects screen context (selection, diffs, raw logs) and requires safety confirmation (`[ Apply ]`).<br>BYOK (OpenAI, Anthropic, Gemini, Ollama) multi-provider support. |
| **Safety Net** | **Soft-Discard Mechanism**. Automatically creates hidden stashes / temporary backups before executing changes discard to prevent accidental loss of code. |
| **Git Extensions** | Git Worktree management, GPG/SSH commit signature verification badges, and quick interactive rebase (Fixup / Squash / Reword). |
| **Design System** | High-definition Nerd Font glyphs with an automatic ASCII/simple-Unicode fallback for non-Nerd Font environments. 20 preconfigured rich color themes. |
| **Internationalization (i18n)** | Full English (EN) and Japanese (JA) coverage across all UI, menus, help, popups, notifications, AI prompts/responses, and docs. Supports pluralization. |
| **Clipboard Integration** | Native OS clipboard integration via the `arboard` crate (`y` key). |
| **API Rate-Limit Protection** | Exponential backoff, HTTP `ETag` caching, and explicit manual refresh (`R` key). |

---

## 3. Project Workflow & Operational Rules

1. **Continuous Handover Updates**:
   - Whenever specifications change, milestones advance, or major architectural decisions are made, this handover document (`HANDOVER.md` and `HANDOVER.ja.md`) must be kept up to date.
2. **Bilingual Documentation**:
   - All future documentation (README, specifications, manuals, handover notes, etc.) must be provided in both **English and an identical Japanese version**.
3. **Proactive Proposals**:
   - As implementation proceeds, any potential improvements, architecture refinements, or recommended features discovered should be actively proposed to the user for decision.
4. **Comprehensive Automated Testing**:
   - Unit tests, integration tests with temporary Git repositories, UI state machine tests, and untranslated i18n key detection tests must be maintained at each phase.

---

## 4. Current Status & Next Steps

- **Current Status**: Design and roadmap completed (Ready to begin implementation).
- **Current Phase**: **Phase 0: Project Scaffolding, Design System, Themes, i18n, Icons, & AI Abstraction**.
- **Immediate Tasks for Phase 0**:
  1. Initialize `Cargo.toml` with dependencies and configuration.
  2. Setup directory hierarchy (`src/ui`, `src/git`, `src/github`, `src/ai`, `src/i18n`, `src/config`, etc.).
  3. Implement the 20-theme color module (`src/ui/theme.rs`).
  4. Implement Nerd Font and ASCII fallback icon definitions (`src/ui/icons.rs`).
  5. Implement the bilingual i18n engine (`src/i18n/mod.rs`).
  6. Implement the AI client abstraction & `ContextProvider` interface (`src/ai/mod.rs`).
  7. Implement configuration file management (`src/config/mod.rs`).
  8. Build and verify the Phase 0 unit test suite.
