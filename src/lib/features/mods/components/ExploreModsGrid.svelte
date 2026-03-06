<script lang="ts" module>
	import type { Mod } from '$lib/features/mods/schema';

	export interface ExploreModsGridProps {
		modsQuery: {
			isPending: boolean;
			data?: Mod[];
		};
		onClearSearch: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import ModCard from '$lib/features/mods/components/ModCard.svelte';
	import ModCardSkeleton from '$lib/features/mods/components/ModCardSkeleton.svelte';

	let { modsQuery, onClearSearch }: ExploreModsGridProps = $props();
</script>

<main class="grid grid-cols-1 gap-4 xl:grid-cols-2">
	{#if modsQuery.isPending && !modsQuery.data}
		{#each { length: 12 }, i (i)}
			<ModCardSkeleton />
		{/each}
	{:else if !modsQuery.data?.length}
		<div class="col-span-full py-32 text-center">
			<h3 class="mb-5 text-xl font-bold">No mods found</h3>
			<Button variant="outline" onclick={onClearSearch}>Clear search</Button>
		</div>
	{:else}
		{#each modsQuery.data as mod (mod.id)}
			<ModCard {mod} />
		{/each}
	{/if}
</main>
