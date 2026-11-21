# Contributing to AsterYggdrasil

## Community Expectations

Please read and follow the [Code of Conduct](CODE_OF_CONDUCT.md) before participating in issues, pull requests, discussions, and review threads.

## Getting Started

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/<your-github-username>/AsterYggdrasil.git
   cd AsterYggdrasil
   git remote add upstream https://github.com/AsterCommunity/AsterYggdrasil.git
   ```
3. Build and run:
   ```bash
   # Frontend
   cd frontend-panel && bun install --frozen-lockfile && bun run build && cd ..

   # Backend
   cargo run
   ```

## Development Workflow

### AI-Assisted Contributions

AsterCommunity welcomes responsible use of AI-assisted development tools. You may use them to read code, learn the architecture, explore an unfamiliar subsystem, draft an implementation, write tests, or review a change. We consider using these tools to understand the project better a productive part of contributing.

The contributor remains responsible for every submitted change, regardless of which tools helped produce it. Before opening a pull request, you must:

- understand the code you submit and be able to explain its behavior and architectural fit;
- review generated suggestions against the current code, project contracts, and authoritative external specifications — for protocol work, the authlib-injector wiki and Yggdrasil protocol documents are the authoritative sources;
- add and run tests that cover the changed behavior, including relevant failure, boundary, rollback, concurrency, permission, protocol, and compatibility cases;
- verify that the change does not expose credentials, personal data, proprietary material, or content with incompatible licensing;
- accurately report what was tested, what was not tested, and any remaining limitations;
- address review feedback and maintain the contribution just as you would for code written without AI assistance.

AI assistance does not lower the bar for correctness, security, maintainability, or test coverage. A generated patch without demonstrated understanding and appropriate verification is not ready to merge. The person submitting the pull request—not the tool—is accountable for the result.

### Branch Naming

- `feat/<description>` - New features
- `fix/<description>` - Bug fixes
- `refactor/<description>` - Refactoring
- `docs/<description>` - Documentation

### Commit Messages

Use conventional commits:

```
feat(yggdrasil): add session-forward texture URL rewriting
fix(auth): handle expired refresh token correctly
refactor(texture): split upload pipeline into modules
docs: update launcher setup guide
```

### Before Submitting a PR

```bash
# Backend checks
cargo fmt --all -- --check
cargo check
cargo test --workspace --lib
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Frontend checks
cd frontend-panel
bun run check
bun run build
```

Prefer a targeted `cargo test --lib <filter>` or `cargo test --test <name> <filter>` while iterating; run a
broader suite when the change crosses service, database, texture storage, or protocol boundaries.
For PostgreSQL or MySQL coverage, use the shared container suites rather than switching runners:

```bash
ASTER_TEST_DATABASE_BACKEND=postgres cargo test --test test_database_backends
ASTER_TEST_DATABASE_BACKEND=mysql cargo test --test test_database_backends
```

If an API surface or OpenAPI schema changes, also run:

```bash
cargo test --features openapi --test generate_openapi
cd frontend-panel
bun run generate-api
```

## Project Conventions

### Error System (Two Layers)

- **Internal**: `AsterError` variants expose `E100`-style internal codes for logs and debugging
- **API**: `AsterErrorCode` exposes stable string wire codes such as `success`, `auth.credentials_failed`, and `minecraft_texture.storage_failed`

### API Response Format

Serialization is delegated to Forge's validated envelope (`ForgeApiResponse`) through the transparent
adapter in `src/api/response.rs`; constructors reject success-code/error-data and failure-code/success-data
mismatches at compile time.

```json
{ "code": "success", "msg": "", "data": { ... } }
{ "code": "auth.credentials_failed", "msg": "Invalid Credentials", "error": { "retryable": false } }
```

### State Traits

- Service functions take a single `state: &impl SharedRuntimeState` (or a narrower capability trait such as `MailRuntimeState`); capability traits compose via supertraits in `src/runtime/state_traits.rs`
- Test states come from `runtime::test_support` fixtures, not hand-assembled `AppStateParts`

### Workspace Layout

- `aster_yggdrasil` — application shell: API routes, services, runtime assembly
- `aster_yggdrasil_model` — SeaORM entities and domain types, no business logic, never depends on the application crate
- `aster_yggdrasil_migration` — database migrations with a standalone CLI binary
- `aster_yggdrasil_render` — skin preview rendering

### Service Module Layout

- Subdomain modules follow the AsterDrive layout: `services/auth_service/{login,registration,session,tokens,...}`, `services/mail/{audit,outbox,sender,template}`
- Service files stay facade-first: `mod.rs` declares submodules and re-exports the public surface, so `crate::services::auth_service::login` paths remain stable across refactors

### Frontend Conventions

- Type checking: TypeScript 7 native `tsgo` with incremental project caches
- Linting: `biome`, not ESLint
- No TS enums (`erasableSyntaxOnly`), use `as const` objects
- Type imports must use `import type` (`verbatimModuleSyntax`)
- Frontend types in `src/types/api.ts` derive from `src/types/api.generated.ts`, which is regenerated from the backend OpenAPI document — never hand-edit the generated file

## Architecture

See the [developer documentation](developer-docs/README.md) for the current module and runtime boundaries, and the [AsterForge documentation](https://forge.docs.astercosm.com/) for the shared runtime mechanics this project consumes.

## License

By contributing, you agree that your contributions will be licensed under either the Apache License, Version 2.0 or the MIT license, at the recipient's option.
