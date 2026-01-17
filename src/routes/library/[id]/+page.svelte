<script lang="ts">
	import { onMount } from 'svelte';
	import { createQuery } from '@tanstack/svelte-query';

	import ProfilesModCard from '$lib/features/mods/components/ProfilesModCard.svelte';
	import { modQueries } from '$lib/features/mods/queries';
	import { profileService } from '$lib/features/profiles/profile-service';
	import type { Profile } from '$lib/features/profiles/schema';
	import Card from '$lib/components/ui/card/card.svelte';
	import { ChevronLeft, Pencil, Plus, Search, Trash2 } from '@jis3r/icons';
	import Input from '$lib/components/ui/input/input.svelte';
	import { BoxIcon, Calendar, ChevronRight, Clock, Folder, PencilLineIcon, Play, SearchIcon, X } from '@lucide/svelte';
	import Button from '$lib/components/ui/button/button.svelte';

	const { params } = $props<{ params: { id: string } }>();

	let profile = $state<Profile | undefined>(undefined);

	onMount(async () => {
		profile = await profileService.getProfileById(params.id);
	});

	const modQueryList = $derived(
		profile
			? profile.mods.map((m) => ({
					id: m.mod_id,
					query: createQuery(() => modQueries.byId(m.mod_id))
			  }))
			: []
	);

	let searchInput = $state('');
    let debouncedSearch = $state('');
	const ITEMS_PER_PAGE = 4;
	let page = $state(0);

    const displayedMods = $derived.by(() => {
        if (!modQueryList) return [];

        const searchLower = debouncedSearch.trim().toLowerCase();
        const filtered = modQueryList.filter((m) =>
            m.query.data?.name.toLowerCase().includes(searchLower)
        );

        const start = page * ITEMS_PER_PAGE;
        const end = start + ITEMS_PER_PAGE;

        return filtered.slice(start, end);
    });

	const isSearching = $derived(debouncedSearch.trim().length > 0);
	const totalPages = $derived(
		isSearching ? null : Math.ceil((modQueryList.length ?? 0) / ITEMS_PER_PAGE)
	);
	const hasNextPage = $derived(
		isSearching
			? (modQueryList.length ?? 0) === ITEMS_PER_PAGE
			: totalPages !== null && page < totalPages - 1
	);
	const showPagination = $derived(page > 0 || hasNextPage);
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

    const searchPlaceholder = $derived(
		modQueryList
			? `Search ${modQueryList.length.toLocaleString()} mods...`
			: 'Search mods...'
	);

    const totalPlayTime = $derived(
		(profile?.total_play_time ?? 0)
	);

    const lastLaunched = $derived(
		profile?.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

    function formatPlayTime(ms: number): string {
		const seconds = Math.floor(ms / 1000);
		const minutes = Math.floor(seconds / 60);
		const hours = Math.floor(minutes / 60);

		if (hours > 0) {
			const remainingMinutes = minutes % 60;
			return remainingMinutes > 0 ? `${hours}h ${remainingMinutes}m` : `${hours}h`;
		}
		if (minutes > 0) return `${minutes}m`;
		return seconds > 0 ? `${seconds}s` : '0m';
	}
</script>

{#if profile}
<div class="px-10 py-8">
    <div class="mb-8 flex flex-col md:flex-row items-start md:items-center gap-6">
        <div class="flex-shrink-0 md:w-45 md:h-45 bg-muted/20 rounded-lg overflow-hidden flex items-center justify-center">
            <BoxIcon class="w-[80%] h-[80%]" />
        </div>

        <div class="flex-1 flex flex-col gap-4">
            <div class="flex items-center gap-2">
                <h1 class="text-4xl font-extrabold tracking-tight">{profile.name}</h1>
                <Button 
                    size="lg" 
                    variant="ghost" 
                    class="flex mt-2 items-center justify-center w-10 rounded-full"
                >
                    <PencilLineIcon class="size-5" />
                </Button>
            </div>

            <div class="flex flex-col gap-2 text-muted-foreground">
                <div class="inline-flex items-center gap-2 text-lg">
                    <Calendar class="size-5 text-muted-foreground/70" />
                    <span>Last Launched: <span class="font-medium text-foreground">{lastLaunched}</span></span>
                </div>

                <div class="inline-flex items-center gap-2 text-lg">
                    <Clock class="size-5 text-muted-foreground/70" />
                    <span>Playtime: <span class="font-medium text-foreground">{formatPlayTime(totalPlayTime)}</span></span>
                </div>
            </div>

            <div class="flex flex-wrap items-center justify-between gap-3">
                <div class="flex flex-1 gap-3 md:flex-none">
                    <Button size="lg" class="justify-center">
                        <Play class="mr-2" />
                        <span class="text-lg">Launch</span>
                    </Button>

                    <Button size="lg" variant="outline" class="justify-center">
                        <Folder/>
                        <span class="text-lg">Open Folder</span>
                    </Button>

                    <Button size="lg" variant="destructive" class="justify-center">
                        <Trash2 class="mr-2" />
                        <span class="text-lg">Delete</span>
                    </Button>
                </div>
            </div>
        </div>
    </div>

    <hr class="border-t border-muted-foreground/20 my-5" />

    <div class="bg-white/3 p-4 rounded-lg">
        <div class="flex justify-center mb-5 gap-3 w-full max-w-3xl mx-auto">
            <div class="relative flex-1">
                <SearchIcon class="absolute top-1/2 left-3.5 -translate-y-1/2 text-muted-foreground/70" />
                <Input
                    placeholder={searchPlaceholder}
                    bind:value={searchInput}
                    class="h-10 w-full pl-10 pr-10 rounded-xl border bg-accent/50"
                />
                {#if searchInput}
                    <button
                    onclick={() => (searchInput = '')}
                    class="absolute top-1/2 right-3 -translate-y-1/2"
                    >
                    <X class="size-4" />
                    </button>
                {/if}
            </div>

            <Button size="lg" class="flex items-center gap-2">
                <Plus />
                <span class="text-lg">Install Mods</span>
            </Button>
        </div>

        <main class="grid grid-cols-1 gap-6 xl:grid-cols-2">
            {#if displayedMods.length === 0}
                <div class="col-span-full flex flex-col items-center justify-center py-32 text-center">
                     <h3 class="text-xl font-bold">No mods found</h3>
                </div>
            {:else}
                {#each displayedMods as item (item.id)}
                    {#if item.query.data}
                        <ProfilesModCard mod={item.query.data} />
                    {/if}
                {/each}
            {/if}
        </main>
    </div>

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
{/if}
