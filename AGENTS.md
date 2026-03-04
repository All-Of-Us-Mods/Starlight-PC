# Agents Guide

> Starlight — Among Us mod manager built with Tauri 2 + SvelteKit

## Stack

| Layer    | Technology                                              |
| -------- | ------------------------------------------------------- |
| Runtime  | Tauri 2.x (Rust backend) + SvelteKit (static adapter)   |
| Frontend | Svelte 5 (runes), TypeScript 5, Tailwind CSS 4          |
| UI       | shadcn-svelte, bits-ui, tailwind-variants, Lucide icons |
| Data     | TanStack Query, arktype (runtime validation)            |
| Package  | bun                                                     |

## Commands

```bash
# Quality
bun lint                   # oxlint (JS/TS/Svelte) + ESLint (Svelte-specific rules only)
bun format                 # oxfmt (JS/TS/JSON/CSS/etc.) + Prettier (.svelte files only)
bun check                  # svelte-check (type checking)

# Rust (run from src-tauri/)
cargo check                # Check Rust code
cargo clippy               # Lint Rust code
cargo fmt                  # Format Rust code
```

## Architecture Boundary

1. Start features in frontend.
2. Move logic to Rust only when it requires filesystem/native APIs, secrets, or heavy computation.
3. Do not add new Rust workflow commands when existing primitives are sufficient.
4. Keep `@tauri-apps/api/core` imports confined to `src/lib/infra/rust`.
