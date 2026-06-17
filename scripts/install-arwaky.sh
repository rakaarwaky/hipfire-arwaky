#!/bin/bash
# hipfire-arwaky installer — installs to ~/.hipfire-arwaky
# Usage: bash scripts/install-arwaky.sh
set -euo pipefail

HIPFIRE_DIR="$HOME/.hipfire-arwaky"
BIN_DIR="$HIPFIRE_DIR/bin"
MODELS_DIR="$HIPFIRE_DIR/models"
SRC_DIR="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$SRC_DIR/target/release"

echo "=== hipfire-arwaky installer ==="
echo ""

# Build first
echo "Building binaries..."
cd "$SRC_DIR"
cargo xtask patch --force 2>/dev/null || true
cargo build --release -p hipfire-runtime --bin hipfire-arwaky 2>&1 | tail -3
cargo build --release -p hipfire-arwaky-bin 2>&1 | tail -3
cargo build --release -p hipfire-tui 2>&1 | tail -3
echo ""

# Check binaries exist
if [ ! -f "$TARGET_DIR/hipfire-arwaky" ]; then
    echo "ERROR: Build failed — $TARGET_DIR/hipfire-arwaky not found"
    exit 1
fi

# Create directories
mkdir -p "$BIN_DIR" "$MODELS_DIR"

# Install binaries
echo "Installing binaries to $BIN_DIR/ ..."
cp "$TARGET_DIR/hipfire-arwaky" "$BIN_DIR/hipfire-arwaky"
cp "$TARGET_DIR/hipfire-arwaky-run" "$BIN_DIR/hipfire-arwaky-run"
cp "$TARGET_DIR/hipfire-arwaky-tui" "$BIN_DIR/hipfire-arwaky-tui" 2>/dev/null || true
chmod +x "$BIN_DIR/hipfire-arwaky" "$BIN_DIR/hipfire-arwaky-run" "$BIN_DIR/hipfire-arwaky-tui" 2>/dev/null || true

# Create symlink for easy access
echo "Creating symlink..."
ln -sf "$BIN_DIR/hipfire-arwaky" "$BIN_DIR/hfa"

# Add to PATH if not already there
if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    SHELL_RC=""
    if [ -f "$HOME/.bashrc" ]; then
        SHELL_RC="$HOME/.bashrc"
    elif [ -f "$HOME/.zshrc" ]; then
        SHELL_RC="$HOME/.zshrc"
    fi
    if [ -n "$SHELL_RC" ]; then
        echo "" >> "$SHELL_RC"
        echo "# hipfire-arwaky" >> "$SHELL_RC"
        echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$SHELL_RC"
        echo "Added $BIN_DIR to PATH in $SHELL_RC"
        echo "Run 'source $SHELL_RC' or restart your shell."
    fi
fi

echo ""
echo "=== Installation complete ==="
echo ""
echo "  Binaries:  $BIN_DIR/"
echo "  Models:    $MODELS_DIR/"
echo ""
echo "Usage:"
echo "  hipfire-arwaky run /path/to/model.mq4"
echo "  hipfire-arwaky config              # TUI config editor"
echo "  hipfire-arwaky list                # list models"
echo "  hipfire-arwaky version"
echo ""
echo "Short alias: hfa run /path/to/model.mq4"
