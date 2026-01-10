# ACP Implementation Plan

## Overview

This plan breaks down the ACP implementation into phases with clear dependencies. Each phase produces working, testable code.

**Target:** Full architecture as specified in `native-app-design.md`
**Approach:** Bottom-up - build foundational layers first, then compose

---

## Phase 1: Foundation

**Goal:** Project skeleton with core types. Everything compiles, nothing runs.

**Dependencies:** None
**Estimated complexity:** Low

### 1.1 Project Setup
- [ ] Cargo workspace with two binaries (`acp`, `acp-server`)
- [ ] Shared library crate for common code
- [ ] CI-friendly structure (single `cargo build` builds all)
- [ ] Basic dependencies declared (tokio, clap, serde, etc.)

### 1.2 Core Types
- [ ] `ACPRequest` - method, url, headers, body
- [ ] `ACPCredentials` - string key-value map
- [ ] `ACPPlugin` - name, match patterns, credential schema, transform
- [ ] `AgentToken` - id, name, prefix, created_at
- [ ] `Config` - proxy/api ports, paths, logging

### 1.3 Error Handling
- [ ] `AcpError` enum with variants for each failure mode
- [ ] `Result<T>` type alias
- [ ] Proper error context via `thiserror` or similar

### 1.4 Deliverable
- `cargo build` succeeds
- `cargo test` runs (empty test suite passes)
- Types documented with rustdoc

---

## Phase 2: Secure Storage

**Goal:** Abstract secret storage with platform implementations.

**Dependencies:** Phase 1 (types)
**Estimated complexity:** Medium

### 2.1 SecretStore Trait
- [ ] Trait definition: `get`, `set`, `delete`, `list`
- [ ] Key namespacing (e.g., `credential:plugin:key`)
- [ ] Binary value support (for plugins, CA keys)

### 2.2 macOS Keychain Implementation
- [ ] `security-framework` crate integration
- [ ] Keychain item creation with ACLs
- [ ] Code signing considerations documented
- [ ] Unit tests (may need to be integration tests on macOS)

### 2.3 Linux File Implementation
- [ ] File-based storage in configurable directory
- [ ] Proper permissions (mode 600/700)
- [ ] Atomic writes (write-to-temp, rename)
- [ ] Unit tests

### 2.4 Platform Detection
- [ ] Runtime selection based on `cfg!(target_os)`
- [ ] Container mode detection (`--data-dir` override)

### 2.5 Deliverable
- Can store/retrieve secrets on both platforms
- Tests pass on macOS and Linux
- Password hash storage works

---

## Phase 3: TLS Infrastructure

**Goal:** Generate CA and sign certificates dynamically.

**Dependencies:** Phase 2 (CA key storage)
**Estimated complexity:** Medium-High

### 3.1 CA Generation
- [ ] Generate RSA or ECDSA key pair
- [ ] Self-signed CA certificate
- [ ] Store private key in SecretStore
- [ ] Export public cert to filesystem (for agent trust)

### 3.2 Dynamic Certificate Signing
- [ ] Generate cert for arbitrary hostname on-demand
- [ ] Short validity period (24h or configurable)
- [ ] Cache generated certs in memory (LRU)
- [ ] Sign with CA private key

### 3.3 Certificate Loading
- [ ] Load CA from SecretStore on server start
- [ ] Graceful handling of missing CA (prompt init)

### 3.4 Deliverable
- `acp-server init` generates CA, stores key, exports cert
- Can generate valid cert for any hostname
- `openssl verify` passes with CA trust

---

## Phase 4: Proxy Core

**Goal:** MITM proxy that forwards HTTPS requests (no transforms yet).

**Dependencies:** Phase 3 (TLS)
**Estimated complexity:** High

### 4.1 HTTP CONNECT Handler
- [ ] Listen on proxy port (default 9443)
- [ ] Parse CONNECT requests
- [ ] Extract target host:port
- [ ] Respond with 200 Connection Established

### 4.2 Agent-Side TLS
- [ ] Accept TLS connection from agent after CONNECT
- [ ] Present dynamically-generated cert for target host
- [ ] Use CA from Phase 3 to sign

