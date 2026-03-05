<script lang="ts">
	import { modQueries } from '$lib/features/mods/queries';
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
	import { Button } from '$lib/components/ui/button';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { Compass, ChevronLeft, ChevronRight } from '@jis3r/icons';
	import { Debounced, watch } from 'runed';
	import ExploreFilters from './_components/ExploreFilters.svelte';
	import ExploreModsGrid from './_components/ExploreModsGrid.svelte';

	type SortKey = 'trending' | 'latest';
	const ITEMS_PER_PAGE = 12;

	let page = $state(0);
	let searchInput = $state('');
	const debouncedSearch = new Debounced(() => searchInput, 250);
	let sortBy = $state<SortKey>('trending');

	const sortOptions: { value: SortKey; label: string }[] = [
		{ value: 'trending', label: 'Trending' },
		{ value: 'latest', label: 'Latest' }
	];

	// Reset pagination when debounced search changes
	watch(
		() => debouncedSearch.current,
		() => {
			page = 0;
		},
		{ lazy: true }
	);

	// Queries
	const totalCountQuery = createQuery(() => modQueries.total());
	const modsQuery = createQuery(() => ({
		...modQueries.explore(debouncedSearch.current, ITEMS_PER_PAGE, page * ITEMS_PER_PAGE, sortBy),
		placeholderData: keepPreviousData
	}));

	const isSearching = $derived(debouncedSearch.current.trim().length > 0);
	const totalMods = $derived(totalCountQuery.data ?? 0);

	const totalPages = $derived(isSearching ? null : Math.ceil(totalMods / ITEMS_PER_PAGE));

	const hasNextPage = $derived(
		(modsQuery.data?.length ?? 0) === ITEMS_PER_PAGE &&
			(isSearching || (totalPages !== null && page < totalPages - 1))
	);

	const showPagination = $derived(page > 0 || hasNextPage);

	const searchPlaceholder = $derived(
		totalMods ? `Search ${totalMods.toLocaleString()} mods...` : 'Search mods...'
	);
</script>

<div class="scrollbar-styled @container h-full space-y-12 overflow-y-auto px-10 py-8">
	<PageHeader
		title="Explore"
		description="Discover and manage mods for Among Us."
		icon={Compass}
		class="flex-col gap-6 @lg:flex-row @lg:items-center @lg:justify-between"
	>
		<ExploreFilters bind:searchInput bind:sortBy {sortOptions} {searchPlaceholder} />
	</PageHeader>

	<ExploreModsGrid {modsQuery} onClearSearch={() => (searchInput = '')} />

	{#if showPagination}
		<footer class="flex items-center justify-center gap-4 py-8">
			<Button variant="outline" size="icon" disabled={page === 0} onclick={() => page--}>
				<ChevronLeft class="size-4" />
			</Button>

			<span class="text-sm font-medium">
				{#if totalPages}
					Page {page + 1} of {totalPages}
				{:else}
					Page {page + 1}
				{/if}
			</span>

			<Button variant="outline" size="icon" disabled={!hasNextPage} onclick={() => page++}>
				<ChevronRight class="size-4" />
			</Button>
		</footer>
	{/if}
</div>
