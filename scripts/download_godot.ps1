$ErrorActionPreference = "Stop"

$scriptPath = $PSScriptRoot
# Navigate up from scripts/ to root/ then to ui/godot/project/tools
$godotDir = Join-Path (Split-Path -Parent $scriptPath) "ui\godot\project\tools"
$godotVersion = "4.6.1"

# Create directory if it doesn't exist
if (-not (Test-Path -Path $godotDir)) {
    New-Item -ItemType Directory -Path $godotDir | Out-Null
}

$fileName = "Godot_v${godotVersion}-stable_win64.exe"
$zipName = "${fileName}.zip"
$downloadUrl = "https://github.com/godotengine/godot/releases/download/${godotVersion}-stable/${zipName}"
$localZip = Join-Path $godotDir $zipName
$targetBin = Join-Path $godotDir $fileName

# Check if already installed
if (Test-Path -Path $targetBin) {
    Write-Host "Godot (Windows) is already installed at: $targetBin"
    exit 0
}

Write-Host "Downloading Godot ${godotVersion} for Windows..."

if (-not (Test-Path -Path $localZip)) {
    Write-Host "Downloading from $downloadUrl..."
    # Using curl (aliased or real) or Invoke-WebRequest depending on environment, but PS core prefers Invoke-WebRequest
    Invoke-WebRequest -Uri $downloadUrl -OutFile $localZip
}

Write-Host "Extracting..."
Expand-Archive -Path $localZip -DestinationPath $godotDir -Force

Write-Host "Godot ready: $targetBin"
