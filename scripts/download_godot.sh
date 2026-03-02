#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT_DIR="$ROOT_DIR/ui/godot/project/tools"
GODOT_ZIP="$GODOT_DIR/godot_linux_4.6.1.zip"
GODOT_BIN="$GODOT_DIR/Godot_v4.6.1-stable_linux.x86_64"
GODOT_URL="https://downloads.godotengine.org/?version=4.6.1&flavor=stable&slug=linux.x86_64.zip&platform=linux.64"

mkdir -p "$GODOT_DIR"

if [ ! -f "$GODOT_ZIP" ]; then
  curl -L "$GODOT_URL" -o "$GODOT_ZIP"
fi

if [ ! -f "$GODOT_BIN" ]; then
  unzip -o "$GODOT_ZIP" -d "$GODOT_DIR" >/dev/null
  chmod +x "$GODOT_BIN"
fi

echo "Godot ready: $GODOT_BIN"
