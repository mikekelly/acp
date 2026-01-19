# Smoke Test: Data Protection Keychain (No Password Prompts)

> Last verified: 2026-01-19 | Status: FAIL

## Prerequisites
- [ ] macOS 10.15 (Catalina) or later
- [ ] Rust toolchain installed
- [ ] Code signing certificate available (self-signed dev cert is fine)
- [ ] No existing GAP keychain items (clean slate)
- [ ] Ports 9080 and 9081 available

## Test Environment
- **OS:** macOS (Darwin 25.1.0)
- **Access Group:** `3R44BTH39W.com.gap.secrets`
- **Service Name:** `com.gap.credentials`
- **Storage:** Data Protection Keychain (entitlement-based)

## What We're Verifying

**Core objective:** Zero keychain password prompts throughout the entire GAP lifecycle when using Data Protection Keychain.

**Why this matters:**
- Traditional keychain uses ACLs (Access Control Lists) that trigger password prompts
- Data Protection Keychain uses entitlements - if the binary is properly signed with matching entitlements, access is automatic
- This is a breaking change from traditional keychain - existing items won't be accessible

**Critical change:**
- Old: `KeychainStore::new()` → Traditional keychain → Password prompts
- New: `KeychainStore::new_with_data_protection()` → Data Protection Keychain → No prompts (if signed correctly)

## Critical Path 1: Clean State

**Goal:** Remove all traces of old keychain storage to ensure we're testing Data Protection Keychain from scratch.

### Steps

