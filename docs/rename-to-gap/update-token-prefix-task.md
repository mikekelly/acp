# Task: Update Token Prefix from acp_ to gap_

## Context
This is part of the ongoing project rename from ACP to GAP. Phases 1-2 are complete (crates, env vars). This task updates the token format.

## Objective
Change all token generation and validation from `acp_*` prefix to `gap_*` prefix.

## Files Containing acp_
Based on grep search, these files contain `acp_`:
- gap-lib/src/types.rs
- gap-lib/src/error.rs
- gap-lib/src/registry.rs
- gap-lib/src/plugin_matcher.rs
- gap-lib/src/plugin_runtime.rs
- gap-server/src/api.rs
- gap-server/src/launchd.rs
- gap-lib/tests/integration_test.rs
- gap-lib/tests/proxy_plugin_integration_test.rs
- gap-lib/tests/proxy_transform_integration_test.rs
- gap-lib/tests/e2e_integration_test.rs

## Implementation Steps

1. **Orient**: Read AGENT_ORIENTATION.md and examine token-related code in gap-lib/src/types.rs (likely location of AgentToken type and generation)

2. **Baseline**: Run full test suite to ensure all tests pass before changes

3. **RED**: Write failing test(s) that expect `gap_` prefix tokens

4. **GREEN**: Update token generation code to use `gap_` prefix instead of `acp_`

5. **Update validation**: Ensure token validation accepts `gap_` prefix

6. **Update test fixtures**: Search and replace all hardcoded `acp_*` tokens in test files

7. **REFACTOR**: Clean up any related code

8. **Verify**: Run full test suite again to ensure everything passes

9. **Commit**: Commit all changes with message describing the token prefix update

## Success Criteria
- All generated tokens use `gap_` prefix
- Token validation accepts `gap_` tokens
- All test fixtures updated
- Full test suite passes (`cargo test`)
- Build succeeds (`cargo build`)
- Changes committed

## Notes
- This is a straightforward find-and-replace in most cases
- Pay special attention to validation logic that might check prefix format
- Check for any documentation or comments referencing the old prefix
