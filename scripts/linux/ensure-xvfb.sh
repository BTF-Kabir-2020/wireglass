#!/usr/bin/env bash
# Optional: virtual display for native Slint preview / slint-viewer on headless VPS.
# See docs/templates/SLINT_HEADLESS_VPS_CHECKLIST.md and docs/guides/slint/UI_PREVIEW.md
set -euo pipefail
DISPLAY_NUM="${DISPLAY_NUM:-99}"
if ! pgrep -f "Xvfb :${DISPLAY_NUM}" >/dev/null 2>&1; then
  Xvfb ":${DISPLAY_NUM}" -screen 0 1280x800x24 -ac >/tmp/xvfb-${DISPLAY_NUM}.log 2>&1 &
  sleep 0.3
fi
export DISPLAY=":${DISPLAY_NUM}"
echo "DISPLAY=$DISPLAY"
