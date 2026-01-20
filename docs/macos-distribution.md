# macOS Code Signing Setup

This document covers the one-time setup for macOS code signing. For the release process, see [RELEASING.md](./RELEASING.md).

## Prerequisites

### Apple Developer Account
- Apple Developer Program membership ($99/year): https://developer.apple.com/programs/

### Developer ID Certificate

1. Go to https://developer.apple.com/account/resources/certificates/list
2. Click "+" -> Select "Developer ID Application"
3. Create a Certificate Signing Request (CSR):
   - Open Keychain Access
   - Menu: Keychain Access -> Certificate Assistant -> Request a Certificate from a Certificate Authority
   - Enter your email, select "Saved to disk"
   - Save the `.certSigningRequest` file
4. Upload CSR to Apple, download the `.cer` file
5. Double-click to install (use **System** keychain, not iCloud)
6. Verify: `security find-identity -v -p codesigning | grep "Developer ID"`

### Notarization Credentials

1. Create app-specific password at https://appleid.apple.com (App-Specific Passwords -> Generate)
2. Store credentials in keychain:
   ```bash
   xcrun notarytool store-credentials "notarytool-profile" \
     --apple-id "your-apple-id@example.com" \
     --team-id "YOUR_TEAM_ID" \
     --password "xxxx-xxxx-xxxx-xxxx"
   ```

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
