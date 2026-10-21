#!/bin/sh
# Installs revmo for the current user, or updates it to the latest release.
set -eu

APP_NAME="revmo"
ICON="revmo.png"
REPO_URL="https://github.com/qzakwani/$APP_NAME"

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
CONFIG_HOME=$(xdg_dir "${XDG_CONFIG_HOME:-}" "$HOME/.config")
STATE_HOME=$(xdg_dir "${XDG_STATE_HOME:-}" "$HOME/.local/state")
CACHE_HOME=$(xdg_dir "${XDG_CACHE_HOME:-}" "$HOME/.cache")
ICON_DIR="$DATA_HOME/icons"
DESKTOP_FILE_DIR="$DATA_HOME/applications"
# Revmo keeps two folders: CONFIG_DIR is the user's (config and their own
# themes), never touched here except for a first config; APP_DIR is
# revmo's (history, caches).
CONFIG_DIR="$CONFIG_HOME/$APP_NAME"
APP_DIR="$DATA_HOME/$APP_NAME"
# The release installed last, to tell an update from a reinstall.
VERSION_FILE="$APP_DIR/version"
CONFIG_FILE="$CONFIG_DIR/config"

# The physical path of folder $1, symlinks and .. resolved; empty if it
# doesn't exist.
real_dir() {
    (cd -P -- "$1" 2>/dev/null && pwd -P) || true
}

red() { printf '\033[1;31m%s\033[0m\n' "$*"; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*"; }
green() { printf '\033[1;32m%s\033[0m\n' "$*"; }

echo "Checking environment..."

if [ "$(uname -m)" != "x86_64" ] || [ "$(uname -s)" != "Linux" ]; then
    red "This app supports only x86_64 Linux systems."
    exit 1
fi

for tool in curl sha256sum; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        red "$tool is needed to install revmo; please install it and try again."
        exit 1
    fi
done

# Some window managers (e.g. i3) don't set it. Revmo works without it, it
# just can't hide apps meant only for other desktops.
if [ -z "${XDG_CURRENT_DESKTOP:-}" ]; then
    yellow "\$XDG_CURRENT_DESKTOP is not set; apps meant only for other desktops will show too."
fi

# releases/latest redirects to releases/tag/<tag>. Every file is then
# downloaded from that tag, so a release published mid-install can't mix
# files from two versions.
LATEST=$(curl -fsSL -o /dev/null -w '%{url_effective}' "$REPO_URL/releases/latest") || {
    red "Couldn't reach GitHub. Please check your internet connection and try again."
    exit 1
}
TAG=${LATEST##*/}
case "$TAG" in
    v[0-9]*) ;;
    *)
        red "Couldn't find the latest revmo release."
        exit 1
        ;;
esac
RELEASE_URL="$REPO_URL/releases/download/$TAG"

INSTALLED=""
if [ -f "$VERSION_FILE" ]; then
    INSTALLED=$(cat "$VERSION_FILE")
fi
if [ "$INSTALLED" = "$TAG" ]; then
    echo "Revmo $TAG is already installed; reinstalling it."
elif [ -n "$INSTALLED" ]; then
    echo "Updating revmo from $INSTALLED to $TAG..."
elif [ -e "$BIN_DIR/$APP_NAME" ]; then
    # Installed before the version was recorded.
    echo "Updating revmo to $TAG..."
else
    echo "Installing revmo $TAG..."
fi

TMP_DIR=$(mktemp -d)
NEW_BIN="$BIN_DIR/.$APP_NAME.new"
# Keeps the exit status, which dash would otherwise take from the rm.
trap 'status=$?; rm -rf "$TMP_DIR"; rm -f "$NEW_BIN"; exit $status' EXIT
# dash skips the EXIT trap when killed by a signal (e.g. Ctrl+C).
trap 'exit 1' INT TERM HUP

download() {
    # -f: fail on HTTP errors instead of saving the error page.
    curl -fL --progress-bar "$RELEASE_URL/$1" -o "$TMP_DIR/$1"
}

echo "Downloading..."
for file in "$APP_NAME" "$ICON" SHA256SUMS; do
    if ! download "$file"; then
        red "Failed to download $file from revmo $TAG. Please check your internet connection and try again."
        exit 1
    fi
