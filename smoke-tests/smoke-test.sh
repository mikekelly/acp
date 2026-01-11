#!/bin/bash
set -e

# Comprehensive ACP Smoke Test
# Tests the full workflow using the CLI as users would in the real world

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

# Setup paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Test configuration
TEST_PASSWORD="smoke-test-password-$(date +%s)"
API_PORT=19080
PROXY_PORT=19443

# CLI binary paths
ACP="$WORKSPACE_ROOT/target/release/acp"
ACP_SERVER="$WORKSPACE_ROOT/target/release/acp-server"

# Export password for CLI (undocumented env var for testing)
export ACP_PASSWORD="$TEST_PASSWORD"

log_info "====================================="
log_info "ACP Comprehensive Smoke Test"
log_info "====================================="
echo ""

# Phase 1: Setup
log_step "Phase 1: Setup"
echo "-----------------------------------"

# 1.1: Build binaries
log_step "Phase 1.1: Building release binaries"
cd "$WORKSPACE_ROOT"
if cargo build --release 2>&1 | tail -5; then
    log_success "Binaries built successfully"
else
    log_error "Failed to build binaries"
    exit 1
fi

# 1.2: Create temp directory
TEMP_DIR=$(mktemp -d)
log_step "Phase 1.2: Created temp directory: $TEMP_DIR"

# Cleanup function
cleanup() {
    log_step "Cleanup: Stopping server and removing temp files"
    if [ -n "$SERVER_PID" ]; then
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    rm -rf "$TEMP_DIR"
    log_success "Cleanup complete"
}
trap cleanup EXIT

# 1.3: Start server
log_step "Phase 1.3: Starting acp-server"
ACP_DATA_DIR="$TEMP_DIR" "$ACP_SERVER" \
    --api-port $API_PORT \
    --proxy-port $PROXY_PORT \
    --log-level warn > "$TEMP_DIR/server.log" 2>&1 &
SERVER_PID=$!
log_info "Server PID: $SERVER_PID"

# 1.4: Health check (use curl just for initial health - server not yet initialized)
log_step "Phase 1.4: Waiting for server health check"
MAX_RETRIES=30
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if curl -f -s "http://localhost:$API_PORT/status" > /dev/null 2>&1; then
        log_success "Server is healthy"
        break
    fi
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
        log_error "Server health check failed after $MAX_RETRIES retries"
        cat "$TEMP_DIR/server.log"
        exit 1
    fi
    sleep 1
done

log_success "Phase 1 complete: Server is running"
echo ""

# Phase 2: Initialization
log_step "Phase 2: Initialization"
echo "-----------------------------------"

# 2.1: Initialize server using CLI
log_step "Phase 2.1: Initializing server with 'acp init'"
INIT_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" init 2>&1)

if echo "$INIT_OUTPUT" | grep -q "initialized successfully"; then
    log_success "Server initialized via CLI"
    echo "$INIT_OUTPUT" | grep -E "CA certificate|Next steps" | head -3
else
    log_error "Initialization failed: $INIT_OUTPUT"
    exit 1
fi

# 2.2: Verify status using CLI
log_step "Phase 2.2: Checking status with 'acp status'"
STATUS_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" status 2>&1)

if echo "$STATUS_OUTPUT" | grep -q "Version:"; then
    VERSION=$(echo "$STATUS_OUTPUT" | grep "Version:" | awk '{print $2}')
    log_success "Status check passed (version: $VERSION)"
else
    log_error "Status check failed: $STATUS_OUTPUT"
    exit 1
fi

log_success "Phase 2 complete: Server initialized"
echo ""

# Phase 3: Token Management
log_step "Phase 3: Token Management"
echo "-----------------------------------"

# 3.1: Create token using CLI
log_step "Phase 3.1: Creating agent token with 'acp token create'"
TOKEN_NAME="smoke-test-agent-$(date +%s)"
TOKEN_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" token create "$TOKEN_NAME" 2>&1)

if echo "$TOKEN_OUTPUT" | grep -q "Token created"; then
    TOKEN_ID=$(echo "$TOKEN_OUTPUT" | grep "ID:" | awk '{print $2}')
    log_success "Token created: $TOKEN_NAME (ID: $TOKEN_ID)"
