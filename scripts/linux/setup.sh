#!/usr/bin/env bash
# Interactive setup wizard for Wireglass Android toolchain (Linux).
# - Detects existing tools and asks before reinstalling.
# - Confirms install locations.
# - Does not overwrite a working location silently.
set -euo pipefail

say() { printf '%s\n' "$*"; }
ask() {
    # ask "prompt" "default"
    local prompt="$1" default="${2:-}"
    local reply
    if [[ -n "$default" ]]; then
        read -r -p "$prompt [$default]: " reply
        reply="${reply:-$default}"
    else
        read -r -p "$prompt: " reply
    fi
    printf '%s' "$reply"
}
confirm() {
    local reply
    read -r -p "$1 [y/N]: " reply
    [[ "$reply" =~ ^[Yy]$ ]]
}

ENV_FILE="${HOME}/.wireglass-android-env"

say "=== Wireglass Android setup (Linux) ==="
say "This wizard will check your toolchain and install missing pieces with confirmation."
say ""

# --- Install root for Android SDK ---
DEFAULT_SDK="${HOME}/Android/Sdk"
SDK_ROOT="$(ask "Android SDK location" "$DEFAULT_SDK")"
if [[ -d "$SDK_ROOT" ]]; then
    say "Existing directory found: $SDK_ROOT"
    if ! confirm "Reuse this location?"; then
        SDK_ROOT="$(ask "New Android SDK location" "$DEFAULT_SDK")"
    fi
else
    if ! confirm "Create and use $SDK_ROOT?"; then
        say "Aborted by user."; exit 1
    fi
fi
mkdir -p "$SDK_ROOT"

# --- Rust ---
if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    say "Rust found: $(rustc --version) at $(command -v rustc)"
    if confirm "Reuse existing Rust toolchain?"; then
        :
    else
        say "Please install Rust manually from https://rustup.rs and re-run."; exit 1
    fi
else
    say "Rust not found."
    if confirm "Install Rust via rustup (default location ~/.rustup)?"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck disable=SC1090
        source "${HOME}/.cargo/env"
    fi
fi
# Android target
if ! rustup target list --installed 2>/dev/null | grep -q aarch64-linux-android; then
    if confirm "Add Rust target aarch64-linux-android?"; then
        rustup target add aarch64-linux-android
    fi
fi

# --- Java ---
if command -v java >/dev/null 2>&1; then
    say "Java found: $(java -version 2>&1 | head -1) at $(command -v java)"
    if ! confirm "Reuse existing Java?"; then
        say "Please install JDK 17 manually and re-run."; exit 1
    fi
else
    say "Java not found."
    if confirm "Install OpenJDK 17 via apt (requires sudo)?"; then
        sudo apt-get update
        sudo apt-get install -y openjdk-17-jdk
    fi
fi
JAVA_HOME="${JAVA_HOME:-$(dirname "$(dirname "$(readlink -f "$(command -v java)")")")}"

# --- Android cmdline-tools / sdkmanager ---
SDKMANAGER=""
if [[ -x "$SDK_ROOT/cmdline-tools/latest/bin/sdkmanager" ]]; then
    SDKMANAGER="$SDK_ROOT/cmdline-tools/latest/bin/sdkmanager"
elif command -v sdkmanager >/dev/null 2>&1; then
    SDKMANAGER="$(command -v sdkmanager)"
fi

if [[ -z "$SDKMANAGER" ]]; then
    say "sdkmanager not found."
    if confirm "Download Android command-line tools into $SDK_ROOT?"; then
        TMPDIR="$(mktemp -d)"
        # Linux x86_64 command line tools
        URL="https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip"
        say "Downloading $URL"
        curl -fL "$URL" -o "$TMPDIR/cmdline.zip"
        mkdir -p "$SDK_ROOT/cmdline-tools"
        unzip -q "$TMPDIR/cmdline.zip" -d "$SDK_ROOT/cmdline-tools"
        mv "$SDK_ROOT/cmdline-tools/cmdline-tools" "$SDK_ROOT/cmdline-tools/latest"
        SDKMANAGER="$SDK_ROOT/cmdline-tools/latest/bin/sdkmanager"
        rm -rf "$TMPDIR"
    fi
fi

if [[ -n "$SDKMANAGER" ]]; then
    export ANDROID_HOME="$SDK_ROOT"
    export ANDROID_SDK_ROOT="$SDK_ROOT"
    yes | "$SDKMANAGER" --licenses >/dev/null 2>&1 || true
    if confirm "Install platform 34 + build-tools 34 + NDK + cmake?"; then
        "$SDKMANAGER" "platforms;android-34" "build-tools;34.0.0" "ndk;26.1.10909125" "cmake;3.22.1"
    fi
    # pick an installed NDK
    NDK_ROOT="$(ls -d "$SDK_ROOT"/ndk/*/ 2>/dev/null | sort | tail -1)"
    NDK_ROOT="${NDK_ROOT%/}"
else
    say "sdkmanager unavailable; install Android SDK manually."
    NDK_ROOT="${ANDROID_NDK_ROOT:-}"
fi

# --- llvm-ar / lld ---
if ! command -v llvm-ar >/dev/null 2>&1 || ! command -v lld >/dev/null 2>&1; then
    say "llvm-ar/lld not found."
    if confirm "Install clang/lld/llvm via apt (requires sudo)?"; then
        sudo apt-get install -y clang lld llvm
    fi
fi

# --- xbuild ---
if command -v x >/dev/null 2>&1; then
    say "xbuild found: $(x --version 2>&1 | head -1) at $(command -v x)"
    if ! confirm "Reuse existing xbuild?"; then
        if confirm "Reinstall xbuild from git?"; then
            cargo install --git https://github.com/rust-mobile/xbuild.git --force
        fi
    fi
else
    say "xbuild not found."
    if confirm "Install xbuild from git (cargo install)?"; then
        cargo install --git https://github.com/rust-mobile/xbuild.git
    fi
fi

# --- Write env file ---
cat > "$ENV_FILE" <<EOF
export ANDROID_HOME="$SDK_ROOT"
export ANDROID_SDK_ROOT="$SDK_ROOT"
export ANDROID_NDK_ROOT="${NDK_ROOT:-}"
export JAVA_HOME="$JAVA_HOME"
export PATH="\$PATH:$SDK_ROOT/platform-tools"
EOF
say ""
say "Wrote $ENV_FILE"
say "Source it with: source $ENV_FILE"
say "Then run: ./scripts/linux/check-android.sh"
