# Downloads the default BepInEx builds into target\bepinex for the installer
# to bundle, so players who can't reach builds.bepinex.dev still get a
# working setup. The URLs are read from the app's defaults so the bundled
# zips always match what the app would otherwise download.
$ErrorActionPreference = 'Stop'
$root = Resolve-Path "$PSScriptRoot\..\.."
$source = Get-Content "$root\src\backend\services\core_service.rs" -Raw
$out = New-Item -ItemType Directory -Force "$root\target\bepinex"

foreach ($arch in 'x86', 'x64') {
    $name = "DEFAULT_BEPINEX_URL_$($arch.ToUpper())"
    $match = [regex]::Match($source, "$name`: &str = `"([^`"]+)`"")
    if (-not $match.Success) { throw "$name not found in core_service.rs" }
    Invoke-WebRequest $match.Groups[1].Value -OutFile "$out\bepinex-$arch.zip"
}
