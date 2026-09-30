# Downloads the default BepInEx builds into target\bepinex for the installer
# to bundle, so players who can't reach builds.bepinex.dev still get a
# working setup. The URLs are read from the app's defaults so the bundled
# zips always match what the app would otherwise download.
$ErrorActionPreference = 'Stop'

# SHA-256 of the default builds (6.0.0-be.752), so the installer never ships
# a substituted archive. Update these whenever the default URLs change.
$expected = @{
    x86 = 'BF83AC97C959CE4012011872A2A381BD7F7D57932E3ED0D2553EAF1C81236664'
    x64 = 'F9D128E162269579B67E91A4923764F3D54FA8C8162A238CE6657D704084334C'
}
$root = Resolve-Path "$PSScriptRoot\..\.."
$source = Get-Content "$root\src\backend\services\core_service.rs" -Raw
$out = New-Item -ItemType Directory -Force "$root\target\bepinex"

foreach ($arch in 'x86', 'x64') {
    $name = "DEFAULT_BEPINEX_URL_$($arch.ToUpper())"
    $match = [regex]::Match($source, "$name`: &str = `"([^`"]+)`"")
    if (-not $match.Success) { throw "$name not found in core_service.rs" }
    # The host drops connections mid-transfer, so retry every kind of failure.
    $zip = "$out\bepinex-$arch.zip"
    curl.exe -fsSL --retry 5 --retry-all-errors --retry-delay 5 -o $zip $match.Groups[1].Value
    if ($LASTEXITCODE -ne 0) { throw "Downloading the $arch BepInEx build failed" }
    $hash = (Get-FileHash $zip -Algorithm SHA256).Hash
    if ($hash -ne $expected[$arch]) {
        Remove-Item $zip
        throw "The $arch BepInEx build has SHA-256 $hash, expected $($expected[$arch])"
    }
}
