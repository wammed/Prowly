#!/usr/bin/env bash
# ==============================================================================
# Prowly Installation & Desktop Integration Script
# Builds release binary, installs to ~/.local/bin, registers scalable icon,
# and creates a FreeDesktop .desktop entry configured to launch in the system's
# default terminal (Terminal=true).
# ==============================================================================

set -euo pipefail

# ANSI color codes
BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"
RESET="\033[0m"

echo -e "${BOLD}${CYAN}🐾 Prowly Installer & Desktop Integrator${RESET}"
echo -e "=================================================="

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
APPS_DIR="${HOME}/.local/share/applications"

# 1. Verify Cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not found in PATH.${RESET}"
    echo "Please install Rust & Cargo via https://rustup.rs before proceeding."
    exit 1
fi

# 2. Build release binary
echo -e "\n${BOLD}${YELLOW}[1/4] Building Prowly in release mode...${RESET}"
cargo build --release --manifest-path "${PROJECT_DIR}/Cargo.toml"

# 3. Install binary to ~/.local/bin
echo -e "\n${BOLD}${YELLOW}[2/4] Installing binary to ${BIN_DIR}...${RESET}"
mkdir -p "${BIN_DIR}"
install -m 755 "${PROJECT_DIR}/target/release/prowly" "${BIN_DIR}/prowly"

# Verify ~/.local/bin is in PATH
if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    echo -e "${YELLOW}Warning: ${BIN_DIR} is not in your PATH.${RESET}"
    echo -e "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo -e "  ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
fi

# 4. Install Application Icon
echo -e "\n${BOLD}${YELLOW}[3/4] Installing scalable icon to ${ICON_DIR}...${RESET}"
mkdir -p "${ICON_DIR}"
if [ -f "${PROJECT_DIR}/images/Prowly-Icon.svg" ]; then
    install -m 644 "${PROJECT_DIR}/images/Prowly-Icon.svg" "${ICON_DIR}/prowly.svg"
else
    echo -e "${YELLOW}Warning: images/Prowly-Icon.svg not found. Skipping icon installation.${RESET}"
fi

# 4.5. Install Config Template if not present
CONFIG_DIR="${HOME}/.config/prowly"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    echo -e "\n${BOLD}${YELLOW}[*] Initializing default configuration at ${CONFIG_DIR}/config.toml...${RESET}"
    mkdir -p "${CONFIG_DIR}"
    if [ -f "${PROJECT_DIR}/config.toml.example" ]; then
        cp "${PROJECT_DIR}/config.toml.example" "${CONFIG_DIR}/config.toml"
    fi
fi

# 5. Create FreeDesktop .desktop entry
echo -e "\n${BOLD}${YELLOW}[4/4] Configuring desktop launcher at ${APPS_DIR}/prowly.desktop...${RESET}"
mkdir -p "${APPS_DIR}"

EXEC_CMD="${BIN_DIR}/prowly"
TERMINAL_SETTING="true"
LAUNCHER_DESC="System default terminal (Terminal=true)"

