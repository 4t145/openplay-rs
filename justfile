set shell := ["bash", "-cu"]

set windows-powershell := true
# 跨平台环境配置
os := os()
# 根据操作系统选择库前缀/后缀
lib_prefix := if os == "windows" { "" } else { "lib" }
lib_ext := if os == "windows" { "dll" } else if os == "macos" { "dylib" } else { "so" }
run_prefix := if os == "windows" { "&" } else { "" }

# Godot 可执行文件名称 (默认 Linux，其他环境需确认可执行文件存在)
godot_bin_name := if os == "windows" { "Godot_v4.6.1-stable_win64.exe" } else if os == "macos" { "Godot.app/Contents/MacOS/Godot" } else { "Godot_v4.6.1-stable_linux.x86_64" }
godot_bin := "ui/godot/project/tools/" + godot_bin_name

download_cmd := if os == "windows" { "powershell -ExecutionPolicy Bypass -File scripts/download_godot.ps1" } else { "bash scripts/download_godot.sh" }

godot-download:
    {{download_cmd}}

godot-build:
    cargo build -p openplay_sdk

# 统一的同步命令，自动适配当前操作系统
godot-sync:
    @echo "Syncing for OS: {{os}}..."
    cargo build -p openplay_sdk
    cp "target/debug/{{lib_prefix}}openplay_sdk.{{lib_ext}}" "ui/godot/project/addons/openplay_sdk/bin/"

godot-sync-release:
    @echo "Syncing release for OS: {{os}}..."
    cargo build -p openplay_sdk --release
    cp "target/release/{{lib_prefix}}openplay_sdk.{{lib_ext}}" "ui/godot/project/addons/openplay_sdk/bin/"

# 兼容旧命令
godot-sync-linux: godot-sync
godot-sync-release-linux: godot-sync-release

godot-clean:
    cargo clean -p openplay_sdk

godot-import:
    {{run_prefix}} "{{godot_bin}}" --path "ui/godot/project" --headless --editor --quit

godot-run:
    {{run_prefix}} "{{godot_bin}}" --path "ui/godot/project"

godot-sync-run: godot-sync godot-run
