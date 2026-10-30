#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
source "${HOME}/.wireglass-android-env" 2>/dev/null || true
source "${HOME}/.cargo/env" 2>/dev/null || true

fail=0
check() {
  if "$@"; then echo "OK  $*"; else echo "FAIL $*"; fail=1; fi
}

check command -v rustc
check command -v x
check command -v llvm-ar
check command -v adb
check test -d "${ANDROID_HOME:-/no}"
check test -d "${ANDROID_NDK_ROOT:-/no}"
check rustup target list --installed \| grep -q aarch64-linux-android

echo "--- cargo test (core) ---"
(cd "$ROOT" && cargo test --lib)

echo "--- cargo check (android app, host) ---"
(cd "$ROOT" && cargo check --manifest-path apps/wireglass-android/Cargo.toml)

exit "$fail"
