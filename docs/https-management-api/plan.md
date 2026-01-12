# HTTPS for Management API - Implementation Plan

## Current State

### Certificate Infrastructure (acp-lib/src/tls.rs)
- `CertificateAuthority` struct handles CA generation and cert signing
- `generate()` creates a self-signed CA (10-year validity)
- `sign_for_hostname(hostname, validity)` signs certs for specific hostnames
- Uses `rcgen` for cert generation, `rustls` for TLS
- Certs stored in `SecretStore` with keys like `ca:cert`, `ca:key`

### Server Startup (acp-server/src/main.rs)
- `load_or_generate_ca()` loads/creates CA at startup (lines 167-192)
- Management API uses plain `axum::serve(listener, app)` (line 161)
- Binds to `0.0.0.0:{api_port}` with `TcpListener` (line 155)
- No TLS wrapper currently

### CLI Client (acp/src/client.rs)
- `ApiClient` uses `reqwest::Client::new()` with no TLS config
- Default server URL: `http://localhost:9080`
- Auth via SHA512 password hash in request body

### Dependencies Already Available
- `tokio-rustls = "0.26"` in acp-lib
- `rustls = "0.23"` in acp-lib
- `rustls-pemfile = "2.0"` in acp-lib
- Workspace has `rustls-tls` feature for reqwest

## Blueprint

```
┌─────────────────────────────────────────────────────────────┐
│                        acp init                             │
│  --management-sans "DNS:localhost,IP:127.0.0.1,IP:::1"     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    POST /init                               │
│  Server generates management cert signed by CA              │
│  Stores: mgmt:cert, mgmt:key                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                Server Startup                               │
│  1. Load CA from storage                                    │
│  2. Load mgmt cert/key from storage                        │
│  3. Create TlsAcceptor with mgmt cert                      │
│  4. Serve HTTPS on api_port                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Client                               │
│  1. Load ca.crt from known path                            │
│  2. Configure reqwest with custom root cert                │
│  3. Connect to https://localhost:9080                      │
└─────────────────────────────────────────────────────────────┘
```

### Cert Rotation Flow

```
┌─────────────────────────────────────────────────────────────┐
│        acp new-management-cert --sans "..."                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              POST /v1/management-cert                       │
│  { "password_hash": "...", "sans": ["DNS:...", ...] }      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Server Handler                             │
│  1. Generate new cert with requested SANs                  │
│  2. Store new cert/key in SecretStore                      │
│  3. Build new TlsAcceptor                                  │
│  4. Swap acceptor (Arc<RwLock<TlsAcceptor>>)              │
└─────────────────────────────────────────────────────────────┘
```

## Phases

### Phase 1: Certificate Generation Infrastructure

Extend `CertificateAuthority` to sign server certs with configurable SANs.

**Changes:**
- `acp-lib/src/tls.rs`: Add `sign_server_cert(sans: &[String])` method
  - Parse SAN strings like `DNS:localhost`, `IP:127.0.0.1`
  - Generate cert with key usages for server auth
  - Return (cert_der, key_der) tuple

**Tests:**
- Unit test: generate server cert with various SANs
- Unit test: verify generated cert is signed by CA

### Phase 2: Server HTTPS Support

Modify server to serve management API over HTTPS.

**Changes:**
- `acp-server/src/main.rs`:
  - Add `load_or_generate_mgmt_cert()` function
  - Create `RustlsConfig` from mgmt cert/key
  - Use `axum_server::bind_rustls()` instead of `axum::serve()`
  - Store TLS config in `Arc<ArcSwap<...>>` for hot-swap

**New dependencies:**
- `axum-server = { version = "0.7", features = ["tls-rustls"] }` in acp-server

**Storage keys:**
- `mgmt:cert` - Management API certificate (PEM)
- `mgmt:key` - Management API private key (PEM)
- `mgmt:sans` - Configured SANs (JSON array, for regeneration reference)

### Phase 3: Init Command Updates

Update `/init` endpoint and CLI to handle management cert generation.

**Changes:**
- `acp-server/src/api.rs`: Extend `init()` handler
  - Accept optional `management_sans` in request body
  - Generate management cert with specified or default SANs
  - Store cert/key in SecretStore

- `acp/src/commands/init.rs`: Add `--management-sans` argument
  - Parse comma-separated SANs
  - Pass to `/init` endpoint

**Default SANs:**
```
DNS:localhost
IP:127.0.0.1
IP:::1
```

### Phase 4: CLI HTTPS Client

Configure CLI to connect over HTTPS with CA verification.

**Changes:**
- `acp/Cargo.toml`: Add `rustls-tls` feature to reqwest
- `acp/src/client.rs`:
  - Accept CA cert path in constructor
  - Build reqwest client with custom root certificate
  - Change default URL scheme to `https://`

- `acp/src/main.rs`:
  - Load CA cert from `~/.config/acp/ca.crt`
  - Pass to ApiClient constructor
  - Update default server URL to `https://localhost:9080`

### Phase 5: Cert Rotation Endpoint

Add endpoint and CLI command for live cert rotation.

**Changes:**
- `acp-server/src/api.rs`: Add `POST /v1/management-cert` handler
  - Validate auth
  - Parse SANs from request body
  - Generate new cert using CA
  - Store new cert/key
  - Hot-swap TLS acceptor via `ArcSwap`

- `acp/src/commands/mod.rs`: Add `new_management_cert` command
- `acp/src/commands/new_management_cert.rs`: New file
  - Accept `--sans` argument
  - Call `/v1/management-cert` endpoint

### Phase 6: Test Updates

Update existing tests and add new ones.

**Changes:**
- Update e2e tests to use HTTPS
- Add integration test for cert rotation
- Update smoke tests to handle HTTPS

## Parallel Opportunities

Within Phase 2-4, some work can be parallelized:
- Server HTTPS support (Phase 2) and CLI HTTPS client (Phase 4) are independent
- Init command updates (Phase 3) depends on Phase 1 only

Suggested parallel execution:
```
Phase 1 (serial - foundation)
    │
    ├── Phase 2 (server HTTPS)
    │
    └── Phase 3 (init updates) ──► Phase 4 (CLI HTTPS)
                                        │
                                        ▼
                                   Phase 5 (rotation)
                                        │
                                        ▼
                                   Phase 6 (tests)
```

## Risks

1. **axum-server compatibility** - Need to verify `axum-server` works with current axum version
2. **Hot-swap complexity** - TLS acceptor hot-swap may have edge cases with in-flight connections
3. **Backwards compatibility** - Existing users will need to re-init after upgrade (cert doesn't exist)

## Migration Path

For existing installations:
1. Server starts, detects no mgmt cert exists
2. Server refuses to start with clear error message
3. User runs `acp init` again (this regenerates CA, which is expected for security upgrade)

Alternative: Add `--migrate-https` flag that generates mgmt cert using existing CA without full re-init.
