# TaskTUI - Windows PowerShell Uninstaller
# Usage: irm https://raw.githubusercontent.com/Fankrits/TaskTUI/main/uninstall.ps1 | iex

$ErrorActionPreference = 'SilentlyContinue'

Write-Host "⚡ TaskTUI Windows Uninstaller" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Gray

$Targets = @(
    (Join-Path $env:LOCALAPPDATA "Programs\TaskTUI\tasktui.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\TaskTUI\task_tui.exe"),
    (Join-Path $HOME ".local\bin\tasktui.exe"),
    (Join-Path $HOME ".local\bin\task_tui.exe"),
    (Join-Path $HOME ".cargo\bin\tasktui.exe"),
    (Join-Path $HOME ".cargo\bin\task_tui.exe")
)

$Removed = $false

foreach ($Target in $Targets) {
    if (Test-Path $Target) {
        Write-Host "Removing $Target..." -ForegroundColor Yellow
        Remove-Item -Force $Target -ErrorAction SilentlyContinue
        $Removed = $true
    }
}

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\TaskTUI"
if (Test-Path $InstallDir) {
    $Remaining = Get-ChildItem -Path $InstallDir
    if ($null -eq $Remaining -or $Remaining.Count -eq 0) {
        Remove-Item -Recurse -Force $InstallDir -ErrorAction SilentlyContinue
    }
}

if ($Removed) {
    Write-Host ""
    Write-Host "🎉 TaskTUI was successfully uninstalled." -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "No TaskTUI installations found in standard paths." -ForegroundColor Yellow
}
