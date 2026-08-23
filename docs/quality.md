---
description: "Required checks, just recipes, thresholds, and when they run."
last_verified: "2026-08-21"
---

# Quality

This registry matches `justfile`. A check is mandatory only after it has a recipe and is part of `just check` (or is marked not applicable).

## Services

| Service | Path | Stack | Aggregate |
| --- | --- | --- | --- |
| `rust` | `src/`, `tests/` | Rust 2021 | `just check` (fmt, clippy, test) |
| `ui` | `assets/ui/` | JavaScript IIFE | `just lint-js` |

## Required categories

| Category | Recipe | What it must catch |
| --- | --- | --- |
| Lint | `just fmt-check`, `just lint-js` | rustfmt drift; JS undefined names |
| Static analysis | `just clippy` | Clippy warnings as errors on the library and binary |
| Types | `just clippy` | rustc type errors (clippy compiles) |
| Architecture | not applicable | Single crate; no layered package graph to enforce |
| Tests | `just test` | Pack import/replace, overlay skins, IPC, inject sanitize; runs serially because tests set process-wide env vars |
| Migrations | not applicable | No database |
| Smoke | `just detect` | Release binary prints harness JSON |
| Artifact regression | not applicable | No installer outside the tree |
| Performance | not applicable | No measured budget in CI |

`just detect` is a smoke recipe. It is not inside `just check` because detect talks to the live OS process table; `just test` already covers `detect_json` with `FakeProbe`.

## Registry

| Service | Category | Recipe | Threshold | Where |
| --- | --- | --- | --- | --- |
| rust | lint | `just fmt-check` | no rustfmt diff | local / before merge |
| rust | static | `just clippy` | zero warnings (`-D warnings`) on `--lib --bins` | local / before merge |
| rust | types | `just clippy` | compile success | local / before merge |
| rust | tests | `just test` | all tests pass with `--test-threads=1` | local / before merge |
| rust | smoke | `just detect` | stdout is a JSON array of harness objects | local, on demand |
| ui | lint | `just lint-js` | ESLint zero errors | local / before merge |
| rust | architecture | not applicable | single crate | — |
| rust | migrations | not applicable | no schema | — |

## Aggregates

```text
just check      # fmt-check + clippy + lint-js + test
just detect     # live harness JSON smoke
```

GitHub Actions runs `.github/workflows/ci.yml` on pushes and pull requests. It checks Windows, Linux/X11, and macOS, installs the locked JavaScript dependencies, runs `just check`, and builds the release artifact. `just detect` stays local because it inspects the live OS process table.

Push and review run `just check`. Do not teach raw `cargo fmt` / `cargo clippy` / `cargo test` in `AGENTS.md`.

## Adding a check

1. Name the defect it catches.
2. Add a `justfile` recipe and attach it to `check` if it must gate merges.
3. Add a row here with a threshold.
4. False positives are defects of the check.

Line coverage (`cargo llvm-cov`, `fail-under-lines = 100` on shipped `src/*.rs` excluding `*_tests.rs`) is an extra local tool. It is not part of `just check`.
