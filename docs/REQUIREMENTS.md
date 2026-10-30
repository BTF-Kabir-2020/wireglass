# Build requirements — Linux & Windows

Wireglass Android must be buildable **Alone** on either OS.  
No hardcoded `C:\...` or `/home/...` in shared Rust/Slint sources — only env vars + thin OS scripts.

## Shared (both OS)

| Requirement | Notes |
| --- | --- |
| Git | clone / PR |
| Rust stable via **rustup** | `rustc`/`cargo` |
| Target `aarch64-linux-android` | `rustup target add aarch64-linux-android` |
| JDK **17** (recommended) | JDK 21 has bitten Slint/Android tooling historically |
| Android SDK | cmdline-tools or Android Studio SDK |
| Android NDK (side-by-side) | set `ANDROID_NDK_ROOT` to exact version dir |
| Platform + Build-Tools | enough for targetSdk (currently **34** in `manifest.yaml`) |
| `adb` (platform-tools) | install / devices |
| **xbuild from git** | `cargo install --git https://github.com/rust-mobile/xbuild.git` — **not** crates.io `0.2.0` |
| `llvm-ar` + `lld` on PATH | xbuild links with `-fuse-ld=lld` |
| Env | `ANDROID_HOME`, `ANDROID_NDK_ROOT`, `JAVA_HOME`, Cargo bin on `PATH` |
| Optional throttle | `CARGO_BUILD_JOBS=2` on small machines |

### Permissions / product rules

See [`PERMISSIONS.md`](PERMISSIONS.md) — v1 allow-list is **only** `INTERNET`.

### xbuild NDK cache quirk (both OS)

If you see `ndk doesn't support sdk version 34`, copy the NDK sysroot API folder into xbuild’s cache (community workaround):

```text
# Linux example
$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/sysroot/usr/lib/aarch64-linux-android/34
  →  ~/.cache/x/Android.ndk/usr/lib/aarch64-linux-android/34

# Windows example (User profile cache)
%ANDROID_NDK_ROOT%\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\aarch64-linux-android\34
  →  %LOCALAPPDATA%\x\Android.ndk\usr\lib\aarch64-linux-android\34
```

(Exact cache path may vary; if unsure, search for `Android.ndk` under the user cache after one `x build`.)

---

## Linux extras

| Package / tool | Why |
| --- | --- |
| `build-essential` | C/C++ host bits |
| `clang` | Skia / native builds |
| `pkg-config` | system libs discovery |
| `libfontconfig1-dev` (+ often `libxkbcommon-dev`, xcb) | host `cargo check` of Slint UI |
| `lld`, `llvm` (provides `llvm-ar`) | Android link via xbuild |

Scripts:

```bash
./scripts/linux/check-android.sh
./scripts/linux/build-android.sh
./scripts/linux/audit-apk-permissions.sh path/to.apk
```

Suggested env file (local, **not** committed): `~/.wireglass-android-env`

---

## Windows extras

| Piece | Why |
| --- | --- |
| **MSVC Build Tools** / VS C++ workload | Rust host + native deps |
| Windows SDK (with VS) | typical MSVC companion |
| Git for Windows | shell-friendly paths |
| Put NDK `bin` + `llvm-ar`/`lld` on `PATH` | same link requirement as Linux |
| WSL | **optional**, not required |

Scripts:

```powershell
.\scripts\windows\check-android.ps1
.\scripts\windows\build-android.ps1
.\scripts\windows\audit-apk-permissions.ps1 path\to.apk
```

Slint **Live Preview** (confirmed on this VPS 2026-10-29):

- Prefer **Stable only** (`Slint.slint`). Do **not** enable Stable and Nightly together.
- **Headless Linux VPS / Remote SSH:**
  1. `"slint.preview.providedByEditor": true` in workspace `.vscode/settings.json`
  2. Same key in `~/.vscode-server/data/Machine/settings.json` (Remote)
  3. Reload Window
  4. Optional Xvfb: `scripts/linux/ensure-xvfb.sh` (`DISPLAY=:99`) for native/`slint-viewer`
  5. Reuse checklist: brief-tree `docs/templates/SLINT_HEADLESS_VPS_CHECKLIST.md`
- **GUI desktop:** native preview or `slint-viewer --auto-reload` with Stable alone.


---

## Sanity checklist (either OS)

```text
rustc --version
rustup target list --installed   # includes aarch64-linux-android
java -version                    # 17.x preferred
echo ANDROID_HOME / %ANDROID_HOME%
echo ANDROID_NDK_ROOT
llvm-ar --version
lld --version   # or ld.lld
x doctor
cargo test --lib
cargo check --manifest-path apps/wireglass-android/Cargo.toml
```

Then from `apps/wireglass-android`:

```text
x build --platform android --arch arm64 --format apk
```

APK path (typical): `apps/wireglass-android/target/x/debug/android/wireglass-android.apk`
