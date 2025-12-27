<script lang="ts">
	// This page intentionally does NOT use useSidebar
	// to demonstrate automatic sidebar collapse
	import { Library } from '@lucide/svelte';
</script>

<div class="px-10 py-8">
	<div class="mb-6 flex items-center gap-3">
		<div
			class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
		>
			<Library class="h-6 w-6 text-primary" />
		</div>
		<div class="space-y-0.5">
			<h1 class="text-4xl font-black tracking-tight">Library</h1>
			<p class="text-sm text-muted-foreground">Manage your installed mods and configurations.</p>
		</div>
	</div>

	<div class="mb-6">
		<p class="text-muted-foreground">
			This page has no sidebar content, so the sidebar automatically collapses.
		</p>
	</div>

	<div class="grid grid-cols-1 gap-4">
		<div class="rounded-lg border border-border p-6 transition-colors hover:bg-accent/50">
			<h3 class="mb-2 text-lg font-semibold">Local Modpacks</h3>
			<p class="text-sm text-muted-foreground">Browse and manage your installed modpacks</p>
		</div>

		<div class="rounded-lg border border-border p-6 transition-colors hover:bg-accent/50">
			<h3 class="mb-2 text-lg font-semibold">Saved Configurations</h3>
			<p class="text-sm text-muted-foreground">
				Access your saved world configurations and settings
			</p>
		</div>

		<div class="rounded-lg border border-border p-6 transition-colors hover:bg-accent/50">
			<h3 class="mb-2 text-lg font-semibold">Downloads</h3>
			<p class="text-sm text-muted-foreground">
				View your download history and manage cached files
			</p>
		</div>
	</div>
</div>
