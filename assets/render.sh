#!/usr/bin/env sh
# Regenerate the PNG assets from the HTML/SVG sources in assets/src/.
# Requires a headless Chrome/Chromium (or set CHROME=/path/to/chrome).
#
#   ./assets/render.sh
#
# Outputs (PNG, @2x for crisp text):
#   og-cover.png            1200x630  Open Graph / Twitter / Reddit
#   github-social-preview.png 1280x640 GitHub repository social preview
#   banner.png              1280x320  LinkedIn / X header
#   terminal-preview.png    1200x720  README / post inline preview
#   logo.png                 512x512  avatar / org icon
set -eu

CHROME="${CHROME:-google-chrome}"
SCALE="${SCALE:-2}"
here="$(cd "$(dirname "$0")" && pwd)"

render() { # <src> <out> <w> <h>
  "$CHROME" --headless=new --no-sandbox --disable-gpu --hide-scrollbars \
    --force-device-scale-factor="$SCALE" --window-size="$3,$4" \
    --screenshot="$here/$2" "file://$here/$1" >/dev/null 2>&1
  printf 'wrote %-26s %sx%s @%sx\n' "$2" "$3" "$4" "$SCALE"
}

render src/cover.html            og-cover.png              1200 630
render src/cover.html            github-social-preview.png 1280 640
render src/banner.html           banner.png                1280 320
render src/terminal-preview.html terminal-preview.png      1200 720
render logo.svg                  logo.png                   512 512
