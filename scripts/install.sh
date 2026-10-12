#!/bin/sh
# Installs (or updates) revmo for the current user.
set -eu

APP_NAME="revmo"
ICON="revmo.png"
THEMES="revmo-themes.tar.gz"
RELEASE_URL="https://github.com/qzakwani/$APP_NAME/releases/latest/download"

BIN_DIR="$HOME/.local/bin"
ICON_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/icons"
DESKTOP_FILE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
# Color themes that come with revmo; users add their own to
# ~/.config/revmo/themes, which this never touches.
THEME_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/revmo/themes"
CONFIG_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/revmo/config"

red() { printf '\033[1;31m%s\033[0m\n' "$*"; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*"; }
green() { printf '\033[1;32m%s\033[0m\n' "$*"; }

echo "Checking environment..."

if [ "$(uname -m)" != "x86_64" ] || [ "$(uname -s)" != "Linux" ]; then
    red "This app supports only x86_64 Linux systems."
    exit 1
fi

if [ -z "${XDG_CURRENT_DESKTOP:-}" ]; then
    red "No desktop environment detected (\$XDG_CURRENT_DESKTOP is not set)."
    exit 1
fi

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

download() {
    # -f: fail on HTTP errors instead of saving the error page.
    curl -fL --progress-bar "$RELEASE_URL/$1" -o "$TMP_DIR/$1"
}

echo "Downloading the latest release..."
if ! download "$APP_NAME" || ! download "$ICON" || ! download "$THEMES"; then
    red "Failed to download revmo. Please check your internet connection and try again."
    exit 1
fi

if download SHA256SUMS 2>/dev/null; then
    if command -v sha256sum >/dev/null 2>&1; then
        if ! (cd "$TMP_DIR" && sha256sum -c --quiet SHA256SUMS); then
            red "Checksum verification failed; not installing."
            exit 1
        fi
    else
        yellow "sha256sum not found; skipping checksum verification."
    fi
else
    yellow "This release has no checksums; skipping verification."
fi

echo "Installing revmo..."
mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_FILE_DIR"
chmod +x "$TMP_DIR/$APP_NAME"
# Move into place only after a complete download, so a failed update
# never leaves a broken binary behind.
mv "$TMP_DIR/$APP_NAME" "$BIN_DIR/$APP_NAME"
mv "$TMP_DIR/$ICON" "$ICON_DIR/$ICON"
# Replaced as a whole, so themes dropped from revmo go away too.
mkdir -p "$TMP_DIR/themes"
tar -xzf "$TMP_DIR/$THEMES" -C "$TMP_DIR/themes"
rm -rf "$THEME_DIR"
mkdir -p "$(dirname "$THEME_DIR")"
mv "$TMP_DIR/themes" "$THEME_DIR"

# A starting config listing every setting, all commented out so the
# defaults apply. Never replaces one the user already has.
if [ ! -e "$CONFIG_FILE" ]; then
    mkdir -p "$(dirname "$CONFIG_FILE")"
    cat <<'EOF' > "$CONFIG_FILE"
# Revmo settings. Every setting is optional: remove the # to use one.
[Revmo]

# Color theme; Ctrl+Left / Ctrl+Right also changes it.
# Theme=Revmo Dark

# Font family name, as listed by `fc-list : family`, e.g. JetBrains Mono.
# Use a monospace font.
# Font=

# UI size, e.g. 1.5; empty or auto picks one from your monitor.
# Scale=

# Desktop file IDs of apps to leave out, separated by ;
# Hidden=

# Layout style. Not used yet: only the terminal style exists.
# Style=terminal
EOF
fi

cat <<EOF > "$DESKTOP_FILE_DIR/$APP_NAME.desktop"
[Desktop Entry]
Type=Application
Version=1.5
Name=Revmo
Comment=Keyboard-driven app launcher
Exec=$BIN_DIR/$APP_NAME
Icon=$ICON_DIR/$ICON
Terminal=false
StartupNotify=true
StartupWMClass=revmo
Categories=Utility;
EOF

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        yellow "WARNING: $BIN_DIR is not in your PATH."
        echo "To add it for the current session, run:"
        green "export PATH=\$PATH:$BIN_DIR"
        echo "To make it permanent, add that line to your shell configuration file (e.g. ~/.bashrc or ~/.zshrc)."
        echo ""
        ;;
esac

echo ""
green "Revmo has been installed successfully!"
echo ""
echo "You can now:"
echo "1. Run it from a terminal with: $APP_NAME"
echo "2. Open it from your application menu."
echo "Tip: bind '$BIN_DIR/$APP_NAME' to a keyboard shortcut; pressing it again closes revmo."
echo ""
