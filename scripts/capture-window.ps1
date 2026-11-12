[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ApplicationPath,
    [Parameter(Mandatory)][string]$WindowTitle
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tool = Join-Path $root 'window-capture.exe'
if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
    $tool = Join-Path $root 'target\x86_64-pc-windows-msvc\release\window-capture.exe'
}
if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
    throw 'Extract the entire Windows x64 release ZIP or run cargo build --release --locked --target x86_64-pc-windows-msvc from the repository root.'
}

$screenshots = Join-Path $root 'screenshots'
[IO.Directory]::CreateDirectory($screenshots) | Out-Null
$outputFile = Join-Path $screenshots ([guid]::NewGuid().ToString('N') + '.png')
& $tool $ApplicationPath $WindowTitle $outputFile
if ($LASTEXITCODE -ne 0) { throw "Window capture failed (exit $LASTEXITCODE)." }
$outputFile
