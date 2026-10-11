<p align="center">
  <img src="images/Prowly-Banner.svg" alt="Prowly Banner" width="100%">
</p>

<p align="center">
  <strong>High-Performance Terminal Git & GitHub Client with Omnipresent AI Co-pilot</strong>
</p>

<p align="center">
  <a href="#key-features">Features</a> •
  <a href="#installation--desktop-launcher">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#keybindings">Keybindings</a> •
  <a href="#ai-co-pilot-setup-byok">AI Setup</a> •
  <a href="#color-themes">Themes</a> •
  <a href="README.ja.md">日本語ドキュメント</a>
</p>

---

## Overview

**Prowly** is a state-of-the-art Terminal User Interface (TUI) client for Git and GitHub built with Rust, Ratatui, and Crossterm. Designed for developers who live in the terminal, Prowly combines blazing-fast local Git operations, deep GitHub workflow integration, safety-first operations (soft-discard backups), and an omnipresent context-aware AI co-pilot.

Whether you prefer keyboard shortcuts or intuitive mouse interactions (clicking, scrolling, right-click context menus), Prowly delivers a fluid, responsive, and visually stunning experience across Linux, macOS, and Windows.

---

## Key Features

- ⚡ **Hybrid Git Performance**: High-speed in-process `git2` inspection combined with CLI fallback for advanced operations.
- 🛡️ **Soft-Discard Safety Engine**: Accidental `discard` or `reset` never deletes code permanently. Changes are automatically safeguarded into temporary backups before execution.
- 🤖 **Omnipresent AI Co-Pilot (BYOK)**:
  - Context-aware modal accessible from anywhere (`Ctrl+A`).
  - Conventional commit generation & pre-commit code review (`Ctrl+R`).
  - Automatic CI run failure root cause analysis.
  - Pull Request draft generation with comprehensive summaries.
  - AI-assisted smart merge conflict resolution.
  - Natural language to Git command translation with safety risk ratings (`Safe` / `Caution` / `Dangerous`).
- 📂 **Directory Map & Stash Filtering**: Visual tree map of repository modifications, allowing instant bulk staging per folder.
- 🌲 **Native Git Worktree Management**: Create, switch, and prune isolated worktree directories with independent branches.
- 🐙 **First-Class GitHub Integration**:
  - Live Pull Requests list with checkout and diff review.
  - GitHub Actions workflow inspection with log streaming.
  - Security, Dependabot, and Code Scanning alert notifications.
  - Rate-limit aware with intelligent in-memory TTL caching.
- ⚔️ **Visual Merge Conflict Resolver**: Side-by-side comparison of `Current` vs `Incoming` branches with interactive hunk resolution and AI smart merge.
- 🔀 **Quick Interactive Rebase**: Fast squash, reword, and fixup without fighting interactive Vim buffers.
- 🎨 **21 Curated Color Themes**: Tokyo Night, Catppuccin, Gruvbox, Nord, Dracula, Solarized, Everforest, Cyberpunk, and more.
- 🌐 **Full Bilingual Support**: Instant switching between English and Japanese (`l`).
- 🚀 **App Launcher Integration**: Seamless FreeDesktop integration with `Terminal=true` to automatically launch in your desktop environment's default terminal.

---

## Installation & Desktop Launcher

### Linux / FreeDesktop Installation (Recommended)

Run the included automated install script:

```bash
git clone https://github.com/wammed/Prowly.git
cd Prowly
./scripts/install.sh
```

#### What does `install.sh` do?
1. Compiles the optimized release binary (`~/.local/bin/prowly`).
2. Installs the scalable SVG application icon (`~/.local/share/icons/hicolor/scalable/apps/prowly.svg`).
3. Registers a FreeDesktop application entry at `~/.local/share/applications/prowly.desktop` with:
   ```ini
   [Desktop Entry]
   Name=Prowly
   Exec=prowly
   Icon=prowly
   Terminal=true
   Type=Application
   Categories=Development;RevisionControl;ConsoleOnly;
   ```
4. **Result**: Prowly immediately appears in your application launcher (GNOME, KDE Plasma, Rofi, Wofi, Albert, Kickoff, etc.) and launches inside your **system's default terminal**!

