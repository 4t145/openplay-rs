set shell := ["bash", "-cu"]

godot-download:
    bash "scripts/download_godot.sh"

godot-build:
    cargo build -p openplay_sdk

godot-sync:
    cargo build -p openplay_sdk --target x86_64-pc-windows-gnu
    cp "target/x86_64-pc-windows-gnu/debug/openplay_sdk.dll" "ui/godot/project/addons/openplay_sdk/bin/"

godot-sync-release:
    cargo build -p openplay_sdk --release --target x86_64-pc-windows-gnu
    cp "target/x86_64-pc-windows-gnu/release/openplay_sdk.dll" "ui/godot/project/addons/openplay_sdk/bin/"

godot-sync-linux:
    cargo build -p openplay_sdk
    cp "target/debug/libopenplay_sdk.so" "ui/godot/project/addons/openplay_sdk/bin/"

godot-sync-run-linux:
    just godot-sync-linux
    "ui/godot/project/tools/Godot_v4.6.1-stable_linux.x86_64" --path "ui/godot/project"

godot-sync-release-linux:
    cargo build -p openplay_sdk --release
    cp "target/release/libopenplay_sdk.so" "ui/godot/project/addons/openplay_sdk/bin/"

godot-clean:
    cargo clean -p openplay_sdk

godot-import:
    "ui/godot/project/tools/Godot_v4.6.1-stable_linux.x86_64" --path "ui/godot/project" --headless --editor --quit

godot-run:
    "ui/godot/project/tools/Godot_v4.6.1-stable_linux.x86_64" --path "ui/godot/project"
