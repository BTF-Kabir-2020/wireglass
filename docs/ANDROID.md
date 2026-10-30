# Wireglass on Android

Engineering preview: Rust core + Slint UI, packaged with [xbuild](https://github.com/rust-mobile/xbuild) (**install from git**, not crates.io `0.2.0`).

## Cross-OS rule

The same repo builds on **Linux and Windows** without VPS dependency.  
Requirements matrix: [`REQUIREMENTS.md`](REQUIREMENTS.md)  
Permissions policy: [`PERMISSIONS.md`](PERMISSIONS.md)

Shared Rust/Slint code must not hardcode OS-specific absolute paths.

## Layout

```text
apps/wireglass-android/          # Slint cdylib + android_main
  manifest.yaml                  # package id, min/target SDK, INTERNET only
src/                             # shared network core (`wireglass::run`)
scripts/linux/                   # bash wrappers
scripts/windows/                 # PowerShell wrappers
docs/REQUIREMENTS.md
docs/PERMISSIONS.md
```

## Build

### Linux

```bash
# set ANDROID_HOME, ANDROID_NDK_ROOT, JAVA_HOME (see REQUIREMENTS.md)
./scripts/linux/check-android.sh
./scripts/linux/build-android.sh
```

### Windows (PowerShell)

```powershell
# set ANDROID_HOME, ANDROID_NDK_ROOT, JAVA_HOME (User or System env)
.\scripts\windows\check-android.ps1
.\scripts\windows\build-android.ps1
```

Typical APK:

`apps/wireglass-android/target/x/debug/android/wireglass-android.apk`

## Current SDK stance (build-verified)

| Field | Value |
| --- | --- |
| minSdk | **26** (Android 8 — Slint backend minimum) |
| targetSdk | **34** (Android 14; 35 packaging flaky with current xbuild cache) |
| ABI | **arm64-v8a** |
| Permission | **INTERNET** only (manifest reconciled 2026-10-30) |

## APK size

| Kind | Expectation |
| --- | --- |
| Debug | Often **~200–300 MB** (unstripped `.so` + debug info) — OK for engineers only |
| Release | Must be the downloadable build (`--release`, LTO/strip) — see [`APK_SIZE.md`](APK_SIZE.md) |

Do **not** distribute the debug APK as the normal sideload file.

Device matrix remains evidence-based (Gate H+).

## Honesty

- HTTP/3 in core is handshake-level; UI shows the core note.
- Prefer HTTPS; do not globally enable cleartext “for convenience”.
- Share Intent on-device is Gate H; JSON export is available in-UI after a run.