# Interactive terminal & size selection when connected to a terminal (TTY)
if [ -t 0 ]; then
    CANDIDATES=()
    for term in alacritty kitty gnome-terminal konsole xfce4-terminal foot xterm wezterm; do
        if command -v "$term" &> /dev/null; then
            CANDIDATES+=("$term")
        fi
    done

    echo -e "Choose launcher launch method:"
    idx=1
    for term in "${CANDIDATES[@]}"; do
        echo -e "  [${idx}] ${term}"
        ((idx++))
    done
    echo -e "  [${idx}] System default terminal (Terminal=true, standard size)"

    DEFAULT_OPT="${idx}"
    read -rp "Select option [1-${idx}] (Default: ${DEFAULT_OPT}): " term_choice
    term_choice="${term_choice:-$DEFAULT_OPT}"

    if [[ "$term_choice" =~ ^[0-9]+$ ]] && [ "$term_choice" -ge 1 ] && [ "$term_choice" -lt "$DEFAULT_OPT" ]; then
        SELECTED_TERM="${CANDIDATES[$((term_choice - 1))]}"
        echo -e "${CYAN}Selected terminal: ${SELECTED_TERM}${RESET}"

        echo -e "\nWindow size configuration (Prowly recommends at least 120 cols x 35 rows):"
        read -rp "  Enter width  (columns) [Default: 140]: " cols_input
        read -rp "  Enter height (rows)    [Default: 40]: " rows_input

        COLS=140
        ROWS=40
        if [[ "$cols_input" =~ ^[0-9]+$ ]] && [ "$cols_input" -gt 0 ]; then
            COLS="$cols_input"
        fi
        if [[ "$rows_input" =~ ^[0-9]+$ ]] && [ "$rows_input" -gt 0 ]; then
            ROWS="$rows_input"
        fi

        TERMINAL_SETTING="false"
        case "$SELECTED_TERM" in
            alacritty)
                EXEC_CMD="alacritty --dimensions ${COLS} ${ROWS} -e ${BIN_DIR}/prowly"
                ;;
            kitty)
                EXEC_CMD="kitty -o initial_window_width=${COLS}c -o initial_window_height=${ROWS}c ${BIN_DIR}/prowly"
                ;;
            gnome-terminal)
                EXEC_CMD="gnome-terminal --geometry=${COLS}x${ROWS} -- ${BIN_DIR}/prowly"
                ;;
            konsole)
                EXEC_CMD="konsole --geometry ${COLS}x${ROWS} -e ${BIN_DIR}/prowly"
                ;;
            xfce4-terminal)
                EXEC_CMD="xfce4-terminal --geometry=${COLS}x${ROWS} -e ${BIN_DIR}/prowly"
                ;;
            foot)
                EXEC_CMD="foot -w ${COLS}x${ROWS} ${BIN_DIR}/prowly"
                ;;
            xterm)
                EXEC_CMD="xterm -geometry ${COLS}x${ROWS} -e ${BIN_DIR}/prowly"
                ;;
            wezterm)
                EXEC_CMD="wezterm start -- ${BIN_DIR}/prowly"
                ;;
            *)
                TERMINAL_SETTING="true"
                EXEC_CMD="${BIN_DIR}/prowly"
                ;;
        esac
        LAUNCHER_DESC="${SELECTED_TERM} (${COLS} cols x ${ROWS} rows)"
        echo -e "${GREEN}Launcher configured: ${LAUNCHER_DESC}${RESET}"
    else
        echo -e "${CYAN}Using system default terminal configuration.${RESET}"
    fi
fi

cat <<EOF > "${APPS_DIR}/prowly.desktop"
[Desktop Entry]
Version=1.0
Type=Application
Name=Prowly
GenericName=Git TUI & AI Co-pilot
Comment=Omnipresent AI & High-Performance Git TUI Client
Exec=${EXEC_CMD}
Icon=prowly
Terminal=${TERMINAL_SETTING}
Categories=Development;RevisionControl;ConsoleOnly;
Keywords=git;tui;terminal;github;ai;copilot;
StartupNotify=false
EOF

chmod 644 "${APPS_DIR}/prowly.desktop"

# 6. Update desktop and icon caches if utilities are available
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "${APPS_DIR}" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo -e "\n${BOLD}${GREEN}✨ Prowly installed successfully!${RESET}"
echo -e "--------------------------------------------------"
echo -e "• Executable:      ${BOLD}${BIN_DIR}/prowly${RESET}"
echo -e "• App Launcher:    ${BOLD}${APPS_DIR}/prowly.desktop${RESET}"
echo -e "• Configuration:   ${LAUNCHER_DESC}"
echo -e "• Scalable Icon:   ${BOLD}${ICON_DIR}/prowly.svg${RESET}"
echo -e "\nYou can now launch Prowly directly from your system application menu / launcher,"
echo -e "or run '${BOLD}prowly${RESET}' in any terminal!"
