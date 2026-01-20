# macOS Code Signing and Notarization Setup

This document covers the **one-time setup** for macOS code signing and notarization. You only need to do this once per machine.

For the ongoing release process, see [RELEASING.md](./RELEASING.md).

## Overview

To distribute GAP as a macOS app, you need:
1. Apple Developer Program membership
2. Developer ID Application certificate
3. App-specific password for notarization
4. Credentials stored in `.env.local` and macOS Keychain

## Prerequisites

### Apple Developer Account
- Apple Developer Program membership ($99/year): https://developer.apple.com/programs/

**Why?** Required to obtain code signing certificates and submit apps for notarization.

## Setup Steps

### Step 1: Install Developer ID Certificate

1. **Go to Apple Developer Portal:**
   - Visit https://developer.apple.com/account/resources/certificates/list

2. **Create a Certificate Signing Request (CSR):**
   - Open **Keychain Access** on your Mac
   - Menu: **Keychain Access -> Certificate Assistant -> Request a Certificate from a Certificate Authority**
   - Enter your email address
   - Select **"Saved to disk"**
   - Save the `.certSigningRequest` file

3. **Create Developer ID Certificate:**
   - Back in the Developer Portal, click **"+"**
   - Select **"Developer ID Application"** (not "Apple Development")
   - Upload your CSR file
   - Download the resulting `.cer` file

4. **Install the certificate:**
   - Double-click the `.cer` file to install
   - **Important:** Install to the **System** keychain, not iCloud Keychain

5. **Verify installation:**
   ```bash
   security find-identity -v -p codesigning | grep "Developer ID"
   ```

   You should see output like:
   ```
   1) ABC123XYZ "Developer ID Application: Your Name (TEAMID123)"
   ```

   Note your **Team ID** (the value in parentheses) - you'll need it in the next step.

### Step 2: Generate App-Specific Password

1. **Go to Apple ID portal:**
   - Visit https://appleid.apple.com
   - Sign in with your Apple ID

2. **Generate app-specific password:**
   - Navigate to **Sign-In and Security -> App-Specific Passwords**
   - Click **"Generate an app-specific password"**
   - Enter a label like "GAP Notarization"
   - Copy the generated password (format: `xxxx-xxxx-xxxx-xxxx`)
   - **Important:** Save this password - you can't view it again!

### Step 3: Create .env.local File

1. **Copy the example file:**
   ```bash
   cd /path/to/gap
   cp .env.local.example .env.local
   ```

2. **Edit `.env.local` with your credentials:**
   ```bash
   # Open in your editor
   nano .env.local
   # or
   code .env.local
   ```

   Fill in:
   - `APPLE_ID`: Your Apple ID email (e.g., `developer@example.com`)
   - `APPLE_TEAM_ID`: From Step 1 (e.g., `3R44BTH39W`)
   - `NOTARYTOOL_PASSWORD`: App-specific password from Step 2

3. **Verify the file is NOT tracked by git:**
   ```bash
   git status
   # .env.local should NOT appear in the output
   ```

   **Critical:** Never commit `.env.local` to version control. It's already in `.gitignore`.

### Step 4: Store Credentials in macOS Keychain

This creates a keychain profile that `notarytool` can use without exposing your password.

```bash
# Source your credentials from .env.local
source .env.local

# Store in macOS Keychain (one-time setup)
xcrun notarytool store-credentials "notarytool-profile" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$NOTARYTOOL_PASSWORD"
```

When prompted, enter a password to protect the keychain profile. This is stored securely in your macOS Keychain.

**Verify the profile was created:**
```bash
xcrun notarytool list-credentials
```

You should see `notarytool-profile` in the output.

### Step 5: Set Environment Variable

Add this to your shell profile (`~/.zshrc` or `~/.bashrc`):

```bash
# GAP notarization
export NOTARIZE_KEYCHAIN_PROFILE="notarytool-profile"
```

Then reload your shell:
```bash
source ~/.zshrc  # or ~/.bashrc
```

**Why?** This lets the notarization script automatically use your keychain profile without requiring the `--keychain-profile` flag every time.

## Verification

Test that everything is set up correctly:

```bash
# Build a test binary
cd /path/to/gap
cargo build --release

# Try notarizing (will fail if not signed, but validates credentials)
./scripts/macos-notarize.sh target/release/gap
```

If credentials are set up correctly, you'll see an error about the binary not being signed (expected), not about invalid credentials.

## Troubleshooting

### "Developer ID certificate not found"
Install your Developer ID certificate from Apple Developer Portal. Must be "Developer ID Application" (not "Apple Development").

### Error -25294 when importing certificate
The private key from your CSR isn't in the keychain. Either:
- The CSR was created on a different Mac
- The private key was deleted

Solution: Revoke the certificate and create a new one with a fresh CSR on this Mac.

### "different Team IDs" error when running binary
The binary needs the `disable-library-validation` entitlement. The build scripts handle this automatically, but if signing manually, use:
```bash
codesign --sign "Developer ID Application" \
  --force --options runtime --timestamp \
  --entitlements entitlements.plist \
  target/release/gap-server
```

Where `entitlements.plist` contains:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>
```
