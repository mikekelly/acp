# Release Process

This document covers the complete release process for GAP across all distribution channels.

## Distribution Channels

| Platform | Channel | Location |
|----------|---------|----------|
| macOS | Native App (DMG) | GitHub Releases |
| Linux | Binary tarball | GitHub Releases |
| Docker | Container image | Docker Hub (`mikekelly321/gap`) |

## Prerequisites

### Apple Developer Account (macOS releases)
- Apple Developer Program membership ($99/year)
- Developer ID Application certificate installed in Keychain
- App-specific password for notarization (generate at https://appleid.apple.com)

### Docker Hub Account (Docker releases)
- Account with push access to `mikekelly321/gap`

## Version Tagging

All releases use semantic versioning:
- Format: `vX.Y.Z` (e.g., `v0.5.0`)
- Update `version` in workspace `Cargo.toml`

## macOS Release

### 1. Bump Version

Update the version in the workspace `Cargo.toml`:
```bash
# Edit Cargo.toml:
# [workspace.package]
# version = "X.Y.Z"

# Verify the change propagates
cargo build --release
./target/release/gap --version
```

### 2. Build the macOS App

```bash
cd macos-app
./build-dmg.sh
```

This script:
- Builds `gap-server` if not already built
- Creates the app bundle structure with embedded helper
- Generates entitlements files
- Outputs an unsigned `build/GAP.app`

### 3. Sign the App

```bash
./sign-and-package.sh
```

This script:
- Embeds provisioning profiles (if available from `setup-app-provisioning.sh`)
- Signs the helper app (inside-out signing is critical)
- Signs the main app
- Creates `build/GAP.dmg` with Applications symlink

**Note:** macOS will prompt for your login password to access the Developer ID certificate in Keychain. This step cannot be automated.

### 4. Notarize the DMG

**One-time setup** (store credentials in keychain):
```bash
xcrun notarytool store-credentials "notarytool-profile" \
    --apple-id "YOUR_APPLE_ID" \
    --team-id "3R44BTH39W" \
    --password "APP_SPECIFIC_PASSWORD"
```

**Notarize the DMG:**
```bash
./scripts/macos-notarize.sh build/GAP.dmg --keychain-profile "notarytool-profile"
```

Or set via environment variable:
```bash
export NOTARIZE_KEYCHAIN_PROFILE="notarytool-profile"
./scripts/macos-notarize.sh build/GAP.dmg
```

**Notes:**
- Credentials are stored securely in macOS Keychain for future use
- Notarization typically completes in 2-5 minutes
- The script automatically staples the notarization ticket to the DMG
- Notarized apps won't trigger "unidentified developer" warnings

### 5. Create GitHub Release

```bash
# Commit version bump
git add Cargo.toml Cargo.lock
git commit -m "Bump version to X.Y.Z"

# Tag and push
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main
git push origin vX.Y.Z

# Create release with DMG
gh release create vX.Y.Z \
    --title "vX.Y.Z" \
    --notes "Release notes here" \
    macos-app/build/GAP.dmg
```

## Docker/Linux Release

### 1. Build Docker Image

```bash
# From repository root
docker build -t mikekelly321/gap:vX.Y.Z .
docker tag mikekelly321/gap:vX.Y.Z mikekelly321/gap:latest
```

### 2. Push to Docker Hub

```bash
docker push mikekelly321/gap:vX.Y.Z
docker push mikekelly321/gap:latest
```

### 3. Build Linux Binaries (optional)

For GitHub Releases, you can also provide standalone Linux binaries:

```bash
# Cross-compile for Linux (requires cross or Docker)
# Or build on a Linux machine:
cargo build --release
tar -czvf gap-linux-amd64.tar.gz -C target/release gap gap-server
```

Attach the tarball to the GitHub release.

## Checklist

Before releasing:
- [ ] All tests pass (`cargo test`)
- [ ] Version bumped in `Cargo.toml`
- [ ] Changelog/release notes prepared

macOS release:
- [ ] App built (`macos-app/build-dmg.sh`)
- [ ] App signed (`macos-app/sign-and-package.sh`)
- [ ] DMG notarized and stapled
- [ ] DMG uploaded to GitHub release

Docker release:
- [ ] Image built and tagged
- [ ] Pushed to Docker Hub

Post-release:
- [ ] GitHub release created with appropriate assets
- [ ] Release notes published

## Troubleshooting

### "Developer ID certificate not found"
Install your Developer ID certificate from Apple Developer Portal. Must be "Developer ID Application" (not "Apple Development").

### Notarization fails
The notarization script displays Apple's error response on failure.

Common issues:
- Binary not signed with hardened runtime
- Missing timestamp
- Unsigned nested components
- Incorrect bundle_id specified

### Stapling fails with Error 73
This is expected for bare executables. Stapling only works for `.app`, `.pkg`, and `.dmg` files.

### Gatekeeper blocks app
The app needs to be notarized. If testing before notarization, right-click > Open to bypass Gatekeeper.
