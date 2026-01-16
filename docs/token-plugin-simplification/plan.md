# Token and Plugin Simplification

## Overview

Remove unnecessary complexity from token storage and plugin matching. These are platform-agnostic improvements that apply to both macOS and Linux.

## Problem 1: Unnecessary Token Indirection

### Current State
- Tokens stored as `token:{id}` → `{ id, name, token_value, created_at }`
- TokenCache loads ALL tokens into HashMap keyed by token value
- Cache needed because lookup is by value but storage is by ID
- Cache creates complexity and potential staleness issues

### Better Approach
- Store tokens as `token:{token_value}` → `{ name, created_at }`
- The token value IS the ID (no separate ID needed)
- Direct lookup: `store.get("token:gap_xxxx")` — one read, no cache
- Remove TokenCache entirely

### Why the Cache Existed
The cache was solving a data model problem, not a performance problem:
1. Proxy receives bearer token value (e.g., `gap_xxxx`)
2. Tokens stored by ID, not by value
3. To find a token by value, must load all tokens and search
4. Cache avoided repeating this O(N) search

Storing by value eliminates the problem entirely.

---

## Problem 2: Inefficient Plugin Matching

### Current State (`plugin_matcher.rs`)
```rust
for entry in plugin_entries {
    // For EVERY plugin on EVERY request:
    let plugin_code = store.get(&key).await?;           // Load code
    let mut runtime = PluginRuntime::new()?;            // Create JS runtime
    runtime.load_plugin_from_code(&entry.name, &code);  // Parse JavaScript
    if plugin.matches_host(host) { ... }                // Check match
}
```

This is O(N) JavaScript executions per request where N = number of plugins.

### Better Approach
- Registry already stores `hosts` in `PluginEntry`
- Match against `PluginEntry.hosts` directly (cheap string matching)
- Only load plugin code for the ONE that matched
- One JS parse per request (for the matched plugin), not N

---

## Changes Required

### Remove TokenCache
- Delete `gap-lib/src/token_cache.rs`
- Update `gap-lib/src/lib.rs` to remove export
- Update `gap-server/src/main.rs` to use direct storage lookup
- Update `gap-lib/src/proxy.rs` to look up tokens directly
- Update `gap-server/src/api.rs` token endpoints

### Change Token Storage Schema
- Store: `token:{token_value}` → `{ name, created_at }`
- Remove `id` field from token struct (value is the ID)
- Migration: read old format, write new format on first access
- Update Registry token entries to use value as key

### Fix Plugin Matching
- Update `plugin_matcher.rs`:
  - Match against `PluginEntry.hosts` patterns
  - Only load plugin code after finding a match
  - Remove unnecessary PluginRuntime creation for non-matches
- Add host pattern matching utility (exact match, wildcard support)

---

## Migration Strategy

### Tokens
1. On token lookup, try new format first: `token:{value}`
2. If not found, try old format: load all `token:{id}`, find matching value
3. If found in old format, write to new format, delete old key
4. Gradual migration as tokens are accessed

### Registry
- Registry metadata may reference tokens by old ID
- Update to reference by value instead
- Or: just regenerate registry on upgrade (tokens are the source of truth)

---

## Testing

- All existing tests must pass
- Add test: direct token lookup by value
- Add test: token migration from old to new format
- Add test: plugin matching uses Registry metadata, not JS parsing
- Add test: plugin code only loaded for matched plugin
- Performance test: measure request latency before/after

---

## Documentation Updates

After implementation:
- Update AGENT_ORIENTATION.md to remove TokenCache references
- Update any architecture docs that mention caching

---

## Acceptance Criteria
- [ ] TokenCache removed from codebase
- [ ] Tokens stored by value, not by separate ID
- [ ] Old token format auto-migrated on access
- [ ] Plugin matching uses Registry metadata for host matching
- [ ] Plugin code only loaded for matched plugin
- [ ] All existing tests pass
- [ ] No regression in proxy request handling

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Token schema migration breaks existing setups | Users locked out | Auto-migrate on first read, keep backwards compat temporarily |
| Plugin matching logic differs from JS-based matching | Some plugins don't match correctly | Port exact matching logic from PluginRuntime, comprehensive tests |
