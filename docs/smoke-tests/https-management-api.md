# Smoke Test: HTTPS Management API

> Last verified: 2026-01-12 | Status: FAILED

## Prerequisites
- [x] Clean environment (no existing ACP data directory)
- [x] Rust toolchain installed
- [x] Both server and CLI binaries can be built
- [x] Ports 9443 and 9080 available

## Configuration Notes
- **Management API Port:** 9080 (NOT 9443 as indicated in initial spec)
- **Proxy Port:** 9443
- **CLI Default:** `https://localhost:9443` (MISMATCH with actual Management API port)
- **Storage:** macOS uses Keychain by default, not `~/.acp-data`
- **CA Certificate Location:** `~/.config/acp/ca.crt`

## Critical Path 1: Fresh Init Flow

**Goal:** Verify that a fresh initialization creates a management certificate and the server serves HTTPS correctly.

### Steps
1. Clean any existing data directory
   - Run: `rm -rf ~/.acp` (Keychain data persists)
   - Expected: Clean slate for testing
   - [x] **PASS**

2. Start the server
   - Run: `cargo run -p acp-server`
   - Expected: Server starts and listens on ports 9443 (proxy) and 9080 (management API)
   - [x] **PASS** - Server starts and auto-generates CA and management cert

3. Run init command with password
   - Run: `echo "testpass123\ntestpass123" | cargo run -p acp -- --server https://localhost:9080 init`
   - Expected: Command succeeds, management certificate generated
   - [x] **FAIL** - Server returns "409 Conflict: Server already initialized"
   - **Issue:** Server auto-initializes on startup, no fresh init flow exists

4. Verify HTTPS is being served
   - Check server logs for HTTPS binding
   - Expected: Server bound to HTTPS on port 9080 (management API)
   - [x] **PASS** - Logs show: "Management API listening on https://0.0.0.0:9080"

5. Verify CLI can communicate
   - Run: `cargo run -p acp -- --server https://localhost:9080 status`
   - Expected: CLI connects via HTTPS and gets response
   - [x] **FAIL** - Certificate signature verification fails
   - **Issue:** Management certificate not properly signed by CA

### Result
- Status: FAILED
- Notes:
  - Server auto-initializes on first startup (CA + management cert generated automatically)
  - No true "fresh init" flow where user sets password before certs are generated
  - Critical bug: Management certificate fails signature verification against CA
  - Verified with: `openssl s_client -connect localhost:9080 -CAfile ~/.config/acp/ca.crt`
  - Error: "Verification error: certificate signature failure"
  - Server IS serving HTTPS and responds correctly when cert verification is disabled (`curl -k`)

---

## Critical Path 2: Basic Operations Over HTTPS

**Goal:** Verify that standard CLI operations work correctly over the HTTPS connection.

### Steps
1. Check server status
   - Run: `cargo run -p acp -- --server https://localhost:9080 status`
   - Expected: Returns server status information
   - [x] **FAIL** - "Error: Failed to send request"
   - **Root cause:** Certificate verification failure (same as Path 1, step 5)

2. Workaround test with curl
   - Run: `curl -k https://localhost:9080/status`
   - Expected: Server responds when cert verification is disabled
   - [x] **PASS** - Returns: `{"version":"0.2.2","uptime_seconds":102,"proxy_port":9443,"api_port":9080}`

3. Verification test
   - Run: `curl --cacert ~/.config/acp/ca.crt https://localhost:9080/status`
   - Expected: Works with proper cert verification
   - [x] **FAIL** - LibreSSL error: "asn1 encoding routines:CRYPTO_internal:EVP lib"

### Result
- Status: BLOCKED
- Notes:
  - Cannot test CLI operations because certificate verification is broken
  - Management API serves HTTPS correctly but certificate chain is invalid
  - All CLI commands that require HTTPS will fail
  - The fundamental HTTPS implementation is broken

---

## Critical Path 3: Certificate Rotation (Hot-Swap)

**Goal:** Verify that management certificates can be rotated without restarting the server.

### Steps
1. Verify current certificate is working
   - Run: `cargo run -p acp -- --server https://localhost:9080 status`
   - Expected: Command succeeds
   - [x] **NOT TESTED** - Baseline certificate verification already broken

2. Rotate the management certificate
   - Run: `cargo run -p acp -- --server https://localhost:9080 new-management-cert --sans "DNS:localhost,IP:127.0.0.1"`
   - Expected: New certificate generated successfully
   - [x] **NOT TESTED** - Cannot test without working baseline

3. Verify server continues working (no restart required)
   - Run: `cargo run -p acp -- --server https://localhost:9080 status`
   - Expected: Command succeeds with new certificate
   - [x] **NOT TESTED**

4. Check server logs
   - Review server output
   - Expected: Evidence of hot-swap (cert reload without restart)
   - [x] **NOT TESTED**

