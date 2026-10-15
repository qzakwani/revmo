#!/bin/sh
# Removes revmo and everything it stored for the current user.
set -u

APP_NAME="revmo"
ICON="revmo.png"

# $1 if it's an absolute path, else the default $2, as revmo itself
# reads the XDG variables.
xdg_dir() {
    case "$1" in
        /*) printf '%s' "$1" ;;
        *) printf '%s' "$2" ;;
    esac
}

BIN_DIR="$HOME/.local/bin"
DATA_HOME=$(xdg_dir "${XDG_DATA_HOME:-}" "$HOME/.local/share")
ICON_DIR="$DATA_HOME/icons"
DESKTOP_FILE_DIR="$DATA_HOME/applications"

# Settings, launch history and cache.
CONFIG_DIR="$(xdg_dir "${XDG_CONFIG_HOME:-}" "$HOME/.config")/$APP_NAME"
STATE_DIR="$(xdg_dir "${XDG_STATE_HOME:-}" "$HOME/.local/state")/$APP_NAME"
# Also holds the socket when $XDG_RUNTIME_DIR isn't set.
CACHE_DIR="$(xdg_dir "${XDG_CACHE_HOME:-}" "$HOME/.cache")/$APP_NAME"
# The installed themes and version (versions before 1.1 kept other data here).
DATA_DIR="$DATA_HOME/$APP_NAME"

yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }

remove_file() {
    if [ -f "$1" ]; then
        rm "$1"
    else
        yellow "Warning: $1 not found."
    fi
}

echo "Uninstalling revmo..."

# Closed first, so it can't write its history or cache back afterwards.
if pkill -u "$(id -u)" -x "$APP_NAME" 2>/dev/null; then
    echo "Closed the running revmo."
fi

# Each part is removed even if others are already gone, so a partial
# install is cleaned up too.
remove_file "$BIN_DIR/$APP_NAME"
remove_file "$ICON_DIR/$ICON"
remove_file "$DESKTOP_FILE_DIR/$APP_NAME.desktop"

echo "Removing settings, themes, history and cache..."
rm -rf "$CONFIG_DIR" "$STATE_DIR" "$CACHE_DIR" "$DATA_DIR"
case "${XDG_RUNTIME_DIR:-}" in
    /*) rm -f "$XDG_RUNTIME_DIR/$APP_NAME.sock" ;;
esac

echo "Uninstallation complete."
echo ""
yellow "If you decide to reinstall, visit https://github.com/qzakwani/revmo"
echo ""
