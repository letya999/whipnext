# Contributing

Thanks for helping with whipnext.

## Before you start

- Read [`AGENTS.md`](AGENTS.md) and the relevant guide in [`docs/`](docs/).
- Keep changes small and focused. Do not add slash-command injection, new GUI frameworks, or loaders outside the supported pack formats.
- Do not commit build output, local IDE metadata, credentials, or WebView user data.

## Development

Requirements: Rust and Cargo, `just`, Node.js, and npm. `ffmpeg` is also needed for video pack tests and video imports. Linux development currently requires X11, `xdotool`, GTK 3, and WebKitGTK 4.1; macOS input tests require Accessibility permission.

```text
just setup
just check
```

`just test` runs the Rust suite with one test thread because the tests exercise process-wide environment overrides. `just detect` is an optional live smoke test and inspects the current machine.

GitHub Actions runs checks and release builds on Windows, Linux/X11, and macOS for pushes and pull requests. A green local `just check` is still expected before opening a pull request.

## Changes

- Add pack characters under `assets/pack/models/<id>/`; do not hardcode their IDs in Rust.
- Put phrases in `assets/pack/phrases.json`.
- Keep OS FFI in `src/*_tests.rs` and domain logic in the other Rust files.
- Update the relevant documentation when behavior or commands change.

## Branches and commits

Create feature branches from `dev` and merge `feature/*` → `dev` → `main`. Do not commit directly to `main` or bypass the repository hooks. See [`docs/branching.md`](docs/branching.md).

## Pull requests

Explain the user-visible change, list the checks you ran, and call out platform-specific limitations. Do not include secrets or private machine data in the description, logs, screenshots, or test fixtures.
