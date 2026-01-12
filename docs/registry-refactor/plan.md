# Registry Simplification Refactor

## Goal

Simplify the registry data model by:
1. Converting `tokens` from array to hash (keyed by token value)
2. Converting `credentials` from metadata array to hash with actual values
3. Eliminating redundant separate storage entries

## Current State

```json
{
  "version": 1,
  "tokens": [{"token_value": "acp_19bb...", "name": "test-agent", "created_at": "..."}],
  "plugins": [{"name": "exa", "hosts": ["api.exa.ai"], "credential_schema": ["api_key"]}],
  "credentials": [{"plugin": "exa", "field": "api_key"}],
  "password_hash": "..."
}
```

Plus separate storage entries:
- `token:{value}` - Full AgentToken with redundant id/prefix/token fields
- `credential:{plugin}:{field}` - Actual secret values

## Target State

```json
{
  "version": 1,
  "tokens": {
    "acp_19bb...": {"name": "test-agent", "created_at": "..."}
  },
  "plugins": [{"name": "exa", "hosts": ["api.exa.ai"], "credential_schema": ["api_key"]}],
  "credentials": {
    "exa": {"api_key": "actual-secret-value"}
  },
  "password_hash": "..."
}
```

No separate entries for tokens or credentials.

**No migration.** Old storage format is not supported - users must reinitialize. Project doesn't have widespread use.

## Changes Required

### Phase 1: Data Structures (`acp-lib/src/registry.rs`)

**TokenEntry** (lines 12-21)
- Remove `token_value` field (becomes the hash key)
- Keep only `name` and `created_at`

**CredentialEntry** (lines 36-42)
- Remove entirely - no longer needed as separate struct

**RegistryData** (lines 47-64)
- `tokens: Vec<TokenEntry>` → `tokens: HashMap<String, TokenMetadata>`
- `credentials: Vec<CredentialEntry>` → `credentials: HashMap<String, HashMap<String, String>>`
- Keep `version` at 1 (no migration support)

**New struct: TokenMetadata**
```rust
pub struct TokenMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
}
```

### Phase 2: Registry Methods (`acp-lib/src/registry.rs`)

**add_token** (lines 116-123)
- Change signature: `add_token(&self, token_value: &str, metadata: &TokenMetadata)`
- Insert into hash instead of push to vec

**remove_token** (lines 125-132)
- Keep signature: `remove_token(&self, token_value: &str)`
- Remove from hash instead of filter vec

**list_tokens** (lines 134-137)
- Return `HashMap<String, TokenMetadata>` or iterator
- Callers need updating

**add_credential** (lines 172-179)
- Change signature: `set_credential(&self, plugin: &str, field: &str, value: &str)`
- No separate storage write needed

**remove_credential** (lines 181-189)
- Keep signature: `remove_credential(&self, plugin: &str, field: &str)`
- Remove from nested hash

**list_credentials** (lines 191-194)
- Return `&HashMap<String, HashMap<String, String>>` or similar
- Or add `get_credential(&self, plugin: &str, field: &str) -> Option<String>`

### Phase 3: Token Validation (`acp-lib/src/proxy.rs`)

**validate_auth** (lines 216-257)
- Line 235: Change from `registry.list_tokens()` to hash lookup
- Use `registry.get_token(token_value)` for O(1) lookup
- Simplifies the find logic (lines 237-240)

### Phase 4: Credential Loading (`acp-lib/src/proxy_transforms.rs`)

**load_plugin_credentials** (lines 18-45)
- Simplify significantly - no separate storage reads
- Get credentials directly from registry: `registry.get_credentials(plugin_name)`
- Remove storage parameter (line 19)

### Phase 5: API Endpoints (`acp-server/src/api.rs`)

**POST /tokens/create** (lines 583-624)
- Remove: Store at `token:{value}` (line 595-599)
- Change: Use new `add_token(value, metadata)` method

**DELETE /tokens/:id** (lines 627-661)
- Remove: Delete from storage (line 652)
- Keep: Remove from registry

**POST /credentials/:plugin/:key** (lines 664-698)
- Remove: Store at `credential:{plugin}:{key}` (line 674)
- Change: Use new `set_credential(plugin, field, value)` method
- Remove: Duplicate prevention logic (line 681) - hash handles this

**DELETE /credentials/:plugin/:key** (lines 701-721)
- Remove: Delete from storage (line 709)
- Keep: Remove from registry

### Phase 6: Cleanup

**Remove AgentToken struct** (`acp-lib/src/types.rs`)
- No longer needed for storage
- May keep for API response formatting

**Remove separate storage key patterns**
- No more `token:{value}` keys
- No more `credential:{plugin}:{field}` keys

**Update tests**
- Registry tests in `acp-lib/src/registry.rs` (lines 400+)
- API tests in `acp-server/src/api.rs`
- Integration tests in `acp-lib/tests/`

## Files to Modify

| File | Changes |
|------|---------|
| `acp-lib/src/registry.rs` | Data structures, all methods, tests |
| `acp-lib/src/proxy.rs` | `validate_auth` token lookup |
| `acp-lib/src/proxy_transforms.rs` | `load_plugin_credentials` |
| `acp-server/src/api.rs` | Token and credential endpoints |
| `acp-lib/src/types.rs` | Possibly remove AgentToken |
| `acp-lib/tests/*.rs` | Update integration tests |

## Risks

| Risk | Mitigation |
|------|------------|
| Breaking existing keychain data | Accepted - users reinitialize (no widespread use) |
| Missing a call site | Compiler will catch type mismatches |

## Acceptance Criteria

- [ ] Registry stores tokens as hash, keyed by token value
- [ ] Registry stores credentials as nested hash with actual values
- [ ] No separate `token:{value}` entries in storage
- [ ] No separate `credential:{plugin}:{field}` entries in storage
- [ ] All existing tests pass (updated as needed)
- [ ] Token validation still works
- [ ] Credential injection still works
