# APK size & ABI policy

## ABI (locked)

**Only `arm64-v8a` (Rust target `aarch64-linux-android`).**

- Not a universal / fat APK (no `armeabi-v7a`, no `x86_64`, no `x86` in the package).
- Build flag is always: `x build --platform android --arch arm64 --format apk`
- `aapt` must report: `native-code: 'arm64-v8a'`

That matches modern Android phones. Emulator x86 is out of scope for the distributed APK unless we deliberately add it later.

## Size

| Build | Measured (2026-10-29) | Who gets it |
| --- | --- | --- |
| **Debug** | ~277 MB | Engineers only (debug symbols in one arm64 `.so`) |
| **Release** | **~6.9 MB** | Sideload / future store (`wireglass-android-release-arm64.apk`) |

Debug is large because **one** unstripped `libwireglass_android.so` is huge — **not** because multiple ABIs are bundled.

## Rules

1. Never ship a multi-ABI “universal” APK for v1.
2. Never hand users the debug APK as the normal download.
3. Release build: `--release` (default in `scripts/*/build-android.*`).
4. After each APK: confirm `native-code: arm64-v8a` and record size + sha256.
