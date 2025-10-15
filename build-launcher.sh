#!/bin/bash
set -e

echo "Building Enigmatick launcher with embedded binaries..."

# Build each component separately
echo "Building main enigmatick binary..."
cargo build --bin enigmatick ${1:-}

echo "Building proxy binary..."
cd proxy && cargo build --target-dir ../target ${1:-} && cd ..

echo "Building tasks binary..."
cd tasks && cargo build --target-dir ../target ${1:-} && cd ..

echo "Building launcher with embedded binaries..."
cd launcher && cargo build ${1:-} && cd ..

echo "Build complete! Launcher binary is at: launcher/target/$(if [ "$1" = "--release" ]; then echo "release"; else echo "debug"; fi)/enigmatick"
