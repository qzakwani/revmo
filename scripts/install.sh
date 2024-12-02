#!/bin/sh

# General Variables
APP_NAME="revmo"
ICON="revmo.png"
VERSION="v1.0.0"
GITHUB_REPO="https://github.com/qzakwani/$APP_NAME/releases/download/$VERSION"

# Directories
DATA_DIR="$HOME/.local/share/$APP_NAME"
CONFIG_DIR="$HOME/.config/$APP_NAME"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons"
DESKTOP_FILE_DIR="$HOME/.local/share/applications"


echo "Checking environment..."


# Check if the system is x86_64 Linux
ARCH=$(uname -m)
OS=$(uname -s)

if [ "$ARCH" != "x86_64" ] || [ "$OS" != "Linux" ]; then
    echo "This app supports only x86_64 Linux systems."
    exit 1
fi

# Check if the environment supports .desktop files
if [ -z "$XDG_CURRENT_DESKTOP" ]; then
    echo "Your environment is not supported."
    exit 1
fi

# Create necessary directories
mkdir -p "$DATA_DIR" "$CONFIG_DIR" "$BIN_DIR" "$ICON_DIR" "$DESKTOP_FILE_DIR"

# Download the app binary from GitHub releases
echo "Downloading the latest release..."
if ! curl -L --progress-bar "$GITHUB_REPO/$APP_NAME" -o "$BIN_DIR/$APP_NAME"; then
    echo -e "\033[1;31m❌ Failed to download revmo. Please check your internet connection and try again.\033[0m"
    exit 1
fi
if ! curl -L --progress-bar "$GITHUB_REPO/$ICON" -o "$ICON_DIR/$ICON"; then
    echo -e "\033[1;31m❌ Failed to download revmo. Please check your internet connection and try again.\033[0m"
    exit 1
fi

echo "Installing revmo..."

# Make the binary executable
chmod +x "$BIN_DIR/$APP_NAME"

# Create the desktop file
cat <<EOF > "$DESKTOP_FILE_DIR/$APP_NAME.desktop"
[Desktop Entry]
Name=Revmo
Version=1.0.0
Exec=$BIN_DIR/$APP_NAME
Icon=$ICON_DIR/$ICON
Terminal=false
Type=Application
StartupNotify=true
StartupWMClass=revmo
Categories=Utility;
EOF

# Check if the bin directory is in PATH
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    echo ""
    echo -e "\033[1;33m⚠️  WARNING:\033[0m The directory \033[1;34m$BIN_DIR\033[0m is not in your PATH."
    echo -e "To temporarily add it to your PATH for the current session, run:"
    echo -e "\033[1;32mexport PATH=\$PATH:$BIN_DIR\033[0m"
    echo -e "To make this change permanent, add the following line to your shell configuration file (e.g., \033[1;36m~/.bashrc\033[0m or \033[1;36m~/.zshrc\033[0m):"
    echo -e "\033[1;32mexport PATH=\$PATH:$BIN_DIR\033[0m"
    echo ""
fi

# Success message
echo ""
echo -e "\033[1;32m🎉 Congratulations!\033[0m The app has been installed successfully!"
echo ""
echo -e "You can now use \033[1;34m$APP_NAME\033[0m in the following ways:"
echo -e "1. Run the app from the CLI using: \033[1;32m$APP_NAME\033[0m"
echo -e "2. Use the desktop application from your system's application menu."
echo ""