<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	// (remove) import { page } from '$app/stores';

	import ProfilesModCard from '$lib/features/mods/components/ProfilesModCard.svelte';
	import { modQueries } from '$lib/features/mods/queries';
	import { profileQueries } from '$lib/features/profiles/queries';
	import { profileMutations } from '$lib/features/profiles/mutations';
	import { launchService } from '$lib/features/profiles/launch-service';
	import { gameState } from '$lib/features/profiles/game-state.svelte';
	import type { Profile, UnifiedMod } from '$lib/features/profiles/schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { formatPlayTime } from '$lib/utils';
	import { showError, showSuccess } from '$lib/utils/toast';

	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import {
		AlertDialog,
		AlertDialogAction,
		AlertDialogCancel,
		AlertDialogContent,
		AlertDialogDescription,
		AlertDialogFooter,
		AlertDialogHeader,
		AlertDialogTitle
	} from '$lib/components/ui/alert-dialog';
	import * as Dialog from '$lib/components/ui/dialog';

	import { ChevronLeft, ChevronRight, Plus, Trash2 } from '@jis3r/icons';
	import {
		BoxIcon,
		Calendar,
		Clock,
		Folder,
		PencilLineIcon,
		Play,
		SearchIcon,
		X,
		ArrowLeft,
		Package
	} from '@lucide/svelte';

	const queryClient = useQueryClient();

	const profileId = $derived($page.params.id ?? '');

	const profilesQuery = createQuery(() => profileQueries.all());
	const unifiedModsQuery = createQuery(() => ({
		...profileQueries.unifiedMods(profileId),
		enabled: !!profileId
	}));

	const profile = $derived(
		(profilesQuery.data as Profile[] | undefined)?.find((p) => p.id === profileId)
	);

	const updateLastLaunched = createMutation(() => profileMutations.updateLastLaunched(queryClient));
	const deleteProfile = createMutation(() => profileMutations.delete(queryClient));
	const renameProfile = createMutation(() => profileMutations.rename(queryClient));
	const deleteUnifiedMod = createMutation(() => profileMutations.deleteUnifiedMod(queryClient));

	const modIds = $derived(profile?.mods.map((m) => m.mod_id) ?? []);
	const modsQueries = $derived(modIds.map((id) => createQuery(() => modQueries.byId(id))));
	const modsMap = $derived(
		new Map(
			modsQueries
				.map((q) => q.data)
				.filter((m): m is Mod => m !== undefined)
				.map((m) => [m.id, m])
		)
	);

	let searchInput = $state('');
	let debouncedSearch = $state('');
	const ITEMS_PER_PAGE = 6;
	let currentPage = $state(0);

	let deleteDialogOpen = $state(false);
	let renameDialogOpen = $state(false);
	let modToDelete = $state<UnifiedMod | null>(null);
	let deleteModDialogOpen = $state(false);
	let newProfileName = $state('');
	let isLaunching = $state(false);
	let renameError = $state('');

	$effect(() => {
		const value = searchInput;
		const timer = setTimeout(() => {
			if (debouncedSearch !== value) {
				debouncedSearch = value;
				currentPage = 0;
			}
		}, 150);
		return () => clearTimeout(timer);
	});

	const displayedMods = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		const searchLower = debouncedSearch.trim().toLowerCase();

		const filtered = unified.filter((mod) => {
			if (mod.source === 'managed') {
				const modInfo = modsMap.get(mod.mod_id);
				return modInfo?.name.toLowerCase().includes(searchLower) ?? false;
			}
			return mod.file.toLowerCase().includes(searchLower);
		});

		const start = currentPage * ITEMS_PER_PAGE;
		return filtered.slice(start, start + ITEMS_PER_PAGE);
	});

	const totalFilteredMods = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		const searchLower = debouncedSearch.trim().toLowerCase();
		if (!searchLower) return unified.length;

		return unified.filter((mod) => {
			if (mod.source === 'managed') {
				const modInfo = modsMap.get(mod.mod_id);
				return modInfo?.name.toLowerCase().includes(searchLower) ?? false;
			}
			return mod.file.toLowerCase().includes(searchLower);
		}).length;
	});

	const isSearching = $derived(debouncedSearch.trim().length > 0);
	const totalPages = $derived(Math.ceil(totalFilteredMods / ITEMS_PER_PAGE));
	const hasNextPage = $derived(currentPage < totalPages - 1);
	const showPagination = $derived(currentPage > 0 || hasNextPage);

	const searchPlaceholder = $derived(
		unifiedModsQuery.data
			? `Search ${unifiedModsQuery.data.length.toLocaleString()} mods...`
			: 'Search mods...'
	);

	const isRunning = $derived(profile ? gameState.isProfileRunning(profile.id) : false);
	const installState = $derived(profile ? gameState.getBepInExState(profile.id) : null);
	const isInstalling = $derived(
		profile?.bepinex_installed === false || installState?.status === 'installing'
	);
	const isDisabled = $derived(isInstalling || isRunning);

	const totalPlayTime = $derived(
		(profile?.total_play_time ?? 0) + (isRunning ? gameState.getSessionDuration() : 0)
	);

	const lastLaunched = $derived(
		profile?.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	async function handleLaunch() {
		if (!profile || isDisabled) return;

		isLaunching = true;
		try {
			await launchService.launchProfile(profile);
			await updateLastLaunched.mutateAsync(profile.id);
		} catch (e) {
			showError(e);
		} finally {
			isLaunching = false;
		}
	}

	async function handleOpenFolder() {
		if (!profile) return;
		try {
			const fullPath = await join(profile.path, 'BepInEx');
			await revealItemInDir(fullPath);
		} catch (e) {
			showError(e, 'Open folder');
		}
	}

	function openDeleteDialog() {
		deleteDialogOpen = true;
	}

	async function handleDeleteProfile() {
		if (!profile) return;

		const profileName = profile.name;
		deleteDialogOpen = false;

		try {
			await deleteProfile.mutateAsync(profile.id);
			queryClient.removeQueries({ queryKey: ['unified-mods', profile.id] });
			showSuccess(`Profile "${profileName}" deleted`);
			goto('/library');
		} catch (e) {
			showError(e);
		}
	}

	function openRenameDialog() {
		if (!profile) return;
		newProfileName = profile.name;
		renameError = '';
		renameDialogOpen = true;
	}

	async function handleRenameProfile() {
		if (!profile || !newProfileName.trim()) return;

		renameError = '';
		try {
			await renameProfile.mutateAsync({ profileId: profile.id, newName: newProfileName });
			showSuccess('Profile renamed');
			renameDialogOpen = false;
		} catch (e) {
			renameError = e instanceof Error ? e.message : 'Failed to rename';
		}
	}

	function handleInstallMods() {
		goto('/explore');
	}

	function confirmDeleteMod(mod: UnifiedMod) {
		modToDelete = mod;
		deleteModDialogOpen = true;
	}

	async function handleDeleteMod() {
		if (!profile || !modToDelete) return;

		deleteModDialogOpen = false;
		try {
			await deleteUnifiedMod.mutateAsync({ profileId: profile.id, mod: modToDelete });
			showSuccess('Mod removed');
		} catch (e) {
			showError(e, 'Remove mod');
		} finally {
			modToDelete = null;
		}
	}

	function cancelDeleteMod() {
		deleteModDialogOpen = false;
		modToDelete = null;
	}
</script>

{#if profilesQuery.isPending}
	<div class="px-10 py-8">
		<div class="mb-8 flex flex-col items-start gap-6 md:flex-row md:items-center">
			<Skeleton class="h-45 w-45 rounded-lg" />
			<div class="flex-1 space-y-4">
				<Skeleton class="h-10 w-64" />
				<Skeleton class="h-5 w-48" />
				<Skeleton class="h-5 w-40" />
				<div class="flex gap-3">
					<Skeleton class="h-10 w-28" />
					<Skeleton class="h-10 w-32" />
					<Skeleton class="h-10 w-24" />
				</div>
			</div>
		</div>
	</div>
{:else if !profile}
	<div class="flex h-full flex-col items-center justify-center gap-4 px-10 py-8">
		<Package class="h-16 w-16 text-muted-foreground/30" />
		<h2 class="text-xl font-bold">Profile not found</h2>
		<p class="text-muted-foreground">This profile may have been deleted.</p>
		<Button onclick={() => goto('/library')}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>
	</div>
{:else}
	<div class="px-10 py-8">
		<Button variant="ghost" size="sm" class="mb-4" onclick={() => goto('/library')}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>

		<div class="mb-8 flex flex-col items-start gap-6 md:flex-row md:items-center">
			<div
				class="flex h-36 w-36 flex-shrink-0 items-center justify-center overflow-hidden rounded-lg bg-muted/20 md:h-45 md:w-45 {isRunning
					? 'ring-2 ring-green-500/50'
					: ''}"
			>
				<BoxIcon class="h-[60%] w-[60%] text-muted-foreground/50" />
			</div>

			<div class="flex flex-1 flex-col gap-4">
				<div class="flex items-center gap-2">
					<h1 class="text-3xl font-extrabold tracking-tight md:text-4xl">{profile.name}</h1>
					<Button
						size="icon"
						variant="ghost"
						class="size-9 rounded-full"
						onclick={openRenameDialog}
						title="Rename profile"
					>
						<PencilLineIcon class="size-5" />
					</Button>
				</div>

				<div class="flex flex-col gap-2 text-muted-foreground">
					<div class="inline-flex items-center gap-2 text-base md:text-lg">
						<Calendar class="size-5 text-muted-foreground/70" />
						<span
							>Last Launched: <span class="font-medium text-foreground">{lastLaunched}</span></span
						>
					</div>

					<div class="inline-flex items-center gap-2 text-base md:text-lg">
						<Clock class="size-5 text-muted-foreground/70" />
						<span
							>Playtime: <span class="font-medium text-foreground"
								>{formatPlayTime(totalPlayTime)}</span
							></span
						>
					</div>
				</div>

				<div class="flex flex-wrap items-center gap-3 pt-2">
					<Button
						size="lg"
						class="gap-2"
						onclick={handleLaunch}
						disabled={isDisabled || isLaunching}
					>
						{#if isLaunching}
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
							></div>
							Launching...
						{:else}
							<Play class="size-5 fill-current" />
							<span>Launch</span>
						{/if}
					</Button>

					<Button size="lg" variant="outline" class="gap-2" onclick={handleOpenFolder}>
						<Folder class="size-5" />
						<span>Open Folder</span>
					</Button>

					<Button
						size="lg"
						variant="destructive"
						class="gap-2"
						onclick={openDeleteDialog}
						disabled={isDisabled}
					>
						<Trash2 class="size-5" />
						<span>Delete</span>
					</Button>
				</div>
			</div>
		</div>

		<hr class="my-5 border-t border-muted-foreground/20" />

		<div class="rounded-lg bg-white/3 p-4">
			<div class="mx-auto mb-5 flex w-full max-w-3xl justify-center gap-3">
				<div class="relative flex-1">
					<SearchIcon class="absolute top-1/2 left-3.5 -translate-y-1/2 text-muted-foreground/70" />
					<Input
						placeholder={searchPlaceholder}
						bind:value={searchInput}
						class="h-10 w-full rounded-xl border bg-accent/50 pr-10 pl-10"
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

				<Button size="lg" class="flex items-center gap-2" onclick={handleInstallMods}>
					<Plus class="size-5" />
					<span>Install Mods</span>
				</Button>
			</div>

			<main class="grid grid-cols-1 gap-4 xl:grid-cols-2">
				{#if unifiedModsQuery.isPending}
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
							<Button variant="outline" onclick={() => (searchInput = '')}>Clear search</Button>
						{:else}
							<h3 class="mb-2 text-xl font-bold">No mods installed</h3>
							<p class="mb-4 text-muted-foreground">Install mods from the Explore page</p>
							<Button onclick={handleInstallMods}>
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
								<ProfilesModCard
									mod={modData}
									{profileMod}
									profileId={profile.id}
									ondelete={() => confirmDeleteMod(mod)}
								/>
							{/if}
						{:else}
							<Card.Root class="p-3">
								<div class="flex items-center gap-3">
									<div
										class="flex h-16 w-16 flex-none items-center justify-center rounded-lg bg-muted"
									>
										<Package class="h-8 w-8 text-muted-foreground/40" />
									</div>
									<div class="min-w-0 flex-1">
										<h3 class="truncate font-bold">{mod.file}</h3>
										<p class="text-sm text-muted-foreground">Custom mod (unmanaged)</p>
									</div>
									<Button
										size="sm"
										variant="destructive"
										onclick={() => confirmDeleteMod(mod)}
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
		</div>

		{#if showPagination}
			<footer class="flex items-center justify-center gap-4 py-8">
				<Button
					variant="outline"
					size="icon"
					disabled={currentPage === 0}
					onclick={() => currentPage--}
				>
					<ChevronLeft class="size-4" />
				</Button>

				<span class="text-sm font-medium">
					Page {currentPage + 1} of {totalPages}
				</span>

				<Button variant="outline" size="icon" disabled={!hasNextPage} onclick={() => currentPage++}>
					<ChevronRight class="size-4" />
				</Button>
			</footer>
		{/if}
	</div>
{/if}

<!-- Delete Profile Dialog -->
<AlertDialog bind:open={deleteDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Delete Profile?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to delete <strong>{profile?.name}</strong>? This action cannot be
				undone and will delete all files associated with this profile.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={() => (deleteDialogOpen = false)}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteProfile}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Delete Profile
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>

<!-- Rename Profile Dialog -->
<Dialog.Root bind:open={renameDialogOpen}>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Rename Profile</Dialog.Title>
			<Dialog.Description>Enter a new name for this profile.</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-4 py-4">
			<Input
				bind:value={newProfileName}
				placeholder="Profile name"
				disabled={renameProfile.isPending}
			/>
			{#if renameError}
				<p class="text-sm text-destructive">{renameError}</p>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={() => (renameDialogOpen = false)}>Cancel</Button>
			<Button
				onclick={handleRenameProfile}
				disabled={renameProfile.isPending || !newProfileName.trim()}
			>
				{#if renameProfile.isPending}
					<div
						class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Renaming...
				{:else}
					Rename
				{/if}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete Mod Dialog -->
<AlertDialog bind:open={deleteModDialogOpen}>
	<AlertDialogContent>
		<AlertDialogHeader>
			<AlertDialogTitle>Remove Mod?</AlertDialogTitle>
			<AlertDialogDescription>
				Are you sure you want to remove this mod from the profile? The mod file will be deleted.
			</AlertDialogDescription>
		</AlertDialogHeader>
		<AlertDialogFooter>
			<AlertDialogCancel onclick={cancelDeleteMod}>Cancel</AlertDialogCancel>
			<AlertDialogAction
				onclick={handleDeleteMod}
				class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
			>
				Remove Mod
			</AlertDialogAction>
		</AlertDialogFooter>
	</AlertDialogContent>
</AlertDialog>
