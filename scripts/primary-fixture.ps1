[CmdletBinding()]
param(
    [ValidateSet('identity', 'runtime', 'gpu', 'all')]
    [string]$Mode = 'all',

    [string]$FixturePath = $env:RIZUM_SILICATE_PRIMARY_FIXTURE,

    [ValidateRange(1, 10000)]
    [int]$Iterations = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$fixtureName = 'Art_SystemPet_Default.procreate'
$expectedBytes = 169646073
$expectedSha256 = 'D34D8594BC3880549D06411123DF28237CF5ADAA58CBF9206C287E46AD189E73'

if ([string]::IsNullOrWhiteSpace($FixturePath)) {
    $FixturePath = Join-Path $HOME "iCloudDrive\Procreate\$fixtureName"
}

$fixture = Get-Item -LiteralPath $FixturePath -ErrorAction Stop
if ($fixture.Name -cne $fixtureName) {
    throw "Primary fixture must be named $fixtureName; received $($fixture.Name)."
}
if ($fixture.Length -ne $expectedBytes) {
    throw "Primary fixture byte count mismatch: expected $expectedBytes, received $($fixture.Length)."
}

# Identity is checked before expensive work so benchmark history cannot silently
# move to a different document revision with the same local filename.
$sha256 = (Get-FileHash -LiteralPath $fixture.FullName -Algorithm SHA256).Hash
if ($sha256 -cne $expectedSha256) {
    throw "Primary fixture SHA-256 mismatch: expected $expectedSha256, received $sha256."
}

Write-Host "fixture=$($fixture.FullName)"
Write-Host "fixture_bytes=$($fixture.Length)"
Write-Host "fixture_sha256=$sha256"

if ($Mode -eq 'identity') {
    return
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
try {
    if ($Mode -in @('runtime', 'all')) {
        & cargo run --release -p silicate-runtime --example benchmark_open --locked -- `
            $fixture.FullName $Iterations
        if ($LASTEXITCODE -ne 0) {
            throw "Runtime benchmark failed with exit code $LASTEXITCODE."
        }
    }

    if ($Mode -in @('gpu', 'all')) {
        & cargo run --release -p silica-gpu --example verify_runtime_visibility --locked -- `
            $fixture.FullName
        if ($LASTEXITCODE -ne 0) {
            throw "GPU mutation smoke failed with exit code $LASTEXITCODE."
        }
    }
}
finally {
    Pop-Location
}
