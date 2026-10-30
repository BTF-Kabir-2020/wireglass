#!/usr/bin/env bash
# Build wireglass-android APK for arm64-v8a ONLY (not universal / multi-ABI).
# Default: --release (downloadable). Pass --debug for engineering builds.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/apps/wireglass-android"

# shellcheck disable=SC1090
source "${HOME}/.wireglass-android-env" 2>/dev/null || true
source "${HOME}/.cargo/env" 2>/dev/null || true

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CMAKE_BUILD_PARALLEL_LEVEL="${CMAKE_BUILD_PARALLEL_LEVEL:-2}"

need() { command -v "$1" >/dev/null || { echo "missing: $1" >&2; exit 1; }; }
need x
need llvm-ar
need java
[[ -n "${ANDROID_HOME:-}" && -d "$ANDROID_HOME" ]] || { echo "ANDROID_HOME unset" >&2; exit 1; }

echo "ROOT=$ROOT"
echo "CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS"
df -h / | tail -1

cd "$APP"
# Default to --release for downloadable APKs unless caller already passed --debug/--release
ARGS=("$@")
has_mode=0
for a in "${ARGS[@]+"${ARGS[@]}"}"; do
  case "$a" in
    --release|--debug|-r) has_mode=1 ;;
  esac
done
if [[ "$has_mode" -eq 0 ]]; then
  ARGS=(--release "${ARGS[@]+"${ARGS[@]}"}")
fi

echo "ABI=arm64-v8a only (aarch64-linux-android) — not universal"
echo "x build mode args: ${ARGS[*]}"
# --arch arm64 is mandatory; do not add other arches here.
nice -n 10 x build --platform android --arch arm64 --format apk "${ARGS[@]}"

echo "Search APK under: $APP/target/x"
mapfile -t APKS < <(find "$APP/target/x" -name '*.apk' -type f 2>/dev/null | sort)
printf '%s\n' "${APKS[@]:-}"
for apk in "${APKS[@]+"${APKS[@]}"}"; do
  ls -lh "$apk"
done
# Prefer auditing the newest release apk if present
AUDIT="$ROOT/scripts/linux/audit-apk-permissions.sh"
CANDIDATE=""
for apk in "${APKS[@]+"${APKS[@]}"}"; do
  if [[ "$apk" == *"/release/"* ]]; then CANDIDATE="$apk"; fi
done
[[ -n "$CANDIDATE" ]] || CANDIDATE="${APKS[-1]:-}"
if [[ -x "$AUDIT" && -n "$CANDIDATE" ]]; then
  "$AUDIT" "$CANDIDATE"
fi
echo "See docs/APK_SIZE.md — do not distribute debug APKs as the normal download."
