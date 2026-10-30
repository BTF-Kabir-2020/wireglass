# Interactive setup wizard for Wireglass Android toolchain (Windows).
# Detects existing tools, confirms locations, avoids silent reinstalls.
$ErrorActionPreference = "Stop"

function Ask($prompt, $default = "") {
    if ($default -ne "") {
        $r = Read-Host "$prompt [$default]"
        if ([string]::IsNullOrWhiteSpace($r)) { return $default }
        return $r
    }
    return Read-Host $prompt
}
function Confirm($prompt) {
    $r = Read-Host "$prompt [y/N]"
    return ($r -match '^[Yy]$')
}

$EnvFile = Join-Path $env:USERPROFILE ".wireglass-android-env.ps1"
Write-Host "=== Wireglass Android setup (Windows) ===" -ForegroundColor Cyan

$defaultSdk = Join-Path $env:USERPROFILE "Android\Sdk"
$sdkRoot = Ask "Android SDK location" $defaultSdk
if (Test-Path $sdkRoot) {
    Write-Host "Existing directory found: $sdkRoot"
    if (-not (Confirm "Reuse this location?")) {
        $sdkRoot = Ask "New Android SDK location" $defaultSdk
    }
} else {
    if (-not (Confirm "Create and use $sdkRoot?")) { throw "Aborted by user." }
}
New-Item -ItemType Directory -Force -Path $sdkRoot | Out-Null

# Rust
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    Write-Host "Rust found: $(rustc --version)"
    if (-not (Confirm "Reuse existing Rust toolchain?")) {
        throw "Install Rust manually from https://rustup.rs and re-run."
    }
} else {
    Write-Host "Rust not found."
    if (Confirm "Install Rust via rustup (default location)?") {
        $rustupInit = Join-Path $env:TEMP "rustup-init.exe"
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
        & $rustupInit -y
        $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    }
}
# target
$installed = (rustup target list --installed) 2>$null
if ($installed -notmatch "aarch64-linux-android") {
    if (Confirm "Add Rust target aarch64-linux-android?") { rustup target add aarch64-linux-android }
}

# Java
if (Get-Command java -ErrorAction SilentlyContinue) {
    Write-Host "Java found: $(java -version 2>&1 | Select-Object -First 1)"
    if (-not (Confirm "Reuse existing Java?")) { throw "Install JDK 17 manually and re-run." }
} else {
    Write-Host "Java not found."
    if (Confirm "Install OpenJDK 17 via winget (if available)?") {
        winget install --id Microsoft.OpenJDK.17 -e --accept-source-agreements --accept-package-agreements
    }
}
$javaHome = $env:JAVA_HOME
if (-not $javaHome) {
    $javaCmd = (Get-Command java -ErrorAction SilentlyContinue).Source
    if ($javaCmd) { $javaHome = (Resolve-Path (Join-Path (Split-Path $javaCmd) "..")).Path }
}

# Android cmdline-tools
$sdkManager = Join-Path $sdkRoot "cmdline-tools\latest\bin\sdkmanager.bat"
if (-not (Test-Path $sdkManager)) {
    if (Get-Command sdkmanager -ErrorAction SilentlyContinue) { $sdkManager = (Get-Command sdkmanager).Source }
}
if (-not (Test-Path $sdkManager)) {
    Write-Host "sdkmanager not found."
    if (Confirm "Download Android command-line tools into $sdkRoot?") {
        $zip = Join-Path $env:TEMP "cmdline-tools.zip"
        Invoke-WebRequest -Uri "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip" -OutFile $zip
        New-Item -ItemType Directory -Force -Path (Join-Path $sdkRoot "cmdline-tools") | Out-Null
        Expand-Archive -Path $zip -DestinationPath (Join-Path $sdkRoot "cmdline-tools") -Force
        $extracted = Join-Path $sdkRoot "cmdline-tools\cmdline-tools"
        $latest = Join-Path $sdkRoot "cmdline-tools\latest"
        if (Test-Path $latest) { Remove-Item -Recurse -Force $latest }
        Rename-Item -Path $extracted -NewName "latest"
        $sdkManager = Join-Path $sdkRoot "cmdline-tools\latest\bin\sdkmanager.bat"
    }
}

$ndkRoot = $env:ANDROID_NDK_ROOT
if (Test-Path $sdkManager) {
    $env:ANDROID_HOME = $sdkRoot
    $env:ANDROID_SDK_ROOT = $sdkRoot
    & $sdkManager --licenses | Out-Null
    if (Confirm "Install platform 34 + build-tools 34 + NDK + cmake?") {
        & $sdkManager "platforms;android-34" "build-tools;34.0.0" "ndk;26.1.10909125" "cmake;3.22.1"
    }
    $ndkRoot = (Get-ChildItem -Path (Join-Path $sdkRoot "ndk") -Directory | Sort-Object Name | Select-Object -Last 1).FullName
} else {
    Write-Host "sdkmanager unavailable; install Android SDK manually."
}

# llvm-ar / lld
if (-not (Get-Command llvm-ar -ErrorAction SilentlyContinue)) {
    Write-Host "llvm-ar not found. Install LLVM (e.g. winget install LLVM.LLVM) and add to PATH."
}

# xbuild
if (Get-Command x -ErrorAction SilentlyContinue) {
    Write-Host "xbuild found."
    if (-not (Confirm "Reuse existing xbuild?")) {
        if (Confirm "Reinstall xbuild from git?") { cargo install --git https://github.com/rust-mobile/xbuild.git --force }
    }
} else {
    Write-Host "xbuild not found."
    if (Confirm "Install xbuild from git (cargo install)?") {
        cargo install --git https://github.com/rust-mobile/xbuild.git
    }
}

# Write env file
$lines = @(
    "`$env:ANDROID_HOME = `"$sdkRoot`"",
    "`$env:ANDROID_SDK_ROOT = `"$sdkRoot`"",
    "`$env:ANDROID_NDK_ROOT = `"$ndkRoot`"",
    "`$env:JAVA_HOME = `"$javaHome`"",
    "`$env:Path = `"$sdkRoot\platform-tools;`$env:Path`""
)
$lines | Set-Content -Path $EnvFile
Write-Host "Wrote $EnvFile"
Write-Host "Load it with: . $EnvFile"
Write-Host "Then run: .\scripts\windows\check-android.ps1"