### Result
- Status: NOT TESTED
- Notes:
  - Cannot test certificate rotation when baseline certificate verification is broken
  - Must fix certificate signing issue before rotation can be verified
  - Testing rotation on a broken baseline would produce meaningless results

---

## Critical Path 4: Custom SANs on Init

**Goal:** Verify that custom Subject Alternative Names can be specified during initialization.

### Steps
1. Clean environment again
   - Run: `rm -rf ~/.acp` (and clear Keychain)
   - Expected: Clean slate
   - [x] **NOT TESTED**

2. Start server
   - Run: `cargo run -p acp-server`
   - Expected: Server starts
   - [x] **NOT TESTED**

3. Init with custom SANs
   - Run: `cargo run -p acp -- --server https://localhost:9080 init --management-sans "DNS:localhost,IP:127.0.0.1,IP:::1"`
   - Expected: Init succeeds with custom SANs
   - [x] **NOT TESTED** - Server auto-initializes before init command can run

4. Verify server accepts connections
   - Run: `cargo run -p acp -- --server https://localhost:9080 status`
   - Expected: CLI connects successfully
   - [x] **NOT TESTED**

### Result
- Status: NOT TESTED
- Notes:
  - Cannot test custom SANs on init because server auto-initializes
  - The init command runs AFTER the server has already started and generated default certs
  - Design issue: No way to specify custom SANs before server generates management cert

---

## Summary
| Path | Status | Notes |
|------|--------|-------|
| Fresh Init Flow | FAILED | Server auto-initializes; cert verification broken |
| Basic Operations Over HTTPS | BLOCKED | Certificate verification fails - cannot test |
| Certificate Rotation | NOT TESTED | Blocked by broken baseline |
| Custom SANs on Init | NOT TESTED | No mechanism to set SANs before auto-init |

## Critical Issues Found

### 1. Certificate Signature Verification Failure (CRITICAL)
**Severity:** Blocker
**Impact:** All HTTPS connections fail with certificate verification errors

**Details:**
- Management certificate fails signature verification against CA certificate
- Error: "certificate signature failure" (OpenSSL)
- Error: "asn1 encoding routines:CRYPTO_internal:EVP lib" (LibreSSL/curl)
- CLI cannot communicate with server over HTTPS
- Server responds correctly when cert verification is disabled (`curl -k`)

**Reproduction:**
```bash
openssl s_client -connect localhost:9080 -CAfile ~/.config/acp/ca.crt -showcerts
# Shows: "Verification error: certificate signature failure"
```

**Root Cause:**
The management certificate is either:
1. Not being signed by the CA that's exported to `~/.config/acp/ca.crt`, OR
2. Signed incorrectly, OR
3. Using a different CA than what's exported

### 2. Port Configuration Mismatch (HIGH)
**Severity:** High
**Impact:** Users will be confused, CLI won't work with default settings

**Details:**
- CLI defaults to `https://localhost:9443`
- Management API actually runs on port 9080
- User spec mentioned port 9443 for management API
- Every CLI command requires `--server https://localhost:9080` override

**Recommendation:**
Either:
- Change Management API to port 9443 (match CLI default), OR
- Change CLI default to port 9080 (match implementation), OR
- Use environment variable SERVER (already supported)

### 3. Server Auto-Initialization (MEDIUM)
**Severity:** Medium
**Impact:** Init flow doesn't work as documented

**Details:**
- Server generates CA and management cert on first startup
- `acp init` command returns "409 Conflict: Server already initialized"
- No way to run init BEFORE certificates are generated
- Password cannot be set before auto-initialization

**Current flow:**
1. Start server
2. Server auto-generates CA + management cert
3. Run `acp init` -> Already initialized error

**Expected flow (based on spec):**
1. Start server (waits for init)
2. Run `acp init --password <pw>` -> Generates certs
3. Server ready to serve

### 4. Custom SANs Not Applicable (MEDIUM)
**Severity:** Medium
**Impact:** Cannot customize SANs on fresh init

**Details:**
- `--management-sans` flag exists on `acp init`
- But init runs AFTER server has already generated management cert
- No way to specify custom SANs before auto-initialization
- The `new-management-cert` command would be the only way to set custom SANs

**Recommendation:**
Either:
- Server should wait for init before generating management cert, OR
- Add `--management-sans` flag to `acp-server` binary, OR
- Document that users must run `new-management-cert` to set custom SANs

## Test Environment
- **OS:** macOS (Darwin 25.1.0)
- **Storage:** Keychain (default on macOS)
- **Rust:** Latest (cargo build succeeded)
- **Server:** acp-server v0.2.2
- **CLI:** acp v0.2.2

## Next Steps
1. **Fix certificate signing** - This is a blocker for all HTTPS functionality
2. **Resolve port mismatch** - Align CLI default with actual Management API port
3. **Review initialization flow** - Decide if auto-init is desired behavior
4. **Retest after fixes** - Run smoke test again once cert issue is resolved
