# Agent Orientation

## Purpose
Agent credential proxy — manages credential access for AI agents.

## Structure
- **Cargo workspace** with 3 crates:
  - `acp-lib` - Shared library (types, errors)
  - `acp` - CLI binary
  - `acp-server` - Server binary
- `docs/` — Architecture decisions and documentation
- See `README.md` for quick start

## Commands
```bash
cargo build          # Build all workspace members
cargo test           # Run all tests
cargo clippy         # Lint
cargo run --bin acp  # Run CLI
cargo run --bin acp-server  # Run server
```

## Core Types (acp-lib)
- `ACPRequest` - HTTP request with method, url, headers, body
- `ACPCredentials` - String key-value map for plugin credentials
- `ACPPlugin` - Plugin definition with host matching (supports wildcards like `*.s3.amazonaws.com`)
- `AgentToken` - Bearer token for agent authentication
- `Config` - Runtime configuration
- `AcpError` - Unified error type with context helpers

## Patterns
- **Wildcard host matching**: `*.example.com` matches `sub.example.com` but NOT `a.b.example.com` (single-level only)
- **Builder pattern**: All types use `.with_*()` methods for fluent construction
- **Error context**: Use `AcpError::storage("msg")` rather than `AcpError::Storage("msg".to_string())`

## Gotchas
- **Wildcard matching is single-level only**: The pattern `*.s3.amazonaws.com` matches `bucket.s3.amazonaws.com` but rejects both `s3.amazonaws.com` (no subdomain) and `evil.com.s3.amazonaws.com` (multiple levels)
- **Token serialization**: `AgentToken` uses `#[serde(skip_serializing)]` on the `token` field to prevent accidental exposure in JSON responses
