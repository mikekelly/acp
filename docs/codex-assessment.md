## Assessment
- The product story and threat model are clear and strong: HTTPS-only proxy, MITM with local CA, explicit agent tokens, and write-only credential flow align with the "agents never see secrets" guarantee.
- The macOS distribution path looks solid: signed native app + Data Protection Keychain reduces friction and addresses prior keychain prompt pain.
- Architecture separation remains a strength: `gap-lib` handles core proxy/TLS/plugins, `gap-server` is the daemon/API, and `gap` is a thin CLI.
- The system is still sharp-edged in a few places where behavior and docs diverge, and where defaults could be tightened for security/clarity.

## Key Risks / Gaps
- Allowlist enforcement gap: The proxy still passes through non-matching hosts. `gap-lib/src/proxy_transforms.rs` returns original bytes when no plugin matches, so `gap-lib/src/proxy.rs` will tunnel to any host. If the intent is "only plugin-declared hosts," this is the biggest behavioral mismatch.
- Token strength: `AgentToken::new` in `gap-lib/src/types.rs` uses a timestamp-based token. Tokens are still gatekeepers, so predictability is a real local-attacker risk.
- Registry as single blob: Credentials and tokens appear to live inside `_registry` (`gap-lib/src/registry.rs`), which increases blast radius and makes concurrent updates prone to lost writes without locking.
- Token exposure in list: `list_tokens` returns full token value in `id` (`gap-server/src/api.rs`), while docs still say list only shows prefix.
- Keep-alive behavior: Only the first request on a tunnel is transformed; subsequent requests are passed through unmodified (`gap-lib/src/proxy.rs`), which can break auth on keep-alive clients.

## Security/Trust Observations
- Plugin code is a trusted supply chain. It is fetched and executed without signature verification (`gap-server/src/api.rs`, `gap-lib/src/plugin_runtime.rs`). The sandbox limits obvious exfil but does not make plugins untrusted.
- Management API binds `0.0.0.0` in `gap-server/src/main.rs`. With HTTPS this might be acceptable, but it is a notable default exposure on dev machines.

## Testing & Reliability
- TLS/proxy integration coverage is good (`gap-lib/tests/proxy_tls_integration_test.rs`, `gap-lib/tests/proxy_plugin_integration_test.rs`), but a key end-to-end auth test remains TODO (`gap-lib/tests/e2e_integration_test.rs`).
- HTTPS management is now default, but `create_api_client` keeps an HTTP fallback (`gap/src/main.rs`), which could be confusing if the server no longer listens over HTTP.

## Doc Drift / Consistency
- `docs/reference/architecture.md` still describes per-key credential/token storage and "prefix-only" token lists. Current code differs on both counts.
- CLI default in docs still references `http://localhost:9080` in places; actual CLI default is `https://localhost:9080` (`gap/src/main.rs`).
