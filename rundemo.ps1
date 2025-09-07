# Usage: .\rundemo.ps1 [-SchemaPath <path>] [-Lang <lang>]
# Defaults: -SchemaPath examples/game_schema.poly -Lang csharp

param(
    [string]$SchemaPath = "examples/game_schema.poly",
    [string]$Lang = "csharp",
    [int]$CargoTimeoutSec = 120,
    [int]$DotnetTimeoutSec = 60
)

$ErrorActionPreference = "Stop"

function Write-Info($msg) { Write-Host "[rundemo] $msg" -ForegroundColor Cyan }
function Write-Step($msg) { Write-Host "==> $msg" -ForegroundColor Green }

function Invoke-WithTimeout([string]$FilePath, [string]$Arguments, [int]$TimeoutSec, [string]$Name) {
    Write-Info "실행: $FilePath $Arguments (timeout=${TimeoutSec}s)"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $FilePath
    $psi.Arguments = $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $false
    $psi.RedirectStandardError = $false
    $psi.CreateNoWindow = $true
    $proc = [System.Diagnostics.Process]::Start($psi)
    try {
        if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
            Write-Host "[rundemo] 경고: $Name 시간 초과. 프로세스 트리를 종료합니다." -ForegroundColor Yellow
            try {
                & taskkill /PID $proc.Id /T /F | Out-Null
            } catch {}
            try { $proc.Kill($true) } catch {}
            throw "$Name timed out after ${TimeoutSec}s"
        }
        if ($proc.ExitCode -ne 0) {
            throw "$Name exited with code $($proc.ExitCode)"
        }
    } finally {
        if (!$proc.HasExited) { try { $proc.Kill($true) } catch {} }
        $proc.Dispose()
    }
}

Push-Location $PSScriptRoot
try {
    Write-Step "Generating $Lang code from '$SchemaPath'"
    Invoke-WithTimeout -FilePath "cargo" -Arguments "run -- --schema-path `"$SchemaPath`" --lang `"$Lang`"" -TimeoutSec $CargoTimeoutSec -Name "cargo run"

    Write-Step "Building and running C# demo"
    $proj = Join-Path -Path $PSScriptRoot -ChildPath "dist/run-csharp/RunDemo/RunDemo.csproj"
    if (!(Test-Path $proj)) {
        throw "RunDemo project not found at $proj"
    }

    # Ensure output exists and looks sane
    $outCommon = Join-Path -Path $PSScriptRoot -ChildPath "output/$Lang/Common"
    if (!(Test-Path $outCommon)) {
        throw "Expected generated output at 'output/$Lang', but it was not found. Generation may have failed."
    }

    Invoke-WithTimeout -FilePath "dotnet" -Arguments "run --project `"$proj`"" -TimeoutSec $DotnetTimeoutSec -Name "dotnet run"
}
finally {
    Pop-Location
}
