# Publication safety checks: scan reachable history without printing content.
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot
$objects = & git rev-list --objects HEAD
if ($LASTEXITCODE -ne 0) { throw 'Cannot enumerate history' }
$forbiddenPaths = @($objects | Where-Object { $_ -match '(?i) (?:.*[/\\])?(?:private-fixtures|target|dist)[/\\]|\.(?:pka|pkt|pkz|pksz|exe|zip|pem|key)$' })
if ($forbiddenPaths.Count) { throw "Forbidden historical paths: $($forbiddenPaths.Count)" }
$patterns = @('gh[pousr]_[A-Za-z0-9]{30,}','github_pat_[A-Za-z0-9_]{30,}','sk-(?:proj-)?[A-Za-z0-9_-]{35,}','AKIA[0-9A-Z]{16}','-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----','[A-Za-z]:[\\/](?:Users|HUTECH)[\\/]')
$blobs=0
foreach ($object in $objects) {
    $id=($object -split ' ',2)[0]
    if ((& git cat-file -t $id) -ne 'blob') { continue }
    $blobs++
    $content=(& git cat-file blob $id) -join "`n"
    foreach ($pattern in $patterns) {
        if ($content -cmatch $pattern) { throw "Potential private content in blob $id; review locally (content suppressed)" }
    }
}
$notices=Get-Content third-party/license-sources.json -Raw | ConvertFrom-Json
foreach ($notice in $notices) {
    if ((Get-FileHash -LiteralPath $notice.file -Algorithm SHA256).Hash.ToLowerInvariant() -ne $notice.sha256) { throw "Supplemental notice digest mismatch: $($notice.file)" }
}
"Checked $blobs reachable blobs and $($notices.Count) notice digests; no forbidden patterns found. This is a heuristic check, not a guarantee."