else
    log_error "Token creation failed: $TOKEN_OUTPUT"
    exit 1
fi

# 3.2: List tokens using CLI
log_step "Phase 3.2: Listing tokens with 'acp token list'"
LIST_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" token list 2>&1)

if echo "$LIST_OUTPUT" | grep -q "$TOKEN_NAME"; then
    log_success "Token appears in list"
else
    log_error "Token not found in list: $LIST_OUTPUT"
    exit 1
fi

# 3.3: Revoke token using CLI
log_step "Phase 3.3: Revoking token with 'acp token revoke'"
REVOKE_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" token revoke "$TOKEN_ID" 2>&1)

if echo "$REVOKE_OUTPUT" | grep -qi "revoked\|deleted\|success"; then
    log_success "Token revoked"
else
    log_error "Token revocation failed: $REVOKE_OUTPUT"
    exit 1
fi

# 3.4: Verify token is gone
log_step "Phase 3.4: Verifying token was deleted"
LIST_AFTER=$("$ACP" --server "http://localhost:$API_PORT" token list 2>&1)

if echo "$LIST_AFTER" | grep -q "$TOKEN_NAME"; then
    log_error "Token still appears after deletion"
    exit 1
else
    log_success "Token successfully removed from list"
fi

log_success "Phase 3 complete: Token management verified"
echo ""

# Phase 4: Plugin Management
log_step "Phase 4: Plugin Management"
echo "-----------------------------------"

# 4.1: Install plugin using CLI
log_step "Phase 4.1: Installing plugin with 'acp install mikekelly/exa-acp'"
INSTALL_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" install mikekelly/exa-acp 2>&1)

if echo "$INSTALL_OUTPUT" | grep -q "installed successfully"; then
    log_success "Plugin installed: mikekelly/exa-acp"
else
    log_error "Plugin installation failed: $INSTALL_OUTPUT"
    exit 1
fi

# 4.2: List plugins using CLI
log_step "Phase 4.2: Listing plugins with 'acp plugins'"
PLUGINS_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" plugins 2>&1)

if echo "$PLUGINS_OUTPUT" | grep -q "mikekelly/exa-acp"; then
    log_success "Plugin appears in list"
    # Show plugin details
    echo "$PLUGINS_OUTPUT" | grep -A5 "mikekelly/exa-acp" | head -6
else
    log_error "Plugin not found in list: $PLUGINS_OUTPUT"
    exit 1
fi

log_success "Phase 4 complete: Plugin management verified"
echo ""

# Phase 5: Credential Management
log_step "Phase 5: Credential Management"
echo "-----------------------------------"

# 5.1: Set credentials using CLI
log_step "Phase 5.1: Setting credential with 'acp set mikekelly/exa-acp:apiKey'"
CRED_VALUE="test-secret-key-$(date +%s)"
export ACP_CREDENTIAL_VALUE="$CRED_VALUE"

SET_OUTPUT=$("$ACP" --server "http://localhost:$API_PORT" set "mikekelly/exa-acp:apiKey" 2>&1)

if echo "$SET_OUTPUT" | grep -qi "set successfully\|success"; then
    log_success "Credential set successfully"
else
    log_error "Credential setting failed: $SET_OUTPUT"
    exit 1
fi

log_success "Phase 5 complete: Credential management verified"
echo ""

# Phase 6: Cleanup
log_step "Phase 6: Cleanup"
echo "-----------------------------------"

# The trap will handle actual cleanup
log_success "Phase 6 complete: Cleanup will execute on exit"
echo ""

# Final summary
echo ""
log_info "====================================="
log_info "ALL TESTS PASSED"
log_info "====================================="
echo ""
echo "Summary:"
echo "  ✓ Phase 1: Setup (build, start server, health check)"
echo "  ✓ Phase 2: Initialization (acp init, acp status)"
echo "  ✓ Phase 3: Token Management (acp token create/list/revoke)"
echo "  ✓ Phase 4: Plugin Management (acp install, acp plugins)"
echo "  ✓ Phase 5: Credential Management (acp set)"
echo "  ✓ Phase 6: Cleanup (server stop, temp dir removal)"
echo ""
log_success "Comprehensive smoke test completed successfully"
exit 0
