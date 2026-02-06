#!/usr/bin/env nix-shell
#!nix-shell -i bash -p wayland libxkbcommon libGL vulkan-loader xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr

# Run script for the renderer with proper library paths from Nix

set -e

# Set display for X11 (fallback if Wayland not available)
export DISPLAY="${DISPLAY:-:0}"

# Run the renderer
cd "$(dirname "$0")"
echo "Running renderer with Nix-provided libraries..."
echo "Display: $DISPLAY"
cargo run --release -- "$@"
