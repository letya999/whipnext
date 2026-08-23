---
title: "whipnext - agent instructions"
description: "Click a pack character overlay; it plays hit media and types a phrase (default next) into the focused coding-agent session."
updated: "2026-08-21"
---

# whipnext

Click a pack character overlay; it plays an animation/sound and injects a phrase into the focused coding-agent session. Default phrase is `next`. It is not a slash-command launcher. Stack: Rust 2021, wry/tao settings UI, layered overlay. License: MIT.

## Read first

| Document | When to open |
| --- | --- |
| [docs/product.md](docs/product.md) | Before adding a user-facing flow or arguing priority. |
| [docs/architecture.md](docs/architecture.md) | The change crosses pack, overlay, settings, or inject. |
| [docs/engineering.md](docs/engineering.md) | Stack, layout, where to put a file, git flow. |
| [docs/quality.md](docs/quality.md) | Adding or renaming a check. |
| [docs/branching.md](docs/branching.md) | Branch or merge rules. |

## Commands

Only through `just`. Flags and tool paths live in `justfile`.

| Command | What it does |
| --- | --- |
| `just` | list recipes |
| `just setup` | rustfmt, clippy, ESLint install |
| `just fmt-check` | rustfmt check |
| `just clippy` | clippy `-D warnings` |
| `just lint-js` | ESLint on `assets/ui` |
| `just test` | cargo test |
| `just detect` | release `--detect` JSON |
| `just dev` | settings + overlay, inject traces in `~/.whipnext/dev.log` |
| `just check` | fmt-check + clippy + lint-js + test |

A change is not done until `just check` passes.

## How to work

**Think before code.** If the request is ambiguous, ask; do not guess.
**Simple.** Minimum that solves the task. Nothing for later.
**Local.** Touch only what the task needs. Do not rewrite working overlay/UI paths to clean them up.
**From the goal.** State the check first. Every step has a verification.
**Fail closed.** If a check fails or context is missing, stop and say so.

## Source of truth

User task -> `docs/` (this file included) -> code, tests, and git history as evidence.

Old chat, closed tasks, and commit messages are not requirements unless restated.

## Agent authority

Inside the task, do reversible local work through a verified result without asking again at each step.

Need an explicit user decision for publish, credentials, deleting data with no undo, `git push`, PR, merge, or deploy.

## Communication

Lead with the answer, then only needed detail. Short, plain words. No jargon unless the user used it. Pick the simplest solution that fully closes the task.

## Hard rules

- Injected text: default `next`. A character phrase may be any non-empty non-slash text (`inject::sanitize_phrase`). Never send `/...`.
- Recording injector tests assert that rule (non-empty, not a slash).
- Toggles apply immediately. Closing the settings window exits the overlay.
- Add a model by dropping `assets/pack/models/<id>/manifest.json` plus frames/video/sounds. Do not hardcode a new character id in Rust unless detect/routing needs it.
- Overlay size, tick, and scale bounds: `src/layout.rs` only. Phrase lines: pack `phrases.json`, not a second Rust list.
- OS FFI stays in `src/*_tests.rs` (Win32, WebView, SendInput, binary entry) so llvm-cov ignores those files. Domain stays in the other `src/*.rs` files.
- Human-facing Russian copy lives only in `README.ru.md` and settings UI i18n. Every other doc, comment, commit message, and agent-facing file is English.
- Default branch: `dev`. Branch from `dev` only. `feature/*` → `dev` → `main`. `main` is protected. After clone: `git config core.hooksPath .githooks`.
- Do not commit `/target`, `/target-fix`, `/target-fix2`, `node_modules`, `.env`, keys, or WebView2 user-data folders.
- Do not add a framework, extra crate, or GitHub workflow unless asked.
- Keep `just test` green for detect, inject, whip, settings, pack, focus.

## Where to add

| What | Where |
| --- | --- |
| Pack character | `assets/pack/models/<id>/` |
| Phrase line | `assets/pack/phrases.json` |
| Domain logic | `src/*.rs` except `*_tests.rs` |
| OS host / FFI | `src/*_tests.rs` |
| Settings UI | `assets/ui/` atoms, molecules, organisms, `app.js` |
| Integration test | `tests/` |
| Command | `justfile` |
| Check registry | `docs/quality.md` after the recipe exists |

## Never add

Slash-command injectors · `obj`/`fbx`/`vrm`/`blend` loaders · extra web frameworks · GitHub workflows without a request · a second hardcoded phrase list in Rust · magic overlay sizes copied next to `src/layout.rs`
