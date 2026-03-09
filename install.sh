#!/bin/sh
set -e

REPO="babinc/groovebox"
INSTALL_DIR="$HOME/.local/bin"

echo "groovebox installer"
echo ""

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64) ASSET="groovebox-linux-x86_64.tar.gz" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64) ASSET="groovebox-macos-x86_64.tar.gz" ;;
            arm64)  ASSET="groovebox-macos-aarch64.tar.gz" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac
        ;;
    *)
        echo "groovebox is built for systems with taste. Try Linux or macOS."
        exit 1
        ;;
esac

# Linux: check for runtime libraries
if [ "$OS" = "Linux" ]; then
    MISSING_LIBS=""
    ldconfig -p 2>/dev/null | grep -q libpipewire-0.3 || MISSING_LIBS="$MISSING_LIBS libpipewire-0.3"
    ldconfig -p 2>/dev/null | grep -q libasound.so.2 || MISSING_LIBS="$MISSING_LIBS libasound2"

    if [ -n "$MISSING_LIBS" ]; then
        echo "Missing runtime libraries:$MISSING_LIBS"
        if command -v apt >/dev/null 2>&1; then
            PKGS=""
            echo "$MISSING_LIBS" | grep -q pipewire && PKGS="$PKGS libpipewire-0.3-0"
            echo "$MISSING_LIBS" | grep -q asound && PKGS="$PKGS libasound2"
            echo "Installing:$PKGS"
            sudo apt install -y $PKGS
        elif command -v dnf >/dev/null 2>&1; then
            PKGS=""
            echo "$MISSING_LIBS" | grep -q pipewire && PKGS="$PKGS pipewire-libs"
            echo "$MISSING_LIBS" | grep -q asound && PKGS="$PKGS alsa-lib"
            echo "Installing:$PKGS"
            sudo dnf install -y $PKGS
        elif command -v pacman >/dev/null 2>&1; then
            PKGS=""
            echo "$MISSING_LIBS" | grep -q pipewire && PKGS="$PKGS pipewire"
            echo "$MISSING_LIBS" | grep -q asound && PKGS="$PKGS alsa-lib"
            echo "Installing:$PKGS"
            sudo pacman -S --noconfirm $PKGS
        else
            echo "Please install these manually and try again."
            exit 1
        fi
    fi
fi

# macOS: check for brew and runtime deps
if [ "$OS" = "Darwin" ]; then
    if ! command -v brew >/dev/null 2>&1; then
        echo "Homebrew is required. Install it from https://brew.sh"
        exit 1
    fi
    MISSING=""
    command -v mpv >/dev/null 2>&1 || MISSING="$MISSING mpv"
    command -v yt-dlp >/dev/null 2>&1 || MISSING="$MISSING yt-dlp"
    command -v ffmpeg >/dev/null 2>&1 || MISSING="$MISSING ffmpeg"
    if [ -n "$MISSING" ]; then
        echo "Installing dependencies:$MISSING"
        brew install $MISSING
    fi
fi

# Download latest release
echo ""
echo "Downloading $ASSET..."
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ASSET"
TMPDIR="$(mktemp -d)"
curl -sL "$DOWNLOAD_URL" -o "$TMPDIR/$ASSET"

# Extract
tar xzf "$TMPDIR/$ASSET" -C "$TMPDIR"

# Install
mkdir -p "$INSTALL_DIR"
mv "$TMPDIR/groovebox" "$INSTALL_DIR/groovebox"
chmod +x "$INSTALL_DIR/groovebox"
rm -rf "$TMPDIR"

echo ""
echo "Installed to $INSTALL_DIR/groovebox"

# Check if install dir is in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "Add this to your shell profile:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        ;;
esac

echo ""
echo "Run 'groovebox' to start!"
