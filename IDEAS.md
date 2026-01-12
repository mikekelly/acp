# Ideas

Raw thoughts, not yet evaluated.

- In memory SurrealDB that flushes to disk ecrypted using secret stored in keychain (might be possible with SurrealDB + RocksDB)
- Policy plugins (rate limiting, rogue agent detection, etc)
- Transparent proxy mode (capture and forward all HTTP(S) traffic) related: SOCKS proxy
    - This is likely dependent on in memory database for recording and analysing activity (occasional flush to disk)
- Plugins that can initiate and handle OAuth dance when installed
- Plugin authoring guide documentation
- Drop-in wrappers for common http libraries that handle ACP proxying transparently (exa-mcp-server fork would be a good test candidate)
