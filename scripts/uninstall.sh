#!/bin/sh
# Removes revmo and everything it stored for the current user.

APP_NAME="revmo"
ICON="revmo.png"

BIN_DIR="$HOME/.local/bin"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_DIR="$DATA_HOME/icons"
DESKTOP_FILE_DIR="$DATA_HOME/applications"

# Settings, launch history and cache.
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_NAME"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/$APP_NAME"
CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/$APP_NAME"
# The themes that come with revmo (versions before 1.1 kept other data here).
DATA_DIR="$DATA_HOME/$APP_NAME"

red() { printf '\033[0;31m%s\033[0m\n' "$*"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }

remove_file() {
    if [ -f "$1" ]; then
        rm "$1"
    else
        yellow "Warning: $1 not found."
    fi
}

echo "Uninstalling revmo..."

if [ ! -f "$BIN_DIR/$APP_NAME" ]; then
    red "Error: $BIN_DIR/$APP_NAME not found."
    exit 1
fi
rm "$BIN_DIR/$APP_NAME"
remove_file "$ICON_DIR/$ICON"
remove_file "$DESKTOP_FILE_DIR/$APP_NAME.desktop"

echo "Removing settings, themes, history and cache..."
rm -rf "$CONFIG_DIR" "$STATE_DIR" "$CACHE_DIR" "$DATA_DIR"
if [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    rm -f "$XDG_RUNTIME_DIR/$APP_NAME.sock"
fi

echo "Uninstallation complete."
echo ""
yellow "If you decide to reinstall, visit https://github.com/qzakwani/revmo"
echo ""