### 4.3 Upstream TLS
- [ ] Establish TLS connection to actual target
- [ ] Verify upstream certificate
- [ ] System CA trust store

### 4.4 Bidirectional Proxying
- [ ] Read HTTP request from agent
- [ ] Forward to upstream
- [ ] Read response from upstream
- [ ] Forward to agent
- [ ] Handle streaming/chunked responses

### 4.5 Agent Authentication
- [ ] Parse `Proxy-Authorization: Bearer <token>` header
- [ ] Validate token against stored tokens
- [ ] Reject with 407 if invalid
- [ ] Attach agent identity to request context

### 4.6 Deliverable
- Agent can `curl --proxy` through ACP to any HTTPS endpoint
- Request/response passes through unmodified
- Invalid tokens rejected

---

## Phase 5: Plugin Runtime

**Goal:** Execute JavaScript transforms on requests.

**Dependencies:** Phase 1 (types), Phase 4 (integration point)
**Estimated complexity:** High

### 5.1 Boa Integration
- [ ] Embed Boa JS engine
- [ ] Create isolated context per request (or pooled)
- [ ] Timeout enforcement
- [ ] Memory limits

### 5.2 Global Objects
- [ ] `ACP.crypto.sha256`, `sha256Hex`, `hmac`
- [ ] `ACP.crypto.signAwsV4` (convenience)
- [ ] `ACP.util.base64`, `hex`, `utf8` encode/decode
- [ ] `ACP.util.now`, `isoDate`, `amzDate`
- [ ] `ACP.log` (captured, not printed)
- [ ] `TextEncoder`, `TextDecoder`, `URL`, `URLSearchParams`

### 5.3 Sandbox Enforcement
- [ ] Block `fetch`, `XMLHttpRequest`
- [ ] Block filesystem APIs
- [ ] Block `eval`, `Function` constructor
- [ ] Block `WebAssembly`
- [ ] Audit blocked APIs list

### 5.4 Plugin Loading
- [ ] Load plugin JS from SecretStore
- [ ] Parse and validate plugin structure
- [ ] Extract `name`, `match`, `credentialSchema`
- [ ] Cache compiled plugin

### 5.5 Transform Execution
- [ ] Convert Rust `ACPRequest` to JS object
- [ ] Call `plugin.transform(request, credentials)`
- [ ] Convert result back to Rust `ACPRequest`
- [ ] Handle errors gracefully

### 5.6 Host Matching
- [ ] Match request host against plugin `match[]` patterns
- [ ] Support wildcards (`*.s3.amazonaws.com`)
- [ ] Reject requests with no matching plugin (403)

### 5.7 Credential Scoping
- [ ] Load only credentials namespaced to matching plugin
- [ ] Pass scoped credentials to transform

### 5.8 Deliverable
- Plugin JS can modify request headers
- Credentials injected correctly
- Non-matching hosts rejected
- Sandbox prevents escape

---

## Phase 6: Management API

**Goal:** HTTP API for CLI and future GUI.

**Dependencies:** Phase 2 (storage), Phase 5 (plugin loading)
**Estimated complexity:** Medium

### 6.1 HTTP Server
- [ ] Listen on management port (default 9080)
- [ ] JSON request/response
- [ ] CORS for future GUI

### 6.2 Authentication Middleware
- [ ] Extract `password_hash` from request body
- [ ] Verify SHA512 hash against stored Argon2 hash
- [ ] Reject with 401 if invalid
- [ ] Skip auth for `/status` endpoint

### 6.3 Status Endpoint
- [ ] `GET /status` - version, uptime, ports
- [ ] No authentication required

### 6.4 Plugin Endpoints
- [ ] `GET /plugins` - list installed
- [ ] `POST /plugins/install` - fetch and preview
- [ ] `POST /plugins/install/confirm` - commit install
- [ ] `DELETE /plugins/:name` - uninstall

### 6.5 Credential Endpoints
- [ ] `POST /credentials/:plugin/:key` - set credential
- [ ] `DELETE /credentials/:plugin/:key` - delete credential

### 6.6 Token Endpoints
- [ ] `GET /tokens` - list (prefixes only)
- [ ] `POST /tokens` - create new
- [ ] `DELETE /tokens/:id` - revoke

