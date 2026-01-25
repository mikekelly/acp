#!/bin/bash
# Regenerate all app icon sizes from gap_logo.png
set -e

cd "$(dirname "$0")/.."

SOURCE_LOGO="gap_logo.png"
ICONSET_DIR="macos-app/GAP-App/Sources/Assets.xcassets/AppIcon.appiconset"

if [ ! -f "$SOURCE_LOGO" ]; then
    echo "Error: Source logo not found at $SOURCE_LOGO"
    exit 1
fi

echo "Regenerating app icons from $SOURCE_LOGO..."

# Generate all required sizes
sips -z 16 16 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_16x16.png"
sips -z 32 32 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_16x16@2x.png"
sips -z 32 32 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_32x32.png"
sips -z 64 64 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_32x32@2x.png"
sips -z 128 128 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_128x128.png"
sips -z 256 256 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_128x128@2x.png"
sips -z 256 256 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_256x256.png"
sips -z 512 512 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_256x256@2x.png"
sips -z 512 512 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_512x512.png"
sips -z 1024 1024 "$SOURCE_LOGO" --out "$ICONSET_DIR/icon_512x512@2x.png"

echo "Icons regenerated successfully!"
echo "Contents.json already exists and should not need updating."
echo "Run ./build-dmg.sh to rebuild the app with new icons."
