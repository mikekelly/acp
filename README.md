# Agent Credential Proxy (ACP)

Secure credential management for AI agents via transparent MITM proxy with JavaScript plugins.

## Project Status

**Phase 1 Complete:** Foundation with core types and workspace structure.

See `docs/implementation-plan.md` for full roadmap.

## Structure

This is a Cargo workspace with three crates:

- **`acp-lib`** - Shared library with core types, error handling, and common logic
- **`acp`** - CLI for managing the proxy (init, plugins, credentials, tokens)
- **`acp-server`** - Daemon running the proxy and management API

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy --all-targets --all-features
```

### Run

```bash
# CLI (placeholder)
cargo run --bin acp -- --help

# Server (placeholder)
cargo run --bin acp-server -- --help
```

## Architecture

Core types defined in `acp-lib/src/types.rs`:

- **`ACPRequest`** - HTTP request representation
- **`ACPCredentials`** - Plugin credential key-value store
- **`ACPPlugin`** - JavaScript plugin definition with host matching
- **`AgentToken`** - Bearer token for agent authentication
- **`Config`** - Runtime configuration

Error handling in `acp-lib/src/error.rs` provides a unified `AcpError` type.

## What's Next

See `docs/implementation-plan.md` for the full implementation plan.

**Phase 2:** Secure storage (Keychain for macOS, file-based for Linux)

## License

MIT
