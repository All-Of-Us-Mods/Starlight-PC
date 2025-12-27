<script lang="ts">
	import { modQueries } from '$lib/features/mods/queries';
	import { createQuery, keepPreviousData } from '@tanstack/svelte-query';
	import ModCard from '$lib/features/mods/components/ModCard.svelte';
	import ModCardSkeleton from '$lib/features/mods/components/ModCardSkeleton.svelte';
	import { Input } from '$lib/components/ui/input';
	import * as NativeSelect from '$lib/components/ui/native-select';
	import { Button } from '$lib/components/ui/button';
	import { Search, ArrowUpDown, X } from '@lucide/svelte';
	import { Compass, ChevronLeft, ChevronRight } from '@jis3r/icons';

	// --- State ---
	const ITEMS_PER_PAGE = 12;
	let page = $state(0);
	let searchInput = $state('');
	let debouncedSearch = $state('');
	let sortBy = $state<SortKey>('downloads');

	// --- Debounce ---
	$effect(() => {
		const value = searchInput;
		const timer = setTimeout(() => {
			if (debouncedSearch !== value) {
				debouncedSearch = value;
				page = 0;
			}
		}, 100);
		return () => clearTimeout(timer);
	});

	// --- Queries ---
	const totalCountQuery = createQuery(() => modQueries.total());
	const modsQuery = createQuery(() => ({
		...modQueries.explore(debouncedSearch, ITEMS_PER_PAGE, page * ITEMS_PER_PAGE),
		placeholderData: keepPreviousData
	}));

	// --- Sort ---
	type Mod = NonNullable<typeof modsQuery.data>[number];
	type SortKey = 'downloads' | 'updated' | 'created' | 'alpha';

	const sortOptions: { value: SortKey; label: string }[] = [
		{ value: 'downloads', label: 'Most Popular' },
		{ value: 'updated', label: 'Recently Updated' },
		{ value: 'created', label: 'Newest Releases' },
		{ value: 'alpha', label: 'Alphabetical (A-Z)' }
	];

	const sortFns: Record<SortKey, (a: Mod, b: Mod) => number> = {
		downloads: (a, b) => b.downloads - a.downloads,
		updated: (a, b) => b.updated_at - a.updated_at,
		created: (a, b) => b.created_at - a.created_at,
		alpha: (a, b) => a.name.localeCompare(b.name)
	};

	// --- Derived ---
	const displayedMods = $derived(modsQuery.data ? [...modsQuery.data].sort(sortFns[sortBy]) : []);
	const isSearching = $derived(debouncedSearch.trim().length > 0);
	const totalPages = $derived(
		isSearching ? null : Math.ceil((totalCountQuery.data ?? 0) / ITEMS_PER_PAGE)
	);
	const hasNextPage = $derived(
		isSearching
			? (modsQuery.data?.length ?? 0) === ITEMS_PER_PAGE
			: totalPages !== null && page < totalPages - 1
	);
	const showPagination = $derived(page > 0 || hasNextPage);
	const searchPlaceholder = $derived(
		totalCountQuery.data
			? `Search ${totalCountQuery.data.toLocaleString()} mods...`
			: 'Search mods...'
	);
</script>

<div class="scrollbar-styled @container h-full space-y-12 overflow-y-auto px-10 py-8">
	<header class="mb-6 flex flex-col gap-6 @lg:flex-row @lg:items-center @lg:justify-between">
		<div class="flex items-center gap-3">
			<div
				class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 ring-1 ring-primary/20"
			>
				<Compass class="h-6 w-6 text-primary" />
			</div>
			<div class="space-y-0.5">
				<h1 class="text-4xl font-black tracking-tight">Explore</h1>
				<p class="text-sm text-muted-foreground">Discover and manage mods for Among Us.</p>
			</div>
		</div>

		<div class="flex items-center gap-3">
			<div class="relative max-w-xs">
				<Search
					class="absolute top-1/2 left-3.5 size-4 -translate-y-1/2 text-muted-foreground/70"
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

			<div class="relative w-48">
				<ArrowUpDown
					class="absolute top-1/2 left-3.5 size-3.5 -translate-y-1/2 text-muted-foreground/70"
				/>
				<NativeSelect.Root bind:value={sortBy} class="h-10 w-full rounded-full pl-10">
					{#each sortOptions as opt (opt.value)}
						<NativeSelect.Option value={opt.value}>{opt.label}</NativeSelect.Option>
					{/each}
				</NativeSelect.Root>
			</div>
		</div>
	</header>

	<main class="grid grid-cols-1 gap-4 xl:grid-cols-2">
		{#if modsQuery.isPending && !modsQuery.data}
			{#each { length: 6 }, i (i)}
				<ModCardSkeleton />
			{/each}
		{:else if displayedMods.length === 0}
			<div class="col-span-full py-32 text-center">
				<h3 class="mb-5 text-xl font-bold">No mods found</h3>
				<Button variant="outline" onclick={() => (searchInput = '')}>Clear search</Button>
			</div>
		{:else}
			{#each displayedMods as mod (mod.id)}
				<a href="/mods/{mod.id}">
					<ModCard {mod} />
				</a>
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
