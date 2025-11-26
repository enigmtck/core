#!/bin/sh
# Enigmatick installation script
# Usage: curl -sSL https://gitlab.com/enigmatick/enigmatick-core/-/raw/master/install.sh | sh

set -e

# GitLab project info
GITLAB_PROJECT="enigmatick/enigmatick-core"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)
        case "$ARCH" in
            x86_64|amd64)
                ARTIFACT_NAME="enigmatick-linux-x86_64"
                JOB_NAME="build:linux-x86_64"
                ;;
            *)
                echo "Unsupported architecture: $ARCH"
                echo "Currently only Linux x86_64 is supported"
                exit 1
                ;;
        esac
        ;;
    darwin)
        echo "macOS builds are not yet available"
        echo "Please build from source: https://gitlab.com/$GITLAB_PROJECT"
        exit 1
        ;;
    *)
        echo "Unsupported OS: $OS"
        echo "Currently only Linux x86_64 is supported"
        exit 1
        ;;
esac

# Get latest release tag
echo "Fetching latest release..."
PROJECT_PATH_ENCODED=$(echo $GITLAB_PROJECT | sed 's/\//%2F/g')
LATEST_TAG=$(curl -sL "https://gitlab.com/api/v4/projects/$PROJECT_PATH_ENCODED/releases" | grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4)

if [ -z "$LATEST_TAG" ]; then
    echo "Failed to fetch latest release"
    exit 1
fi

echo "Latest version: $LATEST_TAG"

# Download from job artifacts
DOWNLOAD_URL="https://gitlab.com/$GITLAB_PROJECT/-/jobs/artifacts/$LATEST_TAG/raw/artifacts/$ARTIFACT_NAME?job=$JOB_NAME"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR=$(mktemp -d)

echo "Downloading enigmatick..."
curl -sSL "$DOWNLOAD_URL" -o "$TMP_DIR/enigmatick"

echo "Installing..."
mkdir -p "$INSTALL_DIR"
mv "$TMP_DIR/enigmatick" "$INSTALL_DIR/enigmatick"
chmod +x "$INSTALL_DIR/enigmatick"
rm -rf "$TMP_DIR"

echo ""
echo "Enigmatick installed successfully to $INSTALL_DIR/enigmatick"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH. Add this to your ~/.bashrc or ~/.zshrc:"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo "Run 'enigmatick --help' to get started"
