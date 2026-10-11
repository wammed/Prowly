#!/usr/bin/env bash
# ==============================================================================
# Prowly Uninstallation Script
# Removes binary, icon, and desktop entry from ~/.local/
# ==============================================================================

set -euo pipefail

BOLD="\033[1m"
GREEN="\033[32m"
YELLOW="\033[33m"
RESET="\033[0m"

echo -e "${BOLD}${YELLOW}🐾 Uninstalling Prowly...${RESET}"

BIN_FILE="${HOME}/.local/bin/prowly"
ICON_FILE="${HOME}/.local/share/icons/hicolor/scalable/apps/prowly.svg"
DESKTOP_FILE="${HOME}/.local/share/applications/prowly.desktop"

rm -f "${BIN_FILE}"
rm -f "${ICON_FILE}"
rm -f "${DESKTOP_FILE}"

if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "${HOME}/.local/share/applications" 2>/dev/null || true
fi

if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo -e "${BOLD}${GREEN}Prowly has been removed from ~/.local/${RESET}"
