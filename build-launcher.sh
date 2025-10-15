#!/bin/bash
set -e

echo "Building Enigmatick launcher with embedded binaries..."

# Get the script's directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

# Parse arguments - separate features from target/release flags
RELEASE_FLAG=""
TARGET_FLAG=""
MAIN_FEATURES=""
PROXY_FEATURES=""

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
            # Parse features to extract vendored-openssl for proxy
            IFS=',' read -ra FEAT_ARRAY <<< "$2"
            MAIN_FEATURES="--features $2"
            for feat in "${FEAT_ARRAY[@]}"; do
                if [[ "$feat" == "vendored-openssl" ]]; then
                    PROXY_FEATURES="--features vendored-openssl"
                fi
            done
            shift 2
            ;;
        --features=*)
            FEATURES_VALUE="${1#*=}"
            IFS=',' read -ra FEAT_ARRAY <<< "$FEATURES_VALUE"
            MAIN_FEATURES="--features=$FEATURES_VALUE"
            for feat in "${FEAT_ARRAY[@]}"; do
                if [[ "$feat" == "vendored-openssl" ]]; then
                    PROXY_FEATURES="--features vendored-openssl"
                fi
            done
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
cargo build --bin enigmatick $COMMON_ARGS $MAIN_FEATURES

echo "Building proxy binary..."
cd "$SCRIPT_DIR/proxy" && cargo build --target-dir ../target $COMMON_ARGS $PROXY_FEATURES && cd "$SCRIPT_DIR"

echo "Building tasks binary..."
cd "$SCRIPT_DIR/tasks" && cargo build --target-dir ../target $COMMON_ARGS && cd "$SCRIPT_DIR"

echo "Building launcher with embedded binaries..."
cd "$SCRIPT_DIR/launcher" && cargo build $COMMON_ARGS && cd "$SCRIPT_DIR"

echo "Build complete!"
