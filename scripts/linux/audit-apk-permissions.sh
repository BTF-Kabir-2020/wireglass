#!/usr/bin/env bash
# Fail if APK declares permissions outside the allow-list.
set -euo pipefail
APK="${1:-}"
if [[ -z "$APK" || ! -f "$APK" ]]; then
  echo "usage: $0 <app.apk>" >&2
  exit 2
fi

AAPT="${AAPT:-}"
if [[ -z "$AAPT" ]]; then
  if [[ -n "${ANDROID_HOME:-}" ]]; then
    AAPT="$(ls -1 "$ANDROID_HOME"/build-tools/*/aapt 2>/dev/null | tail -1 || true)"
  fi
fi
[[ -x "${AAPT:-}" ]] || { echo "aapt not found; set ANDROID_HOME or AAPT" >&2; exit 1; }

# Allow-list (one permission name per line)
ALLOW='android.permission.INTERNET'

mapfile -t FOUND < <("$AAPT" dump permissions "$APK" 2>/dev/null | sed -n 's/^uses-permission: name='\''\([^'\'']*\)'\''.*/\1/p')
# Fallback parse from xmltree if dump permissions empty
if [[ ${#FOUND[@]} -eq 0 ]]; then
  mapfile -t FOUND < <("$AAPT" dump xmltree "$APK" AndroidManifest.xml | sed -n 's/.*android:name.*=\"\(android\.permission\.[^\"]*\)\".*/\1/p' | sort -u)
fi

echo "APK=$APK"
echo "Found permissions:"
if [[ ${#FOUND[@]} -eq 0 ]]; then
  echo "  (none)"
  echo "FAIL: expected at least INTERNET"
  exit 1
fi

bad=0
has_internet=0
for p in "${FOUND[@]}"; do
  echo "  - $p"
  if [[ "$p" == "android.permission.INTERNET" ]]; then
    has_internet=1
  fi
  if ! grep -qxF "$p" <<<"$ALLOW"; then
    echo "UNEXPECTED: $p"
    bad=1
  fi
done

[[ "$has_internet" -eq 1 ]] || { echo "FAIL: missing INTERNET"; bad=1; }
[[ "$bad" -eq 0 ]] || exit 1
echo "OK: permissions allow-list satisfied"
