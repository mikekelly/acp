# Rename ACP to GAP - Code Changes

## Context
The directories have already been renamed via git mv:
- acp/ → gap/
- acp-server/ → gap-server/
- acp-lib/ → gap-lib/

Now update all Cargo.toml files and source code to reflect the new names.

## Tasks

### 1. Update /Users/mike/code/agent-credential-proxy/Cargo.toml
- Line 2: `members = ["acp", "acp-server", "acp-lib"]` → `members = ["gap", "gap-server", "gap-lib"]`
- Line 37: `acp-lib = { path = "acp-lib" }` → `gap-lib = { path = "gap-lib" }`

### 2. Update /Users/mike/code/agent-credential-proxy/gap/Cargo.toml
- Line 2: `name = "acp"` → `name = "gap"`
- Line 10: `name = "acp"` → `name = "gap"`
- Line 14: `acp-lib.workspace = true` → `gap-lib.workspace = true`

### 3. Update /Users/mike/code/agent-credential-proxy/gap-server/Cargo.toml
- Line 2: `name = "acp-server"` → `name = "gap-server"`
- Line 10: `name = "acp-server"` → `name = "gap-server"`
- Line 14: `acp-lib.workspace = true` → `gap-lib.workspace = true`

### 4. Update /Users/mike/code/agent-credential-proxy/gap-lib/Cargo.toml
- Line 2: `name = "acp-lib"` → `name = "gap-lib"`

### 5. Update all import statements
Replace `use acp_lib::` with `use gap_lib::` in these files:
- gap-server/src/api.rs
- gap-server/src/main.rs
- gap/src/client.rs
- gap-lib/tests/e2e_integration_test.rs
- gap-lib/tests/proxy_transform_integration_test.rs
- gap-lib/tests/proxy_plugin_integration_test.rs
- gap-lib/examples/verify_cert.rs
- gap-lib/tests/integration_test.rs

Also replace any standalone `acp_lib::` references (without `use`) in the same files.

### 6. Update internal prefixes
In gap-lib/src/plugin_runtime.rs, replace `__acp_native_` with `__gap_native_` (all occurrences).

## Success Criteria
- All Cargo.toml files updated with new crate names
- All imports compile
- No references to `acp_lib` remain in code
- No references to `__acp_native_` remain
- Do NOT run cargo build or cargo test (that will be done in a separate verification step)
