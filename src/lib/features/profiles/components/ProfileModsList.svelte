<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';

	let {
		allMods,
		showAllMods,
		onToggleShowAll,
		onModClick
	}: {
		allMods: Array<{ id: string; name: string; source: 'managed' | 'custom' }>;
		showAllMods: boolean;
		onToggleShowAll: () => void;
		onModClick?: (modId: string) => void;
	} = $props();

	const displayedMods = $derived(showAllMods ? allMods : allMods.slice(0, 3));
	const hiddenModCount = $derived(allMods.length - 3);
</script>

{#if allMods.length > 0}
	<div class="flex flex-wrap items-center gap-1.5">
		{#each displayedMods as mod (mod.id)}
			{#if mod.source === 'managed' && onModClick}
				<button
					type="button"
					onclick={(e) => {
						e.stopPropagation();
						onModClick(mod.id);
					}}
					class="inline-flex max-w-32 items-center truncate rounded-md border border-transparent bg-secondary px-2.5 py-0.5 text-xs font-semibold text-secondary-foreground transition-colors hover:bg-secondary/80 hover:text-primary"
				>
					{mod.name}
				</button>
			{:else}
				<Badge variant="secondary" class="max-w-32 truncate text-xs">
					{mod.name}
				</Badge>
			{/if}
		{/each}
		{#if hiddenModCount > 0}
			<button
				type="button"
				onclick={onToggleShowAll}
				class="rounded-md px-2 py-0.5 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
			>
				{showAllMods ? 'Show less' : `+${hiddenModCount} more`}
			</button>
		{/if}
	</div>
{/if}
