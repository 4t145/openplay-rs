set shell := ["bash", "-cu"]

set windows-powershell := true
# 跨平台环境配置
os := os()
# 根据操作系统选择库前缀/后缀
lib_prefix := if os == "windows" { "" } else { "lib" }
lib_ext := if os == "windows" { "dll" } else if os == "macos" { "dylib" } else { "so" }
run_prefix := if os == "windows" { "&" } else { "" }

