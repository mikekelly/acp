#!/bin/bash
# Sign the app bundle and create DMG
# Run this after build-dmg.sh and setup-provisioning.sh (to get provisioning profile)
set -e

cd "$(dirname "$0")"

APP_NAME="GAP"
HELPER_NAME="gap-server"
BUNDLE_ID="com.mikekelly.gap"
HELPER_BUNDLE_ID="com.mikekelly.gap-server"

echo "=== Signing and Packaging GAP.app ==="

# Check app bundle exists
if [ ! -d "build/${APP_NAME}.app" ]; then
    echo "ERROR: build/${APP_NAME}.app not found. Run ./build-dmg.sh first."
    exit 1
fi

# Check for provisioning profiles (optional but recommended for keychain access)
if [ -f "build/main.mobileprovision" ] && [ -f "build/helper.mobileprovision" ]; then
    echo "Embedding provisioning profiles..."
    cp "build/main.mobileprovision" "build/${APP_NAME}.app/Contents/embedded.provisionprofile"
    cp "build/helper.mobileprovision" "build/${APP_NAME}.app/Contents/Library/LoginItems/${HELPER_NAME}.app/Contents/embedded.provisionprofile"
elif [ -f "build/embedded.mobileprovision" ]; then
    echo "Embedding single provisioning profile (legacy)..."
    cp "build/embedded.mobileprovision" "build/${APP_NAME}.app/Contents/embedded.provisionprofile"
    cp "build/embedded.mobileprovision" "build/${APP_NAME}.app/Contents/Library/LoginItems/${HELPER_NAME}.app/Contents/embedded.provisionprofile"
else
    echo "WARNING: No provisioning profiles found. Keychain access may prompt for password."
    echo "Run ./setup-app-provisioning.sh to create them."
fi

# Sign INSIDE-OUT (critical!)
echo ""
echo "=== Step 1: Signing helper app (inside) ==="
codesign --sign "Developer ID Application: Mike Kelly (3R44BTH39W)" \
    --force \
    --options runtime \
    --timestamp \
    --entitlements "build/helper.entitlements" \
    "build/${APP_NAME}.app/Contents/Library/LoginItems/${HELPER_NAME}.app"

echo "=== Step 2: Signing main app (outside) ==="
codesign --sign "Developer ID Application: Mike Kelly (3R44BTH39W)" \
    --force \
    --options runtime \
    --timestamp \
    --entitlements "build/main.entitlements" \
    "build/${APP_NAME}.app"

echo ""
echo "=== Step 3: Verifying signatures ==="
codesign --verify --deep --verbose=2 "build/${APP_NAME}.app"

echo ""
echo "=== Step 4: Creating DMG with Applications symlink ==="

# Create staging directory for DMG contents
STAGING_DIR="build/dmg-staging"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

# Copy signed app to staging
cp -R "build/${APP_NAME}.app" "$STAGING_DIR/"

# Create Applications symlink for drag-and-drop installation
ln -s /Applications "$STAGING_DIR/Applications"

echo "Staging directory prepared with Applications symlink"

# Check if create-dmg is installed
if command -v create-dmg &> /dev/null; then
    # Use sindresorhus/create-dmg (simple)
    cd build
    create-dmg dmg-staging || true  # May fail if DMG exists
    cd ..

    DMG_FILE=$(ls -t build/*.dmg 2>/dev/null | head -1)
    if [ -n "$DMG_FILE" ]; then
        # Clean up staging directory
        rm -rf "$STAGING_DIR"

        echo ""
        echo "=== Step 5: Signing DMG ==="
        codesign -s "Developer ID Application: Mike Kelly (3R44BTH39W)" --timestamp "$DMG_FILE"
        echo "DMG signed: $DMG_FILE"

        echo ""
        echo "=== Done! ==="
        echo "DMG created and signed: $DMG_FILE"
        echo ""
        echo "To install:"
        echo "  1. Open the DMG"
        echo "  2. Drag GAP to Applications folder"
        echo "  3. First launch: right-click > Open (to bypass Gatekeeper)"
        echo "  4. Approve 'GAP Server' in System Settings > Login Items"
    fi
else
    echo "create-dmg not found. Install with: brew install create-dmg"
    echo ""
    echo "Creating DMG manually with hdiutil..."

    # Fallback to hdiutil
    rm -f "build/${APP_NAME}.dmg"
    hdiutil create -srcfolder "$STAGING_DIR" \
        -volname "${APP_NAME}" \
        -fs HFS+ \
        -format UDZO \
        "build/${APP_NAME}.dmg"

    # Clean up staging directory
    rm -rf "$STAGING_DIR"

    echo ""
    echo "=== Step 5: Signing DMG ==="
    codesign -s "Developer ID Application: Mike Kelly (3R44BTH39W)" --timestamp "build/${APP_NAME}.dmg"
    echo "DMG signed: build/${APP_NAME}.dmg"

    echo ""
    echo "=== Done! ==="
    echo "DMG created and signed: build/${APP_NAME}.dmg"
    echo ""
    echo "The DMG includes an Applications folder symlink for easy drag-and-drop installation."
fi
