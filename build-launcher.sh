#!/bin/bash
set -e

echo "Building Enigmatick launcher with embedded binaries..."

# Just pass all arguments directly to cargo
BUILD_ARGS="$@"

# Build each component separately
echo "Building main enigmatick binary..."
cargo build --bin enigmatick $BUILD_ARGS

echo "Building proxy binary..."
cd proxy && cargo build --target-dir ../target $BUILD_ARGS && cd ..

echo "Building tasks binary..."
cd tasks && cargo build --target-dir ../target $BUILD_ARGS && cd ..

echo "Building launcher with embedded binaries..."
cd launcher && cargo build $BUILD_ARGS && cd ..

echo "Build complete!"
