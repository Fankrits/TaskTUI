# TaskTUI - Windows PowerShell Installer
# Usage: irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo = "Fankrits/TaskTUI"
$BinName = "tasktui"
$ExeName = "$BinName.exe"

Write-Host "⚡ TaskTUI Windows Installer" -ForegroundColor Cyan
Write-Host "==============================" -ForegroundColor Gray

# 1. Architecture Check
$Arch = "x86_64"
$Target = "$Arch-pc-windows-msvc"

# 2. Determine Release Version
Write-Host "Fetching latest release from GitHub... " -NoNewline
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "TaskTUI-Installer" }
    $Tag = $Release.tag_name
    Write-Host "$Tag" -ForegroundColor Green
} catch {
    $Tag = "v0.1.0"
    Write-Host "$Tag (fallback)" -ForegroundColor Yellow
}

# 3. Download Archive
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir | Out-Null

$ZipFile = Join-Path $TempDir "tasktui.zip"
$Urls = @(
    "https://github.com/$Repo/releases/download/$Tag/tasktui-$Tag-$Target.zip",
    "https://github.com/$Repo/releases/download/$Tag/task_tui-$Tag-$Target.zip",
    "https://github.com/$Repo/releases/download/$Tag/tasktui-$Target.zip"
)

Write-Host "Downloading $BinName..."
$Downloaded = $false

foreach ($Url in $Urls) {
    try {
        Invoke-WebRequest -Uri $Url -OutFile $ZipFile -UseBasicParsing
        $Downloaded = $true
        break
    } catch {
        # continue
    }
}

if (-not $Downloaded) {
    Write-Host "Error: Failed to download release archive from GitHub." -ForegroundColor Red
    Remove-Item -Recurse -Force $TempDir
    exit 1
}

# 4. Extract
Expand-Archive -Path $ZipFile -DestinationPath $TempDir -Force

$ExtractedExe = Get-ChildItem -Path $TempDir -Filter "*.exe" -Recurse | Where-Object { $_.Name -match "^(tasktui|task_tui)\.exe$" } | Select-Object -First 1
if (-not $ExtractedExe) {
    Write-Host "Error: Could not find tasktui.exe in extracted archive." -ForegroundColor Red
    Remove-Item -Recurse -Force $TempDir
    exit 1
}

# 5. Install to User Program Directory
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\TaskTUI"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$DestinationExe = Join-Path $InstallDir $ExeName
Copy-Item -Path $ExtractedExe.FullName -Destination $DestinationExe -Force

# Also provide task_tui.exe copy
$LegacyExe = Join-Path $InstallDir "task_tui.exe"
Copy-Item -Path $ExtractedExe.FullName -Destination $LegacyExe -Force

# Clean up temp
Remove-Item -Recurse -Force $TempDir

# 6. Add to User PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
$PathEntries = $UserPath -split ';' | ForEach-Object { $_.TrimEnd('\') }

if ($PathEntries -notcontains $InstallDir.TrimEnd('\')) {
    $NewPath = if ([string]::IsNullOrWhiteSpace($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added $InstallDir to User PATH." -ForegroundColor Gray
}

Write-Host ""
Write-Host "🎉 TaskTUI was successfully installed!" -ForegroundColor Green
Write-Host "Restart your terminal or run:" -ForegroundColor Cyan
Write-Host "  tasktui" -ForegroundColor White
