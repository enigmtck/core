#!/bin/sh
# Enigmatick installation script
# Usage: curl -sSL https://gitlab.com/enigmatick/enigmatick-core/-/raw/master/install.sh | sh

set -e

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)
        case "$ARCH" in
            x86_64|amd64)
                BINARY_NAME="enigmatick-linux-x86_64"
                ;;
            aarch64|arm64)
                BINARY_NAME="enigmatick-linux-aarch64"
                ;;
            *)
                echo "Unsupported architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    darwin)
        case "$ARCH" in
            x86_64)
                BINARY_NAME="enigmatick-macos-x86_64"
                ;;
            arm64)
                BINARY_NAME="enigmatick-macos-aarch64"
                ;;
            *)
                echo "Unsupported architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

# Get latest release tag
echo "Fetching latest release..."
LATEST_TAG=$(curl -sL https://gitlab.com/api/v4/projects/enigmatick%2Fenigmatick-core/releases | grep -o '"tag_name": "[^"]*' | head -1 | cut -d'"' -f4)

if [ -z "$LATEST_TAG" ]; then
    echo "Failed to fetch latest release"
    exit 1
fi

echo "Latest version: $LATEST_TAG"

# Download binary
DOWNLOAD_URL="https://gitlab.com/enigmatick/enigmatick-core/-/jobs/artifacts/$LATEST_TAG/raw/artifacts/$BINARY_NAME?job=build:linux-x86_64"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

echo "Downloading enigmatick..."
mkdir -p "$INSTALL_DIR"
curl -sSL "$DOWNLOAD_URL" -o "$INSTALL_DIR/enigmatick"
chmod +x "$INSTALL_DIR/enigmatick"

echo ""
echo "✓ Enigmatick installed successfully to $INSTALL_DIR/enigmatick"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH. Add this to your ~/.bashrc or ~/.zshrc:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo "Run 'enigmatick --help' to get started"