done
if ! (cd "$TMP_DIR" && sha256sum -c --quiet SHA256SUMS); then
    red "Checksum verification failed; not installing."
    exit 1
fi

echo "Installing..."
# Closed so the old version can't write its files back after they're
# cleaned up below; the next launch starts the new one.
if pkill -u "$(id -u)" -x "$APP_NAME" 2>/dev/null; then
    echo "Closed the running revmo."
fi
# What earlier versions kept elsewhere is not carried over. 1.0 kept its
# settings and app list under ~/.config and ~/.local/share whatever the
# XDG variables said; their folders go too if that leaves them empty.
# Before the folders below are created, which may be these same ones.
rm -f "$CONFIG_DIR/revmo.conf" "$APP_DIR/apps.store" \
    "$HOME/.config/$APP_NAME/revmo.conf" "$HOME/.local/share/$APP_NAME/apps.store"
rmdir "$HOME/.config/$APP_NAME" "$HOME/.local/share/$APP_NAME" 2>/dev/null || true
mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_FILE_DIR" "$APP_DIR"
# Copied next to the old binary and renamed over it: the rename is atomic,
# so a failed update never leaves a broken binary, and it works while
# revmo is running (writing into a running binary fails).
cp "$TMP_DIR/$APP_NAME" "$NEW_BIN"
chmod +x "$NEW_BIN"
mv -f "$NEW_BIN" "$BIN_DIR/$APP_NAME"
mv -f "$TMP_DIR/$ICON" "$ICON_DIR/$ICON"

# Up to 1.2, the bundled themes were installed as files; they're built in
# now. Kept if the XDG variables make that the user's own themes folder.
if [ "$(real_dir "$APP_DIR")" != "$(real_dir "$CONFIG_DIR")" ]; then
    rm -rf "$APP_DIR/themes"
fi
# Pre-release 1.1 builds kept history and caches in the state and cache
# folders, unless the XDG variables make those one of the two folders
# (compared after resolving symlinks, so differently written paths to the
# same folder are caught).
keep="$(real_dir "$APP_DIR")/:$(real_dir "$CONFIG_DIR")/"
for old in "$STATE_HOME/$APP_NAME" "$CACHE_HOME/$APP_NAME"; do
    real=$(real_dir "$old")
    [ -n "$real" ] || continue
    case "$keep" in
        "$real/"* | *":$real/"*) ;;
        *) rm -rf "$old" ;;
    esac
done

# A starting config listing every setting, all commented out so the
# defaults apply. Never replaces one the user already has.
if [ ! -e "$CONFIG_FILE" ]; then
    mkdir -p "$(dirname "$CONFIG_FILE")"
    cat <<'EOF' > "$CONFIG_FILE"
# Revmo settings. Every setting is optional: remove the # to use one.
[Revmo]

# Color theme; Ctrl+Left / Ctrl+Right also changes it.
# Theme=Revmo Dark

# Font family name; run `revmo list-fonts` to see them. Monospaced ones
# (marked mono) suit revmo best.
# Font=

# UI size, e.g. 1.5; empty or auto picks one from your monitor.
# Scale=

# Apps and actions to leave out, separated by ;
# Run `revmo list-apps` to see their IDs (the last column).
# Hidden=

# Clipboard history (type ; in revmo, or run `revmo clipboard`) comes from
# your clipboard manager. The command that prints the history, one entry
# per line, e.g. cliphist list
# ClipboardList=
# The command that gets the picked line on its input and copies that
# entry, e.g. cliphist decode | wl-copy
# ClipboardPick=
# Columns of each history line to show, e.g. 2.. to hide cliphist's IDs,
# and what separates them (a tab if not set; write a space at an end as \s).
# ClipboardColumns=
# ClipboardSeparator=
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

printf '%s\n' "$TAG" > "$VERSION_FILE"

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
green "Revmo $TAG has been installed successfully!"
echo ""
echo "You can now:"
echo "1. Run it from a terminal with: $APP_NAME"
echo "2. Open it from your application menu."
echo "Tip: bind '$BIN_DIR/$APP_NAME' to a keyboard shortcut; pressing it again closes revmo."
echo "To update later, run the install command again."
echo ""
