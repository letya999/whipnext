#!/usr/bin/env bash
# Build and (optionally) install whipnext on Linux (X11 overlay, GTK settings).
# Usage:
#   scripts/package_linux.sh            # build dist/whipnext-linux/
#   scripts/package_linux.sh install    # also install to ~/.local and register .desktop
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required; install Rust first" >&2
    exit 1
fi
if ! command -v xdotool >/dev/null 2>&1; then
    echo "xdotool is required for Linux focus and cursor detection (X11)" >&2
    exit 1
fi

DIST="dist/whipnext-linux"
rm -rf "$DIST"
mkdir -p "$DIST"

cargo build --release
cp target/release/whipnext "$DIST/"
cp -r assets "$DIST/assets"
cp assets/icon-256.png "$DIST/whipnext.png"

cat > "$DIST/whipnext.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=whipnext
Comment=Click a pack character to inject a phrase into your coding agent
Exec=whipnext
Icon=whipnext
Terminal=false
Categories=Development;
StartupWMClass=whipnext
EOF

if [[ "${1:-}" == "install" ]]; then
    APP_DIR="$HOME/.local/share/whipnext"
    mkdir -p "$APP_DIR" "$HOME/.local/bin" \
        "$HOME/.local/share/applications" \
        "$HOME/.local/share/icons/hicolor/256x256/apps"
    cp -r "$DIST/." "$APP_DIR/"
    ln -sf "$APP_DIR/whipnext" "$HOME/.local/bin/whipnext"
    cp "$DIST/whipnext.png" "$HOME/.local/share/icons/hicolor/256x256/apps/whipnext.png"
    sed "s|^Exec=.*|Exec=$HOME/.local/bin/whipnext|; s|^Icon=.*|Icon=$HOME/.local/share/icons/hicolor/256x256/apps/whipnext.png|" \
        "$DIST/whipnext.desktop" > "$HOME/.local/share/applications/whipnext.desktop"
    update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
    echo "installed: $APP_DIR (launcher: whipnext)"
else
    echo "dist ready: $DIST"
fi
