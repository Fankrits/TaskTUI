#!/bin/sh
# TaskTUI - Fast, lightweight terminal task manager installer
# Usage: curl -fsSL https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.sh | sh

set -e

REPO="Fankrits/TaskTUI"
BIN_NAME="tasktui"

# ANSI color codes
BOLD="$(tput bold 2>/dev/null || printf '')"
GREEN="$(tput setaf 2 2>/dev/null || printf '')"
BLUE="$(tput setaf 4 2>/dev/null || printf '')"
YELLOW="$(tput setaf 3 2>/dev/null || printf '')"
RED="$(tput setaf 1 2>/dev/null || printf '')"
RESET="$(tput sgr0 2>/dev/null || printf '')"

printf "%s\n" "${BLUE}${BOLD}⚡ TaskTUI Installer${RESET}"
printf "%s\n" "==============================="

# 1. Detect OS
OS="$(uname -s)"
case "$OS" in
  Darwin)
    TARGET_OS="apple-darwin"
    ;;
  Linux)
    TARGET_OS="unknown-linux-gnu"
    ;;
  *)
    printf "%s\n" "${RED}Error: Unsupported operating system: $OS${RESET}"
    printf "%s\n" "For Windows, run in PowerShell:"
    printf "%s\n" "  irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.ps1 | iex"
    exit 1
    ;;
esac

# 2. Detect Architecture
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)
    TARGET_ARCH="x86_64"
    ;;
  arm64|aarch64)
    TARGET_ARCH="aarch64"
    ;;
  *)
    printf "%s\n" "${RED}Error: Unsupported CPU architecture: $ARCH${RESET}"
    exit 1
    ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"
printf "%s\n" "Detected platform: ${BOLD}${TARGET}${RESET}"

# 3. Determine release version
if [ -z "$VERSION" ]; then
  printf "Fetching latest release from GitHub... "
  RELEASE_DATA=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)
  TAG=$(printf "%s" "$RELEASE_DATA" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
  
  if [ -z "$TAG" ]; then
    printf "\n%s\n" "${YELLOW}Warning: Could not query GitHub API for latest tag, falling back to 'v0.1.0'${RESET}"
    TAG="v0.1.0"
  else
    printf "%s\n" "${GREEN}${TAG}${RESET}"
  fi
else
  TAG="$VERSION"
  printf "%s\n" "Installing specified version: ${BOLD}${TAG}${RESET}"
fi

# 4. Download Release Tarball
TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'tasktui')"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

DOWNLOAD_URL_TASKTUI="https://github.com/${REPO}/releases/download/${TAG}/tasktui-${TAG}-${TARGET}.tar.gz"
DOWNLOAD_URL_LEGACY="https://github.com/${REPO}/releases/download/${TAG}/task_tui-${TAG}-${TARGET}.tar.gz"
DOWNLOAD_URL_SIMPLE="https://github.com/${REPO}/releases/download/${TAG}/tasktui-${TARGET}.tar.gz"

printf "%s\n" "Downloading ${BIN_NAME}..."
if curl --fail -sSL -o "$TMP_DIR/archive.tar.gz" "$DOWNLOAD_URL_TASKTUI" 2>/dev/null; then
  :
elif curl --fail -sSL -o "$TMP_DIR/archive.tar.gz" "$DOWNLOAD_URL_LEGACY" 2>/dev/null; then
  :
elif curl --fail -sSL -o "$TMP_DIR/archive.tar.gz" "$DOWNLOAD_URL_SIMPLE" 2>/dev/null; then
  :
else
  printf "%s\n" "${RED}Error: Failed to download release archive for target ${TARGET}.${RESET}"
  printf "%s\n" "Tried: $DOWNLOAD_URL_TASKTUI"
  exit 1
fi

# 5. Extract
tar -xzf "$TMP_DIR/archive.tar.gz" -C "$TMP_DIR"

# Find binary (either tasktui or task_tui)
FOUND_BIN=""
if [ -f "$TMP_DIR/tasktui" ]; then
  FOUND_BIN="$TMP_DIR/tasktui"
elif [ -f "$TMP_DIR/task_tui" ]; then
  FOUND_BIN="$TMP_DIR/task_tui"
else
  FOUND_BIN=$(find "$TMP_DIR" -type f \( -name "tasktui" -o -name "task_tui" \) | head -n 1)
fi

if [ -z "$FOUND_BIN" ]; then
  printf "%s\n" "${RED}Error: Binary '$BIN_NAME' not found in downloaded archive.${RESET}"
  exit 1
fi

chmod +x "$FOUND_BIN"

# 6. Determine install location
INSTALL_DIR=""
if [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -n "$SUDO_USER" ] || [ "$(id -u)" -eq 0 ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ] && printf "%s" "$PATH" | grep -q "$HOME/.local/bin"; then
  INSTALL_DIR="$HOME/.local/bin"
elif [ -d "$HOME/.cargo/bin" ] && printf "%s" "$PATH" | grep -q "$HOME/.cargo/bin"; then
  INSTALL_DIR="$HOME/.cargo/bin"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

printf "%s\n" "Installing ${BIN_NAME} to ${BOLD}${INSTALL_DIR}/${BIN_NAME}${RESET}..."

if [ -w "$INSTALL_DIR" ]; then
  mv "$FOUND_BIN" "$INSTALL_DIR/$BIN_NAME"
  # Also create alias/symlink for task_tui
  ln -sf "$INSTALL_DIR/$BIN_NAME" "$INSTALL_DIR/task_tui" 2>/dev/null || true
else
  printf "%s\n" "${YELLOW}Elevated permissions required to write to ${INSTALL_DIR}. Prompting for sudo:${RESET}"
  sudo mv "$FOUND_BIN" "$INSTALL_DIR/$BIN_NAME"
  sudo ln -sf "$INSTALL_DIR/$BIN_NAME" "$INSTALL_DIR/task_tui" 2>/dev/null || true
fi

printf "\n%s\n" "${GREEN}${BOLD}🎉 TaskTUI was successfully installed!${RESET}"
printf "%s\n" "You can now run it anywhere with either:"
printf "%s\n" "  ${BOLD}tasktui${RESET}  (recommended)"
printf "%s\n\n" "  ${BOLD}task_tui${RESET}"

# Check if INSTALL_DIR is in PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    printf "%s\n" "${YELLOW}Notice: ${INSTALL_DIR} is not in your current PATH.${RESET}"
    printf "%s\n" "Add it to your shell configuration (.zshrc or .bashrc):"
    printf "%s\n" "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    ;;
esac
