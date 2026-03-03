
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GODOT_DIR="$ROOT_DIR/ui/godot/project/tools"
GODOT_VERSION="4.6.1"

mkdir -p "$GODOT_DIR"

OS="$(uname -s)"
case "$OS" in
    Linux*)
        PLATFORM="linux"
        FILENAME="Godot_v${GODOT_VERSION}-stable_linux.x86_64"
        ZIP_NAME="${FILENAME}.zip"
        BIN_CHECK="$FILENAME"
        DOWNLOAD_URL="https://github.com/godotengine/godot/releases/download/${GODOT_VERSION}-stable/${ZIP_NAME}"
        ;;
    Darwin*)
        PLATFORM="macos"
        FILENAME="Godot_v${GODOT_VERSION}-stable_macos.universal"
        ZIP_NAME="${FILENAME}.zip"
        BIN_CHECK="Godot.app/Contents/MacOS/Godot"
        DOWNLOAD_URL="https://github.com/godotengine/godot/releases/download/${GODOT_VERSION}-stable/${ZIP_NAME}"
        ;;
    *)
        echo "Unsupported OS for this script: $OS. Please use the PowerShell script for Windows."
        exit 1
        ;;
esac

LOCAL_ZIP="$GODOT_DIR/$ZIP_NAME"
TARGET_BIN="$GODOT_DIR/$BIN_CHECK"

if [ -e "$TARGET_BIN" ]; then
    echo "Godot ($PLATFORM) is already installed at: $TARGET_BIN"
    exit 0
fi

echo "Downloading Godot ${GODOT_VERSION} for ${PLATFORM}..."

if [ ! -f "$LOCAL_ZIP" ]; then
    echo "Downloading from $DOWNLOAD_URL..."
    curl -L "$DOWNLOAD_URL" -o "$LOCAL_ZIP"
fi

echo "Extracting..."
unzip -o "$LOCAL_ZIP" -d "$GODOT_DIR" >/dev/null

if [ "$PLATFORM" == "linux" ]; then
    chmod +x "$TARGET_BIN"
elif [ "$PLATFORM" == "macos" ]; then
    chmod +x "$TARGET_BIN"
fi

echo "Godot ready: $TARGET_BIN"
