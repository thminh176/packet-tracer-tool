param([switch]$SkipChecks)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot
$releaseVersion = 'v0.1.0'
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
$cargoPath = if ($cargoCommand) { $cargoCommand.Source } else { Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe' }
function Invoke-Cargo([string[]]$Arguments) {
    & $cargoPath @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo $Arguments failed ($LASTEXITCODE)" }
}
if (-not $SkipChecks) {
    Invoke-Cargo @('fmt','--all','--','--check')
    Invoke-Cargo @('clippy','--locked','--all-targets','--','-D','warnings')
    Invoke-Cargo @('test','--locked')
    Invoke-Cargo @('test','--manifest-path','vendor/pktfile/Cargo.toml','--lib')
}
Invoke-Cargo @('build','--locked','--release')
$packageDir = Join-Path $repoRoot "dist/packet-tracer-tool-$releaseVersion-windows-x64"
if (Test-Path -LiteralPath $packageDir) { throw 'Package directory already exists. Use a clean build checkout or move the old package before rebuilding.' }
New-Item -ItemType Directory -Force $packageDir | Out-Null
Copy-Item -LiteralPath 'target/release/pka-lab-inspector.exe','README.md','README.vi.md','LICENSE','THIRD_PARTY_NOTICES.md' -Destination $packageDir -Force
Copy-Item -LiteralPath 'docs','locales' -Destination $packageDir -Recurse -Force
Copy-Item -LiteralPath 'third-party' -Destination $packageDir -Recurse -Force
$licenseDir = Join-Path $packageDir 'third-party-licenses'
New-Item -ItemType Directory -Force $licenseDir | Out-Null
$metadataText = & $cargoPath metadata --locked --format-version 1
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$metadata = $metadataText | ConvertFrom-Json
foreach ($package in $metadata.packages) {
    if ($package.name -eq 'pka-lab-inspector') { continue }
    $sourceDir = Split-Path -Parent $package.manifest_path
    $destinationDir = Join-Path $licenseDir "$($package.name)-$($package.version)"
    New-Item -ItemType Directory -Force $destinationDir | Out-Null
    $licenseFiles = @(Get-ChildItem -LiteralPath $sourceDir -File -Recurse | Where-Object { $_.Name -match '(?i)(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE|AUTHORS|^OFL|^UFL|Hack-Regular.txt)' })
    if ($package.license_file) {
        $explicit = Join-Path $sourceDir $package.license_file
        if (Test-Path -LiteralPath $explicit) { $licenseFiles += Get-Item -LiteralPath $explicit }
    }
    foreach ($license in $licenseFiles) {
        $relative = $license.FullName.Substring($sourceDir.Length).TrimStart([char[]]'\/')
        $target = Join-Path $destinationDir $relative
        New-Item -ItemType Directory -Force (Split-Path -Parent $target) | Out-Null
        Copy-Item -LiteralPath $license.FullName -Destination $target -Force
    }
    "Crate: $($package.name)`nVersion: $($package.version)`nLicense: $($package.license)`nRepository: $($package.repository)" | Set-Content -LiteralPath (Join-Path $destinationDir 'CRATE.txt') -Encoding utf8
}
$exeHash = (Get-FileHash -LiteralPath (Join-Path $packageDir 'pka-lab-inspector.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
"$exeHash  pka-lab-inspector.exe" | Set-Content -Encoding ascii -LiteralPath (Join-Path $packageDir 'SHA256.txt')
$archive = Join-Path $repoRoot "dist/packet-tracer-tool-$releaseVersion-windows-x64.zip"
# Registry archives often use a reproducible 1970 timestamp, outside ZIP's range.
Get-ChildItem -LiteralPath $packageDir -Recurse | Where-Object { $_.LastWriteTime.Year -lt 1980 -or $_.LastWriteTime.Year -gt 2107 } | ForEach-Object { $_.LastWriteTime = [datetime]'2000-01-01T12:00:00' }
Compress-Archive -LiteralPath $packageDir -DestinationPath $archive -Force
$zipHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$zipHash  $([IO.Path]::GetFileName($archive))" | Set-Content -Encoding ascii -LiteralPath "$archive.sha256"
Write-Output "Local package: $archive"
