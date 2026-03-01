<script lang="ts">
	import { page } from '$app/state';
	import { save as saveDialog } from '@tauri-apps/plugin-dialog';
	import { createMutation, createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Debounced, watch } from 'runed';
	import { SvelteSet } from 'svelte/reactivity';

	import { profileQueries } from '$lib/features/profiles/queries';
	import { settingsQueries } from '$lib/features/settings/queries';
	import { profileMutations } from '$lib/features/profiles/mutations';
	import { modQueries } from '$lib/features/mods/queries';
	import { gameState } from '$lib/features/profiles/game-state.svelte';
	import { formatPlayTime } from '$lib/utils';
	import { showError, showSuccess } from '$lib/utils/toast';
	import type { Profile, UnifiedMod } from '$lib/features/profiles/schema';
	import type { Mod } from '$lib/features/mods/schema';
	import { profileUnifiedModsKey, profilesQueryKey } from '$lib/features/profiles/profile-keys';
	import { mapModsById } from '$lib/features/mods/ui/mod-query-controller';
	import { findProfileById } from '$lib/features/profiles/ui/profile-query-controller';
	import {
		fetchProfileModUpdates,
		type ProfileModUpdatesMap
	} from '$lib/features/profiles/ui/profile-mod-updates-model';
	import {
		createProfileDetailController,
		profileDetailRuntime
	} from '$lib/features/profiles/ui/profile-detail-controller';
	import {
		filterProfileMods,
		getProfileModsPagination,
		paginateProfileMods,
		PROFILE_MODS_PAGE_SIZE
	} from '$lib/features/profiles/ui/profile-mods-view-model';

	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { ArrowLeft, Package } from '@lucide/svelte';
	import ProfileHeroSection from './_components/ProfileHeroSection.svelte';
	import ProfileModsToolbar from './_components/ProfileModsToolbar.svelte';
	import ProfileModsList from './_components/ProfileModsList.svelte';
	import ProfileDialogs from './_components/ProfileDialogs.svelte';
	import ProfileLogViewer from './_components/ProfileLogViewer.svelte';

	const queryClient = useQueryClient();
	const profileId = $derived(page.params.id ?? '');

	const profilesQuery = createQuery(() => profileQueries.all());
	const settingsQuery = createQuery(() => settingsQueries.get());
	const unifiedModsQuery = createQuery(() => ({
		...profileQueries.unifiedMods(profileId),
		enabled: !!profileId
	}));

	const profile = $derived(findProfileById(profilesQuery.data as Profile[] | undefined, profileId));

	const updateLastLaunched = createMutation(() => profileMutations.updateLastLaunched(queryClient));
	const deleteProfile = createMutation(() => profileMutations.delete(queryClient));
	const renameProfile = createMutation(() => profileMutations.rename(queryClient));
	const updateProfileIcon = createMutation(() => profileMutations.updateIcon(queryClient));
	const deleteUnifiedMod = createMutation(() => profileMutations.deleteUnifiedMod(queryClient));
	const installMods = createMutation(() => profileMutations.installMods(queryClient));
	const exportProfileZip = createMutation(() => profileMutations.exportZip());

	const controller = createProfileDetailController({
		launchProfile: profileDetailRuntime.launchProfile,
		updateLastLaunched: (id) => updateLastLaunched.mutateAsync(id),
		deleteProfile: (id) => deleteProfile.mutateAsync(id),
		removeProfileQueries: (id) =>
			queryClient.removeQueries({ queryKey: profileUnifiedModsKey(id) }),
		renameProfile: (id, newName) => renameProfile.mutateAsync({ profileId: id, newName }),
		deleteUnifiedMod: (id, mod) => deleteUnifiedMod.mutateAsync({ profileId: id, mod })
	});

	const modIds = $derived(Array.from(new Set(profile?.mods.map((mod) => mod.mod_id) ?? [])));
	const profileModsQuery = createQuery(() => ({
		queryKey: ['mods', 'profile-batch', ...modIds],
		enabled: modIds.length > 0,
		queryFn: async () => {
			const results = await Promise.allSettled(
				modIds.map((id) => queryClient.fetchQuery(modQueries.byId(id)))
			);
			return results
				.filter((result): result is PromiseFulfilledResult<Mod> => result.status === 'fulfilled')
				.map((result) => result.value);
		}
	}));
	const modsMap = $derived(mapModsById(profileModsQuery.data ?? []));

	let searchInput = $state('');
	const debouncedSearch = new Debounced(() => searchInput, 150);
	let currentPage = $state(0);

	let deleteDialogOpen = $state(false);
	let renameDialogOpen = $state(false);
	let iconDialogOpen = $state(false);
	let modToDelete = $state<UnifiedMod | null>(null);
	let deleteModDialogOpen = $state(false);
	let newProfileName = $state('');
	let iconModeDraft = $state<'default' | 'custom' | 'mod'>('default');
	let customIconDataUrlDraft = $state('');
	let iconModIdDraft = $state('');
	let iconError = $state('');
	let isLaunching = $state(false);
	let isReadingIconFile = $state(false);
	let renameError = $state('');
	let isUpdatingAll = $state(false);
	const updatingModIds = new SvelteSet<string>();
	let customIconInput = $state<HTMLInputElement | null>(null);

	watch(
		() => debouncedSearch.current,
		() => {
			currentPage = 0;
		},
		{ lazy: true }
	);

	const filteredMods = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		return filterProfileMods(unified, modsMap, debouncedSearch.current);
	});
	const managedModsForUpdates = $derived.by(() => {
		const unified = unifiedModsQuery.data ?? [];
		return unified
			.filter((mod) => mod.source === 'managed')
			.map((mod) => ({
				modId: mod.mod_id,
				installedVersion: mod.version
			}));
	});
	const modUpdatesSignature = $derived(
		managedModsForUpdates
			.map((mod) => `${mod.modId}@${mod.installedVersion}`)
			.sort()
			.join('|')
	);
	const modUpdatesQuery = createQuery(() => ({
		queryKey: ['profile-mod-updates', profileId, modUpdatesSignature],
		enabled: !!profileId && managedModsForUpdates.length > 0,
		queryFn: () => fetchProfileModUpdates(queryClient, managedModsForUpdates)
	}));
	const modUpdatesQueryKey = $derived(['profile-mod-updates', profileId, modUpdatesSignature]);
	const modUpdateStatuses = $derived((modUpdatesQuery.data ?? {}) as ProfileModUpdatesMap);
	const displayedMods = $derived(
		paginateProfileMods(filteredMods, currentPage, PROFILE_MODS_PAGE_SIZE)
	);
	const pagination = $derived(
		getProfileModsPagination(filteredMods.length, currentPage, PROFILE_MODS_PAGE_SIZE)
	);

	const isSearching = $derived(debouncedSearch.current.trim().length > 0);
	const updatesAvailableCount = $derived(
		Object.values(modUpdateStatuses).filter((status) => status.isOutdated).length
	);
	const hasManagedModsForUpdates = $derived(managedModsForUpdates.length > 0);
	const isCheckingUpdates = $derived(
		hasManagedModsForUpdates && (modUpdatesQuery.isPending || modUpdatesQuery.isFetching)
	);
	const searchPlaceholder = $derived(
		unifiedModsQuery.data
			? `Search ${unifiedModsQuery.data.length.toLocaleString()} mods...`
			: 'Search mods...'
	);
	const installedModsWithIcons = $derived.by(() => {
		if (!profile) return [] as Array<{ id: string; name: string; thumbnail: string }>;

		const mods = profile.mods
			.map((profileMod) => modsMap.get(profileMod.mod_id))
			.filter((mod): mod is Mod => !!mod && !!mod._links.thumbnail)
			.filter((mod, index, entries) => entries.findIndex((entry) => entry.id === mod.id) === index)
			.map((mod) => ({ id: mod.id, name: mod.name, thumbnail: mod._links.thumbnail }))
			.sort((a, b) => a.name.localeCompare(b.name));

		return mods;
	});
	const selectedIconMod = $derived(
		installedModsWithIcons.find((mod) => mod.id === iconModIdDraft) ?? null
	);
	const profileIconSrc = $derived.by(() => {
		if (!profile) return null;

		if (profile.icon_mode === 'custom' && profile.custom_icon_data_url) {
			return profile.custom_icon_data_url;
		}
		if (profile.icon_mode === 'mod' && profile.icon_mod_id) {
			const iconMod = modsMap.get(profile.icon_mod_id);
			if (iconMod?._links.thumbnail) {
				return iconMod._links.thumbnail;
			}
		}
		return null;
	});

	const runningInstanceCount = $derived(
		profile ? gameState.getProfileRunningInstanceCount(profile.id) : 0
	);
	const isRunning = $derived(runningInstanceCount > 0);
	const installState = $derived(profile ? gameState.getBepInExState(profile.id) : null);
	const allowMultiInstanceLaunch = $derived(
		(settingsQuery.data?.allow_multi_instance_launch ?? false) as boolean
	);
	const isInstalling = $derived(
		profile?.bepinex_installed === false || installState?.status === 'installing'
	);
	const isDisabled = $derived(isInstalling || isRunning);
	const isLaunchDisabled = $derived(isInstalling || (isRunning && !allowMultiInstanceLaunch));

	const totalPlayTime = $derived(
		(profile?.total_play_time ?? 0) + (isRunning ? gameState.getSessionDuration() : 0)
	);
	const lastLaunched = $derived(
		profile?.last_launched_at ? new Date(profile.last_launched_at).toLocaleDateString() : 'Never'
	);

	async function handleLaunch() {
		if (!profile || isLaunchDisabled) return;
		isLaunching = true;
		try {
			await controller.launchProfile(profile);
		} catch (error) {
			showError(error);
		} finally {
			isLaunching = false;
		}
	}

	async function handleExportProfile() {
		if (!profile) return;

		try {
			const destination = await saveDialog({
				title: 'Export Profile ZIP',
				defaultPath: `${profile.name}.zip`,
				filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
			});
			if (!destination) return;

			await exportProfileZip.mutateAsync({ profileId: profile.id, destination });
			showSuccess(`Exported "${profile.name}"`);
		} catch (error) {
			showError(error, 'Export profile');
		}
	}

	async function handleDeleteProfile() {
		if (!profile) return;
		deleteDialogOpen = false;
		try {
			await controller.deleteProfile(profile);
		} catch (error) {
			showError(error);
		}
	}

	function openRenameDialog() {
		if (!profile) return;
		newProfileName = profile.name;
		renameError = '';
		renameDialogOpen = true;
	}

	function openIconDialog() {
		if (!profile) return;

		iconModeDraft = profile.icon_mode ?? 'default';
		customIconDataUrlDraft = profile.custom_icon_data_url ?? '';
		iconModIdDraft = profile.icon_mod_id ?? installedModsWithIcons[0]?.id ?? '';
		if (iconModeDraft === 'mod' && !iconModIdDraft) {
			iconModeDraft = 'default';
		}
		iconError = '';
		iconDialogOpen = true;
	}

	function setIconMode(mode: 'default' | 'custom' | 'mod') {
		iconModeDraft = mode;
		iconError = '';
		if (mode === 'mod' && !iconModIdDraft && installedModsWithIcons.length > 0) {
			iconModIdDraft = installedModsWithIcons[0].id;
		}
	}

	function readFileAsDataUrl(file: File): Promise<string> {
		return new Promise((resolve, reject) => {
			const reader = new FileReader();
			reader.onload = () => {
				if (typeof reader.result === 'string') {
					resolve(reader.result);
					return;
				}
				reject(new Error('Failed to read selected image'));
			};
			reader.onerror = () => {
				reject(reader.error ?? new Error('Failed to read selected image'));
			};
			reader.readAsDataURL(file);
		});
	}

	async function handleCustomIconInput(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;

		iconError = '';
		if (!file.type.startsWith('image/')) {
			iconError = 'Please select an image file';
			input.value = '';
			return;
		}
		if (file.size > 500 * 1024) {
			iconError = 'Image must be 500 KB or smaller';
			input.value = '';
			return;
		}

		isReadingIconFile = true;
		try {
			customIconDataUrlDraft = await readFileAsDataUrl(file);
			iconModeDraft = 'custom';
		} catch (error) {
			iconError = error instanceof Error ? error.message : 'Failed to read selected image';
		} finally {
			isReadingIconFile = false;
			input.value = '';
		}
	}

	async function handleSaveProfileIcon() {
		if (!profile) return;
		iconError = '';

		try {
			if (iconModeDraft === 'custom') {
				if (!customIconDataUrlDraft) {
					iconError = 'Choose an image for the custom icon';
					return;
				}

				await updateProfileIcon.mutateAsync({
					profileId: profile.id,
					selection: { mode: 'custom', dataUrl: customIconDataUrlDraft }
				});
			} else if (iconModeDraft === 'mod') {
				if (!iconModIdDraft) {
					iconError = 'Select an installed mod icon';
					return;
				}

				await updateProfileIcon.mutateAsync({
					profileId: profile.id,
					selection: { mode: 'mod', modId: iconModIdDraft }
				});
			} else {
				await updateProfileIcon.mutateAsync({
					profileId: profile.id,
					selection: { mode: 'default' }
				});
			}

			iconDialogOpen = false;
			showSuccess('Profile icon updated');
		} catch (error) {
			iconError = error instanceof Error ? error.message : 'Failed to update profile icon';
		}
	}

	async function handleRenameProfile() {
		if (!profile || !newProfileName.trim()) return;
		renameError = '';
		try {
			await controller.renameProfile(profile, newProfileName);
			renameDialogOpen = false;
		} catch (error) {
			renameError = error instanceof Error ? error.message : 'Failed to rename';
		}
	}

	function confirmDeleteMod(mod: UnifiedMod) {
		modToDelete = mod;
		deleteModDialogOpen = true;
	}

	async function handleDeleteMod() {
		if (!profile || !modToDelete) return;
		deleteModDialogOpen = false;
		try {
			await controller.removeMod(profile, modToDelete);
		} catch (error) {
			showError(error, 'Remove mod');
		} finally {
			modToDelete = null;
		}
	}

	function cancelDeleteMod() {
		deleteModDialogOpen = false;
		modToDelete = null;
	}

	function applyInstantUpdate(updatedMods: Array<{ modId: string; version: string }>) {
		if (!profile) return;
		const nextByModId = new Map(updatedMods.map((mod) => [mod.modId, mod.version]));

		queryClient.setQueryData<Profile[]>(profilesQueryKey, (current) => {
			if (!current) return current;
			return current.map((entry) => {
				if (entry.id !== profile.id) return entry;
				return {
					...entry,
					mods: entry.mods.map((mod) => {
						const nextVersion = nextByModId.get(mod.mod_id);
						return nextVersion ? { ...mod, version: nextVersion } : mod;
					})
				};
			});
		});

		queryClient.setQueryData<UnifiedMod[]>(profileUnifiedModsKey(profile.id), (current) => {
			if (!current) return current;
			return current.map((mod) => {
				if (mod.source !== 'managed') return mod;
				const nextVersion = nextByModId.get(mod.mod_id);
				return nextVersion ? { ...mod, version: nextVersion } : mod;
			});
		});

		queryClient.setQueryData<ProfileModUpdatesMap>(modUpdatesQueryKey, (current) => {
			if (!current) return current;
			const next = { ...current };
			for (const mod of updatedMods) {
				const status = next[mod.modId];
				if (!status) continue;
				next[mod.modId] = {
					...status,
					installedVersion: mod.version,
					latestVersion: mod.version,
					isOutdated: false,
					status: 'ready'
				};
			}
			return next;
		});
	}

	async function handleRefreshUpdates() {
		if (!managedModsForUpdates.length) return;
		await modUpdatesQuery.refetch();
	}

	async function handleUpdateOne(modId: string) {
		if (!profile) return;
		const status = modUpdateStatuses[modId];
		if (!status?.isOutdated || !status.latestVersion) return;

		updatingModIds.add(modId);
		try {
			await installMods.mutateAsync({
				profileId: profile.id,
				profilePath: profile.path,
				mods: [{ modId, version: status.latestVersion }]
			});
			applyInstantUpdate([{ modId, version: status.latestVersion }]);
			showSuccess(`Updated ${modsMap.get(modId)?.name ?? modId}`);
			void modUpdatesQuery.refetch();
		} catch (error) {
			showError(error, 'Update mod');
		} finally {
			updatingModIds.delete(modId);
		}
	}

	async function handleUpdateAll() {
		if (!profile || updatesAvailableCount === 0) return;

		const modsToUpdate = managedModsForUpdates
			.map((mod) => {
				const status = modUpdateStatuses[mod.modId];
				return status?.isOutdated && status.latestVersion
					? { modId: mod.modId, version: status.latestVersion }
					: null;
			})
			.filter((mod): mod is { modId: string; version: string } => mod !== null);
		if (modsToUpdate.length === 0) return;

		isUpdatingAll = true;
		updatingModIds.clear();
		for (const mod of modsToUpdate) {
			updatingModIds.add(mod.modId);
		}
		try {
			await installMods.mutateAsync({
				profileId: profile.id,
				profilePath: profile.path,
				mods: modsToUpdate
			});
			applyInstantUpdate(modsToUpdate);
			showSuccess(`Updated ${modsToUpdate.length} mod${modsToUpdate.length === 1 ? '' : 's'}`);
			void modUpdatesQuery.refetch();
		} catch (error) {
			showError(error, 'Update all mods');
		} finally {
			isUpdatingAll = false;
			updatingModIds.clear();
		}
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
		<Button href="/library">
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>
	</div>
{:else}
	<div class="px-10 py-8">
		<Button variant="ghost" size="sm" class="mb-4" href="/library">
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Library
		</Button>

		<ProfileHeroSection
			{profile}
			iconSrc={profileIconSrc}
			{isRunning}
			{runningInstanceCount}
			{allowMultiInstanceLaunch}
			{lastLaunched}
			totalPlayTimeLabel={formatPlayTime(totalPlayTime)}
			{isDisabled}
			{isLaunchDisabled}
			{isLaunching}
			onLaunch={handleLaunch}
			onOpenFolder={() => controller.openProfileFolder(profile)}
			onExport={handleExportProfile}
			onOpenIconEditor={openIconDialog}
			onOpenRename={openRenameDialog}
			onOpenDelete={() => (deleteDialogOpen = true)}
		/>

		<hr class="my-5 border-t border-muted-foreground/20" />

		<div class="rounded-lg bg-white/3 p-4">
			<ProfileModsToolbar
				bind:searchInput
				{searchPlaceholder}
				{updatesAvailableCount}
				{isCheckingUpdates}
				{isUpdatingAll}
				onInstallMods={controller.goToInstallMods}
				onRefreshUpdates={handleRefreshUpdates}
				onUpdateAll={handleUpdateAll}
			/>
			<ProfileModsList
				isPending={unifiedModsQuery.isPending}
				{displayedMods}
				{isSearching}
				{profile}
				{modsMap}
				{isDisabled}
				{modUpdateStatuses}
				{updatingModIds}
				{isUpdatingAll}
				showPagination={pagination.showPagination}
				{currentPage}
				totalPages={pagination.totalPages}
				hasNextPage={pagination.hasNextPage}
				onClearSearch={() => (searchInput = '')}
				onInstallMods={controller.goToInstallMods}
				onDeleteMod={confirmDeleteMod}
				onUpdateMod={handleUpdateOne}
				onPrevPage={() => currentPage--}
				onNextPage={() => currentPage++}
			/>
		</div>
		<ProfileLogViewer {profile} {isRunning} />
	</div>

	<Dialog.Root bind:open={iconDialogOpen}>
		<Dialog.Content>
			<Dialog.Header>
				<Dialog.Title>Edit Profile Icon</Dialog.Title>
				<Dialog.Description>
					Use the default cube, set a custom image, or use an installed mod icon.
				</Dialog.Description>
			</Dialog.Header>

			<div class="space-y-4 py-3">
				<div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
					<Button
						type="button"
						variant={iconModeDraft === 'default' ? 'default' : 'outline'}
						onclick={() => setIconMode('default')}
					>
						Default
					</Button>
					<Button
						type="button"
						variant={iconModeDraft === 'custom' ? 'default' : 'outline'}
						onclick={() => setIconMode('custom')}
					>
						Custom Image
					</Button>
					<Button
						type="button"
						variant={iconModeDraft === 'mod' ? 'default' : 'outline'}
						onclick={() => setIconMode('mod')}
					>
						Installed Mod
					</Button>
				</div>

				{#if iconModeDraft === 'custom'}
					<input
						bind:this={customIconInput}
						type="file"
						accept="image/*"
						class="hidden"
						onchange={handleCustomIconInput}
					/>

					<div class="space-y-3">
						<div class="flex flex-wrap items-center gap-3">
							<div
								class="flex h-16 w-16 items-center justify-center overflow-hidden rounded-lg bg-muted/30"
							>
								{#if customIconDataUrlDraft}
									<img
										src={customIconDataUrlDraft}
										alt="Custom icon preview"
										class="h-full w-full object-cover"
									/>
								{:else}
									<Package class="h-7 w-7 text-muted-foreground/40" />
								{/if}
							</div>
							<Button
								type="button"
								variant="outline"
								disabled={isReadingIconFile}
								onclick={() => customIconInput?.click()}
							>
								{isReadingIconFile
									? 'Reading...'
									: customIconDataUrlDraft
										? 'Change Image'
										: 'Choose Image'}
							</Button>
							{#if customIconDataUrlDraft}
								<Button type="button" variant="ghost" onclick={() => (customIconDataUrlDraft = '')}>
									Clear
								</Button>
							{/if}
						</div>
						<p class="text-xs text-muted-foreground">PNG/JPG/GIF/WebP up to 500 KB.</p>
					</div>
				{:else if iconModeDraft === 'mod'}
					{#if installedModsWithIcons.length === 0}
						<p class="text-sm text-muted-foreground">
							No installed managed mods with icons are available for this profile.
						</p>
					{:else}
						<div class="space-y-3">
							<label class="text-sm font-medium" for="profile-icon-mod">Installed mod icon</label>
							<select
								id="profile-icon-mod"
								class="h-10 w-full rounded-md border bg-background px-3 text-sm"
								bind:value={iconModIdDraft}
							>
								{#each installedModsWithIcons as mod (mod.id)}
									<option value={mod.id}>{mod.name}</option>
								{/each}
							</select>
							{#if selectedIconMod}
								<div class="flex items-center gap-3 rounded-md border bg-muted/20 p-2">
									<img
										src={selectedIconMod.thumbnail}
										alt={`${selectedIconMod.name} icon`}
										class="h-12 w-12 rounded object-cover"
									/>
									<p class="text-sm text-muted-foreground">{selectedIconMod.name}</p>
								</div>
							{/if}
						</div>
					{/if}
				{/if}

				{#if iconError}
					<p class="text-sm text-destructive">{iconError}</p>
				{/if}
			</div>

			<Dialog.Footer>
				<Button variant="outline" onclick={() => (iconDialogOpen = false)}>Cancel</Button>
				<Button
					onclick={handleSaveProfileIcon}
					disabled={updateProfileIcon.isPending ||
						isReadingIconFile ||
						(iconModeDraft === 'mod' && installedModsWithIcons.length === 0)}
				>
					{#if updateProfileIcon.isPending}
						Saving...
					{:else}
						Save Icon
					{/if}
				</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>

	<ProfileDialogs
		{profile}
		bind:deleteDialogOpen
		bind:renameDialogOpen
		bind:deleteModDialogOpen
		bind:newProfileName
		{renameError}
		renamePending={renameProfile.isPending}
		onNewProfileNameInput={(event) =>
			(newProfileName = (event.currentTarget as HTMLInputElement).value)}
		onCancelDeleteProfile={() => (deleteDialogOpen = false)}
		onConfirmDeleteProfile={handleDeleteProfile}
		onCancelRename={() => (renameDialogOpen = false)}
		onConfirmRename={handleRenameProfile}
		onCancelDeleteMod={cancelDeleteMod}
		onConfirmDeleteMod={handleDeleteMod}
	/>
{/if}
