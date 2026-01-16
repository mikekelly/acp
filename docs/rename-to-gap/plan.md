# Rename to GAP (Gated Agent Proxy)

## Why

"ACP" conflicts with [Agent Client Protocol](https://agentclientprotocol.com/). GAP avoids this and enables natural terminology ("gapped tools", "gated MCP").

## Scope

This is a **breaking change** affecting:

- Package/binary names: `acp` → `gap`, `acp-server` → `gap-server`, `acp-lib` → `gap-lib`
- Directory structure: `acp/`, `acp-server/`, `acp-lib/` directories
- Environment variables: `ACP_*` → `GAP_*` (15+ variables)
- Data paths: `~/.config/acp/` → `~/.config/gap/`
- Token format: `acp_*` → `gap_*`
- Docker: images, volumes, service names
- macOS GUI: Xcode project, bundle identifier, Swift files
- Homebrew tap and formula
- All documentation and tests

## Approach

### Phase 1: Core Rust Crates
Rename the Rust packages and directories. This is the foundation everything else depends on.

1. Rename directories: `acp/` → `gap/`, `acp-server/` → `gap-server/`, `acp-lib/` → `gap-lib/`
2. Update all `Cargo.toml` files (package names, binary names, dependencies)
3. Update all `use acp_lib::` imports to `use gap_lib::`
4. Update internal prefixes (`__acp_native_*` → `__gap_native_*`)
5. Verify build passes

### Phase 2: Environment Variables & Paths
Update all environment variable references and default paths.

1. Rename all `ACP_*` env vars to `GAP_*` in code
2. Update default data directory from `acp` to `gap`
3. Update CA certificate paths
4. Update logging prefixes (`RUST_LOG=acp_server` → `gap_server`)

### Phase 3: Token Format
Update the token prefix.

1. Change token generation from `acp_*` to `gap_*`
2. Update token validation
3. Update all test fixtures

### Phase 4: Docker & Distribution
Update containerization and distribution artifacts.

1. Update Dockerfile (user/group, paths)
2. Update docker-compose.yml (service names, volumes, networks)
3. Update docker-entrypoint.sh
4. Update install.sh script
5. Update archive naming in release process

### Phase 5: macOS GUI
Rename the Xcode project and all Swift code.

1. Rename `ACP.xcodeproj` → `GAP.xcodeproj`
2. Rename `macos-gui/ACP/` → `macos-gui/GAP/`
3. Rename Swift files (`ACPApp.swift` → `GAPApp.swift`, etc.)
4. Update bundle identifier
5. Update all internal references in project.pbxproj

### Phase 6: Documentation & Tests
Update all docs and test files.

1. Update README.md
2. Update AGENT_ORIENTATION.md
3. Update all docs/ files
4. Update smoke test scripts and docs
5. Update integration test service names

### Phase 7: External Ecosystem (Post-Release)
These happen after the main rename ships.

1. Rename GitHub repository
2. Update Homebrew tap and formula
3. Publish migration guide
4. Update any external plugin examples

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| **Existing tokens invalidated** | Document as breaking change. No migration path—users regenerate tokens. |
| **Data directory migration** | Document manual migration: `mv ~/.config/acp ~/.config/gap` |
| **Docker volume migration** | Document in release notes. Users backup and recreate. |
| **Keychain entries** | Old entries orphaned. Users re-add credentials after upgrade. |
| **External plugins break** | Plugin authors update independently. Consider grace period. |

## Testing Strategy

1. After each phase, run `cargo build` and `cargo test`
2. After Phase 4, run smoke tests
3. After Phase 5, build and test macOS GUI
4. Final full smoke test before release

## Not In Scope

- Backward compatibility shims (clean break)
- Automated migration tooling (manual migration documented)
- Supporting both names during transition
