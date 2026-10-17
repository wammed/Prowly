#!/usr/bin/env bash
# ==============================================================================
# Prowly Installation & Desktop Integration Script
# Target Platforms: Linux & macOS (Apple Silicon M1+ & Intel)
# Builds release binary, installs to ~/.local/bin, registers scalable icon,
# configures secure config permissions, and sets up desktop integration on Linux.
# ==============================================================================

set -euo pipefail

# ANSI color codes
BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"
RESET="\033[0m"

echo -e "${BOLD}${CYAN}🐾 Prowly Installer (Linux & macOS)${RESET}"
echo -e "=================================================="

OS_TYPE="$(uname -s)"
ARCH_TYPE="$(uname -m)"

echo -e "Operating System: ${BOLD}${OS_TYPE}${RESET}"
echo -e "Architecture:     ${BOLD}${ARCH_TYPE}${RESET}"
if [[ "${OS_TYPE}" == "Darwin" && "${ARCH_TYPE}" == "arm64" ]]; then
    echo -e "${GREEN}✓ Apple Silicon (M1/M2/M3/M4) environment recognized.${RESET}"
fi

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${HOME}/.local/bin"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
APPS_DIR="${HOME}/.local/share/applications"
CONFIG_DIR="${HOME}/.config/prowly"

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
    SHELL_PROFILE="~/.bashrc"
    if [[ "${OS_TYPE}" == "Darwin" ]]; then
        SHELL_PROFILE="~/.zshrc"
    fi
    echo -e "Add the following line to your shell profile (${SHELL_PROFILE}):"
    echo -e "  ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
fi

# 4. Install Application Icon and Secure Config Template
echo -e "\n${BOLD}${YELLOW}[3/4] Setting up icons and secure configuration...${RESET}"
mkdir -p "${ICON_DIR}"
if [ -f "${PROJECT_DIR}/images/Prowly-Icon.svg" ]; then
    install -m 644 "${PROJECT_DIR}/images/Prowly-Icon.svg" "${ICON_DIR}/prowly.svg"
fi

mkdir -p "${CONFIG_DIR}"
chmod 700 "${CONFIG_DIR}"
if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    if [ -f "${PROJECT_DIR}/config.toml.example" ]; then
        cp "${PROJECT_DIR}/config.toml.example" "${CONFIG_DIR}/config.toml"
        chmod 600 "${CONFIG_DIR}/config.toml"
        echo -e "Initialized default configuration at ${CONFIG_DIR}/config.toml (mode 0600)"
    fi
else
    chmod 600 "${CONFIG_DIR}/config.toml"
fi

# 5. Desktop launcher setup (Linux FreeDesktop standard)
if [[ "${OS_TYPE}" == "Linux" ]]; then
    echo -e "\n${BOLD}${YELLOW}[4/4] Creating Linux desktop launcher at ${APPS_DIR}/prowly.desktop...${RESET}"
    mkdir -p "${APPS_DIR}"

    cat <<EOF > "${APPS_DIR}/prowly.desktop"
[Desktop Entry]
Version=1.0
Type=Application
Name=Prowly
GenericName=Git TUI & AI Co-pilot
Comment=Omnipresent AI & High-Performance Git TUI Client
Exec=${BIN_DIR}/prowly
Icon=prowly
Terminal=true
Categories=Development;RevisionControl;ConsoleOnly;
Keywords=git;tui;terminal;github;ai;copilot;
StartupNotify=false
EOF

    chmod 644 "${APPS_DIR}/prowly.desktop"

    if command -v update-desktop-database &> /dev/null; then
        update-desktop-database "${APPS_DIR}" 2>/dev/null || true
    fi

    if command -v gtk-update-icon-cache &> /dev/null; then
        gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
    fi
elif [[ "${OS_TYPE}" == "Darwin" ]]; then
    echo -e "\n${BOLD}${YELLOW}[4/4] macOS configuration completed.${RESET}"
    echo -e "Prowly can be run from Terminal, iTerm2, Alacritty, Kitty, or Ghostty via '${BOLD}prowly${RESET}'."
fi

echo -e "\n${BOLD}${GREEN}✨ Prowly installed successfully!${RESET}"
echo -e "--------------------------------------------------"
echo -e "• Executable:      ${BOLD}${BIN_DIR}/prowly${RESET}"
echo -e "• Config path:     ${BOLD}${CONFIG_DIR}/config.toml${RESET}"
if [[ "${OS_TYPE}" == "Linux" ]]; then
    echo -e "• App Launcher:    ${BOLD}${APPS_DIR}/prowly.desktop${RESET} (Terminal=true)"
fi
echo -e "\nRun '${BOLD}prowly${RESET}' or '${BOLD}prowly /path/to/repo${RESET}' to get started!"
