param([Parameter(Mandatory=$true)][string]$Archive)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zipPath=(Resolve-Path -LiteralPath $Archive).Path
$expected=((Get-Content -LiteralPath "$zipPath.sha256" -Raw).Trim() -split '\s+')[0]
if((Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected){throw 'ZIP checksum mismatch'}
$zip=[IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $entries=@($zip.Entries|Where-Object{$_.Name})
    $names=@($entries|ForEach-Object{$_.FullName.Replace('\','/')})
    if(@($names|Where-Object{$_ -match '(?i)\.(pka|pkt|pkz|pksz|pem|key|ttf|otf)$|private-fixtures|/target/|\.\./'}).Count){throw 'Forbidden ZIP entry'}
    if(@($names|Where-Object{$_ -match '(?i)\.exe$'}).Count -ne 1){throw 'Expected exactly one application EXE'}
    foreach($suffix in @('/pka-lab-inspector.exe','/locales/en.json','/locales/vi.json','/LICENSE','/THIRD_PARTY_NOTICES.md','/SHA256.txt','/third-party/license-sources.json')){
        if(@($names|Where-Object{$_.EndsWith($suffix)}).Count -ne 1){throw "Missing package requirement: $suffix"}
    }
    foreach($fontNotice in @('Hack-Regular.txt','OFL.txt','UFL.txt','emoji-icon-font-mit-license.txt')){
        if(!@($names|Where-Object{$_.EndsWith("/$fontNotice")})){throw "Missing embedded-font notice: $fontNotice"}
    }
    $exeEntry=$entries|Where-Object{$_.FullName.Replace('\','/').EndsWith('/pka-lab-inspector.exe')}
    $hashEntry=$entries|Where-Object{$_.FullName.Replace('\','/').EndsWith('/SHA256.txt')}
    $reader=[IO.StreamReader]::new($hashEntry.Open());try{$exeExpected=($reader.ReadToEnd().Trim() -split '\s+')[0]}finally{$reader.Dispose()}
    $stream=$exeEntry.Open();$sha=[Security.Cryptography.SHA256]::Create()
    try{$exeActual=([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$sha.Dispose()}
    if($exeActual -ne $exeExpected){throw 'EXE checksum mismatch'}
    $stream=$exeEntry.Open();$memory=[IO.MemoryStream]::new()
    try{$stream.CopyTo($memory);$bytes=$memory.ToArray()}finally{$stream.Dispose();$memory.Dispose()}
    if($bytes.Length -lt 64 -or $bytes[0] -ne 0x4d -or $bytes[1] -ne 0x5a){throw 'Invalid executable header'}
    $peOffset=[BitConverter]::ToInt32($bytes,60)
    if($peOffset -lt 64 -or $peOffset -gt $bytes.Length-6 -or [BitConverter]::ToUInt32($bytes,$peOffset) -ne 0x4550 -or [BitConverter]::ToUInt16($bytes,$peOffset+4) -ne 0x8664){throw 'Expected Windows x86_64 PE executable'}
    $manifestEntry=$entries|Where-Object{$_.FullName.Replace('\','/').EndsWith('/third-party/license-sources.json')}
    $reader=[IO.StreamReader]::new($manifestEntry.Open());try{$notices=$reader.ReadToEnd()|ConvertFrom-Json}finally{$reader.Dispose()}
    foreach($notice in $notices){
        $entry=$entries|Where-Object{$_.FullName.Replace('\','/').EndsWith('/'+$notice.file)}
        if(!$entry){throw "Missing supplemental notice: $($notice.file)"}
        $stream=$entry.Open();$sha=[Security.Cryptography.SHA256]::Create()
        try{$actual=([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$sha.Dispose()}
        if($actual -ne $notice.sha256){throw "Supplemental notice checksum mismatch: $($notice.file)"}
    }
    "Verified ZIP/EXE SHA256, Windows x86_64 PE, $($entries.Count) files, $($notices.Count) supplemental notice digests and embedded-font notices; no private activity or font payloads."
}finally{$zip.Dispose()}
