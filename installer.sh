#!/bin/sh
set -e

REPO="DavidValin/ai-mate"
APP="ai-mate"

# -------------------------
# Get latest version from GitHub API
# -------------------------
if command -v curl >/dev/null 2>&1; then
  VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest \
    | grep '"tag_name":' \
    | cut -d '"' -f 4)
elif command -v wget >/dev/null 2>&1; then
  VERSION=$(wget -qO- https://api.github.com/repos/$REPO/releases/latest \
    | grep '"tag_name":' \
    | cut -d '"' -f 4)
else
  echo "Need curl or wget"
  exit 1
fi

if [ -z "$VERSION" ]; then
  echo "Failed to fetch latest version"
  exit 1
fi

echo "Latest version: $VERSION"

BASE_URL="https://github.com/$REPO/releases/download/$VERSION"

# -------------------------
# Detect OS / Arch
# -------------------------
OS="$(uname -s 2>/dev/null || echo unknown)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"

case "$OS" in
  Linux*) OS_NAME="linux" ;;
  Darwin*) OS_NAME="macos" ;;
  MINGW*|MSYS*|CYGWIN*) OS_NAME="windows" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_NAME="x86" ;;
  arm64|aarch64) ARCH_NAME="arm64" ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

# -------------------------
# GPU detection
# -------------------------
CUDA=0
VULKAN=0

command -v nvidia-smi >/dev/null 2>&1 && CUDA=1
command -v vulkaninfo >/dev/null 2>&1 && VULKAN=1

echo "OS=$OS_NAME ARCH=$ARCH_NAME CUDA=$CUDA VULKAN=$VULKAN"

# -------------------------
# Select binary
# -------------------------
BINARY=""

case "$OS_NAME" in

  macos)
    BINARY="${APP}-macos-arm64"
    ;;

  windows)
    if [ "$CUDA" -eq 1 ]; then
      BINARY="${APP}-windows-x86-cuda.exe"
    else
      BINARY="${APP}-windows-x86-cpu.exe"
    fi
    ;;

  linux)
    if [ "$ARCH_NAME" = "x86" ]; then
      if [ "$CUDA" -eq 1 ]; then
        BINARY="${APP}-linux-x86-cuda"
      else
        BINARY="${APP}-linux-x86-cpu"
      fi

    elif [ "$ARCH_NAME" = "arm64" ]; then
      if [ "$CUDA" -eq 1 ]; then
        BINARY="${APP}-linux-arm64-cuda"
      elif [ "$VULKAN" -eq 1 ]; then
        BINARY="${APP}-linux-arm64-vulkan"
      else
        BINARY="${APP}-linux-arm64-cpu"
      fi
    fi
    ;;
esac

if [ -z "$BINARY" ]; then
  echo "Failed to select binary"
  exit 1
fi

URL="$BASE_URL/$BINARY"

echo "Downloading: $URL"

# -------------------------
# Download
# -------------------------
TMP_FILE="/tmp/$BINARY"

if command -v curl >/dev/null 2>&1; then
  curl -L -o "$TMP_FILE" "$URL"
elif command -v wget >/dev/null 2>&1; then
  wget -O "$TMP_FILE" "$URL"
else
  echo "Need curl or wget"
  exit 1
fi

# -------------------------
# Install
# -------------------------
case "$OS_NAME" in
  windows)
    INSTALL_DIR="$HOME/bin"
    mkdir -p "$INSTALL_DIR"
    cp "$TMP_FILE" "$INSTALL_DIR/$APP.exe"
    ;;
  *)
    if [ -w "/usr/local/bin" ]; then
      INSTALL_DIR="/usr/local/bin"
    else
      INSTALL_DIR="$HOME/.local/bin"
      mkdir -p "$INSTALL_DIR"
    fi

    cp "$TMP_FILE" "$INSTALL_DIR/$APP"
    chmod +x "$INSTALL_DIR/$APP"
    ;;
esac

echo "Installed to: $INSTALL_DIR"
echo "Done."
