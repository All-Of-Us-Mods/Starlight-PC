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
# Development
bun dev                    # Vite dev server (frontend only)
bun tauri dev              # Full Tauri app with hot reload

# Build
bun build                  # Build frontend to /build
bun tauri build            # Build production Tauri app

# Quality
bun lint                   # ESLint + Prettier check
bun format                 # Prettier write
bun check                  # svelte-check (type checking)

# Rust (run from src-tauri/)
cargo check                # Check Rust code
cargo clippy               # Lint Rust code
cargo fmt                  # Format Rust code
cargo test                 # Run all tests
cargo test test_name       # Run single test by name
```

## Project Structure

```
src/
├── routes/                 # SvelteKit pages (+page.svelte, +layout.svelte)
├── lib/
│   ├── api/client.ts       # Fetch wrapper with arktype validation
│   ├── components/
│   │   ├── layout/         # AppShell, SideNav, TopBar
│   │   ├── shared/         # Reusable non-UI components
│   │   └── ui/             # shadcn-svelte primitives
│   ├── features/           # Domain modules
│   │   └── {feature}/
│   │       ├── components/ # Feature-specific components
│   │       ├── queries.ts  # TanStack Query options
│   │       └── schema.ts   # arktype schemas
│   ├── state/              # Svelte 5 reactive state
│   └── utils.ts            # cn(), type helpers
└── app.css                 # Tailwind + CSS variables

src-tauri/src/
├── lib.rs                  # Tauri app entry, plugin registration
├── commands/               # Tauri commands (mod.rs exports submodules)
└── utils/                  # Rust utilities (error.rs, download.rs, etc.)
```

## Code Style

### Formatting (Prettier)

- Tabs for indentation
- Single quotes
- No trailing commas
- 100 char print width

### TypeScript/Svelte

- Use `$lib/` alias for all imports from `src/lib/`
- Use Svelte 5 runes: `$state`, `$derived`, `$props`, `$effect`
- Never use `any` without `eslint-disable` comment and justification
- Validate all external data with arktype schemas

### Svelte 5 Component Pattern

```svelte
<script lang="ts" module>
	// Module-level: exports, types, variants
	import { tv, type VariantProps } from 'tailwind-variants';
	export const variants = tv({ base: '...', variants: { size: { sm: '...', md: '...' } } });
	export type Props = { size?: VariantProps<typeof variants>['size'] };
</script>

<script lang="ts">
	// Instance-level: props, state, logic
	import { cn } from '$lib/utils';
	let { size = 'md', class: className, children }: Props = $props();
</script>

<div class={cn(variants({ size }), className)}>
	{@render children?.()}
</div>
```

### TanStack Query Pattern

```ts
// queries.ts - define query options
export const featureQueries = {
	list: (limit = 20) =>
		queryOptions({
			queryKey: ['feature', 'list', { limit }] as const,
			queryFn: () => apiFetch('/api/v2/items', type(ItemSchema.array()))
		})
};

// Component usage
const items = createQuery(featureQueries.list(10));
```

### arktype Schema Pattern

```ts
import { type } from 'arktype';

export const ItemSchema = type({
	id: 'string <= 100',
	name: 'string <= 100',
	count: 'number',
	'optional?': 'string'
});

export type Item = typeof ItemSchema.infer;
```

### Rust Commands

```rust
use log::{debug, error, info};

#[tauri::command]
pub async fn do_something(app: tauri::AppHandle, param: String) -> Result<String, String> {
    info!("do_something called with: {}", param);

    some_operation().map_err(|e| {
        error!("Operation failed: {}", e);
        format!("Operation failed: {}", e)
    })?;

    Ok("success".to_string())
}

// Register in lib.rs:
// .invoke_handler(tauri::generate_handler![commands::module::do_something])
```

### Error Handling

**TypeScript:** Use try/catch, log with `@tauri-apps/plugin-log`, re-throw or return error state.

**Rust:** Return `Result<T, String>`, use `map_err` to convert errors, log with `log` crate before returning.

### Naming Conventions

| Item             | Convention      | Example                  |
| ---------------- | --------------- | ------------------------ |
| Files            | kebab-case      | `mod-install-service.ts` |
| Components       | PascalCase      | `ModCard.svelte`         |
| Functions        | camelCase       | `formatPlayTime`         |
| Types/Interfaces | PascalCase      | `ModResponse`            |
| Constants        | SCREAMING_SNAKE | `CONNECT_TIMEOUT`        |
| Rust functions   | snake_case      | `download_mod`           |
| Rust structs     | PascalCase      | `ModDownloadProgress`    |

## Git Workflow

Use conventional commits:

```
feat: add mod installation flow
fix: correct profile path resolution on Windows
chore: update dependencies
```

## Boundaries

### Never

- Commit secrets, API keys, or `.env` files
- Modify `src-tauri/gen/`, `build/`, or `.svelte-kit/`
- Use `any` without eslint-disable and justification
- Add blocking operations in Rust commands (use async)

### Always

- Validate external data with arktype schemas
- Use `$lib/` alias for imports
- Use TanStack Query for server/async state
- Use Svelte 5 runes (`$state`, `$derived`, `$props`)
- Return `Result<T, String>` from Tauri commands
- Run `bun lint` before committing

## Key Files

| File                        | Purpose                           |
| --------------------------- | --------------------------------- |
| `src/routes/+layout.svelte` | Root layout, QueryClient setup    |
| `src/lib/api/client.ts`     | API fetch with arktype validation |
| `src/lib/utils.ts`          | `cn()` and type helpers           |
| `src/app.css`               | Tailwind config, CSS variables    |
| `src-tauri/src/lib.rs`      | Tauri entry, command registration |
| `src-tauri/tauri.conf.json` | Tauri build/runtime config        |
| `components.json`           | shadcn-svelte configuration       |
