#!/bin/bash
set -e

echo "Building Enigmatick launcher with embedded binaries..."

# Parse arguments - separate features from target/release flags
RELEASE_FLAG=""
TARGET_FLAG=""
FEATURES=""

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
        --features)
            FEATURES="--features $2"
            shift 2
            ;;
        --features=*)
            FEATURES="--features=${1#*=}"
            shift
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
cargo build --bin enigmatick $COMMON_ARGS $FEATURES

echo "Building proxy binary..."
cd proxy && cargo build --target-dir ../target $COMMON_ARGS && cd ..

echo "Building tasks binary..."
cd tasks && cargo build --target-dir ../target $COMMON_ARGS && cd ..

echo "Building launcher with embedded binaries..."
cd launcher && cargo build $COMMON_ARGS && cd ..

echo "Build complete!"
