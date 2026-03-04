# Rust Primitives Boundary

This project is frontend-first. Rust is used only for trust/system primitives.

## Keep in Rust

- Filesystem and persistence
  - profile metadata file reads/writes
  - plugin file reads/list/delete
  - zip import/export
  - cached BepInEx archive read/write/download/extract
- Native OS APIs
  - process launch and lifecycle hooks
  - Xbox shell/app-id operations
  - platform detection on host
- Secrets and secure auth
  - Epic auth flows/session/token handling
- System integration
  - updater hooks and long-running progress events

## Keep in Frontend

- Workflow orchestration
  - dependency resolution and version policy
  - mod install sequencing and rollback strategy
  - unified mod composition and cleanup policy
  - launch flow branching and close-on-launch behavior
- UI state, cache, and reactive behavior
- UX validation and immediate feedback
- External API requests (direct `fetch` + runtime validation)

## Rules

1. Start features in frontend.
2. Move logic to Rust only when it requires filesystem/native APIs, secrets, or heavy computation.
3. Do not add new Rust workflow commands when existing primitives are sufficient.
4. Keep `@tauri-apps/api/core` imports confined to `src/lib/infra/rust`.
