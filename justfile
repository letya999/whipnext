set windows-shell := ["cmd.exe", "/d", "/c"]
set dotenv-load := false

default:
    @just --list --unsorted

# Format Rust sources.
fmt:
    cargo fmt --all

# Fail if rustfmt would change files.
fmt-check:
    cargo fmt --all -- --check

# Clippy on shipped library and binary (not integration-test nits).
clippy:
    cargo clippy --lib --bins -- -D warnings

# Settings UI JavaScript lint.
lint-js:
    npm exec -- eslint assets/ui

# Unit and integration tests.
test:
    cargo test -- --test-threads=1

# Installed/running harness catalog as JSON.
detect:
    cargo run --release -- --detect

# Overlay-only demo (no inject).
demo:
    cargo run --release -- --demo

# Settings window + overlay (all platforms).
run: build
    cargo run --release

# Settings + overlay with inject traces in ~/.whipnext/dev.log
dev: build
    cargo run --release -- --dev

# Release binary.
build:
    cargo build --release

# Install to %LOCALAPPDATA%\whipnext (Windows).
install-windows:
    scripts\install_windows.cmd

# Remove all Cargo build artifacts.
clean:
    cargo clean

# JS lint tools.
setup-js:
    npm ci

# rustfmt + clippy components.
setup-rust:
    rustup component add rustfmt clippy

setup: setup-rust setup-js

# Completion gate: format, static analysis, JS lint, tests.
check: fmt-check clippy lint-js test
