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
- 📜 **Git Graph, Commit History & Time Machine (`2` / `h`)**:
  - Interactive visual Git Graph map (`git log --graph` based) with automatic junction-line skipping during navigation.
  - Split view showing real-time commit metadata (author, relative time, SHA) and instant diff preview (~20 lines).
  - Full commit manipulation suite: Checkout (`c`), Create Branch (`b`), Cherry-Pick (`p`), and Revert (`r`).
  - **Time Machine Reset (`t`)**: Select any past commit to hard-reset the repository (invalidating subsequent commits) with zero risk via automatic backup branches (`backup/timemachine-...`).
  - One-key clipboard copy of commit SHA (`y`).
- 🌲 **Native Git Worktree Management**: Create, switch, and prune isolated worktree directories with independent branches.
- 🐙 **First-Class GitHub Integration**:
  - Live Pull Requests list with checkout and diff review.
  - GitHub Actions workflow inspection with log streaming.
  - Security, Dependabot, and Code Scanning alert notifications.
  - Rate-limit aware with intelligent in-memory TTL caching.
- ⚔️ **Visual Merge Conflict Resolver**: Side-by-side comparison of `Current` vs `Incoming` branches with interactive hunk resolution and AI smart merge.
- 🔀 **Quick Interactive Rebase**: Fast squash, reword, and fixup without fighting interactive Vim buffers.
- 🧭 **3-Layer Hierarchical Command Palette (`:` / `F1` / `Ctrl+P`)**:
  - Layer 1: Prowly Native Actions with instant fuzzy search & smooth full virtual scrolling.
  - Layer 2: 110+ Standard Git CLI Subcommands Catalog with category badges & descriptions.
  - Layer 3: Direct argument/option input with Tab flag autocompletion.
  - Features breadcrumbs navigation, Esc/Backspace hierarchical rollback, direct shortcut jumps, and a safety guard with two-step Enter confirmation for destructive operations.
- 📝 **Markdown Rich Preview (`m` / Context Menu / Command Palette)**:
  - Scrollable floating modal overlay rendering READMEs, documentation, and PR descriptions in rich terminal formatting matching your active color theme.
  - Full support for headings (H1-H6), bold, italic, inline code, fenced code blocks with language tags, checklists, bullet/numbered lists, blockquotes, and links.
  - Fast page scrolling (`PgUp`/`PgDn`), top/bottom jump (`g`/`G`), and direct clipboard copy (`y`).
- 🎨 **21 Curated Color Themes**: Catppuccin, Tokyo Night, Gruvbox, Nord, Dracula, Solarized, One, Rosé Pine, Monokai Pro, Synthwave '84, Everforest Dark.
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
| `2` | Switch to **Commit History & Git Graph** View (`h`) |
| `3` | Switch to **Issues** View (`i`) |
| `4` | Switch to **Pull Requests** View |
| `5` | Switch to **Actions & CI** View |
| `6` | Switch to **Alerts** View |
| `7` | Switch to **Worktrees** View |
| `Tab` | Cycle focus between panes |
| `:` or `F1` | Open **Command Palette** (Fuzzy search & NL-to-Git) |
| `Ctrl+A` | Open **Omnipresent AI Assistant** |
| `Ctrl+D` | Run **Repository Health Diagnostic** |
| `t` | Cycle Color Theme (21 themes) |
| `l` | Toggle Language (English / 日本語) |
| `e` | Suspend Prowly and open `$EDITOR` |
| `m` | Open Context Menu (equivalent to left-clicking `[ 󰍜 Menu ▾ ]`) |
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

### Commit History & Time Machine View (`2` / `h`)
| Shortcut | Action |
|:---|:---|
| `j` / `k` or `↓` / `↑` | Navigate commits (automatically skips graph junction lines) |
| `PageDown` / `PageUp` | Scroll commit details and diff preview |
| `Enter` or `m` | Open **Commit Actions Menu** (Checkout, Branch, Cherry-Pick, Revert, Time Machine) |
| `c` | Checkout selected commit directly |
| `b` | Create new branch from selected commit |
| `p` | Cherry-pick selected commit onto current branch |
| `r` | Revert selected commit |
| `t` | **Time Machine**: Reset repository to this commit (invalidating subsequent commits, with auto-backup) |
| `y` | Copy commit SHA hash to clipboard |