1. Stop any running gap-server process
   - Run: `killall gap-server` or `launchctl unload ~/Library/LaunchAgents/com.gap.server.plist`
   - Expected: Server stopped (may error if not running - that's fine)
   - [ ] Pass / Fail
   - Notes:

2. Delete old keychain items (traditional keychain)
   - Run: `security delete-generic-password -s "com.gap.credentials" -a "ca_cert" 2>/dev/null || true`
   - Run: `security delete-generic-password -s "com.gap.credentials" -a "management_cert" 2>/dev/null || true`
   - Run: `security delete-generic-password -s "com.gap.credentials" -a "management_key" 2>/dev/null || true`
   - Run: `security delete-generic-password -s "com.gap.credentials" -a "_registry" 2>/dev/null || true`
   - Expected: Items deleted (may error if not found - that's fine)
   - [ ] Pass / Fail
   - Notes:

3. Delete CA certificate file
   - Run: `rm -f ~/Library/Application\ Support/gap/ca.crt`
   - Run: `rm -f ~/.config/gap/ca.crt`
   - Expected: Files removed
   - [ ] Pass / Fail
   - Notes:

4. Delete GAP data directories
   - Run: `rm -rf ~/.config/gap ~/.local/share/gap /tmp/gap-test-data`
   - Expected: Clean slate
   - [ ] Pass / Fail
   - Notes:

### Result
- Status: [ ] PASS / [ ] FAIL
- Notes:

---

## Critical Path 2: Build and Sign Binaries

**Goal:** Build release binaries and sign them with entitlements required for Data Protection Keychain.

### Steps

1. Build release binaries
   - Run: `cargo build --workspace --release`
   - Expected: Clean build completes successfully
   - [ ] Pass / Fail
   - Notes:

2. Create entitlements file with keychain-access-groups
   - Create: `/tmp/gap.entitlements`
   - Content:
     ```xml
     <?xml version="1.0" encoding="UTF-8"?>
     <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
     <plist version="1.0">
     <dict>
         <key>keychain-access-groups</key>
         <array>
             <string>3R44BTH39W.com.gap.secrets</string>
         </array>
     </dict>
     </plist>
     ```
   - Expected: File created
   - [ ] Pass / Fail
   - Notes:

3. Sign binaries with entitlements
   - Run: `codesign --sign "GAP Development" --force --entitlements /tmp/gap.entitlements target/release/gap-server`
   - Run: `codesign --sign "GAP Development" --force --entitlements /tmp/gap.entitlements target/release/gap`
   - Expected: Both binaries signed successfully (may prompt to create "GAP Development" cert first)
   - [ ] Pass / Fail
   - Notes:

4. Verify signatures include entitlements
   - Run: `codesign --display --entitlements - target/release/gap-server`
   - Expected: Output shows `<string>3R44BTH39W.com.gap.secrets</string>`
   - [ ] Pass / Fail
   - Notes:

### Result
- Status: [ ] PASS / [ ] FAIL
- Notes:

---

## Critical Path 3: Start gap-server (Zero Prompts Expected)

**Goal:** Verify that gap-server starts and creates keychain items WITHOUT any password prompts.

### Steps

1. Start gap-server in foreground (watch for prompts)
   - Run: `./target/release/gap-server --data-dir /tmp/gap-test-data`
   - Expected: Server starts, logs show CA and management cert generation, NO password prompts appear
   - [ ] Pass / Fail
   - Watch for:
     - [ ] "Generating new CA certificate"
     - [ ] "CA certificate saved to storage"
     - [ ] "Generating new management certificate"
     - [ ] "Management certificate saved to storage"
     - [ ] "Management API listening on https://0.0.0.0:9080"
     - [ ] NO password prompt dialog
   - Notes:

2. Verify server is running
   - Run in another terminal: `curl --cacert ~/.config/gap/ca.crt https://localhost:9080/status 2>/dev/null || echo "CA cert not exported yet"`
   - Expected: Either status JSON or error about missing CA cert (we haven't run init yet)
   - [ ] Pass / Fail
   - Notes:

### Result
- Status: [ ] PASS / [ ] FAIL
- Notes:

---

## Critical Path 4: Run gap init (Zero Prompts Expected)

**Goal:** Verify that gap init downloads the CA cert WITHOUT any password prompts.

### Steps

1. Run gap init (watch for prompts)
   - Run: `GAP_PASSWORD=testpass123 ./target/release/gap init`
   - Expected: Initializes successfully, CA cert saved to ~/.config/gap/ca.crt, NO password prompts
   - [ ] Pass / Fail
   - Watch for:
     - [ ] "GAP initialized successfully!"
     - [ ] "CA certificate saved to: /Users/[user]/.config/gap/ca.crt"
     - [ ] NO password prompt dialog
   - Notes:

2. Verify CA cert was exported
   - Run: `ls -la ~/.config/gap/ca.crt`
   - Expected: File exists
   - [ ] Pass / Fail
   - Notes:

3. Test CLI command (watch for prompts)
   - Run: `./target/release/gap status`
   - Expected: Shows status, NO password prompts
   - [ ] Pass / Fail
   - Notes:

### Result
- Status: [ ] PASS / [ ] FAIL
- Notes:

---

## Critical Path 5: Verify Data Protection Keychain Storage

**Goal:** Confirm that items were actually stored in Data Protection Keychain, not traditional keychain.

### Steps

1. Attempt to access items without Data Protection Keychain API
   - Run: `security find-generic-password -s "com.gap.credentials" -a "ca_cert" 2>&1`
   - Expected: Should fail or show limited info (Data Protection items aren't fully accessible via security command)
   - [ ] Pass / Fail
   - Notes:

2. Stop and restart gap-server (watch for prompts on second start)
   - Run: Kill the first gap-server process
   - Run: `./target/release/gap-server --data-dir /tmp/gap-test-data`
   - Expected: Server starts, finds existing keychain items, NO password prompts, NO regeneration of CA/certs
   - [ ] Pass / Fail
   - Watch for:
     - [ ] Server starts quickly
     - [ ] NO "Generating new CA certificate" (should load existing)
     - [ ] NO password prompt
   - Notes:

### Result
- Status: [ ] PASS / [ ] FAIL
- Notes:

---

## Summary

| Path | Status | Notes |
|------|--------|-------|
| Clean State | PASS | Cleaned up launch agent and CA cert |
| Build and Sign Binaries | FAIL | Missing keychain-access-groups entitlement |
| Start gap-server | FAIL | Error -34018 (errSecMissingEntitlement) |
| Run gap init | NOT TESTED | Blocked by server crash |
| Verify Data Protection Keychain | NOT TESTED | Blocked by server crash |

## Test Results (2026-01-19)

### Critical Path 2: Build and Sign Binaries

**Steps Completed:**
1. Built release binaries: PASS
   - Command: `cargo build --workspace --release`
   - Result: Finished in 0.46s

2. Signed with production mode: PASS
   - Command: `./scripts/macos-sign.sh --production`
   - Result: Signed successfully with Developer ID Application: Mike Kelly (3R44BTH39W)
   - Hardened runtime: Enabled
   - Timestamp: Included

3. Verified signatures: PASS
   - Command: `codesign --verify --verbose target/release/gap-server`
   - Result: Valid on disk, satisfies Designated Requirement

4. Checked entitlements: FAIL
   - Command: `codesign -d --entitlements :- ./target/release/gap-server`
   - Result: Only contains `com.apple.security.cs.disable-library-validation`
   - Missing: `keychain-access-groups` entitlement

**Status: FAIL** - Entitlements do not include required keychain-access-groups

### Critical Path 3: Start gap-server

**Steps Completed:**
1. Started gap-server: FAIL
   - Command: `./target/release/gap-server`
   - Result: Error: Storage error: Keychain operation failed with status: -34018
   - Error code -34018 is errSecMissingEntitlement
   - Server could not create Data Protection Keychain items

**Status: FAIL** - Server crashed due to missing entitlements

## Root Cause Analysis

**Issue:** The `--production` signing mode does NOT include the `keychain-access-groups` entitlement required for Data Protection Keychain.

**Evidence:**
1. Code uses Data Protection Keychain (gap-lib/src/storage.rs:289)
   ```rust
   let store = KeychainStore::new_with_data_protection(
       "com.gap.credentials",
       "3R44BTH39W.com.gap.secrets",
   )?;
   ```

2. Script comment (scripts/macos-sign.sh:129-130) says:
   > Note: keychain-access-groups is a restricted entitlement that requires
   > Apple provisioning for Developer ID apps. We don't need it - the keychain
   > kSecAttrAccessGroup still works using the Team ID from code signature.

3. Reality: Data Protection Keychain DOES require the entitlement. The comment is incorrect.

**Impact:**
- gap-server cannot start in production mode
- Error -34018 (errSecMissingEntitlement) occurs immediately on first keychain access
- No password prompts because server crashes before reaching that point

## Known Issues

### Issue 1: macos-sign.sh --production does not include keychain-access-groups entitlement

**Symptom:** Server crashes with error -34018 when using --production signing

**Root cause:** The signing script's production mode creates an entitlements file with only `com.apple.security.cs.disable-library-validation`, missing the required `keychain-access-groups` array.

**Affected code:** scripts/macos-sign.sh lines 131-141

**Workaround:** Manual signing with correct entitlements:
```bash
cat > /tmp/gap-full.entitlements <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
    <key>keychain-access-groups</key>
    <array>
        <string>3R44BTH39W.com.gap.secrets</string>
    </array>
</dict>
</plist>
EOF

codesign --sign "Developer ID Application" --force --options runtime --timestamp --entitlements /tmp/gap-full.entitlements target/release/gap-server
codesign --sign "Developer ID Application" --force --options runtime --timestamp --entitlements /tmp/gap-full.entitlements target/release/gap
```

**Fix needed:** Update scripts/macos-sign.sh production entitlements to include keychain-access-groups

## Troubleshooting

### If password prompts still appear:

1. **Verify entitlements are embedded:**
   ```bash
   codesign --display --entitlements - target/release/gap-server
   ```
   Should show `keychain-access-groups` with `3R44BTH39W.com.gap.secrets`.

2. **Verify signature is valid:**
   ```bash
   codesign --verify --verbose target/release/gap-server
   ```
   Should show "valid on disk" and "satisfies its Designated Requirement".

3. **Check for traditional keychain items:**
   If old items exist in traditional keychain, Data Protection Keychain won't find them (by design). Delete old items:
   ```bash
   security delete-generic-password -s "com.gap.credentials" 2>/dev/null || true
   ```

4. **Verify macOS version:**
   ```bash
   sw_vers
   ```
   Needs macOS 10.15 (Catalina) or later.

### Expected Behavior vs Failure Modes

| Scenario | Expected | Failure Mode |
|----------|----------|--------------|
| First start | No prompts, creates items | Prompts for password = entitlements not working |
| Second start | No prompts, loads items | Prompts or recreates = not finding items |
| Init command | No prompts, exports CA | Prompts = entitlements not working |
| Status command | No prompts, connects | Prompts = reading cert requires password |

## Success Criteria

**PASS requires:**
- Zero password prompts throughout entire flow
- Server starts successfully twice (create, then load)
- Init command completes successfully
- Status command works

**FAIL if:**
- Any password prompt appears
- Items not found on second start (forces regeneration)
- Entitlements not embedded in signature

## Notes

**Why this test is critical:**
- Password prompts defeat the purpose of automated proxy
- Agents can't respond to password dialogs
- Data Protection Keychain is the solution - but only if implemented correctly
- Signing with entitlements is easy to get wrong

**Reference:**
- AGENT_ORIENTATION.md lines 41-60 (Data Protection Keychain section)
- gap-lib/src/storage.rs:289 (usage in production code)
- Access group: `3R44BTH39W.com.gap.secrets` (Team ID prefix + bundle ID)
