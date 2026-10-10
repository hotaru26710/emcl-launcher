$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$exePath = Join-Path $repoRoot "src-tauri\target\release\emcl-launcher.exe"

if (-not (Test-Path -LiteralPath $exePath)) {
    throw "EMCL executable was not found. Run `pnpm tauri build --no-bundle` first."
}

$desktop = [Environment]::GetFolderPath("Desktop")
$shortcutPath = Join-Path $desktop "EMCL.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.WorkingDirectory = Split-Path -Parent $exePath
$shortcut.IconLocation = "$exePath,0"
$shortcut.Description = "EMCL - Ember Minecraft Launcher"
$shortcut.Save()

Write-Output "Created shortcut: $shortcutPath"
