#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")

function Need-Cmd([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "missing command: $Name"
    }
    Write-Host "OK  $Name = $((Get-Command $Name).Source)"
}

Need-Cmd rustc
Need-Cmd cargo
Need-Cmd rustup
Need-Cmd java
Need-Cmd adb
Need-Cmd x
Need-Cmd llvm-ar

if (-not $env:ANDROID_HOME) { throw "ANDROID_HOME unset" }
if (-not (Test-Path $env:ANDROID_HOME)) { throw "ANDROID_HOME missing: $env:ANDROID_HOME" }
if (-not $env:ANDROID_NDK_ROOT) { throw "ANDROID_NDK_ROOT unset" }
if (-not (Test-Path $env:ANDROID_NDK_ROOT)) { throw "ANDROID_NDK_ROOT missing: $env:ANDROID_NDK_ROOT" }

$targets = rustup target list --installed
if ($targets -notmatch "aarch64-linux-android") {
    throw "missing rustup target aarch64-linux-android"
}
Write-Host "OK  aarch64-linux-android"

Write-Host "--- cargo test (core) ---"
Push-Location $Root
try {
    cargo test --lib
    Write-Host "--- cargo check (android app, host) ---"
    cargo check --manifest-path "apps/wireglass-android/Cargo.toml"
}
finally {
    Pop-Location
}

Write-Host "--- x doctor (tail) ---"
x doctor
Write-Host "check-android: OK"
