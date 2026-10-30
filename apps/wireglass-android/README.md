# wireglass-android

Slint + Rust UI for Wireglass on Android (`cdylib` + `android_main`).

## Cross-platform builds

Same sources build on **Linux and Windows**.  
Do not hardcode machine paths in Rust/Slint — use `ANDROID_HOME` / `ANDROID_NDK_ROOT` / scripts.

| Doc | Content |
| --- | --- |
| [`../../docs/REQUIREMENTS.md`](../../docs/REQUIREMENTS.md) | toolchain damreqs for Linux + Windows |
| [`../../docs/PERMISSIONS.md`](../../docs/PERMISSIONS.md) | INTERNET-only policy |
| [`../../docs/ANDROID.md`](../../docs/ANDROID.md) | build overview |
| [`../../scripts/README.md`](../../scripts/README.md) | OS wrappers |

```bash
# Linux
./scripts/linux/check-android.sh
./scripts/linux/build-android.sh
```

```powershell
# Windows
.\scripts\windows\check-android.ps1
.\scripts\windows\build-android.ps1
```

## Layout

```text
wireglass/
├── src/                      # CLI + core library
├── apps/wireglass-android/   # this package (path dep on ../..)
│   ├── ui/app.slint
│   ├── src/lib.rs
│   ├── manifest.yaml         # package / SDK / INTERNET
│   └── Cargo.toml
└── Cargo.toml                # core package (no workspace — xbuild-friendly)
```

## Current build stance

- minSdk **26**, targetSdk **34**, ABI **arm64-v8a**
- Permission allow-list: **INTERNET** only (enforced by audit scripts)
- Preview: VS Code **Stable** Slint only (`Slint.slint` — never with Nightly). Headless VPS/Remote: `"slint.preview.providedByEditor": true` (WASM tab). Native window needs GUI/Xvfb.