### 6.7 Activity Endpoints
- [ ] `GET /activity` - recent requests
- [ ] `GET /activity/stream` - SSE stream

### 6.8 Deliverable
- All endpoints functional
- Authentication enforced
- Proper error responses

---

## Phase 7: CLI

**Goal:** Full command-line interface.

**Dependencies:** Phase 6 (API client)
**Estimated complexity:** Medium

### 7.1 CLI Framework
- [ ] `clap` with derive macros
- [ ] Subcommand structure
- [ ] `--server` flag and `ACP_SERVER` env var
- [ ] Help text and examples

### 7.2 Password Input
- [ ] Secure input (no echo)
- [ ] SHA512 hashing client-side
- [ ] Confirmation prompt for `init`

### 7.3 Init Command
- [ ] `acp init [--ca-path]`
- [ ] Set password
- [ ] Trigger server CA generation
- [ ] Print CA cert path

### 7.4 Status Command
- [ ] `acp status`
- [ ] No auth required
- [ ] Show version, uptime, ports

### 7.5 Plugin Commands
- [ ] `acp plugins` - list
- [ ] `acp install <name>` - install
- [ ] `acp uninstall <name>` - remove

### 7.6 Credential Commands
- [ ] `acp set <plugin>:<key>` - set (interactive value)

### 7.7 Token Commands
- [ ] `acp tokens` - list
- [ ] `acp token create <name>` - create
- [ ] `acp token revoke <id>` - revoke

### 7.8 Activity Commands
- [ ] `acp activity` - recent
- [ ] `acp activity --follow` - stream

### 7.9 Deliverable
- All commands working
- Consistent UX
- Proper exit codes

---

## Phase 8: Integration & Polish

**Goal:** End-to-end working system with bundled plugins.

**Dependencies:** All previous phases
**Estimated complexity:** Medium

### 8.1 Bundled Plugins
- [ ] Exa plugin (`plugins/exa.js`)
- [ ] AWS S3 plugin (`plugins/aws-s3.js`)
- [ ] Include in binary or install on init

### 8.2 End-to-End Tests
- [ ] Docker Compose test environment
- [ ] Test full flow: init → install → configure → proxy
- [ ] Test with real APIs (Exa, etc.) if keys available
- [ ] Test with mock upstream

### 8.3 Installation Scripts
- [ ] macOS: Homebrew formula or manual install
- [ ] Linux: `install.sh` script
- [ ] Container: Dockerfile

### 8.4 Documentation
- [ ] README with quick start
- [ ] Plugin authoring guide
- [ ] Security model explanation

### 8.5 Deliverable
- Complete working system
- Installable on macOS and Linux
- Runnable in containers

---

## Dependency Graph

```
Phase 1: Foundation
    │
    ├──────────────────┬─────────────────┐
    ▼                  ▼                 ▼
Phase 2: Storage   Phase 3: TLS    Phase 5.1-5.3: Boa Runtime
    │                  │                 │
    │                  ▼                 │
    │            Phase 4: Proxy ─────────┤
    │                  │                 │
    ▼                  │                 ▼
Phase 6: Mgmt API ◄────┴────────► Phase 5.4-5.8: Plugin System
    │
    ▼
Phase 7: CLI
    │
    ▼
Phase 8: Integration
```

## Parallelization Opportunities

After Phase 1 completes:
- **Phase 2** and **Phase 3** can run in parallel
- **Phase 5.1-5.3** (Boa setup) can run in parallel with 2 & 3

After Phase 2 & 3 complete:
- **Phase 4** depends on Phase 3
- **Phase 6** depends on Phase 2

After Phase 4 completes:
- **Phase 5.4-5.8** integrates with proxy

---

## Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Boa JS engine | Performance, ES compatibility | Spike early; have fallback plan (V8 isolates) |
| macOS Keychain ACLs | Code signing complexity | Test on real macOS; may need signing for full protection |
| TLS MITM | Certificate handling edge cases | Extensive testing with various TLS configs |
| Plugin sandbox | Escape vectors | Security review; limit attack surface |

---

## Next Steps

1. Review and approve this plan
2. Create Phase 1 tasks on kanban
3. Begin implementation

