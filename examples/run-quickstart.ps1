param(
    [string]$Polygen = "",
    [ValidateSet("csharp", "rust", "sqlite", "all")]
    [string]$Lang = "csharp",
    [string]$OutputDir = "output\quickstart"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Polygen)) {
    if (Test-Path -LiteralPath ".\polygen.exe") {
        $Polygen = ".\polygen.exe"
    } else {
        $Polygen = "polygen.exe"
    }
}

$targets = if ($Lang -eq "all") { @("csharp", "rust", "sqlite") } else { @($Lang) }

foreach ($target in $targets) {
    $targetOutput = if ($Lang -eq "all") { Join-Path $OutputDir $target } else { $OutputDir }
    & $Polygen --schema-path examples\quickstart.poly --sources examples\quickstart.sources.toml --lang $target --output-dir $targetOutput
}

Write-Host "Quickstart generated into $OutputDir"