To uninstall:
```bash
./scripts/uninstall.sh
```

### Windows Installation

Run via PowerShell:
```powershell
.\scripts\install.ps1
```

### Building Manually via Cargo

```bash
cargo build --release
cp target/release/prowly ~/.local/bin/
```

---

## Quick Start

Open any Git repository and launch:

```bash
prowly
```

Or pass a specific repository path:

```bash
prowly /path/to/repo
```

---

## Keybindings

### Global Navigation
| Shortcut | Action |
|:---|:---|
| `1` | Switch to **Changes** View |
| `2` | Switch to **Pull Requests** View |
| `3` | Switch to **Actions & CI** View |
| `4` | Switch to **Alerts** View |
| `5` | Switch to **Worktrees** View |
| `Tab` | Cycle focus between panes |
| `:` or `F1` | Open **Command Palette** (Fuzzy search & NL-to-Git) |
| `Ctrl+A` | Open **Omnipresent AI Assistant** |
| `Ctrl+D` | Run **Repository Health Diagnostic** |
| `t` | Cycle Color Theme (21 themes) |
| `l` | Toggle Language (English / 日本語) |
| `e` | Suspend Prowly and open `$EDITOR` |
| `?` | Toggle Help & Keybindings Modal |
| `q` / `Esc` | Close Modal or Quit Prowly |

### Changes & Diff View
| Shortcut | Action |
|:---|:---|
| `Space` | Toggle Stage / Unstage selected file |
| `a` | Stage all files |
| `u` | Unstage all files |
| `d` | Soft-discard changes (safely backed up) |
| `c` | Open commit dialog |
| `Ctrl+G` | Generate conventional commit message with AI |
| `Ctrl+R` | Trigger pre-commit AI code review |
| `j` / `k` or `↓` / `↑` | Navigate file list / diff scroll |
| `PageDown` / `PageUp` | Fast scroll diff |

### Conflict Resolver & Worktrees
| Shortcut | Action |
|:---|:---|
| `1` | Accept Current branch (HEAD) |
| `2` | Accept Incoming branch |
| `3` | Accept Both changes |
| `m` | Trigger AI Smart Merge |
| `w` | Save resolved changes to file |
| `n` | Create new worktree |
| `x` | Prune / Remove selected worktree |

---

## AI Co-Pilot Setup (BYOK)

Prowly features a **Bring-Your-Own-Key (BYOK)** architecture supporting major LLM providers:

- **OpenAI** (`gpt-4o`, `gpt-4o-mini`)
- **Anthropic** (`claude-3-5-sonnet-20241022`)
- **Google Gemini** (`gemini-1.5-flash`, `gemini-1.5-pro`)
- **Ollama** (Local models: `llama3.2`, `mistral`, etc.)
- **Custom / OpenAI-Compatible** endpoints

### Environment Variables
Configure your provider key in your shell profile:

```bash
export OPENAI_API_KEY="sk-..."
# or
export ANTHROPIC_API_KEY="sk-ant-..."
# or
export GEMINI_API_KEY="AIza..."
```

### Configuration File (`~/.config/prowly/config.toml`)
```toml
[ui]
theme = "tokyo_night"
language = "en"
nerd_fonts = true

[ai]
provider = "openai"
model = "gpt-4o-mini"
timeout_secs = 30
auto_review = true

[github]
token = "ghp_..."
```

---

## Color Themes

Prowly includes 21 hand-crafted color themes:

- **Tokyo Night** (Default dark)
- **Catppuccin Mocha**, **Catppuccin Macchiato**, **Catppuccin Frappe**, **Catppuccin Latte**
- **Gruvbox Dark**, **Gruvbox Light**
- **Nord**
- **Dracula**
- **Solarized Dark**, **Solarized Light**
- **One Dark**, **One Light**
- **Everforest Dark**, **Everforest Light**
- **Rose Pine**, **Rose Pine Dawn**, **Rose Pine Moon**
- **Monokai Pro**
- **Kanagawa**
- **Cyberpunk**

---

## License

Prowly is open-source software licensed under the MIT License.
