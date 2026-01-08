<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';

	let {
		allMods,
		showAllMods,
		onToggleShowAll
	}: {
		allMods: Array<{ id: string; name: string; source: 'managed' | 'custom' }>;
		showAllMods: boolean;
		onToggleShowAll: () => void;
	} = $props();

	const displayedMods = $derived(showAllMods ? allMods : allMods.slice(0, 3));
	const hiddenModCount = $derived(allMods.length - 3);
</script>

{#if allMods.length > 0}
	<div class="flex flex-wrap items-center gap-1.5">
		{#each displayedMods as mod (mod.id)}
			<Badge variant="secondary" class="max-w-32 truncate text-xs">
				{mod.name}
			</Badge>
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
