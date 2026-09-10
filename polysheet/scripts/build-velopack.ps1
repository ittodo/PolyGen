[CmdletBinding()]
param(
    [string]$Version,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$polySheetRoot = Split-Path -Parent $PSScriptRoot
$tauriConfigPath = Join-Path $polySheetRoot "src-tauri\tauri.conf.json"
$tauriConfig = Get-Content -Raw -LiteralPath $tauriConfigPath | ConvertFrom-Json
if (-not $Version) {
    $Version = [string]$tauriConfig.version
}

$targetTriple = "x86_64-pc-windows-msvc"
$releaseRoot = Join-Path $polySheetRoot "src-tauri\target\$targetTriple\release"
$stageRoot = Join-Path $releaseRoot "velopack-stage"
$outputRoot = Join-Path $releaseRoot "velopack"
$compiledExe = Join-Path $releaseRoot "polysheet-app.exe"
$packagedExe = Join-Path $stageRoot "PolySheet.exe"
$iconPath = Join-Path (Split-Path -Parent $polySheetRoot) "gui\src-tauri\icons\icon.ico"

function Assert-ChildPath {
    param(
        [Parameter(Mandatory)]
        [string]$Parent,
        [Parameter(Mandatory)]
        [string]$Child
    )

    $parentPath = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    $childPath = [System.IO.Path]::GetFullPath($Child)
    if (-not $childPath.StartsWith($parentPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to modify a path outside '$parentPath': $childPath"
    }
}

Assert-ChildPath -Parent $releaseRoot -Child $stageRoot
Assert-ChildPath -Parent $releaseRoot -Child $outputRoot

Push-Location $polySheetRoot
try {
    & npm.cmd run tauri -- build --target $targetTriple --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri release build failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $compiledExe -PathType Leaf)) {
        throw "Tauri executable was not produced: $compiledExe"
    }
    if (-not (Test-Path -LiteralPath $iconPath -PathType Leaf)) {
        throw "PolySheet icon was not found: $iconPath"
    }

    if (Test-Path -LiteralPath $stageRoot) {
        Remove-Item -LiteralPath $stageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stageRoot | Out-Null
    Copy-Item -LiteralPath $compiledExe -Destination $packagedExe

    if ($Clean -and (Test-Path -LiteralPath $outputRoot)) {
        Remove-Item -LiteralPath $outputRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

    & vpk pack `
        --packId "PolyGen.PolySheet" `
        --packVersion $Version `
        --packDir $stageRoot `
        --mainExe "PolySheet.exe" `
        --packTitle "PolySheet" `
        --packAuthors "PolyGen Team" `
        --runtime "win10-x64" `
        --icon $iconPath `
        --shortcuts "Desktop,StartMenuRoot" `
        --outputDir $outputRoot `
        --skipVeloAppCheck true `
        --noPortable true
    if ($LASTEXITCODE -ne 0) {
        throw "Velopack packaging failed with exit code $LASTEXITCODE."
    }

    Write-Host "Velopack release created in: $outputRoot"
} finally {
    Pop-Location
}
