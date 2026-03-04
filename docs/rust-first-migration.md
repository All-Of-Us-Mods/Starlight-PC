# Rust-First Version Plan

This project can run with a strict split:

- Rust (`src-tauri/`): all domain logic, filesystem/network/process logic, validation, persistence, orchestration.
- TypeScript (`src/`): view state, routing, UI composition, command/query calls to Rust, and rendering-only formatting.

## What is already moved

- Settings persistence and settings mutation now run through Rust commands:
  - `core_get_settings`
  - `core_update_settings`
  - `core_get_app_data_dir`
  - `core_get_bepinex_cache_path`
  - `core_auto_detect_bepinex_architecture`
- TypeScript settings repository/service now act as thin adapters around these commands.

## Target architecture

## 1) Rust app core

Create a single backend domain layer with feature modules:

- `core/settings`: settings model, persistence, migration, validation.
- `core/profiles`: profile lifecycle, metadata I/O, icon management, migration from legacy store.
- `core/mods`: dependency resolution decisions, install plans, download orchestration.
- `core/launch`: game launch policy, runtime state transitions, post-launch cleanup.
- `core/news`: API fetch + cache + schema checks.
- `core/updates`: update availability + rollout policy.

Expose only stable command/query contracts from `commands/*`.

## 2) TypeScript UI boundary

Keep TS as:

- Svelte pages/components.
- Query/mutation hooks (TanStack Query).
- Tiny client wrappers over `invoke` and event listeners.
- Presentation-only derived state.

Remove TS responsibilities for:

- File/directory reads/writes.
- Business invariants (duplicate name checks, lifecycle transitions).
- Merge/migration logic.
- Network-fetch validation logic outside UI formatting.

## 3) Data contracts

- Define canonical Rust DTOs with `serde`.
- Keep field names aligned to existing TS schema (`snake_case`) to minimize churn.
- Use TS `arktype` only as runtime guard for backend responses at UI boundary.

## Remaining migration work by file group

- `src/lib/features/profiles/profile-repository.ts` -> move to Rust `core/profiles` commands.
- `src/lib/features/profiles/profile-workflow-service.ts` -> move orchestration to Rust `core/profiles` + `core/mods`.
- `src/lib/features/profiles/mod-install-service.ts` -> move install planning and sequencing to Rust.
- `src/lib/features/mods/ui/*controller.ts` -> keep only presentation logic; push install list/dependency rules to Rust.
- `src/lib/api/client.ts` and feature `queries.ts` -> optional move of data fetching to Rust for strict “all logic in Rust”.
- `src/lib/features/news/*` -> move fetch/validate/cache to Rust command.
- `src/lib/features/updates/*` -> move update decision logic to Rust.

## Phased rollout

1. Settings (done): command-backed Rust storage and architecture auto-detect.
2. Profiles CRUD + metadata migration: replace TS repository/workflow reads/writes.
3. Mod install planner + dependency resolution in Rust.
4. Launch orchestration + runtime state in Rust.
5. News/updates/network rules in Rust (optional but required for strict interpretation).
6. Remove obsolete TS service logic and plugin-fs/plugin-store usage from frontend.

## Definition of done for “all logic in Rust”

- No frontend use of `@tauri-apps/plugin-fs` or `@tauri-apps/plugin-store` for domain behavior.
- TS services contain only command invocations, event wiring, and UI messages.
- Domain tests are in Rust (`cargo test`) for settings/profiles/mods/launch workflows.
- Frontend tests validate rendering + interaction only.
