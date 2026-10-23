[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$BinaryDirectory,
    [Parameter(Mandatory)][string]$SidecarDirectory,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../dist'),
    [string]$Compiler = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'
)
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$binaryRoot = (Resolve-Path -LiteralPath $BinaryDirectory).Path
$sidecarRoot = (Resolve-Path -LiteralPath $SidecarDirectory).Path
$toolsRoot = Join-Path $sidecarRoot 'tools'
foreach ($file in @('silicate.exe', 'rizum_silicate_thumb.dll')) {
    if (-not (Test-Path -LiteralPath (Join-Path $binaryRoot $file) -PathType Leaf)) {
        throw "Missing release binary: $file"
    }
}
foreach ($file in @('ffmpeg.exe', 'ffprobe.exe', 'FFMPEG-LICENSE.txt', 'NOTICE.txt',
                    'build/binary-sha256.txt', 'build/build-ffmpeg.sh')) {
    if (-not (Test-Path -LiteralPath (Join-Path $toolsRoot $file) -PathType Leaf)) {
        throw "Missing source-built sidecar material: $file"
    }
}
if (-not (Test-Path -LiteralPath (Join-Path $toolsRoot 'sources') -PathType Container)) {
    throw 'Missing FFmpeg source archives'
}
if (-not (Test-Path -LiteralPath $Compiler -PathType Leaf)) {
    throw 'Install Inno Setup 6.5.4 or supply -Compiler with its ISCC.exe path'
}
$metadataText = & cargo metadata --locked --no-deps --format-version 1 --manifest-path (Join-Path $repository 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'Unable to read Cargo package metadata' }
$metadata = $metadataText | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -EQ 'silicate').version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Unsupported installer version: $version" }
$output = (New-Item -ItemType Directory -Force -Path $OutputDirectory).FullName
# Unique staging keeps rebuilds free of files left by earlier sidecar versions.
$stage = Join-Path $output ('windows-stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    Copy-Item -LiteralPath (Join-Path $binaryRoot 'silicate.exe') -Destination (Join-Path $stage 'Rizum Silicate.exe')
    Copy-Item -LiteralPath (Join-Path $binaryRoot 'rizum_silicate_thumb.dll') -Destination $stage
    Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination (Join-Path $stage 'LICENSE.txt')
    Copy-Item -LiteralPath $toolsRoot -Destination $stage -Recurse
    & $Compiler "/DPackageDir=$stage" "/DOutputDir=$output" "/DAppVersion=$version" (Join-Path $repository 'packaging/windows/silicate.iss')
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed: $LASTEXITCODE" }
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath (Join-Path $output 'Rizum-Silicate-windows-x86_64.zip') -Force
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    if (-not $resolvedStage.StartsWith($output.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Staging path escaped output directory'
    }
    Remove-Item -LiteralPath $resolvedStage -Recurse -Force
}
