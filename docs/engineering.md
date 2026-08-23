---
description: "Stack, layout, commands, and how to change whipnext."
last_verified: "2026-08-21"
---

# Engineering

## Stack

| Layer | Technology | Version | Why |
| --- | --- | --- | --- |
| Language | Rust | 1.96.1 (edition 2021) | One binary, no extra runtime |
| Package | Cargo | 1.96.1 | Lockfile is committed |
| Overlay window | minifb + Win32 layered host | minifb 0.28 | Bitmap overlay on the cursor |
| Settings window | wry 0.53 + tao 0.34 | WebView2 / WebKitGTK / WebKit | Settings UI without a web framework |
| Images | image 0.25 | png/jpeg/gif | Pack rasters and GIF expansion |
| Video | ffmpeg on PATH | 8.x | Decode idle/action video into overlay frames |
| Format | rustfmt | 1.9.0-stable | `just fmt-check` |
| Static | clippy | 0.1.96 | `just clippy` |
| JS lint | ESLint 9 | `eslint.config.js` | Settings IIFE scripts |
| Tests | cargo test | integration tests under `tests/` | Drive shipped functions, not window FFI |
| Command surface | just 1.56 | `justfile` | Only entry taught to agents |

Lowest supported language edition is 2021. Runtime hosts are supported on Windows and macOS; Linux targets X11. Wayland is not supported because focus, cursor, and input currently use X11 `xdotool`.

## Key crates

| Crate | Role | If removed | Fallback |
| --- | --- | --- | --- |
| serde / serde_json | Settings, IPC, manifests | Pack and UI stop | None |
| image | PNG/GIF/JPEG frames | Overlay skins empty | None |
| wry / tao | Settings WebView | No settings UI | CLI flags only |
| minifb | Unix overlay host | Unix overlay gone | Win32 host unchanged |

New crates: stdlib first; then size and license (MIT-compatible). A crate that pulls a GUI framework is refused.

Forbidden: extra web frameworks, GitHub workflows added without a request, slash-command injectors, loaders for `obj`/`fbx`/`vrm`/`blend`.

Lockfile: `Cargo.lock`, committed. Updates are human-driven.

## Repository

```text
whipnext/
├── src/                 domain + coordinators; OS FFI in *_tests.rs
├── tests/               integration tests of shipped functions
├── assets/pack/         bundled models, sounds, phrases.json
├── assets/ui/           settings atoms/molecules/organisms
├── docs/                product, engineering, quality, architecture
├── .github/workflows/   GitHub Actions CI
├── .repo-safety/        local secret/SAST guardrails
├── justfile             command surface
└── AGENTS.md            agent instructions
```

| Add | Where |
| --- | --- |
| Pack character | `assets/pack/models/<id>/manifest.json` plus frames/video/sounds; UI imports go to `~/.whipnext/catalog/models/<id>/` |
| Phrase line | `assets/pack/phrases.json` (user copy: `~/.whipnext/catalog/phrases.json`) |
| Domain behavior | `src/*.rs` that is not `*_tests.rs` |
| Win32/WebView/SendInput | `src/*_tests.rs` |
| Integration test | `tests/` |
| Command | `justfile` only |

OS FFI filenames end in `_tests.rs` so `cargo llvm-cov` skips them. That is production host code, not unit tests.

## Commands

Only through `just`. Flags live in `justfile`.

| Command | What it does |
| --- | --- |
| `just` | list recipes |
| `just setup` | rustfmt/clippy components + npm ESLint |
| `just fmt` | rustfmt write |
| `just fmt-check` | rustfmt check |
| `just clippy` | clippy `-D warnings` |
| `just lint-js` | ESLint on `assets/ui` |
| `just test` | `cargo test -- --test-threads=1` (tests touch process-wide env vars) |
| `just detect` | `cargo run --release -- --detect` |
| `just demo` | overlay only |
| `just run` | settings + overlay |
| `just dev` | settings + overlay, inject traces in `~/.whipnext/dev.log` |
| `just build` | release binary |
| `just install-windows` | install to `%LOCALAPPDATA%\whipnext` + Start Menu shortcut |
| `just check` | fmt-check + clippy + lint-js + test |

A change is not done until `just check` passes.

## Packaging

| Platform | Script | Result |
| --- | --- | --- |
| Windows | `scripts/install_windows.ps1` (via `just install-windows`) | exe with baked icon in `%LOCALAPPDATA%\whipnext`, Start Menu shortcut with `System.AppUserModel.ID = whipnext.app` |
| Linux/X11 | `bash scripts/package_linux.sh [install]` | dist dir; `install` puts binary in `~/.local/share/whipnext`, `.desktop` + hicolor icon in `~/.local/share` (`StartupWMClass=whipnext`) |
| macOS | `scripts/package_macos.sh` | `dist/Whipnext.app` bundle with icns icon and bundled assets |

Settings UI host is cross-platform (`src/ui_host_tests.rs`, tao/wry); overlay input is Win32 on Windows, CoreGraphics on macOS, and X11 `/proc` plus `xdotool` on Linux. The app id `whipnext.app` must match between the exe runtime AUMID and the Windows shortcut property.

## Code rules that linters do not own

- Injected text is `sanitize_phrase`: empty or slash becomes `next`.
- Overlay size, tick, and scale bounds live in `src/layout.rs`. UI reads them from the snapshot `layout` object.
- Phrase catalog is pack JSON, not a second list in Rust.
- Errors from import/replace are strings the UI can show. Do not swallow an unplayable format into a placeholder that claims 3D or video.
- Comments explain non-obvious constraints only.

## Git

Default branch: `dev`. Feature branches from `dev`. Merge `feature/*` → `dev` → `main`. No direct commits to `main`. After clone: `git config core.hooksPath .githooks`. Details: `docs/branching.md`.
