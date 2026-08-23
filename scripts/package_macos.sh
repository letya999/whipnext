#!/usr/bin/env bash
# Build Whipnext.app on macOS (icon baked via icns, assets bundled).
# Usage: scripts/package_macos.sh
set -euo pipefail

cd "$(dirname "$0")/.."

for command in cargo sips iconutil codesign; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "$command is required on macOS" >&2
        exit 1
    fi
done

APP="dist/Whipnext.app"
CONTENTS="$APP/Contents"
rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

cargo build --release
cp target/release/whipnext "$CONTENTS/MacOS/"
cp -r assets "$CONTENTS/MacOS/assets"

# icns from the 256px PNG (sips/iconutil are macOS-only; script must run on a Mac).
ICONSET="dist/whipnext.iconset"
rm -rf "$ICONSET"
mkdir -p "$ICONSET"
for size in 16 32 64 128 256 512; do
    sips -z "$size" "$size" assets/icon-256.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" assets/icon-256.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$CONTENTS/Resources/whipnext.icns"
rm -rf "$ICONSET"

cat > "$CONTENTS/Info.plist" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>whipnext</string>
    <key>CFBundleIdentifier</key><string>app.whipnext.whipnext</string>
    <key>CFBundleName</key><string>whipnext</string>
    <key>CFBundleDisplayName</key><string>whipnext</string>
    <key>CFBundleIconFile</key><string>whipnext</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundleVersion</key><string>0.1.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSMinimumSystemVersion</key><string>10.13</string>
</dict>
</plist>
EOF

codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "bundle ready: $APP (drag to /Applications)"
