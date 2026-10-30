# Scripts

Thin OS wrappers around the same Cargo / xbuild flow.  
Logic stays in the repo; paths come from environment variables.

| Linux | Windows | Purpose |
| --- | --- | --- |
| `linux/setup.sh` | `windows/setup.ps1` | interactive toolchain setup (confirms locations) |
| `linux/check-android.sh` | `windows/check-android.ps1` | toolchain + `cargo test` / host check |
| `linux/build-android.sh` | `windows/build-android.ps1` | `x build` APK arm64 |
| `linux/audit-apk-permissions.sh` | `windows/audit-apk-permissions.ps1` | allow-list = INTERNET only |
| `linux/ensure-xvfb.sh` | — | optional virtual display for native Slint/`slint-viewer` on headless VPS |

Slint Preview on VPS: prefer editor WASM (`slint.preview.providedByEditor`) — see brief docs `docs/templates/SLINT_HEADLESS_VPS_CHECKLIST.md`.

See [`docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md).