### Issues View (`3` / `i`)
| Shortcut | Action |
|:---|:---|
| `j` / `k` or `↓` / `↑` | Navigate issues / scroll preview |
| `PageDown` / `PageUp` | Fast scroll issue description preview |
| `o` | Open selected issue in web browser |
| `r` | Reload issues from GitHub |
| `m` | Open context menu |

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

### Markdown Preview Modal
| Shortcut | Action |
|:---|:---|
| `j` / `k` or `↓` / `↑` | Scroll down / up by 1 line |
| `PageDown` / `PageUp` or `Ctrl+d` / `Ctrl+u` | Fast scroll by 15 / 10 lines |
| `g` / `G` or `Home` / `End` | Jump to top / bottom of document |
| `y` | Copy raw Markdown text to clipboard |
| `q` / `Esc` | Close preview modal |

### Open Repository / Directory Switcher Modal (Header click or `o`)
| Operation | Action |
|:---|:---|
| `Tab` (in path input) | **TAB Path Autocompletion** (common prefix completion and candidate cycling) |
| `Enter` (in path input) | Open repository at the typed path |
| `↓` / `↑` | Move focus between path input, subdirectories, and recent repositories |
| `Enter` (in subdirectories) | Enter selected subdirectory (`..` to go up) |
| `Enter` (in recent repos) | Open selected recent repository |
| `d` / `x` / `Delete` | Remove selected recent repository from history |
| Mouse Click | Navigate directories, select recent repos, click `[  Open ]` to open, `[ ✕ ]` to delete |

### Mouse Operations & Terminal Compatibility (Right-Click Interception)

Prowly fully supports mouse navigation, including row selection, checkbox staging, wheel scrolling, and clickable buttons.

> [!NOTE]
> **Right-Click Interception in Terminal Emulators**
> 
> - **Behavior Across Terminals**:
>   - **TUI-focused Terminals (Alacritty, Kitty, WezTerm, Foot, etc.)**: Mouse pass-through is active by default; right-clicking files directly opens Prowly's context menu.
>   - **Desktop Terminals (GNOME Terminal, XFCE Terminal, Windows Terminal default)**: The terminal emulator's GUI window intercepts right-click events in the OS event loop to show its native context menu (Copy, Paste, etc.). Prowly (as a child process on stdin) never receives these right-click events. This is an unavoidable limitation of those terminal emulators.
> - **Prowly's Universal Compatibility Approach**:
>   - To guarantee full context operations in 100% of terminal environments, Prowly provides an explicit **`[ 󰍜 Menu ▾ ]` button** at the top of the file list and header.
>   - In terminals where right-click is intercepted, you can **left-click `[ 󰍜 Menu ▾ ]`** or press the **`m` key** to seamlessly access all context actions (open in editor, copy path/commit, discard, etc.).

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
theme = "catppuccin_mocha"
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

### Local Ollama Setup
When using a locally running [Ollama](https://ollama.com/) instance, no API key is required.

```toml
[ai]
provider = "ollama"
# Model name installed in your local Ollama (e.g. "llama3.2:latest", "deepseek-r1:latest", "ornith-1.5:35b")
model = "ornith-1.5:35b"
# Endpoint URL (supports both http://127.0.0.1:11434 and http://127.0.0.1:11434/api)
api_base = "http://127.0.0.1:11434"
```

> **Note (Endpoint Specification):**  
> While many other tools or agents use Ollama's OpenAI-compatible endpoint (`http://127.0.0.1:11434/v1`), Prowly's `provider = "ollama"` communicates directly with Ollama's native `/api/chat` for lightweight, dependency-free execution. Prowly automatically normalizes `http://127.0.0.1:11434` to the correct endpoint.  
> If you prefer using Ollama's OpenAI-compatible `/v1` endpoint, you can set `provider = "custom"` with `api_base = "http://127.0.0.1:11434/v1"`.

---

## Color Themes

Prowly includes 21 hand-crafted color themes:

- **Catppuccin**: Catppuccin Mocha (Default), Catppuccin Macchiato, Catppuccin Frappe, Catppuccin Latte
- **Tokyo Night**: Tokyo Night, Tokyo Night Storm, Tokyo Night Light
- **Gruvbox**: Gruvbox Dark, Gruvbox Light
- **Solarized**: Solarized Dark, Solarized Light
- **One**: One Dark, One Light
- **Rosé Pine**: Rosé Pine, Rosé Pine Moon, Rosé Pine Dawn
- **Nord**
- **Dracula**
- **Monokai Pro**
- **Synthwave '84**
- **Everforest Dark**

---

## License

Prowly is open-source software licensed under the MIT License.
