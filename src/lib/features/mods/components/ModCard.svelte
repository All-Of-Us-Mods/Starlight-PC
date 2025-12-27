<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { ImageOff, Download, Clock } from '@jis3r/icons';
	import AddToProfileDialog from '$lib/features/profiles/components/AddToProfileDialog.svelte';
	import { createQuery } from '@tanstack/svelte-query';
	import { profileQueries } from '$lib/features/profiles/queries';
	import type { Mod } from '$lib/features/mods/schema';

	let { mod }: { mod: Mod } = $props();

	const hasProfilesQuery = createQuery(() => profileQueries.hasAny());
	const hasProfiles = $derived(hasProfilesQuery.data ?? false);
</script>

<Card.Root class="overflow-hidden p-0 transition-colors hover:bg-accent/50">
	<div class="flex h-40">
		<div class="h-40 w-40 flex-none shrink-0 bg-muted">
			{#if mod._links.thumbnail}
				<img src={mod._links.thumbnail} alt={mod.name} class="h-full w-full object-contain" />
			{:else}
				<div class="flex h-full w-full items-center justify-center">
					<ImageOff class="h-10 w-10 text-muted-foreground/40" />
				</div>
			{/if}
		</div>

		<div class="flex min-w-0 flex-1 flex-col p-3">
			<div class="min-w-0 flex-1">
				<h3 class="mb-0.5 truncate text-base leading-tight font-bold" title={mod.name}>
					{mod.name}
				</h3>
				<p class="mb-2 truncate text-sm text-muted-foreground/80">
					by {mod.author}
				</p>
				<p class="line-clamp-3 text-sm leading-snug text-muted-foreground">
					{mod.description}
				</p>
			</div>

			<div class="flex items-center justify-between gap-4 pt-2 text-sm font-medium">
				<div class="flex items-center gap-4">
					<div class="flex items-center gap-1.5">
						<Download class="text-primary" />
						<span>{mod.downloads.toLocaleString()}</span>
					</div>
					<div class="flex items-center gap-1.5 text-muted-foreground">
						<Clock />
						<span>{new Date(mod.updated_at).toLocaleDateString()}</span>
					</div>
				</div>
				{#if hasProfiles}
					<AddToProfileDialog modId={mod.id} />
				{/if}
			</div>
		</div>
	</div>
</Card.Root>
