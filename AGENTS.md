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

---

## Commands

```bash
# Development
bun dev                    # Start Vite dev server (frontend only)
bun tauri dev              # Start full Tauri app with hot reload

# Build
bun build                  # Build frontend to /build
bun tauri build            # Build production Tauri app

# Quality
bun lint                   # ESLint + Prettier check
bun format                 # Prettier write
bun check                  # svelte-check (type checking)

# Rust
cd src-tauri && cargo check      # Check Rust code
cd src-tauri && cargo clippy     # Lint Rust code
cd src-tauri && cargo fmt        # Format Rust code
```

---

## Project Structure

```
src/
├── routes/                 # SvelteKit pages
│   ├── +layout.svelte      # Root layout (QueryClient, AppShell)
│   ├── +page.svelte        # Home
│   ├── explore/            # Mod discovery
│   └── library/            # User's mods
├── lib/
│   ├── api/client.ts       # Fetch wrapper with arktype validation
│   ├── components/
│   │   ├── layout/         # AppShell, Titlebar
│   │   ├── shared/         # Reusable non-UI components
│   │   └── ui/             # shadcn-svelte primitives
│   ├── features/           # Domain modules (mods, news)
│   │   └── {feature}/
│   │       ├── components/ # Feature-specific components
│   │       ├── queries.ts  # TanStack Query options
│   │       └── schema.ts   # arktype schemas
│   ├── state/              # Svelte 5 reactive state
│   └── utils.ts            # cn(), type helpers
└── app.css                 # Tailwind + CSS variables

src-tauri/
├── src/
│   ├── lib.rs              # Tauri app entry, plugin registration
│   ├── commands/           # Tauri commands (init, paths, profiles, launch)
│   └── utils/              # Rust utilities (finder, game)
├── Cargo.toml
└── tauri.conf.json         # Tauri configuration
```

---

## Code Style

### Svelte 5 Components

```svelte
<script lang="ts" module>
	// Module-level: exports, types, variants
	import { tv, type VariantProps } from 'tailwind-variants';

	export const cardVariants = tv({
		base: 'rounded-lg border bg-card',
		variants: {
			size: { sm: 'p-4', md: 'p-6', lg: 'p-8' }
		},
		defaultVariants: { size: 'md' }
	});

	export type CardProps = { size?: VariantProps<typeof cardVariants>['size'] };
</script>

<script lang="ts">
	// Instance-level: props, state, logic
	import { cn } from '$lib/utils';

	let { size = 'md', class: className, children }: CardProps = $props();
</script>

<div class={cn(cardVariants({ size }), className)}>
	{@render children?.()}
</div>
```

### TanStack Query

```ts
// src/lib/features/mods/queries.ts
import { queryOptions } from '@tanstack/svelte-query';
import { apiFetch } from '$lib/api/client';
import { ModResponse } from './schema';

export const modQueries = {
	latest: (limit = 20) =>
		queryOptions({
			queryKey: ['mods', 'list', { limit }] as const,
			queryFn: () => apiFetch('/api/v2/mods', type(ModResponse.array())),
			staleTime: 1000 * 60 * 5
		})
};
```

```svelte
<!-- Usage in component -->
<script lang="ts">
	import { createQuery } from '@tanstack/svelte-query';
	import { modQueries } from '$lib/features/mods/queries';

	const mods = createQuery(modQueries.latest(10));
</script>

{#if mods.isPending}
	<Skeleton />
{:else if mods.data}
	{#each mods.data as mod}
		<ModCard {mod} />
	{/each}
{/if}
```

### arktype Schemas

```ts
// src/lib/features/mods/schema.ts
import { type } from 'arktype';

export const ModResponse = type({
	id: 'string <= 100',
	name: 'string <= 100',
	author: 'string <= 100',
	downloads: 'number',
	_links: {
		self: 'string',
		thumbnail: 'string'
	}
});

export type Mod = typeof ModResponse.infer;
```

### Tauri Commands (Rust)

```rust
// src-tauri/src/commands/example.rs
use tauri::Manager;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub fn get_data(app: tauri::AppHandle) -> Result<String, String> {
    let store = app
        .store("registry.json")
        .map_err(|e| format!("Failed to load store: {}", e))?;

    store
        .get("key")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| "Key not found".to_string())
}
```

```rust
// Register in lib.rs
.invoke_handler(tauri::generate_handler![
    commands::example::get_data
])
```

### Tailwind CSS

- Use `cn()` for conditional classes
- Use CSS variables from `app.css` for theming (`--primary`, `--background`, etc.)
- Use `tailwind-variants` for component variants

```svelte
<div class={cn('bg-card text-card-foreground', isActive && 'ring-2 ring-primary')}>
```

## Git Workflow

### Pre-commit Hook

Husky runs `bun lint` before each commit. Ensure code passes:

```bash
bun lint    # Must pass
bun format  # Fix formatting issues
```

### Commit Messages

Use conventional commits:

```
feat: add mod installation flow
fix: correct profile path resolution on Windows
chore: update dependencies
```

### Branches

- `main` — production-ready
- `feat/*` — new features
- `fix/*` — bug fixes

---

## Boundaries

### Never

- Commit secrets, API keys, or `.env` files (gitignored)
- Modify `src-tauri/gen/` (auto-generated)
- Modify `build/` or `.svelte-kit/` (build outputs)
- Use `any` without eslint-disable comment and justification
- Add blocking operations in Rust command handlers (use async)

### Always

- Validate external data with arktype schemas
- Use `$lib/` alias for imports
- Use TanStack Query for server state
- Use Svelte 5 runes (`$state`, `$derived`, `$props`)
- Return `Result<T, String>` from Tauri commands
- Run `bun lint` before committing

### Environment

```bash
# .env (gitignored)
PUBLIC_API_URL=https://api.example.com
```

Access in frontend:

```ts
import { PUBLIC_API_URL } from '$env/static/public';
```

---

## Key Files

| File                        | Purpose                               |
| --------------------------- | ------------------------------------- |
| `src/routes/+layout.svelte` | Root layout, QueryClient setup        |
| `src/lib/api/client.ts`     | API fetch with validation             |
| `src/lib/utils.ts`          | `cn()` and type helpers               |
| `src/app.css`               | Tailwind config, CSS variables        |
| `src-tauri/src/lib.rs`      | Tauri app entry, command registration |
| `src-tauri/tauri.conf.json` | Tauri build/runtime config            |
| `components.json`           | shadcn-svelte configuration           |
