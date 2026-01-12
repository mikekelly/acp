# HTTPS for Management API

## Why

Defense in depth. The CLI currently communicates with the management API over plain HTTP on localhost. While passwords are SHA512-hashed before transmission, HTTPS prevents network observers from seeing even the hashed secrets. This closes a gap in the security model.

## What Success Looks Like

1. **CLI → management API communication is encrypted** using TLS with the existing CA infrastructure
2. **Zero-config for localhost** - default SANs (localhost, 127.0.0.1, ::1) work out of the box
3. **Configurable SANs at init time** - users can specify additional SANs for remote management scenarios
4. **Live cert rotation** - management cert can be regenerated without server restart

## Acceptance Criteria

### Init generates management certificate
- [ ] `acp init` generates a management API cert/key signed by the CA
- [ ] Default SANs: `DNS:localhost`, `IP:127.0.0.1`, `IP:::1`
- [ ] `acp init --management-sans "DNS:localhost,DNS:example.com,IP:192.168.1.1"` overrides defaults
- [ ] Cert/key stored securely (same storage as CA - keychain on macOS, encrypted file on Linux)

### Server serves HTTPS
- [ ] Management API serves HTTPS instead of HTTP
- [ ] Server loads management cert/key from storage on startup
- [ ] Server logs indicate HTTPS is active

### CLI verifies server certificate
- [ ] CLI configures reqwest to trust the CA certificate
- [ ] CLI connects to `https://localhost:9080` by default
- [ ] Connection fails if server cert is not signed by trusted CA (no silent downgrade)

### Live certificate rotation
- [ ] New endpoint: `POST /v1/management-cert` accepts desired SANs
- [ ] Server regenerates cert using its CA, stores it, and hot-swaps TLS config
- [ ] New CLI command: `acp new-management-cert --sans "..."` calls the endpoint
- [ ] Existing connections continue; new connections use new cert

## Out of Scope

- Client certificate authentication (mTLS)
- Certificate revocation
- Automatic cert renewal (management certs are long-lived, user-initiated rotation is sufficient)

## Constraints

- Must use existing CA infrastructure (no separate PKI)
- Must not break existing functionality (all current tests pass)
- CLI must fail closed (no fallback to HTTP if HTTPS fails)
