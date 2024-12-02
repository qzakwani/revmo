#!/bin/sh

# General Variables
APP_NAME="revmo"
ICON="revmo.png"
VERSION="v1.0.0"

# Colors
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' 

# Directories
DATA_DIR="$HOME/.local/share/$APP_NAME"
CONFIG_DIR="$HOME/.config/$APP_NAME"
BIN_DIR="$HOME/.local/bin"
ICON_DIR="$HOME/.local/share/icons"
DESKTOP_FILE_DIR="$HOME/.local/share/applications"

# Function to remove the binary
remove_binary() {
    if [ -f "$BIN_DIR/$APP_NAME" ]; then
        rm "$BIN_DIR/$APP_NAME"
    else
        echo -e "${RED}Error: $BIN_DIR/$APP_NAME not found.${NC}"
        exit 1
    fi
}

# Function to remove the icon
remove_icon() {
    if [ -f "$ICON_DIR/$ICON" ]; then
        rm "$ICON_DIR/$ICON"
    else
        echo -e "${YELLOW}Warning: $ICON_DIR/$ICON not found.${NC}"
    fi
}

# Function to remove the desktop file
remove_desktop_file() {
    if [ -f "$DESKTOP_FILE_DIR/$APP_NAME.desktop" ]; then
        rm "$DESKTOP_FILE_DIR/$APP_NAME.desktop"
    else
        echo -e "${YELLOW}Warning: $DESKTOP_FILE_DIR/$APP_NAME.desktop not found.${NC}"
    fi
}

# Function to remove application data and configuration
remove_app_data() {
    if [ -d "$DATA_DIR" ] || [ -d "$CONFIG_DIR" ]; then
        echo "Removing data..."
        rm -rf "$DATA_DIR"
        rm -rf "$CONFIG_DIR"
    else
        echo -e "${YELLOW}Warning: No data or configuration directories found.${NC}"
    fi
}

echo "Uninstalling revmo..."
remove_binary
remove_icon
remove_desktop_file
remove_app_data

echo "Uninstallation complete."
echo ""
echo -e "${YELLOW}If you decide to reinstall, you can visit the GitHub page: https://github.com/qzakwani/revmo${NC}"
echo ""





