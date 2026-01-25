#!/bin/bash

# Setup script for cross-compilation

echo "Setting up cross-compilation targets..."
echo ""

# Add Windows target
echo "Adding Windows target (x86_64-pc-windows-gnu)..."
rustup target add x86_64-pc-windows-gnu

# Add Linux target (usually already installed)
echo "Adding Linux target (x86_64-unknown-linux-gnu)..."
rustup target add x86_64-unknown-linux-gnu

echo ""
echo "Installing MinGW-w64 (Windows cross-compiler)..."
if command -v apt-get &> /dev/null; then
    sudo apt-get install -y mingw-w64
elif command -v pacman &> /dev/null; then
    sudo pacman -S --needed mingw-w64-gcc
elif command -v dnf &> /dev/null; then
    sudo dnf install -y mingw64-gcc
else
    echo "Please install mingw-w64 manually for your distribution"
fi

echo ""
echo "Setup complete! You can now run ./build.sh to build for all platforms."
