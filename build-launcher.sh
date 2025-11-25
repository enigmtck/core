#!/bin/bash
set -e

echo "Building Enigmatick launcher with embedded binaries..."

# Get the script's directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Parse arguments - separate features from target/release flags
RELEASE_FLAG=""
TARGET_FLAG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE_FLAG="--release"
            shift
            ;;
        --target)
            TARGET_FLAG="--target $2"
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

# Common args for all builds (target and release)
COMMON_ARGS="$RELEASE_FLAG $TARGET_FLAG"

# Build each component separately
echo "Building main enigmatick binary..."
cargo build --bin enigmatick $COMMON_ARGS

echo "Building proxy binary..."
cd "$SCRIPT_DIR/proxy" && cargo build --target-dir ../target $COMMON_ARGS && cd "$SCRIPT_DIR"

echo "Building tasks binary..."
cd "$SCRIPT_DIR/tasks" && cargo build --target-dir ../target $COMMON_ARGS && cd "$SCRIPT_DIR"

echo "Building launcher with embedded binaries..."
cd "$SCRIPT_DIR/launcher" && cargo build $COMMON_ARGS && cd "$SCRIPT_DIR"

echo "Build complete!"
