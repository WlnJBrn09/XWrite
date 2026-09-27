# XWrite — Native Windows host

Thin Win32 desktop shell that **does not use Electron**. It spawns the existing Rust backend (`xwrite.exe`), waits for `GET /api/health`, and loads the product UI in a **WebView2** window.

## Prerequisites

- Rust toolchain (MSVC)
- Microsoft Edge **WebView2** Runtime (preinstalled on most Windows 10/11 machines)
- Backend built once: `cargo build --release` from `xwrite/`

## Build

From `xwrite/`:

```bat
npm run native:build
```

Or:

```bat
cargo build --release
cargo build --release --manifest-path native-host/Cargo.toml
```

Binary: `native-host/target/release/xwrite-native.exe`

## Run

```bat
npm run native
```

Headless (starts backend + health wait, no window; useful for CI/HTTP checks):

```bat
set XWRITE_NATIVE_HEADLESS_SECS=20
native-host\target\release\xwrite-native.exe --headless
```

Default port: **8787** (`PORT` env overrides).

## Layout

- **Dev**: host walks up from its exe path until it finds `static/index.html` and uses `target/release|debug/xwrite.exe`.
- **Packaged**: place `xwrite-native.exe` next to `backend/xwrite.exe` and `static/`.

## Packaging

From the product root on Windows:

```bat
npm run dist
```

Produces `dist/XWrite_v*_win.zip` with `XWrite.exe`, `backend/`, and `static/`.

macOS and Linux natives are built in CI (`.github/workflows/native.yml`).
