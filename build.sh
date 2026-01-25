#!/bin/bash

# Build script for obj2brs - builds for both Linux and Windows

set -e  # Exit on error

echo "==================================="
echo "Building obj2brs for all platforms"
echo "==================================="

# Create output directory
mkdir -p dist

echo ""
echo "Building for Linux (x86_64)..."
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/obj2brs dist/obj2brs-linux-x86_64

echo ""
echo "Building for Windows (x86_64)..."
cargo build --release --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/obj2brs.exe dist/obj2brs-windows-x86_64.exe

echo ""
echo "==================================="
echo "Build complete! Binaries are in dist/"
echo "==================================="
ls -lh dist/
