#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$App = Join-Path $Root "apps\wireglass-android"

if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = "2" }
if (-not $env:CMAKE_BUILD_PARALLEL_LEVEL) { $env:CMAKE_BUILD_PARALLEL_LEVEL = "2" }

foreach ($c in @("x", "llvm-ar", "java")) {
    if (-not (Get-Command $c -ErrorAction SilentlyContinue)) { throw "missing: $c" }
}
if (-not $env:ANDROID_HOME -or -not (Test-Path $env:ANDROID_HOME)) {
    throw "ANDROID_HOME unset or missing"
}

Write-Host "ROOT=$Root"
Write-Host "CARGO_BUILD_JOBS=$($env:CARGO_BUILD_JOBS)"

Push-Location $App
try {
    Write-Host "ABI=arm64-v8a only (not universal)"
    # --arch arm64 only — do not add armeabi-v7a / x86_64 for v1 distributes
    x build --platform android --arch arm64 --format apk @Args
    $apks = Get-ChildItem -Path (Join-Path $App "target\x") -Filter *.apk -Recurse -ErrorAction SilentlyContinue
    $apks | ForEach-Object { Write-Host $_.FullName }
    if ($apks) {
        & (Join-Path $PSScriptRoot "audit-apk-permissions.ps1") $apks[0].FullName
    }
}
finally {
    Pop-Location
}
