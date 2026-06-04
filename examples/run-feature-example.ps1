param(
    [string]$Polygen = "",
    [ValidateSet("constraints", "relations", "sources", "pack", "search", "all")]
    [string]$Feature = "constraints",
    [string]$Lang = "csharp",
    [string]$OutputDir = "output\features"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Polygen)) {
    if (Test-Path -LiteralPath ".\polygen.exe") {
        $Polygen = ".\polygen.exe"
    } else {
        $Polygen = "polygen.exe"
    }
}

$examples = @{
    constraints = @{
        schema = "examples\features\constraints.poly"
        output = "constraints"
    }
    relations = @{
        schema = "examples\features\relations_indexes.poly"
        output = "relations"
    }
    sources = @{
        schema = "examples\features\sources.poly"
        sources = "examples\features\sources.sources.toml"
        output = "sources"
    }
    pack = @{
        schema = "examples\features\pack_embed.poly"
        output = "pack"
    }
    search = @{
        schema = "examples\features\search.poly"
        output = "search"
    }
}

$names = if ($Feature -eq "all") {
    @("constraints", "relations", "sources", "pack", "search")
} else {
    @($Feature)
}

foreach ($name in $names) {
    $example = $examples[$name]
    $targetOutput = if ($Feature -eq "all") {
        Join-Path $OutputDir $example.output
    } else {
        $OutputDir
    }

    $args = @(
        "--schema-path", $example.schema,
        "--lang", $Lang,
        "--output-dir", $targetOutput
    )

    if ($example.ContainsKey("sources")) {
        $args = @(
            "--schema-path", $example.schema,
            "--sources", $example.sources,
            "--lang", $Lang,
            "--output-dir", $targetOutput
        )
    }

    & $Polygen @args
}

Write-Host "Feature example generated into $OutputDir"
