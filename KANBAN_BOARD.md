# Project Kanban

## Ideas
<!-- Raw thoughts, not yet evaluated -->

## Designed
<!-- Has clear outcomes/spec -->

## Ready
<!-- Designed + planned, can be picked up -->

### Phase 1: Foundation
Project skeleton with core types, error handling, Cargo workspace.
See: `docs/implementation-plan.md#phase-1-foundation`

### Phase 2: Secure Storage
SecretStore trait + Keychain (macOS) + File (Linux) implementations.
See: `docs/implementation-plan.md#phase-2-secure-storage`

### Phase 3: TLS Infrastructure
CA generation, dynamic cert signing for MITM proxy.
See: `docs/implementation-plan.md#phase-3-tls-infrastructure`

### Phase 4: Proxy Core
MITM proxy that forwards HTTPS requests with agent auth.
See: `docs/implementation-plan.md#phase-4-proxy-core`

### Phase 5: Plugin Runtime
Boa JS engine with sandboxed globals for request transforms.
See: `docs/implementation-plan.md#phase-5-plugin-runtime`

### Phase 6: Management API
HTTP API for CLI (plugins, credentials, tokens, activity).
See: `docs/implementation-plan.md#phase-6-management-api`

### Phase 7: CLI
Full command-line interface with secure password input.
See: `docs/implementation-plan.md#phase-7-cli`

### Phase 8: Integration & Polish
Bundled plugins, e2e tests, install scripts, docs.
See: `docs/implementation-plan.md#phase-8-integration--polish`

## In Progress
<!-- Currently being worked on -->

## Done
<!-- Shipped — archive periodically -->

