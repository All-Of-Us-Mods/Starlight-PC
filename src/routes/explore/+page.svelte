<script lang="ts">
	import { modQueries } from '$lib/features/mods/queries';
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
	import ModCard from '$lib/features/mods/components/ModCard.svelte';
	import ModCardSkeleton from '$lib/features/mods/components/ModCardSkeleton.svelte';
	import { Input } from '$lib/components/ui/input';
	import * as Select from '$lib/components/ui/select';
	import { Button } from '$lib/components/ui/button';
	import PageHeader from '$lib/components/shared/PageHeader.svelte';
	import { ArrowUpDown, X } from '@lucide/svelte';
	import { Search, Compass, ChevronLeft, ChevronRight } from '@jis3r/icons';

	type SortKey = 'trending' | 'latest';
	const ITEMS_PER_PAGE = 12;

	let page = $state(0);
	let searchInput = $state('');
	let debouncedSearch = $state('');
	let sortBy = $state<SortKey>('trending');

	const sortOptions: { value: SortKey; label: string }[] = [
		{ value: 'trending', label: 'Trending' },
		{ value: 'latest', label: 'Latest' }
	];

	$effect(() => {
		const value = searchInput;
		const timer = setTimeout(() => {
			debouncedSearch = value;
			page = 0; // Reset pagination when search term actually updates
		}, 250);
		return () => clearTimeout(timer);
	});

	// Queries
	const totalCountQuery = createQuery(() => modQueries.total());
	const modsQuery = createQuery(() => ({
		...modQueries.explore(debouncedSearch, ITEMS_PER_PAGE, page * ITEMS_PER_PAGE, sortBy),
		placeholderData: keepPreviousData
	}));

	const isSearching = $derived(debouncedSearch.trim().length > 0);
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
		<div class="flex items-center gap-3">
			<div class="relative max-w-xs">
				<Search
					class="absolute top-1/2 left-3.5 -translate-y-1/2 text-muted-foreground/70"
					size={16}
				/>
				<Input
					placeholder={searchPlaceholder}
					bind:value={searchInput}
					class="h-10 w-full rounded-full border-muted-foreground/10 bg-muted/50 pr-10 pl-10"
				/>
				{#if searchInput}
					<button
						onclick={() => (searchInput = '')}
						class="absolute top-1/2 right-3 -translate-y-1/2"
					>
						<X class="size-3.5" />
					</button>
				{/if}
			</div>

			<div class="relative">
				<ArrowUpDown
					class="absolute top-1/2 left-3.5 z-10 size-3.5 -translate-y-1/2 text-muted-foreground/70"
				/>
				<Select.Root bind:value={sortBy} type="single">
					<Select.Trigger
						class="h-10 w-full rounded-full border-muted-foreground/10 bg-muted/50 pl-10"
					>
						{#each sortOptions as opt (opt.value)}
							{#if opt.value === sortBy}
								{opt.label}
							{/if}
						{/each}
					</Select.Trigger>
					<Select.Content>
						{#each sortOptions as opt (opt.value)}
							<Select.Item value={opt.value}>{opt.label}</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>
		</div>
	</PageHeader>

	<main class="grid grid-cols-1 gap-4 xl:grid-cols-2">
		{#if modsQuery.isPending && !modsQuery.data}
			{#each { length: 6 }, i (i)}
				<ModCardSkeleton />
			{/each}
		{:else if !modsQuery.data?.length}
			<div class="col-span-full py-32 text-center">
				<h3 class="mb-5 text-xl font-bold">No mods found</h3>
				<Button variant="outline" onclick={() => (searchInput = '')}>Clear search</Button>
			</div>
		{:else}
			{#each modsQuery.data as mod (mod.id)}
				<ModCard {mod} />
			{/each}
		{/if}
	</main>

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
