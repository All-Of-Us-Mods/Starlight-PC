<script lang="ts" module>
	import type { Mod } from '$lib/features/mods/schema';
	import type { Profile, UnifiedMod } from '$lib/features/profiles/schema';

	type ProfileModUpdateStatus = {
		installedVersion: string;
		latestVersion: string | null;
		isOutdated: boolean;
		status: 'checking' | 'ready' | 'error';
	};
	type ProfileModUpdatesMap = Record<string, ProfileModUpdateStatus>;

	export interface ProfileModsListProps {
		isPending: boolean;
		displayedMods: UnifiedMod[];
		isSearching: boolean;
		profile: Profile;
		modsMap: Map<string, Mod>;
		isDisabled: boolean;
		modUpdateStatuses: ProfileModUpdatesMap;
		updatingModIds: Set<string>;
		isUpdatingAll: boolean;
		showPagination: boolean;
		currentPage: number;
		totalPages: number;
		hasNextPage: boolean;
		onClearSearch: () => void;
		onInstallMods: () => void;
		onDeleteMod: (mod: UnifiedMod) => void;
		onUpdateMod: (modId: string) => void;
		onPrevPage: () => void;
		onNextPage: () => void;
	}
</script>

<script lang="ts">
	import ProfilesModCard from '$lib/features/mods/components/ProfilesModCard.svelte';
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { ChevronLeft, ChevronRight, Plus, Trash2 } from '@jis3r/icons';
	import { Package } from '@lucide/svelte';

	let {
		isPending,
		displayedMods,
		isSearching,
		profile,
		modsMap,
		isDisabled,
		modUpdateStatuses,
		updatingModIds,
		isUpdatingAll,
		showPagination,
		currentPage,
		totalPages,
		hasNextPage,
		onClearSearch,
		onInstallMods,
		onDeleteMod,
		onUpdateMod,
		onPrevPage,
		onNextPage
	}: ProfileModsListProps = $props();
</script>

<main class="grid grid-cols-1 gap-4 lg:grid-cols-2 xl:grid-cols-3">
	{#if isPending}
		{#each { length: 4 }, i (i)}
			<Card.Root class="p-3">
				<div class="flex gap-3">
					<Skeleton class="h-16 w-16 rounded-lg" />
					<div class="flex-1 space-y-2">
						<Skeleton class="h-5 w-48" />
						<Skeleton class="h-4 w-32" />
						<Skeleton class="h-4 w-full" />
					</div>
				</div>
			</Card.Root>
		{/each}
	{:else if displayedMods.length === 0}
		<div class="col-span-full flex flex-col items-center justify-center py-24 text-center">
			<Package class="mb-4 h-12 w-12 text-muted-foreground/30" />
			{#if isSearching}
				<h3 class="mb-2 text-xl font-bold">No mods found</h3>
				<p class="mb-4 text-muted-foreground">Try a different search term</p>
				<Button variant="outline" onclick={onClearSearch}>Clear search</Button>
			{:else}
				<h3 class="mb-2 text-xl font-bold">No mods installed</h3>
				<p class="mb-4 text-muted-foreground">Install mods from the Explore page</p>
				<Button onclick={onInstallMods}>
					<Plus class="mr-2 size-4" />
					Install Mods
				</Button>
			{/if}
		</div>
	{:else}
		{#each displayedMods as mod (mod.source === 'managed' ? mod.mod_id : mod.file)}
			{#if mod.source === 'managed'}
				{@const modData = modsMap.get(mod.mod_id)}
				{@const profileMod = profile.mods.find((m) => m.mod_id === mod.mod_id)}
				{#if modData && profileMod}
					{@const modUpdateStatus = modUpdateStatuses[mod.mod_id]}
					<ProfilesModCard
						mod={modData}
						{profileMod}
						profileId={profile.id}
						latestVersion={modUpdateStatus?.latestVersion ?? undefined}
						isOutdated={modUpdateStatus?.isOutdated ?? false}
						isCheckingUpdate={modUpdateStatus?.status === 'checking'}
						isUpdating={isUpdatingAll || updatingModIds.has(mod.mod_id)}
						onUpdate={() => onUpdateMod(mod.mod_id)}
						ondelete={() => onDeleteMod(mod)}
					/>
				{/if}
			{:else}
				<Card.Root class="p-3">
					<div class="flex items-center gap-3">
						<div class="flex h-16 w-16 flex-none items-center justify-center rounded-lg bg-muted">
							<Package class="h-8 w-8 text-muted-foreground/40" />
						</div>
						<div class="min-w-0 flex-1">
							<h3 class="truncate font-bold">{mod.file}</h3>
							<p class="text-sm text-muted-foreground">Custom mod (unmanaged)</p>
						</div>
						<Button
							size="sm"
							variant="destructive"
							onclick={() => onDeleteMod(mod)}
							disabled={isDisabled}
						>
							<Trash2 class="size-4" />
						</Button>
					</div>
				</Card.Root>
			{/if}
		{/each}
	{/if}
</main>

{#if showPagination}
	<footer class="flex items-center justify-center gap-4 py-8">
		<Button variant="outline" size="icon" disabled={currentPage === 0} onclick={onPrevPage}>
			<ChevronLeft class="size-4" />
		</Button>

		<span class="text-sm font-medium">Page {currentPage + 1} of {totalPages}</span>

		<Button variant="outline" size="icon" disabled={!hasNextPage} onclick={onNextPage}>
			<ChevronRight class="size-4" />
		</Button>
	</footer>
{/if}
