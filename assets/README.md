# assets

Images used in the README and in launch posts (Open Graph, GitHub social
preview, banners, terminal previews).

| File | Size | Use |
|---|---|---|
| `og-cover.png` | 1200×630 | Open Graph / Twitter Card / Reddit / HN |
| `github-social-preview.png` | 1280×640 | GitHub repo *Settings → Social preview* |
| `banner.png` | 1280×320 | LinkedIn / X header image |
| `terminal-preview.png` | 1200×720 | Inline preview in README or a post |
| `logo.png`, `logo.svg` | 512×512 | Avatar / org icon (SVG is the source) |

## Regenerate

The PNGs are rendered from the editable sources in [`src/`](src) (plain HTML +
one SVG) using headless Chrome, so no design tool is required:

```sh
./assets/render.sh
```

Override the binary or scale if needed:

```sh
CHROME=/usr/bin/chromium SCALE=1 ./assets/render.sh
```

Edit `src/*.html` (or `logo.svg`) and re-run the script to update the PNGs. Do
not edit the PNGs by hand.
